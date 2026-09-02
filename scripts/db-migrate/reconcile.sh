#!/usr/bin/env bash
# reconcile.sh — 迁移对账：迁移后 PG 库 vs SQLite 源库基线（阶段 C5）
#
# 对比项（全部动态推导，不写死表/列）：
#   行数 / 位域值域 / 时间戳（UTC 文本逐字符） / JSON 归一化 / 布尔计数 /
#   config md5 / UNIQUE 约束实测 / 序列对齐
#
# 用法：
#   reconcile.sh <SQLITE_DB> <PGURL> [基线目录，默认 SQLITE_DB 同目录 baseline]

set -euo pipefail

SQLITE_DB="${1:?用法: reconcile.sh <SQLITE_DB> <PGURL> [基线目录]}"
PGURL="${2:?缺 PGURL}"
BASE="${3:-$(dirname "$SQLITE_DB")/baseline}"

# 从公共库加载 PGURL 结构切分（提取库名/服务器段/query 参数）
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib-pgurl.sh"
split_pgurl
# 密码改经 PGPASSWORD 传递：psql 的 argv 会被 `ps` 全程可见，一律用 PGURL_NOPASS
strip_pg_password
[[ -n "$PGURL_DB" ]] || { echo "错误: 连接串缺少库名（连接串需形如 postgres://user:pass@host:5432/db）" >&2; exit 1; }
TARGET_DB="$PGURL_DB"

# sqlite3 连不存在路径会自动建空库文件，路径打错会把空库当作源来对账——必须先验存在
[[ -f "$SQLITE_DB" ]] || { echo "错误: SQLite 库不存在: $SQLITE_DB" >&2; exit 1; }
# jq 缺失时 4/8 的「jq ... || echo PARSE_ERR」会让两侧同时输出 PARSE_ERR 假通过——
# 必须显式前置检查，否则 JSON 对账形同虚设
command -v jq >/dev/null 2>&1 || { echo "错误: 未安装 jq（4/8 JSON 归一化对账需要）" >&2; exit 1; }

PASS=0; FAIL=0
ok()   { echo "  ✓ $1"; PASS=$((PASS+1)); }
bad()  { echo "  ✗ $1"; FAIL=$((FAIL+1)); }

