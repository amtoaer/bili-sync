#[macro_use]
extern crate log;

mod bilibili;
mod config;
mod core;
mod database;
mod downloader;
mod error;

use env_logger::Env;
use once_cell::sync::Lazy;

use self::bilibili::BiliClient;
use self::config::CONFIG;
use self::core::command::process_favorite_list;
use self::database::{database_connection, migrate_database};

#[tokio::main]
async fn main() -> ! {
    env_logger::init_from_env(Env::default().default_filter_or("None,bili_sync=info"));
    Lazy::force(&CONFIG);
    let mut anchor = chrono::Local::now().date_naive();
    let bili_client = BiliClient::new();
    let connection = database_connection().await.unwrap();
    migrate_database(&connection).await.unwrap();
    loop {
        if anchor != chrono::Local::now().date_naive() {
            if let Err(e) = bili_client.check_refresh().await {
                error!("检查刷新 Credential 遇到错误：{e}，等待下一轮执行");
                tokio::time::sleep(std::time::Duration::from_secs(CONFIG.interval)).await;
                continue;
            }
            anchor = chrono::Local::now().date_naive();
        }
        for (fid, path) in &CONFIG.favorite_list {
            if let Err(e) = process_favorite_list(&bili_client, fid, path, &connection).await {
                // 可预期的错误都被内部处理了，这里漏出来应该是大问题
                error!("处理收藏夹 {fid} 时遇到非预期的错误：{e}");
            }
        }
        info!("所有收藏夹处理完毕，等待下一轮执行");
        tokio::time::sleep(std::time::Duration::from_secs(CONFIG.interval)).await;
    }
}
