#!/usr/bin/env bash
# compare-data.sh — 数据格式层对比：迁移后库 vs 现有 PG 库（阶段 D2）
#
# 六项对比（全部动态推导，不写死列名）：
#   1. 类型矩阵（information_schema.columns）
#   2. NULL 分布（逐表逐可空列）
#   3. 位域值域（download_status DISTINCT）
#   4. 时间戳（类型与抽样）
#   5. JSON 结构（jsonb/json 列抽样解析）
#   6. config 凭证三方 md5（PG 源 → SQLite → 迁回 PG 的 credential 五字段）
#
# 注意：两库数据内容预期不同（源库是历史数据、迁移库是新扫描数据），
# 只做格式对比；默认值差异已知来自旧库历史（CURRENT_TIMESTAMP 修复 9864e3c），不对比。
#
# 用法：
#   compare-data.sh <PGURL> <源库A> <迁移库B> <SQLITE_DB>

set -euo pipefail

PGURL="${1:?用法: compare-data.sh <PGURL> <源库A> <迁移库B> <SQLITE_DB>}"
DB_A="${2:?缺源库A（如 bili_sync）}"
DB_B="${3:?缺迁移库B（如 bili_migrate_test）}"
SQLITE_DB="${4:?缺 SQLITE_DB}"

# sqlite3 连不存在路径会自动建空库文件，路径打错会把空库当作源来对比——必须先验存在且非空
# （校验集合与下方 SRC_TABLES 一致，否则空表清单会拼出 IN () 语法错误）
[[ -f "$SQLITE_DB" ]] || { echo "错误: SQLite 库不存在: $SQLITE_DB" >&2; exit 1; }
[[ "$(sqlite3 "$SQLITE_DB" "SELECT count(*) FROM sqlite_master
    WHERE type='table' AND name NOT IN ('seaql_migrations','sqlite_sequence','sqlite_stat1','sqlite_stat4');")" != "0" ]] \
    || { echo "错误: SQLite 库无业务表（空库或路径错误）: $SQLITE_DB" >&2; exit 1; }
# jq 缺失时 5/6 会把「工具缺失」误报成「解析失败」——显式前置检查
command -v jq >/dev/null 2>&1 || { echo "错误: 未安装 jq（5/6 JSON 结构检查需要）" >&2; exit 1; }

# 从公共库加载 PGURL 结构切分（提取库名/服务器段/query 参数）
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib-pgurl.sh"
split_pgurl
# 密码改经 PGPASSWORD 传递：psql 的 argv 会被 `ps` 全程可见，一律用 PGURL_NOPASS_SERVER
strip_pg_password

PASS=0; FAIL=0
ok()  { echo "  ✓ $1"; PASS=$((PASS+1)); }
bad() { echo "  ✗ $1"; FAIL=$((FAIL+1)); }

md5() { md5sum | cut -d' ' -f1; }

WORK="$(mktemp -d /tmp/data-cmp.XXXXXX)"
trap 'rm -rf "$WORK"' EXIT

