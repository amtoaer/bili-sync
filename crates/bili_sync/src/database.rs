use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use bili_sync_migration::{Migrator, MigratorTrait};
use sea_orm::sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqliteSynchronous};
use sea_orm::sqlx::{ConnectOptions as SqlxConnectOptions, Sqlite};
use sea_orm::{ConnectOptions, ConnectionTrait, Database, DatabaseConnection, SqlxSqliteConnector, Statement};

fn database_url(path: &Path) -> String {
    format!("sqlite://{}?mode=rwc", path.to_string_lossy())
}

async fn database_connection(database_url: &str) -> Result<DatabaseConnection> {
    let mut option = ConnectOptions::new(database_url);
    option
        .max_connections(50)
        .min_connections(5)
        .acquire_timeout(Duration::from_secs(90));
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
}

async fn migrate_database(database_url: &str) -> Result<()> {
    // 注意此处使用内部构造的 DatabaseConnection，而不是通过 database_connection() 获取
    // 这是因为使用多个连接的 Connection 会导致奇怪的迁移顺序问题，而使用默认的连接选项不会
    let connection = Database::connect(database_url).await?;
    // 避免 https://github.com/amtoaer/bili-sync/issues/571 问题，迁移前根据 migration 确认当前版本
    // 如果用户从 2.6.0 以下版本直接升级，migration 不满足需求，直接报错而不执行迁移
    if connection
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
pub async fn setup_database(path: &Path) -> Result<DatabaseConnection> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await.context(
            "Failed to create config directory. Please check if you have granted necessary permissions to your folder.",
        )?;
    }
    let database_url = database_url(path);
    migrate_database(&database_url)
        .await
        .context("Failed to migrate database")?;
    database_connection(&database_url)
        .await
        .context("Failed to connect to database")
}
