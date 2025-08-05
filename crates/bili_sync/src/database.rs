use anyhow::{anyhow, Context, Result};
use bili_sync_migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectOptions, Database, DatabaseConnection};

use crate::config::CONFIG_DIR;

fn database_url() -> String {
    format!("sqlite://{}?mode=rwc", CONFIG_DIR.join("data.sqlite").to_string_lossy())
}

async fn database_connection() -> Result<DatabaseConnection> {
    let mut option = ConnectOptions::new(database_url());
    option
        .max_connections(100)
        .min_connections(5)
        .acquire_timeout(std::time::Duration::from_secs(90));
    Ok(Database::connect(option).await?)
}

async fn migrate_database() -> Result<()> {
    // 注意此处使用内部构造的 DatabaseConnection，而不是通过 database_connection() 获取
    // 这是因为使用多个连接的 Connection 会导致奇怪的迁移顺序问题，而使用默认的连接选项不会
    let connection = Database::connect(database_url()).await?;
    Ok(Migrator::up(&connection, None).await?)
}

/// 进行数据库迁移并获取数据库连接，供外部使用
pub async fn setup_database() -> Result<DatabaseConnection> {
    tokio::fs::create_dir_all(CONFIG_DIR.as_path())
        .await
        .context("Failed to create config directory. Please check if you have granted necessary permissions to your folder.")?;
    migrate_database().await.context("Failed to migrate database")?;
    database_connection().await.context("Failed to connect to database")
}
