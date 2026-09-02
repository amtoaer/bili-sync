#!/usr/bin/env bash
# inject-credential.sh — 从 PostgreSQL config 表提取整份配置注入 SQLite config 表（整体替换 data 列）
#
# 用途：SQLite→PG 迁移实测验证的阶段 A（A1-A2）。
#   以 PostgreSQL 后端跑过 bili-sync 的库（如 bili_sync）中存有真实登录凭证，
#   本脚本将其中 config 行（id=1, data 为整份 JSON）原样搬入 SQLite 库，
#   使 SQLite 后端得以用真实凭证运行，生成真实形态的迁移源数据。
#
# 用法：
#   inject-credential.sh <PGURL> <SQLITE_DB> [工作目录]
#   不传工作目录时使用 mktemp -d（0700，退出自动清理，凭据不落盘）；
#   传入工作目录时目录由用户管理，脚本异常退出会提示清理。
#
# 前置条件：
#   - SQLite 库已由 bili-sync 首次启动建好（存在 config 表）
#   - psql / sqlite3 / jq 可用
#
# 凭证安全约定：密码经 PGPASSWORD 传递（不进 argv），凭证明文只落工作目录，
# 注入经 stdin 传入 sqlite3（不进 argv）；终端只输出 md5 与存在性矩阵（布尔/长度）。

set -euo pipefail

PGURL="${1:?用法: inject-credential.sh <PGURL> <SQLITE_DB> [工作目录]}"
SQLITE_DB="${2:?缺 SQLITE_DB}"
WORK="${3:-}"


# 从公共库加载 PGURL 结构切分（提取库名/服务器段/query 参数）
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib-pgurl.sh"
split_pgurl
# 密码改经 PGPASSWORD 传递：psql 的 argv 会被 `ps` 全程可见，一律用 PGURL_NOPASS_SERVER
strip_pg_password
# PG 侧库名：优先连接串里的库名，其次 PG_DB 环境变量（旧约定后备），缺省 bili_sync
PG_DB="${PGURL_DB:-${PG_DB:-bili_sync}}"

if [[ -n "$WORK" ]]; then
    WORK_USER_KEEP=1
    mkdir -p "$WORK"
    # 用户指定工作目录：脚本异常退出时提醒清理，不自动删除（目录由用户管理）
    trap '[[ $? -ne 0 ]] && [[ -f "$WORK/config_src.json" ]] && echo "注意: 异常退出，凭证明文残留于 $WORK/config_src.json，请及时清理" >&2' EXIT
else
    # 默认使用 mktemp -d（0700 仅当前用户可读）：退出即自动清理，凭证明文不落盘
    WORK="$(mktemp -d)"
    trap 'rm -rf "$WORK"' EXIT
fi

# --- 1. 从 PG 提取 config 行（直落文件，不进终端） --------------------------------
# 不能用 COPY TO STDOUT：COPY 文本格式会把数据内的反斜杠/换行转义，
# 凭证含这些字节时会静默损坏；SELECT 原样输出（末尾带一个行尾换行，与 SQLite 侧一致）
psql "$PGURL_NOPASS_SERVER/$PG_DB$PGURL_QUERY" -Atc "SELECT data FROM config WHERE id=1" > "$WORK/config_src.json" 2>/dev/null \
    || { echo "错误: 读取 PG 侧 config 失败（库 $PG_DB 无 config 表？）"; exit 1; }
test -s "$WORK/config_src.json" || { echo "错误: PG 侧 config 为空"; exit 1; }

# --- 2. JSON 有效性 + credential 存在性矩阵（只打印布尔/长度） --------------------
jq -e . "$WORK/config_src.json" > /dev/null || { echo "错误: PG config 不是合法 JSON"; exit 1; }

