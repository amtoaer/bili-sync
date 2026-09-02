#!/usr/bin/env bash
# snapshot-baseline.sh — 导出 SQLite 库形态基线（迁移前冻结 + 迁移后对账的对照基准）
#
# 用途：SQLite→PG 迁移实测验证的阶段 B（B2）。
#   在迁移源库冻结（实例停止 + WAL checkpoint）后运行，导出：
#     表清单 / 行数 / 完整 DDL / 列信息 / 位域值域 / 时间戳抽样 /
#     JSON 抽样 / NULL 分布 / 布尔分布 / config md5 / integrity_check
#   全部动态推导（遍历 sqlite_master / PRAGMA），不写死表名列名——
#   维护者加表加列后重跑即得到新基线，无需改脚本。
#
# 用法：
#   snapshot-baseline.sh <SQLITE_DB> <输出目录>
#
# 前置条件：迁移源库已冻结（bili-sync 已停止、WAL 已 checkpoint）。

set -euo pipefail

SQLITE_DB="${1:?用法: snapshot-baseline.sh <SQLITE_DB> <输出目录>}"
OUT="${2:?缺输出目录}"
# sqlite3 连不存在路径会自动建空库文件，路径打错会导出空基线——必须先验存在
[[ -f "$SQLITE_DB" ]] || { echo "错误: SQLite 库不存在: $SQLITE_DB" >&2; exit 1; }
# sqlite3 连不存在路径会自动建空库文件，路径打错会导出空基线——必须先验非空
# （校验集合与下方 tables() 一致：排除 seaql_migrations 与 SQLite 内部表）
[[ "$(sqlite3 "$SQLITE_DB" "SELECT count(*) FROM sqlite_master
    WHERE type='table' AND name NOT IN ('seaql_migrations','sqlite_sequence','sqlite_stat1','sqlite_stat4');")" != "0" ]] \
    || { echo "错误: SQLite 库无业务表（空库或路径错误）: $SQLITE_DB" >&2; exit 1; }
mkdir -p "$OUT"

# 业务表清单（排除 seaql_migrations 与 SQLite 内部表）
tables() {
    sqlite3 "$SQLITE_DB" "SELECT name FROM sqlite_master
        WHERE type='table' AND name NOT IN ('seaql_migrations','sqlite_sequence','sqlite_stat1','sqlite_stat4')
        ORDER BY name;"
}

echo "=== 1/9 完整性检查 ==="
sqlite3 "$SQLITE_DB" "PRAGMA integrity_check;" > "$OUT/integrity.txt"
cat "$OUT/integrity.txt"
# 完整性检查必须每一行都是 ok（大库可能输出多行）——损坏库导出基线毫无意义，
# 下游对账全链路都会基于垃圾数据
if grep -qvx 'ok' "$OUT/integrity.txt" || [[ ! -s "$OUT/integrity.txt" ]]; then
    echo "错误: SQLite 完整性检查失败（PRAGMA integrity_check 非 ok）: $(tr '\n' ' ' < "$OUT/integrity.txt")" >&2
    exit 1
fi

echo "=== 2/9 表清单 ==="
tables > "$OUT/tables.txt"
cat "$OUT/tables.txt"
echo "迁移记录数: $(sqlite3 "$SQLITE_DB" "SELECT count(*) FROM seaql_migrations;")"

echo "=== 3/9 行数 ==="
: > "$OUT/rowcounts.txt"
while read -r t; do
    printf '%-16s %s\n' "$t" "$(sqlite3 "$SQLITE_DB" "SELECT count(*) FROM \"$t\";")" >> "$OUT/rowcounts.txt"
done < "$OUT/tables.txt"
cat "$OUT/rowcounts.txt"

echo "=== 4/9 完整 DDL ==="
sqlite3 "$SQLITE_DB" ".schema" > "$OUT/schema.sql"
echo "已导出 schema.sql ($(wc -l < "$OUT/schema.sql") 行)"

echo "=== 5/9 列信息（PRAGMA table_info）==="
: > "$OUT/table_info.txt"
while read -r t; do
    {
        echo "--- $t ---"
        sqlite3 "$SQLITE_DB" "PRAGMA table_info(\"$t\");"
    } >> "$OUT/table_info.txt"
done < "$OUT/tables.txt"
grep -c '^---' "$OUT/table_info.txt" | xargs echo "共导出表信息份数:"

echo "=== 6/9 位域值域（download_status 列的表）==="
: > "$OUT/status_groups.txt"
for t in $(tables); do
    if sqlite3 "$SQLITE_DB" "PRAGMA table_info(\"$t\");" | grep -q '|download_status|'; then
        echo "--- $t ---" >> "$OUT/status_groups.txt"
        sqlite3 "$SQLITE_DB" "SELECT download_status, count(*) FROM \"$t\" GROUP BY download_status ORDER BY 1;" >> "$OUT/status_groups.txt"
    fi
done
cat "$OUT/status_groups.txt"

echo "=== 7/9 时间戳与 JSON 抽样（timestamp_text/json_text/jsonb_text 列）==="
: > "$OUT/sample.txt"
for t in $(tables); do
    cols=$(sqlite3 "$SQLITE_DB" "PRAGMA table_info(\"$t\");" | awk -F'|' '$3 ~ /timestamp_text|json_text|jsonb_text/ {print $2}')
    for c in $cols; do
        echo "--- $t.$c ---" >> "$OUT/sample.txt"
        sqlite3 "$SQLITE_DB" "SELECT substr(\"$c\",1,60) FROM \"$t\" LIMIT 3;" >> "$OUT/sample.txt"
    done
done
cat "$OUT/sample.txt"

echo "=== 8/9 NULL 与布尔分布（可空/boolean 列）==="
: > "$OUT/null_bool.txt"
for t in $(tables); do
    sqlite3 "$SQLITE_DB" "PRAGMA table_info(\"$t\");" | awk -F'|' 'BEGIN{printf "--- %s ---\n", "'"$t"'"} $3=="boolean"{printf "BOOL %s\n", $2} $4=="0"{printf "NULLABLE %s\n", $2}' >> "$OUT/null_bool.txt"
done
# 对每个可空列统计 NULL 数、对每个 boolean 列统计 0/1 分布
: > "$OUT/null_counts.txt"
for t in $(tables); do
    while read -r c; do
        n=$(sqlite3 "$SQLITE_DB" "SELECT count(*) FROM \"$t\" WHERE \"$c\" IS NULL;")
        echo "$t.$c NULL=$n" >> "$OUT/null_counts.txt"
    done < <(sqlite3 "$SQLITE_DB" "PRAGMA table_info(\"$t\");" | awk -F'|' '$4=="0" && $2!="id" {print $2}')
    while read -r c; do
        sqlite3 "$SQLITE_DB" "SELECT \"$c\", count(*) FROM \"$t\" GROUP BY \"$c\" ORDER BY 1;" | sed "s/^/$t.$c = /" >> "$OUT/null_bool.txt"
    done < <(sqlite3 "$SQLITE_DB" "PRAGMA table_info(\"$t\");" | awk -F'|' '$3=="boolean" {print $2}')
done
cat "$OUT/null_counts.txt"

echo "=== 9/9 config md5 ==="
sqlite3 "$SQLITE_DB" "SELECT data FROM config WHERE id=1;" | md5sum > "$OUT/config.md5"
cat "$OUT/config.md5"

echo "基线导出完成 → $OUT"
