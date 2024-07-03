use anyhow::Result;
use bili_sync_migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectOptions, Database, DatabaseConnection};

use crate::config::CONFIG_DIR;

fn database_url() -> String {
    format!("sqlite://{}?mode=rwc", CONFIG_DIR.join("data.sqlite").to_string_lossy())
}

pub async fn database_connection() -> Result<DatabaseConnection> {
    let mut option = ConnectOptions::new(database_url());
    option
        .max_connections(100)
        .min_connections(5)
        .acquire_timeout(std::time::Duration::from_secs(90));
    Ok(Database::connect(option).await?)
}

pub async fn migrate_database() -> Result<()> {
    // 注意此处使用内部构造的 DatabaseConnection，而不是通过 database_connection() 获取
    // 这是因为使用多个连接的 Connection 会导致奇怪的迁移顺序问题，而使用默认的连接选项不会
    let connection = Database::connect(database_url()).await?;
    Ok(Migrator::up(&connection, None).await?)
}