cred_matrix() { # $1=config JSON 文件
    jq -c -e '.credential | {
        sessdata:   (.sessdata   | type == "string" and length > 0),
        bili_jct:   (.bili_jct   | type == "string" and length > 0),
        dedeuserid: (.dedeuserid | type == "string" and length > 0),
        buvid3:     (.buvid3     | type == "string" and length > 0),
        ac_time_value: (.ac_time_value | type == "string" and length > 0),
    }' "$1"
}

echo "[1/5] PG config 提取完成 (md5=$(md5sum "$WORK/config_src.json" | cut -d' ' -f1))"
cred_matrix "$WORK/config_src.json" | jq . > "$WORK/matrix_pg.json"
cat "$WORK/matrix_pg.json"

# --- 3. 校验 SQLite 库与 config 表存在 -------------------------------------------
[[ -f "$SQLITE_DB" ]] || { echo "错误: SQLite 库不存在: $SQLITE_DB（请先用 bili-sync 首次启动建库）"; exit 1; }
sqlite3 "$SQLITE_DB" "SELECT count(*) FROM config;" > /dev/null 2>&1 \
    || { echo "错误: SQLite 库 $SQLITE_DB 无 config 表，请先用 bili-sync 首次启动建库"; exit 1; }

# --- 4. 注入：SQL 单引号转义后经 stdin 传入，整体替换 data 列 ----------------------
# SQL 字符串字面量用单引号包裹，JSON 中出现的单引号双写（''）即可；
# 双引号/反斜杠在 SQL 字符串中无特殊含义，原样保留。
# 整份凭证明文经 heredoc 管道（stdin）进入 sqlite3，而非命令行参数——
# 后者会被 `ps` 全程可见；$(sed ...) 同时剥掉 psql 输出末尾的换行（与历史语义一致）
SQLITE_ESCAPED=$(sed "s/'/''/g" "$WORK/config_src.json")
sqlite3 "$SQLITE_DB" <<SQL || { echo "错误: SQLite 注入失败"; exit 1; }
UPDATE config SET data = '$SQLITE_ESCAPED' WHERE id = 1;
SQL
echo "[2/5] 注入完成: UPDATE config SET data=<整份 JSON> WHERE id=1"

# --- 5. SQLite 侧校验：json_valid + 矩阵一致 + md5 一致 ----------------------------
sqlite3 "$SQLITE_DB" "SELECT json_valid(data) FROM config WHERE id=1;" | grep -qx 1 \
    || { echo "错误: SQLite config.data 不是合法 JSON"; exit 1; }
sqlite3 "$SQLITE_DB" "SELECT data FROM config WHERE id=1;" > "$WORK/config_sqlite.json"
md5sum "$WORK/config_sqlite.json" | cut -d' ' -f1 > "$WORK/md5_sqlite.txt"
md5sum "$WORK/config_src.json" | cut -d' ' -f1 > "$WORK/md5_pg.txt"

cred_matrix "$WORK/config_sqlite.json" | jq . > "$WORK/matrix_sqlite.json"
echo "[3/5] SQLite 侧矩阵:"
cat "$WORK/matrix_sqlite.json"

if diff -q "$WORK/matrix_pg.json" "$WORK/matrix_sqlite.json" > /dev/null; then
    echo "[4/5] credential 存在性矩阵: 一致"
else
    echo "[4/5] credential 存在性矩阵: 不一致!"; exit 1
fi

if diff -q "$WORK/md5_pg.txt" "$WORK/md5_sqlite.txt" > /dev/null; then
    echo "[5/5] config 字节级一致 (md5=$(cat "$WORK/md5_pg.txt"))"
else
    # 两侧均为 SELECT 原样输出（无 COPY 转义问题，见步骤 1 的说明），
    # 不存在「合法的不一致」：任何字节差异都意味着注入过程改写了数据
    echo "[5/5] 错误: md5 不一致，注入过程改写了 config 数据" >&2
    exit 1
fi

if [[ -n "${WORK_USER_KEEP:-}" ]]; then
    echo "完成。工作目录: $WORK（含凭证明文，验证结束后请清理）"
else
    echo "完成。临时工作目录已自动清理"
fi
