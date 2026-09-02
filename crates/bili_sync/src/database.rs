use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use bili_sync_migration::{Migrator, MigratorTrait};
use sea_orm::sqlx::postgres::PgConnectOptions;
use sea_orm::sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqliteSynchronous};
use sea_orm::sqlx::{ConnectOptions as SqlxConnectOptions, Postgres, Sqlite};
use sea_orm::{
    ConnectOptions, ConnectionTrait, Database, DatabaseBackend, DatabaseConnection, SqlxPostgresConnector,
    SqlxSqliteConnector, Statement,
};

pub fn database_url(path: &Path) -> String {
    format!("sqlite://{}?mode=rwc", path.to_string_lossy())
}

async fn database_connection(database_url: &str) -> Result<DatabaseConnection> {
    let mut option = ConnectOptions::new(database_url);
    option
        .max_connections(50)
        .min_connections(5)
        .acquire_timeout(Duration::from_secs(90));
    // 按 scheme 区分后端：覆盖 sqlite: 与 sqlite:// 两种写法，其余按 PostgreSQL 解析
    // scheme 按 RFC 3986 大小写不敏感，先统一转小写再判定，避免 SQLite:// 之类写法被误路由
    let scheme = database_url.split(':').next().unwrap_or_default().to_ascii_lowercase();
    if scheme == "sqlite" {
        let connect_option = option
            .get_url()
            .parse::<SqliteConnectOptions>()
            .context("Failed to parse database URL")?
            .disable_statement_logging()
            .busy_timeout(Duration::from_secs(90))
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal)
            .optimize_on_close(true, None);
        Ok(SqlxSqliteConnector::from_sqlx_sqlite_pool(
            option
                .sqlx_pool_options::<Sqlite>()
                .connect_with(connect_option)
                .await?,
        ))
    } else {
        // sqlx 的 PgConnectOptions 不校验 scheme，任意非 sqlite URL 都会按 PG 解析，
        // 显式校验避免 mysql:// 之类连接串得到费解的握手错误
        if !matches!(scheme.as_str(), "postgres" | "postgresql") {
            bail!("不支持的数据库 scheme，仅支持 sqlite:// 与 postgres:// 两种连接串");
        }
        let connect_option = option
            .get_url()
            .parse::<PgConnectOptions>()
            .context("Failed to parse database URL")?
            .disable_statement_logging();
        Ok(SqlxPostgresConnector::from_sqlx_postgres_pool(
            option
                .sqlx_pool_options::<Postgres>()
                .connect_with(connect_option)
                .await?,
        ))
    }
}

async fn migrate_database(database_url: &str) -> Result<()> {
    // 注意此处使用内部构造的 DatabaseConnection，而不是通过 database_connection() 获取
    // 这是因为使用多个连接的 Connection 会导致奇怪的迁移顺序问题，而使用默认的连接选项不会
    let connection = Database::connect(database_url).await?;
    // 避免 https://github.com/amtoaer/bili-sync/issues/571 问题，迁移前根据 migration 确认当前版本
    // 如果用户从 2.6.0 以下版本直接升级，migration 不满足需求，直接报错而不执行迁移
    // 版本守卫仅对 SQLite 生效：PostgreSQL 总是全新安装，不存在从旧版本升级的问题
    if matches!(connection.get_database_backend(), DatabaseBackend::Sqlite)
        && connection
            .query_one(Statement::from_string(
                connection.get_database_backend(),
                "SELECT 1 FROM seaql_migrations WHERE version = 'm20250613_043257_add_config';",
            ))
            .await
            .is_ok_and(|res| res.is_none())
    {
        // 查询成功且结果为空，即没有 m20250613_043257_add_config，说明版本低于 2.6.0
        bail!("该版本仅支持从 2.6.x 以上的版本升级，请先升级至 2.6.x 或 2.7.x 完成配置迁移，再升级至最新版本。");
    }
    Ok(Migrator::up(&connection, None).await?)
}

