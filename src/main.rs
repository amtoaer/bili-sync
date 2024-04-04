#[macro_use]
extern crate log;

mod bilibili;
mod config;
mod core;
mod database;
mod downloader;
mod error;

use once_cell::sync::Lazy;

use self::bilibili::BiliClient;
use self::config::CONFIG;
use self::core::command::process_favorite_list;
use self::database::{database_connection, migrate_database};

#[tokio::main]
async fn main() -> ! {
    env_logger::init();
    Lazy::force(&CONFIG);
    let mut anchor = chrono::Local::now().date_naive();
    let bili_client = BiliClient::new();
    let connection = database_connection().await.unwrap();
    migrate_database(&connection).await.unwrap();
    loop {
        if anchor != chrono::Local::now().date_naive() {
            if let Err(e) = bili_client.check_refresh().await {
                error!("Error: {e}");
                tokio::time::sleep(std::time::Duration::from_secs(CONFIG.interval)).await;
                continue;
            }
            anchor = chrono::Local::now().date_naive();
        }
        for (fid, path) in &CONFIG.favorite_list {
            let res = process_favorite_list(&bili_client, fid, path, &connection).await;
            if let Err(e) = res {
                error!("Error: {e}");
            }
        }

        tokio::time::sleep(std::time::Duration::from_secs(CONFIG.interval)).await;
    }
}
