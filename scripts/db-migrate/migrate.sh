#!/usr/bin/env bash
# migrate.sh — SQLite→PostgreSQL data-only 迁移（外部工具实测）
#
# 流程（对应计划阶段 C1-C5）：
#   1. 校验/准备目标库：不 DROP 数据库，目标库需为空（无表）或加 --force 清空库内表；
#      库不存在时默认报错退出（自动建库需 --create-db 显式开启）
#   2. 用 bili-sync 启动一次让 sea-orm 迁移建 schema（PG 视为全新安装，全量 Migrator::up）
#   3. TRUNCATE 业务表 RESTART IDENTITY（清掉 VersionedConfig::init 自动插入的默认 config 行）
#   4. gen-migrate-conf.sh 动态生成 pgloader 配置（表清单/cast 全推导，不手写）并执行导入
#   5. 序列对齐 + 行数对账（pgloader 行级错误不反映到退出码，必须显式验证）
#
# 用法：
#   migrate.sh <PGURL> <SQLITE_DB> <BILI_SYNC_BIN> [--config-dir <dir>] [--force] [--create-db]
#
# PGURL 为完整连接串（含库名与可选 query 参数），目标库名从连接串提取。
# 导入工具固定为 pgloader（唯一实测验证过的工具）。
# 安全约定：脚本不 DROP 数据库，目标库由用户提供；已有表时默认拒绝清库
# （防止库名打错清掉生产库），确认目标库可清空后加 --force（只清库内表，
# 数据库本体与其他对象永不被删除）。目标库不存在时同样默认不自动创建
# （防止库名打错把数据迁进新建的错误库），自动建库需 --create-db（需 CREATEDB 权限）。
#
# 环境变量：PGURL、SQLITE_DB、BILI_SYNC_BIN 亦可通过环境变量传入。

set -euo pipefail

# 可选参数：--config-dir、--force、--create-db（也支持环境变量 CONFIG_DIR）
FORCE=0
CREATE_DB=0
# 自建的临时 config 目录单独记录：--config-dir 会在参数解析时覆盖 CONFIG_DIR，
# 若只记在 CONFIG_DIR 上，覆盖后 trap 就找不到待删的临时目录（泄漏空目录）
TMP_CONFIG_DIR=""
if [[ -z "${CONFIG_DIR:-}" ]]; then
    TMP_CONFIG_DIR="$(mktemp -d)"
    CONFIG_DIR="$TMP_CONFIG_DIR"
fi
# 清理 trap 在临时目录创建后立即安装：参数解析/依赖校验存在多条提前退出路径，
# 装晚了会让自建的临时目录在报错退出时泄漏（TMP_DIR 晚于此创建，用判空守卫）
trap 'if [[ -n "${TMP_CONFIG_DIR:-}" ]]; then rm -rf "$TMP_CONFIG_DIR"; fi; if [[ -n "${TMP_DIR:-}" ]]; then rm -rf "$TMP_DIR"; fi' EXIT

if [[ "${1:-}" == --* ]]; then
    # 全部走环境变量 + 可选 flag 的调用形式
    PGURL="${PGURL:?缺 PGURL（环境变量或位置参数）}"
    SQLITE_DB="${SQLITE_DB:?缺 SQLITE_DB}"
    BIN="${BILI_SYNC_BIN:?缺 bili-sync 二进制}"
else
    PGURL="${1:-${PGURL:?用法: migrate.sh <PGURL> <SQLITE_DB> <BILI_SYNC_BIN> [--config-dir <dir>] [--force] [--create-db]}}"
    SQLITE_DB="${2:-${SQLITE_DB:?缺 SQLITE_DB}}"
    BIN="${3:-${BILI_SYNC_BIN:?缺 bili-sync 二进制}}"
    # 位置参数不足 3 个时（BIN 走环境变量），shift 3 会以 "shift count out of range"
    # 裸崩——回退为清空剩余位置参数
    shift 3 2>/dev/null || shift $#