# 业务表清单来自基线（snapshot-baseline.sh 已排除 seaql_migrations 与 SQLite 内部表），
# 实体加表后重跑 snapshot 即纳入对账，无需改脚本
[[ -f "$BASE/tables.txt" ]] || { echo "错误: 缺少基线 $BASE/tables.txt（先跑 snapshot-baseline.sh）" >&2; exit 1; }
mapfile -t TABLES < "$BASE/tables.txt"
[[ ${#TABLES[@]} -gt 0 ]] || { echo "错误: 基线表清单为空" >&2; exit 1; }

echo "=== 1/8 行数逐表对比（基线: $BASE/rowcounts.txt）==="
while read -r t n; do
    pg_n=$(psql "$PGURL_NOPASS" -Atc "SELECT count(*) FROM \"$t\";")
    if [[ "$n" == "$pg_n" ]]; then ok "$t: $n = $pg_n"; else bad "$t: 基线 $n != PG $pg_n"; fi
done < "$BASE/rowcounts.txt"

echo "=== 2/8 位域值域对比 ==="
for t in "${TABLES[@]}"; do
    sqlite3 "$SQLITE_DB" "PRAGMA table_info(\"$t\");" | grep -q '|download_status|' || continue
    # ORDER BY 必须显式写列名：ORDER BY 1（SELECT 序号）在标准 PG 里指拼接字符串，
    # 但部分 PG 兼容引擎解析成分组列——字符串序与数值序不同时（多位数位域值）
    # 会假报不一致（远程引擎实测踩过，page 恰好两种排序一致而 video 不一致）
    s=$(sqlite3 "$SQLITE_DB" "SELECT download_status || ':' || count(*) FROM \"$t\" GROUP BY download_status ORDER BY download_status;" | md5sum | cut -d' ' -f1)
    p=$(psql "$PGURL_NOPASS" -Atc "SELECT download_status || ':' || count(*) FROM \"$t\" GROUP BY download_status ORDER BY download_status;" | md5sum | cut -d' ' -f1)
    if [[ "$s" == "$p" ]]; then ok "$t 位域一致 ($s)"; else bad "$t 位域: SQLite[$s] != PG[$p]"; fi
done

echo "=== 3/8 时间戳全量对比（UTC 文本逐字符）==="
# 不用 epoch 换算：SQLite strftime('%s') 经 float64 舍入、PG EXTRACT(EPOCH)::bigint
# 四舍五入，亚秒 ≥0.5 的时间戳两侧都不准（会假报差异）；直接比较文本最可靠。
# 亚秒归一化：两侧 rtrim 尾部 0（'.500000' == '.5' 视为同一值）。
for t in "${TABLES[@]}"; do
    cols=$(sqlite3 "$SQLITE_DB" "PRAGMA table_info(\"$t\");" | awk -F'|' '$3 ~ /timestamp_text/ {print $2}')
    for c in $cols; do
        s=$(sqlite3 "$SQLITE_DB" "SELECT id || '=' || rtrim(strftime('%Y-%m-%d %H:%M:%S', \"$c\") || CASE WHEN instr(\"$c\",'.')>0 THEN substr(\"$c\",instr(\"$c\",'.')) ELSE '.0' END, '0') FROM \"$t\" ORDER BY id;" | md5sum | cut -d' ' -f1)
        p=$(psql "$PGURL_NOPASS" -Atc "SELECT id || '=' || rtrim(to_char(\"$c\", 'YYYY-MM-DD HH24:MI:SS.US'), '0') FROM \"$t\" ORDER BY id;" | md5sum | cut -d' ' -f1)
        if [[ "$s" == "$p" ]]; then ok "$t.$c 时间戳一致 ($s)"; else bad "$t.$c 时间戳: SQLite[$s] != PG[$p]"; fi
    done
done

echo "=== 4/8 JSON 归一化对比 ==="
for t in "${TABLES[@]}"; do
    cols=$(sqlite3 "$SQLITE_DB" "PRAGMA table_info(\"$t\");" | awk -F'|' '$3 ~ /json/ {print $2}')
    for c in $cols; do
        s=$(sqlite3 "$SQLITE_DB" "SELECT COALESCE(\"$c\",'NULL') FROM \"$t\" ORDER BY id;" | while read -r v; do
                [[ "$v" == "NULL" ]] && { echo NULL; continue; }
                echo "$v" | jq -c -S . 2>/dev/null || echo "PARSE_ERR"
            done | md5sum | cut -d' ' -f1)
        p=$(psql "$PGURL_NOPASS" -Atc "SELECT COALESCE(\"$c\"::text,'NULL') FROM \"$t\" ORDER BY id;" | while read -r v; do
                [[ "$v" == "NULL" ]] && { echo NULL; continue; }
                echo "$v" | jq -c -S . 2>/dev/null || echo "PARSE_ERR"
            done | md5sum | cut -d' ' -f1)
        if [[ "$s" == "$p" ]]; then ok "$t.$c JSON 归一化一致 ($s)"; else bad "$t.$c JSON: SQLite[$s] != PG[$p]"; fi
    done
done

echo "=== 5/8 布尔计数对比 ==="
# SQLite 布尔存 0/1，PG 侧 ::int 对齐；NULL 两侧统一归一化为 'null' 再排序
# （SQLite ORDER BY 空值在前、PG 空值在后，不归一化会假报差异）
for t in "${TABLES[@]}"; do
    cols=$(sqlite3 "$SQLITE_DB" "PRAGMA table_info(\"$t\");" | awk -F'|' '$3=="boolean" {print $2}')
    for c in $cols; do
        s=$(sqlite3 "$SQLITE_DB" "SELECT COALESCE(\"$c\",'null') || ':' || count(*) FROM \"$t\" GROUP BY \"$c\" ORDER BY 1;" | md5sum | cut -d' ' -f1)
        p=$(psql "$PGURL_NOPASS" -Atc "SELECT COALESCE(\"$c\"::int::text,'null') || ':' || count(*) FROM \"$t\" GROUP BY \"$c\" ORDER BY 1;" | md5sum | cut -d' ' -f1)
        if [[ "$s" == "$p" ]]; then ok "$t.$c 布尔分布一致 ($s)"; else bad "$t.$c 布尔: SQLite[$s] != PG[$p]"; fi
    done
done

echo "=== 6/8 config md5 对比（迁移时点基线）==="
# 不能用 COPY TO STDOUT：COPY 文本格式会把数据内的反斜杠/换行转义，
# config 含这些字节时会假报不一致；SELECT 原样输出（两侧各带一个行尾换行）
pg_md5=$(psql "$PGURL_NOPASS" -Atc "SELECT data FROM config WHERE id=1;" | md5sum | cut -d' ' -f1)
base_md5=$(cut -d' ' -f1 "$BASE/config.md5")
if [[ "$pg_md5" == "$base_md5" ]]; then ok "config md5 一致: $pg_md5"; else bad "config md5: 基线[$base_md5] != PG[$pg_md5]"; fi

echo "=== 7/8 UNIQUE 约束实测（复制一行插入应触发 idx_video_unique 冲突）==="
video_rows=$(psql "$PGURL_NOPASS" -Atc "SELECT count(*) FROM video;")
if [[ "$video_rows" == "0" ]]; then
    ok "video 为空，跳过 UNIQUE 冲突实测"
else
    # 列清单动态推导（实体加列后无需改脚本）；显式写 id = MAX(id)+1，
    # 不走序列默认值——序列非事务性，ROLLBACK 不还原 nextval，会扰动 8/8 的序列判读
    cols=$(psql "$PGURL_NOPASS" -Atc \
        "SELECT string_agg(column_name, ', ' ORDER BY ordinal_position) FROM information_schema.columns
         WHERE table_schema='public' AND table_name='video' AND column_name <> 'id';")
    err_file=$(mktemp)
    # UNIQUE 实测可能被中断（SIGINT/SIGTERM）而残留临时文件，EXIT trap 兜底清理
    trap '[[ -n "${err_file:-}" ]] && rm -f "$err_file"' EXIT
    if psql "$PGURL_NOPASS" -v ON_ERROR_STOP=1 -Atc \
        "BEGIN; INSERT INTO video (id, $cols) SELECT (SELECT COALESCE(MAX(id),0)+1 FROM video), $cols FROM video ORDER BY id LIMIT 1; ROLLBACK;" \
        > /dev/null 2>"$err_file"; then
        bad "插入重复 bvid 未报错（UNIQUE 索引失效？）"
    elif grep -q 'idx_video_unique' "$err_file"; then
        ok "插入重复 bvid 触发 idx_video_unique 冲突（COALESCE 索引生效）"
    else
        bad "插入失败但非 idx_video_unique 冲突: $(tail -1 "$err_file")"
    fi
    rm -f "$err_file"
fi

echo "=== 8/8 序列对齐（下一个分配值 == MAX(id)+1）==="
# pgloader reset sequences 的实测行为（3.6.10）：非空表 last_value=MAX(id)、
# is_called=true（对齐）；空表 last_value=1、is_called=true（跳 1 号），需手动 setval。
# 正确性判据：next = is_called ? last+1 : last，必须 == MAX(id)+1
for t in "${TABLES[@]}"; do
    seq=$(psql "$PGURL_NOPASS" -Atc "SELECT pg_get_serial_sequence('$t','id');")
    [[ -n "$seq" ]] || continue
    IFS='|' read -r last called < <(psql "$PGURL_NOPASS" -Atc "SELECT last_value, is_called FROM $seq;")
    maxid=$(psql "$PGURL_NOPASS" -Atc "SELECT COALESCE(MAX(id),0) FROM \"$t\";")
    # psql 中途失败时读到的值为空，裸算术会以语法错误收场——先防御再判读
    if [[ -z "$last" || -z "$called" || -z "$maxid" ]]; then
        bad "$t 序列读取失败（last=$last is_called=$called max=$maxid），无法判读"
        continue
    fi
    if [[ "$called" == "t" ]]; then next=$((last + 1)); else next=$last; fi
    if [[ "$next" == "$((maxid + 1))" ]]; then
        ok "$t 序列对齐 (next=$next, max=$maxid)"
    else
        bad "$t 序列未对齐 (last=$last is_called=$called, next=$next, max=$maxid) → 需手动 setval"
    fi
done

echo ""
echo "对账结果: PASS=$PASS FAIL=$FAIL"
[[ $FAIL -eq 0 ]] || exit 1
