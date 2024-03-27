use sea_orm::{Database, DatabaseConnection};

use crate::Result;
pub async fn database_connection() -> Result<DatabaseConnection> {
    Ok(Database::connect("sqlite://./data.sqlite?mode=rwc").await?)
}