fi
while [[ $# -gt 0 ]]; do
    case "$1" in
        --config-dir) CONFIG_DIR="${2:?--config-dir 缺参数}"; shift 2 ;;
        --force) FORCE=1; shift ;;
        --create-db) CREATE_DB=1; shift ;;
        *) echo "未知参数: $1" >&2; exit 1 ;;
    esac
done

# 从公共库加载 PGURL 结构切分（提取库名/服务器段/query 参数）
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib-pgurl.sh"
split_pgurl
# 数据库名必须在连接串里（各工具的自然用法），缺库名直接拒绝
[[ -n "$PGURL_DB" ]] || { echo "错误: 连接串缺少库名（连接串需形如 postgres://user:pass@host:5432/db）" >&2; exit 1; }
# 密码改经 PGPASSWORD 传递：psql/pg_dump 的 argv 会被 `ps` 全程可见，
# 后续所有 psql 调用一律用 PGURL_NOPASS
strip_pg_password

# bili-sync 经 sqlx 连接，不读 PGPASSWORD 环境变量，密码必须留在 URL 里；
# 仅此一处使用完整连接串（进程短暂存在），其余 psql 调用一律用 PGURL_NOPASS
DB_URL="$PGURL"

# query 参数与 pgloader 的兼容性（2026-08-29 源码核对 + TLS 沙箱实测，pgloader 3.6.10）：
# pgloader 用 postmodern+CL+SSL 连 PG（不走 libpq），.load 的 INTO 串不带 query——
# 其 URI 参数白名单只有 sslmode/host/port/dbname/user/password/tablename，且 uri-param
# 类参数作末参数会把 .load 后续内容整个吞进参数值；环境变量里它也只读 PGSSLMODE 一个，
# PGSSLROOTCERT/PGSSLCERT/PGSSLKEY/PGCONNECT_TIMEOUT/PGAPPNAME/PGOPTIONS 一概不读
# （客户端证书只认硬编码 ~/.postgresql/postgresql.crt）。psql（libpq）与 bili-sync
# （sqlx）直接消费 URL 参数，URL 优先级高于环境变量（libpq 标准），无需重复导出。
if [[ -n "$PGURL_QUERY" ]]; then
    IFS='&' read -r -a QPARAMS <<< "${PGURL_QUERY#\?}"
    for kv in "${QPARAMS[@]}"; do
        key="${kv%%=*}"; val="${kv#*=}"
        case "$key" in
            # 唯一能传给 pgloader 的通道（INTO 串无 query，靠 PGSSLMODE 兜底）
            sslmode) export PGSSLMODE="$val" ;;
            sslrootcert|sslcert|sslkey|application_name|options)
                echo "警告: pgloader 不消费连接参数 $key（它只读 PGSSLMODE），此参数仅对 psql/bili-sync 生效" >&2 ;;
            sslpassword|connect_timeout)
                echo "警告: pgloader 不消费连接参数 $key（它只读 PGSSLMODE），sqlx 也不识别该参数，仅 psql 生效" >&2 ;;
            *) echo "警告: 忽略 pgloader 不支持的连接参数: $key" >&2 ;;
        esac
    done
fi
# pgloader 的 PGSSLMODE 只认 disable/allow/prefer/require 四个值：verify-full/verify-ca
# 会让它在 C4 解析 .load 时直接崩溃（exit 1）。值无论来自 URL 的 sslmode 还是外部
# PGSSLMODE 环境变量，都在此提前拦截并给出可行替代
case "${PGSSLMODE:-}" in
    ""|disable|allow|prefer|require) ;;
    *)
        echo "错误: PGSSLMODE=$PGSSLMODE 与 pgloader 不兼容（其 URI/环境变量仅认 disable/allow/prefer/require，" >&2
        echo "      verify-* 会让 pgloader 解析崩溃）。需要证书验证请改用 sslmode=require，并把 CA 装入" >&2
        echo "      运行 pgloader 机器的系统信任库（update-ca-certificates）——注意 pgloader 的 require 会按" >&2
        echo "      系统信任库验证服务器证书，与 libpq 的 require（只加密不验证）语义不同" >&2
        exit 1
        ;;
