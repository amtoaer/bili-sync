use sea_orm::DatabaseConnection;
use anyhow::Result;

use crate::config::Config;
use crate::adapter::bangumi::BangumiSource;

pub async fn init_sources(config: &Config, conn: &DatabaseConnection) -> Result<(), anyhow::Error> {
    // 初始化番剧源
    for bangumi_config in &config.bangumi {
        let _ = BangumiSource::init_to_db(bangumi_config, conn).await?;
    }
    Ok(())
} 