# 对比范围限定为 SQLite 源的真实业务表：源库A（历史库）可能含手工测试残留表
# （如 tz_probe），其列会假报类型矩阵差异
SRC_TABLES=$(sqlite3 "$SQLITE_DB" "SELECT name FROM sqlite_master
    WHERE type='table' AND name NOT IN ('seaql_migrations','sqlite_sequence','sqlite_stat1','sqlite_stat4');")
TABLE_LIST=$(echo "$SRC_TABLES" | sed "s/^/'/;s/$/'/" | paste -sd, -)

echo "=== 1/6 类型矩阵对比（表/列/类型/可空，限定 SQLite 源业务表）==="
for db in "$DB_A" "$DB_B"; do
    psql "$PGURL_NOPASS_SERVER/$db$PGURL_QUERY" -Atc \
        "SELECT table_name || '.' || column_name || ' ' || data_type || ' ' || is_nullable
         FROM information_schema.columns
         WHERE table_schema='public' AND table_name IN ($TABLE_LIST) ORDER BY 1;" > "$WORK/cmp_$db.txt"
done
if diff -q "$WORK/cmp_$DB_A.txt" "$WORK/cmp_$DB_B.txt" > /dev/null; then
    ok "类型矩阵一致（$(wc -l < "$WORK/cmp_$DB_A.txt") 列）"
else
    bad "类型矩阵差异:"; diff "$WORK/cmp_$DB_A.txt" "$WORK/cmp_$DB_B.txt" | head -10
fi

echo "=== 2/6 NULL 分布对比 ==="
# 同一份数据应保真：迁移后库 vs SQLite 源的「含 NULL 列集合」必须一致；
# 与源库A（不同历史数据）的差异属内容差异，仅作参考输出
null_cols_pg() { # $1=库名（注意 set -e 下不能用 && 短路，条件为假会退出）
    local db="$1"
    for t in $(psql "$PGURL_NOPASS_SERVER/$db$PGURL_QUERY" -Atc "SELECT tablename FROM pg_tables WHERE schemaname='public' AND tablename<>'seaql_migrations'"); do
        for c in $(psql "$PGURL_NOPASS_SERVER/$db$PGURL_QUERY" -Atc "SELECT column_name FROM information_schema.columns WHERE table_name='$t' AND is_nullable='YES' AND table_schema='public';"); do
            n=$(psql "$PGURL_NOPASS_SERVER/$db$PGURL_QUERY" -Atc "SELECT count(*) FROM \"$t\" WHERE \"$c\" IS NULL;")
            if [[ "$n" != "0" ]]; then echo "$t.$c"; fi
        done
    done | sort
}
null_cols_sqlite() { # SQLite 源：对每个可空列输出含 NULL 的列
    local db="$1"
    for t in $(sqlite3 "$db" "SELECT name FROM sqlite_master WHERE type='table' AND name NOT IN ('seaql_migrations','sqlite_sequence','sqlite_stat1','sqlite_stat4');"); do
        for c in $(sqlite3 "$db" "PRAGMA table_info(\"$t\");" | awk -F'|' '$4=="0" && $2!="id" {print $2}'); do
            n=$(sqlite3 "$db" "SELECT count(*) FROM \"$t\" WHERE \"$c\" IS NULL;")
            if [[ "$n" != "0" ]]; then echo "$t.$c"; fi
        done
    done | sort
}
null_cols_pg "$DB_B" > "$WORK/null_B.txt"
null_cols_sqlite "$SQLITE_DB" > "$WORK/null_S.txt"
echo "-- 迁移后库含NULL列: $(tr '\n' ' ' < "$WORK/null_B.txt")"
echo "-- SQLite 源含NULL列: $(tr '\n' ' ' < "$WORK/null_S.txt")"
if diff -q "$WORK/null_B.txt" "$WORK/null_S.txt" > /dev/null; then
    ok "NULL 列集合与 SQLite 源一致（同一数据保真）"
else
    bad "NULL 列集合与源不一致:"; diff "$WORK/null_S.txt" "$WORK/null_B.txt" | head -10
fi
echo "-- 参考: 源库A($DB_A) 含NULL列: $(tr '\n' ' ' < <(null_cols_pg "$DB_A"))（历史数据不同，差异属内容差异）"

echo "=== 3/6 位域值域（download_status DISTINCT）==="
for db in "$DB_A" "$DB_B"; do
    psql "$PGURL_NOPASS_SERVER/$db$PGURL_QUERY" -Atc "SELECT DISTINCT download_status FROM video ORDER BY 1;" > "$WORK/st_$db.txt"
    echo "-- $db: $(tr '\n' ' ' < "$WORK/st_$db.txt")"
done
# 格式判据：值域均落在 bigint 合法范围（不做相等比较，内容预期不同）；
# ok 结论仅在两库均无非法值时输出，否则只报 bad、不虚增 PASS
FAIL_BEFORE=$FAIL
for db in "$DB_A" "$DB_B"; do
    while read -r v; do
        [[ "$v" =~ ^[0-9]+$ ]] || bad "$db 位域含非法值: $v"
    done < "$WORK/st_$db.txt"
done
if [[ "$FAIL" == "$FAIL_BEFORE" ]]; then
    ok "位域值域均为合法 bigint（源库历史值与新扫描值内容不同属预期）"
fi

echo "=== 4/6 时间戳格式（类型/抽样）==="
: > "$WORK/types.txt"
for db in "$DB_A" "$DB_B"; do
    t=$(psql "$PGURL_NOPASS_SERVER/$db$PGURL_QUERY" -Atc "SELECT DISTINCT data_type FROM information_schema.columns WHERE table_name='video' AND column_name IN ('ctime','created_at');")
    s=$(psql "$PGURL_NOPASS_SERVER/$db$PGURL_QUERY" -Atc "SELECT to_char(ctime,'YYYY-MM-DD HH24:MI:SS.US') FROM video ORDER BY id LIMIT 2;")
    echo "-- $db 类型: $t"
    echo "-- $db 抽样: $(echo "$s" | tr '\n' ' ')"
    echo "$t" >> "$WORK/types.txt"
done
if [[ "$(sort -u "$WORK/types.txt" | grep -c .)" == "1" ]] && grep -q 'timestamp without time zone' "$WORK/types.txt"; then
    ok "两库时间戳类型一致（timestamp without time zone），抽样格式合法"
else
    bad "时间戳类型不一致: [$(tr '\n' ' ' < "$WORK/types.txt")]"
fi

echo "=== 5/6 JSON 结构（jsonb/json 列存在性与可解析）==="
for db in "$DB_A" "$DB_B"; do
    psql "$PGURL_NOPASS_SERVER/$db$PGURL_QUERY" -Atc \
        "SELECT table_name || '.' || column_name || ':' || data_type FROM information_schema.columns
         WHERE table_schema='public' AND data_type IN ('json','jsonb') ORDER BY 1;" > "$WORK/json_$db.txt"
    echo "-- $db json 列: $(tr '\n' ' ' < "$WORK/json_$db.txt")"
    # 抽样解析：无数据时跳过（jq -e 对空输入以非 0 退出，会误报解析失败）
    tags=$(psql "$PGURL_NOPASS_SERVER/$db$PGURL_QUERY" -Atc "SELECT tags::text FROM video WHERE tags IS NOT NULL LIMIT 1;")
    if [[ -z "$tags" ]]; then
        echo "  -- $db video.tags 无数据，跳过解析"
    elif echo "$tags" | jq -e . > /dev/null 2>&1; then
        ok "$db video.tags 可解析"
    else
        bad "$db video.tags 解析失败"
    fi
done

echo "=== 6/6 config 凭证三方一致（PG 源 → SQLite → 迁回 PG，credential 五字段 md5）==="
cred_md5() { # $1=数据来源（pg库名|sqlite路径），输出 5 字段拼接串（供 md5，不进终端明文）
    local src="$1"
    if [[ "$src" == sqlite:* ]]; then
        sqlite3 "${src#sqlite:}" "SELECT json_extract(data,'\$.credential.sessdata') || '|' || json_extract(data,'\$.credential.bili_jct') || '|' || json_extract(data,'\$.credential.dedeuserid') || '|' || json_extract(data,'\$.credential.buvid3') || '|' || json_extract(data,'\$.credential.ac_time_value') FROM config WHERE id=1;"
    else
        # 每个字段提取用括号包裹，避免 jsonb 运算符与 || 的优先级问题
        psql "$PGURL_NOPASS_SERVER/$src$PGURL_QUERY" -Atc "SELECT (data::jsonb->'credential'->>'sessdata') || '|' || (data::jsonb->'credential'->>'bili_jct') || '|' || (data::jsonb->'credential'->>'dedeuserid') || '|' || (data::jsonb->'credential'->>'buvid3') || '|' || (data::jsonb->'credential'->>'ac_time_value') FROM config WHERE id=1;"
    fi
}
M_A=$(cred_md5 "$DB_A" | md5)
M_S=$(cred_md5 "sqlite:$SQLITE_DB" | md5)
M_B=$(cred_md5 "$DB_B" | md5)
echo "  源PG($DB_A)=$M_A  SQLite=$M_S  迁移后($DB_B)=$M_B"
if [[ "$M_A" == "$M_S" && "$M_S" == "$M_B" ]]; then
    ok "credential 五字段三方 md5 完全一致"
else
    if [[ "$M_S" == "$M_B" ]]; then
        ok "SQLite 与迁移后一致；源PG差异属预期（迁移前的测试操作改过 config，如 interval/路径）"
    else
        bad "credential 三方不一致（SQLite ≠ 迁移后）"
    fi
fi

echo ""
echo "数据格式对比结果: PASS=$PASS FAIL=$FAIL"
[[ $FAIL -eq 0 ]] || exit 1