esac

[[ -f "$SQLITE_DB" ]] || { echo "错误: SQLite 库不存在: $SQLITE_DB" >&2; exit 1; }
# sqlite3 缺失时下方的命令替换会产出空串、被误判为"非空"，必须显式前置检查（失败关闭）
command -v sqlite3 >/dev/null 2>&1 || { echo "错误: 未安装 sqlite3（非空校验与行数对账需要）" >&2; exit 1; }
# sqlite3/pgloader 连不存在路径会自动建空库文件，路径打错会把空库「迁移成功」——必须先验非空。
# 排除集合与 C5 表清单一致（seaql_migrations + SQLite 内部表）：仅剩 seaql_migrations 的
# 文件不是合法迁移源，放行会让 C5 表清单为空而打印假成功。
# NOT GLOB 而非 NOT LIKE：LIKE 的 _ 是单字符通配符（sqlite_% 实际匹配 sqlite+任意字符），
# GLOB 的 * 才按字面 sqlite_ 前缀匹配 SQLite 内部表
[[ "$(sqlite3 "$SQLITE_DB" "SELECT count(*) FROM sqlite_master WHERE type='table' AND name <> 'seaql_migrations' AND name NOT GLOB 'sqlite_*';")" != "0" ]] \
    || { echo "错误: SQLite 库无业务表（空库或路径错误）: $SQLITE_DB" >&2; exit 1; }
[[ -x "$BIN" ]] || { echo "错误: bili-sync 二进制不存在或不可执行: $BIN" >&2; exit 1; }
command -v pgloader >/dev/null 2>&1 || { echo "错误: 未安装 pgloader" >&2; exit 1; }
# psql 缺失时下方 `! psql ...` 会落入「目标库不存在或无法连接」分支误导排查——显式前置检查
command -v psql >/dev/null 2>&1 || { echo "错误: 未安装 psql（校验/准备目标库需要）" >&2; exit 1; }

TMP_DIR="$(mktemp -d /tmp/migrate-run.XXXXXX)"

echo "=== C1 校验/准备目标库 $PGURL_DB（脚本不 DROP 数据库）==="
# 目标库必须无表：bili-sync 的 sea-orm 迁移是全量建 schema 的前提
# （增量迁移不会修正既有列定义，如旧库 created_at 默认值遗留问题）。
# 不 DROP 数据库：目标库由用户提供（空库或允许清空的库），脚本最多清库内表，
# 数据库本体与其他对象（扩展/角色等）永不被脚本删除
if ! psql "$PGURL_NOPASS" -Atc "SELECT 1" >/dev/null 2>&1; then
    # 连接失败通常是库不存在，也可能是密码错/连接参数有误。默认不自动建库：
    # 库名打错时静默 CREATE DATABASE 会把数据迁进新建的错误库、真实目标毫发无损，
    # 与「绝不 DROP」的谨慎不对称——自动建库需 --create-db 显式开启（需 CREATEDB 权限）
    if [[ "$CREATE_DB" != 1 ]]; then
        echo "错误: 目标库 $PGURL_DB 不存在或无法连接（库名错误或连接参数有误）" >&2
        echo "      确认连接串后重试；确认要自动创建空库请加 --create-db，或手动执行:" >&2
        echo "        CREATE DATABASE \"$PGURL_DB\";" >&2
        exit 1
    fi
    if psql "$PGURL_NOPASS_SERVER/postgres$PGURL_QUERY" -c "CREATE DATABASE \"$PGURL_DB\";" > /dev/null 2>&1; then
        echo "已创建空库 $PGURL_DB"
    else
        echo "错误: 目标库 $PGURL_DB 创建失败（权限不足或连接参数有误）" >&2
        echo "      请先手动创建空库后重试" >&2
        exit 1
    fi
