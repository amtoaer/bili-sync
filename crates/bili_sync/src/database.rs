use anyhow::Result;
use bili_sync_migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectOptions, Database, DatabaseConnection};

use crate::config::CONFIG_DIR;

fn database_url() -> String {
    // 确保配置目录存在
    if !CONFIG_DIR.exists() {
        std::fs::create_dir_all(&*CONFIG_DIR).expect("创建配置目录失败");
    }
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
    // 检查数据库文件是否存在，不存在则会在连接时自动创建
    let db_path = CONFIG_DIR.join("data.sqlite");
    if !db_path.exists() {
        info!("数据库文件不存在，将创建新的数据库");
    } else {
        info!("检测到现有数据库文件，将在必要时应用迁移");
    }
    
    // 注意此处使用内部构造的 DatabaseConnection，而不是通过 database_connection() 获取
    // 这是因为使用多个连接的 Connection 会导致奇怪的迁移顺序问题，而使用默认的连接选项不会
    let connection = Database::connect(database_url()).await?;
    
    // 确保所有迁移都应用
    Ok(Migrator::up(&connection, None).await?)
}

/// 进行数据库迁移并获取数据库连接，供外部使用
pub async fn setup_database() -> DatabaseConnection {
    migrate_database().await.expect("数据库迁移失败");
    database_connection().await.expect("获取数据库连接失败")
}
