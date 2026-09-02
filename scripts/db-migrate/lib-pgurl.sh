#!/usr/bin/env bash
# lib-pgurl.sh — PGURL 结构切分公共函数（供 db-migrate 工具链各脚本 source）
#
# 不解析、不转义任何字符，只在结构位置切分（对合法 URL 安全）：
#   1) query 段：第一个未编码 ? 之后全部原样保留（内部再含 ?、多参数 & 均不受影响）
#   2) 库名段：authority（// 之后、第一个 / 之前）之后的第一个 / 起视为 dbname——
#      数据库名就在连接串里（各工具的自然用法），脚本提取出来供拼接使用
#
# 调用后设置：
#   PGURL_SERVER — 服务器级连接串（无库名、无 query、无尾斜杠）
#   PGURL_DB     — 库名（连接串未带库名时为空串，调用方按需校验）
#   PGURL_QUERY  — query 段（含前导 ?；无参数时为空串）
split_pgurl() {
    PGURL_BASE="${PGURL%%\?*}"
    PGURL_BASE="${PGURL_BASE%/}"
    local rest="${PGURL_BASE#*//}"
    case "$rest" in
        */*)
            PGURL_DB="${rest#*/}"
            PGURL_SERVER="${PGURL_BASE%/*}"
            ;;
        *)
            PGURL_DB=""
            PGURL_SERVER="$PGURL_BASE"
            ;;
    esac
    PGURL_QUERY=""
    # 不能用「[[ ]] && 赋值」收尾：URL 无 query 时条件为假会使函数返回 1，
    # set -e 的调用方会静默退出（普通无参数 URL 全部中招，实测踩坑）
    if [[ "$PGURL" == *\?* ]]; then
        PGURL_QUERY="?${PGURL#*\?}"
    fi
}

# percent-decode：把 %XX 还原为对应字节（仅用于密码，密码可能含 URL 保留字符；
# 未编码的 % 或非法十六进制原样保留）。每个 %XX 单独经 printf '%b' 解码，
# 解码产物不会再次被转义解释（字面反斜杠/十六进制文本原样拼接）
pgurl_percent_decode() {
    local s="$1" out="" c
    while [[ -n "$s" ]]; do
        if [[ "$s" == %[0-9A-Fa-f][0-9A-Fa-f]* ]]; then
            out+="$(printf '%b' "\\x${s:1:2}")"
            s="${s:3}"
        else
            c="${s:0:1}"
            out+="$c"
            s="${s:1}"
        fi
    done
    printf '%s' "$out"
}

# 剥离连接串中的密码段并导出 PGPASSWORD（供 psql/pg_dump 等 libpq 客户端使用）。
# 动机：带密码的完整连接串直接作 psql/pg_dump 的 argv 会全程暴露在 `ps` 输出里；
# libpq 的 URL 密码优先级高于 PGPASSWORD，必须同时从 URL 中移除密码段。
#
# 调用前需先设置 PGURL（并已调用 split_pgurl），调用后：
#   PGPASSWORD          — percent-decode 后的密码（URL 未带密码时不导出，保留外部环境值）
#   PGURL_NOPASS        — 完整连接串剥离密码段（query 保留）
#   PGURL_NOPASS_SERVER — 服务器级连接串剥离密码段（无库名/query/尾斜杠）
strip_pg_password() {
    local base="${PGURL%%\?*}"
    base="${base%/}"
    # userinfo 段：authority 中第一个未编码 @ 之前的部分；
    # 无 @ 时是 host[:port]（如 postgres://host:5432/db），不是 user:pass，不可拆
    local authority="${base#*//}"
    local userinfo="${authority%%/*}"
    if [[ "$userinfo" == *@* ]]; then
        local cred="${userinfo%%@*}"   # user:pass（user 可能为空，如 :pass@host）
        local base_nopass="$base"
        if [[ "$cred" == *:* ]]; then
            PGPASSWORD="$(pgurl_percent_decode "${cred#*:}")"
            export PGPASSWORD
            # 重建无密码 URL：保留 user，去掉 :pass 段
            base_nopass="${base%%//*}//${cred%%:*}@${authority#*@}"
        fi
        PGURL_NOPASS="$base_nopass${PGURL#"$base"}"
        PGURL_NOPASS_SERVER="${base_nopass%/*}"
    else
        # 无 userinfo：URL 原样保留（外部 PGPASSWORD 若存在则继续生效），只派生服务器段
        PGURL_NOPASS="$PGURL"
        PGURL_NOPASS_SERVER="${base%/*}"
    fi
}