fi
table_count=$(psql "$PGURL_NOPASS" -Atc "SELECT count(*) FROM pg_tables WHERE schemaname='public';" 2>/dev/null || echo 0)
if [[ "$table_count" != "0" ]]; then
    if [[ "$FORCE" != 1 ]]; then
        echo "错误: 目标库 $PGURL_DB 已有 $table_count 张表，拒绝清库" >&2
        echo "      确认目标库可清空后加 --force 重试（清库内所有表，不删除数据库本身）" >&2
        exit 1
    fi
    # --force：清空库内所有表（含 seaql_migrations——保留它会导致迁移按旧记录跳过建表）
    # format('%I') 对表名加引号，避免非全小写表名被 PG 折叠后 DROP 落空；
    # pg_tables 不含视图/物化视图，bili-sync 的库只有普通表，视图类残留不在清库范围
    TABLES_DROP=$(psql "$PGURL_NOPASS" -Atc \
        "SELECT string_agg(format('%I', tablename), ', ') FROM pg_tables WHERE schemaname='public';")
    psql "$PGURL_NOPASS" -v ON_ERROR_STOP=1 -c "DROP TABLE IF EXISTS $TABLES_DROP CASCADE;" > /dev/null
    echo "已清空目标库内所有表（数据库本体保留）"
fi


# 服务器开着 TLS 而 pgloader 侧未配 sslmode 时，pgloader 会静默以明文连接
# （其 use-ssl 默认 :no；psql/bili-sync 默认会协商 TLS）——提前提醒，避免
# 「迁移成功但导入这一腿走了明文」。SHOW ssl 对普通用户不可读时静默跳过
if [[ -z "${PGSSLMODE:-}" ]] && [[ "$(psql "$PGURL_NOPASS" -Atc "SHOW ssl" 2>/dev/null)" == "on" ]]; then
    echo "警告: 目标服务器已启用 TLS，但连接串未带 sslmode：pgloader 将以明文连接（psql/bili-sync 不受影响）" >&2
    echo "      全链路 TLS 请加 sslmode=require（pgloader 按系统信任库验证证书，内部 CA 需先装入）" >&2
fi

echo "=== C2 bili-sync 建 schema（sea-orm 迁移）==="
# 后台启动，等数据库初始化完成（迁移执行）后停止；首轮定时任务不触发（interval 默认较长）
"$BIN" --config-dir "$CONFIG_DIR" --database-url "$DB_URL" --scan-only \
    --log-level 'None,bili_sync=info' > "$TMP_DIR/app.log" 2>&1 &
BIN_PID=$!
READY=0
for _ in $(seq 1 30); do
    # 主判据：日志出现初始化完成（快路径）
    grep -q '数据库初始化完成' "$TMP_DIR/app.log" 2>/dev/null && { READY=1; break; }
    # 进程死亡优先检出（避免半建 schema 时被兜底判据误判就绪、掩盖真实报错）
    kill -0 $BIN_PID 2>/dev/null || { echo "错误: bili-sync 提前退出"; tail -20 "$TMP_DIR/app.log" >&2; exit 1; }
    # 兜底判据：目标库 schema 已建成（日志文案/级别变动时仍能就绪）
    [[ "$(psql "$PGURL_NOPASS" -Atc "SELECT count(*) FROM pg_tables WHERE schemaname='public';" 2>/dev/null || echo 0)" != "0" ]] && { READY=1; break; }
    sleep 1
done
kill $BIN_PID 2>/dev/null || true
wait $BIN_PID 2>/dev/null || true
sleep 1
[[ "$READY" == 1 ]] || { echo "错误: 等待数据库初始化就绪超时（30s，未见日志信号或 schema）"; tail -20 "$TMP_DIR/app.log" >&2; exit 1; }
if ! grep -q '数据库初始化完成' "$TMP_DIR/app.log" 2>/dev/null; then
    echo "提示: 未匹配到「数据库初始化完成」日志（文案/级别可能有变），已按目标库 schema 就绪判定"
fi
echo "schema 已建好"

echo "=== C2b TRUNCATE 业务表（清默认 config 行）==="
TABLES=$(psql "$PGURL_NOPASS" -Atc \
    "SELECT string_agg(tablename, ', ') FROM pg_tables WHERE schemaname='public' AND tablename<>'seaql_migrations';")
