use migration::{Migrator, MigratorTrait};
use sea_orm::{Database, DatabaseConnection};
use tokio::fs;

use crate::Result;
pub async fn database_connection() -> Result<DatabaseConnection> {
    let config_dir = dirs::config_dir().ok_or("No config path found")?;
    let target = config_dir.join("bili-sync").join("data.sqlite");
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).await?;
    }
    Ok(Database::connect(format!(
        "sqlite://{}?mode=rwc",
        config_dir.join("bili-sync").join("data.sqlite").to_str().unwrap()
    ))
    .await?)
}

pub async fn migrate_database(connection: &DatabaseConnection) -> Result<()> {
    Ok(Migrator::up(connection, None).await?)
}
