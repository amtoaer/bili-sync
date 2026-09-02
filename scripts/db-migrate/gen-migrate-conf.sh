#!/usr/bin/env bash
# gen-migrate-conf.sh — 动态生成 pgloader 迁移配置（表清单 + cast 规则），不手写清单
#
# 设计原则（单一事实来源）：目标库 schema 由 bili-sync 自己的 sea-orm 迁移生成，
# 是实体代码的镜像。表清单从目标库 pg_tables 推导、cast 规则从 information_schema
# 的 data_type 集合推导——维护者给实体加表/加列后，目标库（迁移自动建好）里就有，
# 重跑本脚本即得到新的迁移配置，无需修改任何"迁移配置清单"。
#
# 唯一需人工维护的点：SEAQ_ORM_TYPE_MAP（sea-query 的确定性类型映射）。
# 仅当未来 sea-query 支持新的列类型映射时才需更新（集中在下方常量处）。
# 目标库出现未映射的列类型时脚本直接报错退出（fail-closed），
# 不会带着 pgloader 的未知默认转换继续——保真优先。
#
# 用法：
#   gen-migrate-conf.sh <PGURL> <SQLITE_DB> <输出 .load 文件>

set -euo pipefail

PGURL="${1:?用法: gen-migrate-conf.sh <PGURL> <SQLITE_DB> <输出>}"
SQLITE_DB="${2:?缺 SQLITE_DB}"
OUT="${3:?缺输出文件}"
# FROM 必须写绝对路径：相对路径会被 pgloader 按 .load 文件所在目录解析（3.6.7 实测），
# sqlite3 驱动对不存在路径会自动建空库文件——「零错误零行数」假成功，靠 C5 对账才兜住。
# 存在性不能依赖 realpath 退出码：coreutils 9.10 对不存在的路径也返回 0（-m 语义），
# 需显式 -f 守卫
SQLITE_DB_ABS="$(realpath "$SQLITE_DB")"
[[ -f "$SQLITE_DB_ABS" ]] || { echo "错误: SQLite 库不存在: $SQLITE_DB_ABS" >&2; exit 1; }

# 从公共库加载 PGURL 结构切分（提取库名/服务器段/query 参数）
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib-pgurl.sh"
split_pgurl
# 密码改经 PGPASSWORD 传递：psql 的 argv 会被 `ps` 全程可见，一律用 PGURL_NOPASS；
# INTO 串（写进 .load 文件）保留完整 URL——pgloader 从文件读取、不经 argv，
# 且生成后文件会收紧为 0600
strip_pg_password
# 库名从连接串提取（必须带库名）
[[ -n "$PGURL_DB" ]] || { echo "错误: 连接串缺少库名（连接串需形如 postgres://user:pass@host:5432/db）" >&2; exit 1; }
TARGET_DB="$PGURL_DB"

# sea-query 0.32.7 确定性类型映射（PG data_type → SQLite 声明类型名）
# 依据 crates/bili_sync_migration 与 sea-query SQLite 后端渲染行为核对
# 注意 character varying 的 SQLite 声明类型是 varchar（sea-query SQLite 后端对
# ColumnType::String 渲染为 varchar）。映射错误会生成与源列类型不匹配的 cast 规则：
# 源库 varchar 列匹配不到任何规则（退回 pgloader 默认转换并刷 WARNING），
# 源库 text 列（config.data）则会同时匹配两条同名规则，走哪条无保证。
SEAQ_ORM_TYPE_MAP=(
    "timestamp without time zone|timestamp_text"
    "json|json_text"
    "jsonb|jsonb_text"
    "boolean|boolean"
    "bigint|bigint"
    "integer|integer"
    "smallint|smallint"
    "text|text"
    "character varying|varchar"
    "character|char"
)

# --- 1. 表清单：从目标库动态推导（排除 seaql_migrations） ------------------------
TABLES=$(psql "$PGURL_NOPASS" -Atc \
    "SELECT tablename FROM pg_tables WHERE schemaname='public' AND tablename<>'seaql_migrations' ORDER BY tablename;")
test -n "$TABLES" || { echo "错误: 目标库 $TARGET_DB 无业务表（请先用 bili-sync 启动建 schema）"; exit 1; }

