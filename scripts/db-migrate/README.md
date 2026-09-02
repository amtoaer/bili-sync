# db-migrate — SQLite→PostgreSQL 迁移实测验证工具链

面向维护者的迁移验证工具链，用于实测并评估 SQLite→PG 数据迁移的保真度。
本目录脚本全部**动态推导**（表清单/列类型/cast 规则从数据库 catalog 获取，不手写），
维护者给实体加列/加表后重跑即可，无需修改任何"迁移配置清单"。

## 背景结论（2026-08-27 实测）

- **schema 路线**：目标 PG 库的 schema 由 bili-sync 自己的 sea-orm 迁移生成（启动一次即全量建好），
  迁移工具只做 data-only 导入。`seaql_migrations` 不随迁。
- **pgloader 3.6.10 实测结论**：
  - 无损：行数、位域 bigint（含 ≥2³¹）、时间戳（SQLite 文本 → timestamp，微秒精度）、
    JSON（`json_text→json`、`jsonb_text→jsonb`）、布尔（0/1→boolean）、config（TEXT 字节级）、
    唯一索引（目标库迁移自建，COALESCE 版本生效）
  - **行级错误不影响退出码**：坏数据（如无法解析的时间戳）导致整表 COPY 失败时
    仍 `exit 0`，只有 summary 的 errors 列有计数；`migrate.sh` C5 的行数对账为此兜底
  - **序列**：`reset sequences` 对非空表已对齐（last_value=MAX(id)、is_called=true），
    但会把空表重置成 last_value=1、is_called=true（下一个分配值跳 1 号）；
    `migrate.sh` C4b 统一按 MAX(id) 幂等重设（含空表），无需人工 setval
- **DDL 对比注意**：对比基准需用当前代码新建的库（如 bili_sync_tz）。
  旧库（bili_sync）的 `created_at DEFAULT CURRENT_TIMESTAMP` 是 9864e3c 修复前的历史遗留，
  SeaORM 迁移不修改既有列默认值，与迁移工具无关。

## 脚本清单

| 脚本 | 职责 | 推导点 |
|---|---|---|
| `inject-credential.sh <PGURL> <SQLITE_DB> [工作目录]` | PG config 整行注入 SQLite config 表（凭证制备，默认 `mktemp -d` 退出自动清理） | config 表 id=1 约定 |
| `snapshot-baseline.sh <SQLITE_DB> <输出目录>` | SQLite 形态基线导出（行数/schema/位域/时间戳/JSON/NULL/布尔/md5） | 遍历 sqlite_master |
| `gen-migrate-conf.sh <PGURL> <SQLITE_DB> <输出>` | 动态生成 pgloader .load（表清单 + cast 规则） | pg_tables + information_schema |
| `migrate.sh <PGURL> <SQLITE_DB> <BIN> [--config-dir <dir>] [--force] [--create-db]` | 校验/准备目标库 → bili-sync 建 schema → TRUNCATE → 导入 → 序列对齐 + 行数对账 | 同上 |
| `reconcile.sh <SQLITE_DB> <PGURL> [基线目录]` | 对账：行数/位域/时间戳/JSON/布尔/config md5/UNIQUE/序列 | 两库 catalog 遍历 |
| `compare-ddl.sh <PGURL> <库A> <库B>` | pg_dump schema 对比（自动剔除 tz_probe 类残留表） | pg_dump 全量 |
| `compare-data.sh <PGURL> <源库A> <迁移库B> <SQLITE_DB>` | 数据格式五维对比 + config 凭证三方 md5 | information_schema |

## PGURL 连接参数（TLS 等）

- PGURL 为**完整连接串**（含库名与可选 query 参数，各工具的自然用法），库名由 `split_pgurl`
  从连接串提取（缺库名显式报错）；多库对比类脚本（compare-ddl/compare-data）仍以
  「服务器级 URL + 库名参数」调用。切分逻辑集中在公共库 `lib-pgurl.sh`
  （各脚本 source，不解析不转义）。连接串按 URL 规则解析：密码等含特殊字符
  （`/`、`?`、`@`、`%` 等）时必须 percent-encode（如 `/` → `%2F`），
  与 libpq 及各工具的要求一致
- **密码传递（2026-09-03 安全加固）**：连接串里的密码会从 psql/pg_dump 的 argv 中移除
  （argv 会被 `ps` 全程可见），改经 `PGPASSWORD` 环境变量传递（libpq 的 URL 密码优先级
  高于 PGPASSWORD，故必须同时移除 URL 密码段；percent-encode 的密码由 `lib-pgurl.sh`
  自动解码）。**唯一例外**：`migrate.sh` 启动 bili-sync 建 schema 时仍传完整 URL——
  sqlx 不读 PGPASSWORD，且该进程短暂存在。`gen-migrate-conf.sh` 生成的 `.load` 的
  INTO 串含密码（pgloader 从文件读取、不经 argv），生成后自动 `chmod 600`
- query 参数可写多个（如 `?sslmode=require&connect_timeout=15`）。**psql（libpq）与
  bili-sync（sqlx）直接消费 URL 参数**：sqlx 支持 sslmode（含 verify-full）/sslrootcert/
  sslcert/sslkey/application_name/options，未知参数（如 connect_timeout）忽略并告警；
  URL 参数优先级高于环境变量（libpq 标准），两处同传无冲突
