use sea_orm::{ConnectOptions, Database, DatabaseConnection};

use crate::Result;
pub async fn database_connection() -> Result<DatabaseConnection> {
    let opt = ConnectOptions::new("sqlite://./data.sqlite?mode=rwc");
    Ok(Database::connect(opt).await?)
}
