#!/usr/bin/env bash
# compare-ddl.sh — DDL 层对比：迁移后库 vs 现有 PG 库（阶段 D1）
#
# 用途：判定迁移目标库的 schema 与 bili-sync 迁移产物完全一致。
# 方法：pg_dump --schema-only 两库后 diff。
#
# 说明：现有库（如 bili_sync）可能含迁移文件之外的手工测试残留表
# （如 tz_probe），对比前剔除并说明，不视为 DDL 差异。
#
# 用法：
#   compare-ddl.sh <PGURL> <库A> <库B> [--keep-workdir]

set -euo pipefail

PGURL="${1:?用法: compare-ddl.sh <PGURL> <库A> <库B>}"
DB_A="${2:?缺库A（基准，如 bili_sync）}"
DB_B="${3:?缺库B（迁移后，如 bili_migrate_test）}"
[[ "${4:-}" == "--keep-workdir" ]] && KEEP=1

# 从公共库加载 PGURL 结构切分（提取库名/服务器段/query 参数）
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib-pgurl.sh"
split_pgurl
# 密码改经 PGPASSWORD 传递：pg_dump 的 argv 会被 `ps` 全程可见，一律用 PGURL_NOPASS_SERVER
strip_pg_password

PASS=0; FAIL=0
ok()  { echo "  ✓ $1"; PASS=$((PASS+1)); }
bad() { echo "  ✗ $1"; FAIL=$((FAIL+1)); }

WORK="$(mktemp -d /tmp/ddl-cmp.XXXXXX)"
trap '[[ "${KEEP:-}" != 1 ]] && rm -rf "$WORK"' EXIT

strip_tz_probe() { # 剔除 tz_probe 建表段（遗留手工表，见 memory: 时区探针测试残留）
    awk '
        /^CREATE TABLE public\.tz_probe/ { skip=1 }
        skip && /^\);$/ { skip=0; print "-- [剔除] 手工残留表 tz_probe（不在迁移文件中，属测试遗留，非 DDL 差异）"; next }
        !skip { print }
    ' "$1" > "$2"
}

strip_dump_noise() { # $1=输入 $2=输出；剔除 pg_dump 17 每次随机生成的 restrict 令牌（与 schema 无关，直接 diff 会恒定误报）
    sed -e '/^\\restrict /d' -e '/^\\unrestrict /d' "$1" > "$2"
}

echo "=== pg_dump schema-only ==="
# 不吞 stderr：pg_dump 失败（如库名打错）时保留错误输出，set -e 中止也可见原因
pg_dump -s --no-owner "$PGURL_NOPASS_SERVER/$DB_A$PGURL_QUERY" > "$WORK/a.sql"
pg_dump -s --no-owner "$PGURL_NOPASS_SERVER/$DB_B$PGURL_QUERY" > "$WORK/b.sql"
echo "库A($DB_A) dump: $(wc -l < "$WORK/a.sql") 行；库B($DB_B) dump: $(wc -l < "$WORK/b.sql") 行"

# 库A 若有 tz_probe 则剔除（库B 由迁移生成，天然不含）；两库均剔除 dump 噪声
if grep -q 'CREATE TABLE public.tz_probe' "$WORK/a.sql"; then
    echo "检测到库A含 tz_probe 残留表，已剔除"
    strip_tz_probe "$WORK/a.sql" "$WORK/a.tmp.sql"
else
    cp "$WORK/a.sql" "$WORK/a.tmp.sql"
fi
strip_dump_noise "$WORK/a.tmp.sql" "$WORK/a.clean.sql"
strip_dump_noise "$WORK/b.sql" "$WORK/b.clean.sql"
A_FILE="$WORK/a.clean.sql"
B_FILE="$WORK/b.clean.sql"

echo "=== diff -u（空输出=完全一致）==="
if diff -u "$A_FILE" "$B_FILE" > "$WORK/diff.txt"; then
    ok "DDL 完全一致"
else
    bad "存在差异（见下）:"
    cat "$WORK/diff.txt"
fi

echo "=== 关键 DDL 抽查 ==="
echo "--- 库B idx_video_unique ---"
grep -A3 'idx_video_unique' "$B_FILE" | head -4
echo "--- 库B created_at 默认值 ---"
grep 'CURRENT_TIMESTAMP AT TIME ZONE' "$B_FILE" | head -2
echo "--- 库B favorite latest_row_at 默认值 ---"
grep "'1970-01-01 00:00:00'" "$B_FILE" | head -1

echo ""
echo "DDL 对比结果: PASS=$PASS FAIL=$FAIL"
# 与 reconcile.sh / compare-data.sh 对齐：存在差异必须以非零退出，
# 否则串联执行（A && B）或接入 CI 时 schema 漂移会静默通过
[[ $FAIL -eq 0 ]] || exit 1