- **pgloader 例外**（2026-08-29 源码核对 + TLS 沙箱实测，3.6.10）：它用 postmodern+CL+SSL
  连 PG，**不走 libpq**。`.load` 的 INTO 串不带 query（其 URI 参数白名单只有
  sslmode/host/port/dbname/user/password/tablename，且 uri-param 类参数作末参数会把
  `.load` 后续内容整个吞进参数值），`migrate.sh` 把 sslmode 导出为 `PGSSLMODE` 环境变量
  传递。环境变量中 pgloader **只读 PGSSLMODE**，值仅认 disable/allow/prefer/require——
  `verify-full`/`verify-ca` 会让它解析 .load 时直接崩溃（`migrate.sh` 提前报错拦截）；
  `PGSSLROOTCERT`/`PGSSLCERT`/`PGSSLKEY`/`PGCONNECT_TIMEOUT`/`PGAPPNAME`/`PGOPTIONS`
  一概不读（客户端证书只认硬编码 `~/.postgresql/postgresql.crt`，`migrate.sh` 对这些
  参数会发警告）
- **pgloader 的 require 与 libpq 的 require 语义不同**：pgloader 会按**系统 CA 信任库**验证
  服务器证书（libpq 的 require 只加密不验证）。目标服务器用内部 CA 时，把 CA 装入运行
  pgloader 机器的系统信任库（Debian/Ubuntu：`update-ca-certificates`）后用
  `sslmode=require` 即可全链路 TLS（pgloader 不做主机名校验）。什么 TLS 参数都不配时
  pgloader 默认**明文**连接（其 use-ssl 默认 `:no`，服务器同时接受明文时会静默降级）——
  `migrate.sh` 会在服务器开着 TLS 而未配 sslmode 时发出警告

## 常用流程（维护者改数据库后回归）

```bash
# 1. 冻结迁移源（先停 bili-sync）并导出基线
sqlite3 data.sqlite 'PRAGMA wal_checkpoint(TRUNCATE);'
snapshot-baseline.sh data.sqlite baseline/

# 2. 全新迁移（校验/建库→建 schema→清默认行→动态配置→pgloader 导入→序列对齐+行数对账）
#    脚本不 DROP 数据库、也不自动建库：目标库需为空（无表），或加 --force 清空库内表后重跑；
#    目标库不存在时加 --create-db 自动创建空库（需 CREATEDB 权限）
migrate.sh "$PGURL" data.sqlite /path/to/bili-sync-rs --force   # PGURL 含库名，如 .../bili_migrate_test?sslmode=require

# 3. 对账（对照基线；序列已由 migrate.sh C4b 自动对齐，此处为复核）
reconcile.sh data.sqlite "$PGURL" baseline/   # PGURL 含库名

# 4. 与现有库对比（DDL 用 bili_sync_tz 等全新库做基准；数据格式与 bili_sync 对比）
compare-ddl.sh "$PGURL" bili_sync_tz bili_migrate_test
compare-data.sh "$PGURL" bili_sync bili_migrate_test data.sqlite
```

## 序列对齐

已由 `migrate.sh` C4b 自动完成：对全部业务表按 `MAX(id)` 幂等重设
（非空表 `setval(seq, MAX(id), true)`、空表 `setval(seq, 1, false)`），
`reconcile.sh` 8/8 仍会逐表复核「下一个分配值 == MAX(id)+1」。

## 唯一需人工维护的点

`gen-migrate-conf.sh` 顶部的 `SEAQ_ORM_TYPE_MAP`（sea-query 的确定性类型映射，
PG data_type → SQLite 声明类型名，当前 10 条：timestamp/json/jsonb/boolean/bigint/integer/smallint/text/varchar/char）。
仅当 sea-query 支持新的列类型映射时才需更新——集中一处，不会散落多处配置漏改。

## 凭证安全约定

- 密码经 `PGPASSWORD` 环境变量传递，不进入 psql/pg_dump 的命令行参数（`ps` 不可见）；
  bili-sync 建 schema 一腿除外（sqlx 不读 PGPASSWORD，进程短暂存在）。
- 凭证明文只落工作目录：`inject-credential.sh` 不传工作目录时用 `mktemp -d`（0700，
  仅当前用户可读），退出自动清理；传入工作目录时由用户管理，脚本异常退出会提示清理。
- `gen-migrate-conf.sh` 生成的 `.load` 含密码，自动 `chmod 600`。
- 终端只输出 md5 与存在性矩阵（布尔/长度），不打印凭证值。

## 环境

- 本机无 cargo，全部命令在 podman 容器（bili-dev）内执行；PG 目标库在 bili-pg 容器
  （pasta 网络下容器内连 host 服务需用 host 真实 IP：`PGURL=postgres://postgres:bili@10.1.2.1:5433/bili_migrate_test`，
  `host.containers.internal` 与容器内 `127.0.0.1` 均不通，2026-08-29 实测）。
- 依赖：psql / sqlite3 / jq / pgloader / pg_dump（导入工具固定为 pgloader）。