/// 进行数据库迁移并获取数据库连接，供外部使用
pub async fn setup_database(database_url: &str) -> Result<DatabaseConnection> {
    // 仅 SQLite 需要预先创建数据库文件所在的目录
    if let Some(path) = sqlite_path_from_url(database_url)
        && let Some(parent) = path.parent()
        // 相对路径（如 sqlite:data.sqlite）或内存库（:memory:）没有父目录，跳过
        && !parent.as_os_str().is_empty()
    {
        tokio::fs::create_dir_all(parent).await.context(
            "Failed to create config directory. Please check if you have granted necessary permissions to your folder.",
        )?;
    }
    migrate_database(database_url)
        .await
        .context("Failed to migrate database")?;
    database_connection(database_url)
        .await
        .context("Failed to connect to database")
}

fn sqlite_path_from_url(url: &str) -> Option<&Path> {
    // sqlx 接受 sqlite: 与 sqlite:// 两种写法，两者都需要预创建父目录
    url.strip_prefix("sqlite://")
        .or_else(|| url.strip_prefix("sqlite:"))
        .map(|rest| rest.split('?').next().unwrap_or(rest))
        .map(Path::new)
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDateTime;
    use sea_orm::ActiveValue::Set;
    use sea_orm::{ColumnTrait, EntityTrait, FromQueryResult, PaginatorTrait, QueryFilter};

    use super::*;
    use crate::api::response::DayCountPair;
    use crate::api::routes::dashboard::dashboard_day_count_sql;

    /// SQLite 回归测试：覆盖 created_at 由 CURRENT_TIMESTAMP 默认值写入、实体按
    /// DateTime 解码（UTC 语义）以及 dashboard SQL 的 SQLite 分支——这些路径不依赖
    /// PostgreSQL，常驻运行作为升级兼容性的护栏
    #[tokio::test]
    async fn test_sqlite_created_at_and_dashboard() -> Result<()> {
        let dir = std::env::temp_dir().join(format!("bili-sync-test-{}", std::process::id()));
        tokio::fs::create_dir_all(&dir).await?;
        let url = database_url(&dir.join("test.sqlite"));
        let connection = setup_database(&url).await?;
        // 清理上次运行遗留的测试数据（临时目录按 PID 命名，进程被杀后可能残留）
        bili_sync_entity::video::Entity::delete_many()
            .filter(bili_sync_entity::video::Column::Bvid.eq("BV1sqlite00001"))
            .exec(&connection)
            .await?;
        let timestamp = NaiveDateTime::parse_from_str("2024-01-01 00:00:00", "%Y-%m-%d %H:%M:%S")?;
        // created_at 不显式赋值，由 CURRENT_TIMESTAMP 默认值写入
        let insert_result = bili_sync_entity::video::Entity::insert(bili_sync_entity::video::ActiveModel {
            upper_id: Set(1),
            upper_name: Set("测试".to_string()),
            upper_face: Set("".to_string()),
            name: Set("测试视频".to_string()),
            path: Set("/tmp/bili-sync-sqlite-regression".to_string()),
            category: Set(2),
            bvid: Set("BV1sqlite00001".to_string()),
            intro: Set("".to_string()),
            cover: Set("".to_string()),
            ctime: Set(timestamp),
            pubtime: Set(timestamp),
            favtime: Set(timestamp),
            download_status: Set(0),
            valid: Set(true),
            should_download: Set(true),
            ..Default::default()
        })
        .exec(&connection)
        .await?;
        let video_id = insert_result.last_insert_id;
        // 读回验证 DateTime 解码与 UTC 语义
        let model = bili_sync_entity::video::Entity::find_by_id(video_id as i32)
            .one(&connection)
            .await?
            .expect("视频应当存在");
        let delta_minutes = (chrono::Utc::now().naive_utc() - model.created_at).num_minutes().abs();
        assert!(
            delta_minutes < 10,
            "created_at 应解码为 UTC，与 UTC 相差 {delta_minutes} 分钟"
        );
        // dashboard SQLite 分支返回最近 7 天且今日计数包含刚插入的视频
        let day_counts = DayCountPair::find_by_statement(Statement::from_string(
            connection.get_database_backend(),
            dashboard_day_count_sql(connection.get_database_backend()),
        ))
        .all(&connection)
        .await?;
        assert_eq!(day_counts.len(), 7, "dashboard 应返回最近 7 天");
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        assert!(
            day_counts.iter().any(|day| day.day == today && day.cnt >= 1),
            "今日应有至少 1 条新增视频，实际为 {day_counts:?}"
        );
        connection.close().await?;
        tokio::fs::remove_dir_all(&dir).await?;
        Ok(())
    }
}