[[ -n "$TABLES" ]] || { echo "错误: 目标库无业务表（bili-sync 建 schema 失败？）" >&2; exit 1; }
psql "$PGURL_NOPASS" -v ON_ERROR_STOP=1 -c "TRUNCATE $TABLES RESTART IDENTITY;" > /dev/null
echo "已清空业务表: $TABLES"

echo "=== C3 动态生成迁移配置 ==="
CONF="$TMP_DIR/migrate.load"
bash "$SCRIPT_DIR/gen-migrate-conf.sh" "$PGURL" "$SQLITE_DB" "$CONF"

echo "=== C4 执行导入（pgloader）==="
pgloader "$CONF" 2>&1 | tee "$TMP_DIR/pgloader.log"

echo "=== C4b 序列对齐（修复 reset sequences 对空表的重置）==="
# pgloader reset sequences 实测（3.6.10）：非空表 last_value=MAX(id)、is_called=true（对齐），
# 空表被重置成 last_value=1、is_called=true（下一个分配值跳 1）。统一按 MAX(id) 幂等重设：
# 非空表 setval(seq, MAX(id), true) -> 下一值 MAX(id)+1；空表 setval(seq, 1, false) -> 下一值 1
psql "$PGURL_NOPASS" -v ON_ERROR_STOP=1 -c "
DO \$\$
DECLARE
    r record;
    max_id bigint;
BEGIN
    FOR r IN
        SELECT tablename FROM pg_tables
        WHERE schemaname = 'public' AND tablename <> 'seaql_migrations'
    LOOP
        CONTINUE WHEN pg_get_serial_sequence(r.tablename, 'id') IS NULL;
        EXECUTE format('SELECT COALESCE(MAX(id), 0) FROM %I', r.tablename) INTO max_id;
        PERFORM setval(pg_get_serial_sequence(r.tablename, 'id'), GREATEST(max_id, 1), max_id > 0);
    END LOOP;
END
\$\$;" > /dev/null
echo "序列已按 MAX(id) 对齐（含空表）"

echo "=== C5 行数对账（源 SQLite vs 目标 PG）==="
# pgloader 行级错误（如时间戳无法解析）不会反映到退出码：实测 3.6.10 整表 COPY
# 失败仍 exit 0，仅 summary 的 errors 列有计数。导入后必须对账行数兜底。
# 用 count(*) 而非 pg_stat_user_tables.n_live_tup（后者不 ANALYZE 不准，TRUNCATE+COPY 后可能显示 0）
ROW_FAIL=0
ALL_TARGET_EMPTY=1
SRC_TABLES=$(sqlite3 "$SQLITE_DB" "SELECT name FROM sqlite_master
    WHERE type='table' AND name <> 'seaql_migrations' AND name NOT GLOB 'sqlite_*'
    ORDER BY name;")
for t in $SRC_TABLES; do
    src_n=$(sqlite3 "$SQLITE_DB" "SELECT count(*) FROM \"$t\";")
    pg_n=$(psql "$PGURL_NOPASS" -Atc "SELECT count(*) FROM \"$t\";")
    if [[ "$src_n" == "$pg_n" ]]; then
        printf '%-16s %s = %s ✓\n' "$t" "$src_n" "$pg_n"
    else
        printf '%-16s 源 %s != PG %s ✗\n' "$t" "$src_n" "$pg_n" >&2
        ROW_FAIL=1
        [[ "$pg_n" == "0" ]] || ALL_TARGET_EMPTY=0
    fi
done
if [[ "$ROW_FAIL" != 0 ]]; then
    echo "错误: 行数对账不一致，导入不完整（对照上方 pgloader summary 的 errors 列定位）" >&2
    if [[ "$ALL_TARGET_EMPTY" == 1 ]]; then
        echo "      目标侧行数全为 0 且 summary 无 errors：多为 pgloader 未读到真实源库" >&2
        echo "      （.load 的 FROM 未指向源 SQLite，检查 gen-migrate-conf 生成的绝对路径）" >&2
    fi
    exit 1
fi
echo "迁移完成，行数全部一致"