# --- 2. cast 规则：从目标库 data_type 集合推导 ------------------------------------
TYPES=$(psql "$PGURL_NOPASS" -Atc \
    "SELECT DISTINCT data_type FROM information_schema.columns
     WHERE table_schema='public' AND table_name<>'seaql_migrations' ORDER BY 1;")

# pgloader 不接受带空格的目标类型名（实测 3.6.10），PG data_type → cast 目标名
PG_CAST_TYPE_MAP=(
    "timestamp without time zone|timestamp"
    "character varying|varchar"
    "character|varchar"
)

cast_rule() { # $1=PG data_type，输出 pgloader cast 行（未映射时输出空串，由调用方汇总后报错）
    local pg_type="$1" sqlite_type="" cast_type=""
    for pair in "${SEAQ_ORM_TYPE_MAP[@]}"; do
        if [[ "$pg_type" == "${pair%%|*}" ]]; then
            sqlite_type="${pair##*|}"
            break
        fi
    done
    for pair in "${PG_CAST_TYPE_MAP[@]}"; do
        if [[ "$pg_type" == "${pair%%|*}" ]]; then
            cast_type="${pair##*|}"
            break
        fi
    done
    cast_type="${cast_type:-$pg_type}"
    if [[ -n "$sqlite_type" ]]; then
        echo "    type $sqlite_type to $cast_type drop typemod"
    fi
}

# --- 3. 推导 cast 规则（先于写文件：未映射类型 fail-closed，不留半成品 .load） ------
rules=()
unmapped_types=()
while read -r t; do
    rule=$(cast_rule "$t")
    if [[ -n "$rule" ]]; then
        rules+=("$rule")
    else
        unmapped_types+=("$t")
    fi
done <<< "$TYPES"
# 未映射类型不允许带着未知转换继续：pgloader 会退回默认转换，可能静默改写值，
# 而它行级失败 exit 0、migrate.sh 行数对账也捕捉不到值级损坏——保真工具链必须失败关闭
if [[ ${#unmapped_types[@]} -gt 0 ]]; then
    echo "错误: 以下 PG 列类型在 SEAQ_ORM_TYPE_MAP 中无映射:" >&2
    printf '      %s\n' "${unmapped_types[@]}" >&2
    echo "      实体新增列类型时需同步更新 SEAQ_ORM_TYPE_MAP（本文件顶部）" >&2
    exit 1
fi

# --- 4. 生成 .load 文件 ------------------------------------------------------------
{
    # pgloader 的 INTO 需完整连接串（postgresql:// 空 URI 会解析失败）；
    # 不带 query 参数：pgloader 的 URI 参数白名单只有 sslmode/host/port/dbname/user/
    # password/tablename（uri-param 类参数作末参数还会吞掉 .load 后续内容），连接参数
    # 经 PGSSLMODE 环境变量传递（migrate.sh 导出，pgloader 的环境变量也只认这一个）
    INTO_URL="${PGURL/postgres:\/\//postgresql://}"
    INTO_URL="${INTO_URL%%\?*}"
    echo "LOAD DATABASE"
    echo "    FROM sqlite://$SQLITE_DB_ABS"
    echo "    INTO $INTO_URL"
    echo ""
    echo "WITH create no tables, reset sequences"
    echo ""
    echo "SET PostgreSQL PARAMETERS"
    echo "    maintenance_work_mem = '128MB'"
    echo ""
    echo "CAST"
    # pgloader 多条 cast 规则间需逗号分隔（最后一条后换行接分号）；
    # 逗号按「实际输出的规则数」计算（未映射类型已在上面拦截，不会留下悬空逗号）
    if [[ ${#rules[@]} -gt 0 ]]; then
        for ((idx = 0; idx < ${#rules[@]} - 1; idx++)); do
            echo "${rules[$idx]},"
        done
        echo "${rules[-1]}"
    fi
    echo ""
    # 排除迁移记录表（seaql_migrations 不随迁，目标库由 bili-sync 自己的迁移建）
    # 语法注意（pgloader 3.6 实测）：EXCLUDING 必须是 LOAD DATABASE 的子句（分号之前），
    # 关键字是 LIKE 且只接受一个 SQL LIKE 模式；sqlite_ 内部表由 pgloader 驱动自动跳过
    echo "EXCLUDING TABLE NAMES LIKE 'seaql_migrations'"
    echo ";"
} > "$OUT"
# .load 的 INTO 串含明文密码（pgloader 从文件读取、不经 argv），
# 重定向按 umask 生成 0644——收紧为仅所有者可读写
chmod 600 "$OUT"

echo "已生成迁移配置: $OUT"
echo "--- 表清单（$(echo "$TABLES" | wc -l) 张，自动推导）---"
echo "$TABLES"
echo "--- cast 规则（$(echo "$TYPES" | wc -l) 种类型，自动推导）---"
grep -E '^    type ' "$OUT" || true
