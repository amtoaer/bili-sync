use anyhow::Result;
use migration::{Migrator, MigratorTrait};
use sea_orm::{Database, DatabaseConnection};
use tokio::fs;

use crate::config::CONFIG_DIR;
pub async fn database_connection() -> Result<DatabaseConnection> {
    let target = CONFIG_DIR.join("data.sqlite");
    fs::create_dir_all(&*CONFIG_DIR).await?;
    Ok(Database::connect(format!("sqlite://{}?mode=rwc", target.to_str().unwrap())).await?)
}

pub async fn migrate_database(connection: &DatabaseConnection) -> Result<()> {
    Ok(Migrator::up(connection, None).await?)
}
