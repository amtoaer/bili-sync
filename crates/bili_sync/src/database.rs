use anyhow::Result;
use bili_sync_migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use tokio::fs;

use crate::config::CONFIG_DIR;
pub async fn database_connection() -> Result<DatabaseConnection> {
    let target = CONFIG_DIR.join("data.sqlite");
    fs::create_dir_all(&*CONFIG_DIR).await?;
    let mut option = ConnectOptions::new(format!("sqlite://{}?mode=rwc", target.to_str().unwrap()));
    option
        .max_connections(100)
        .min_connections(5)
        .acquire_timeout(std::time::Duration::from_secs(90));
    Ok(Database::connect(option).await?)
}

pub async fn migrate_database(connection: &DatabaseConnection) -> Result<()> {
    Ok(Migrator::up(connection, None).await?)
}
