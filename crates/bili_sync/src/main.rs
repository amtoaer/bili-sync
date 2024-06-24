#[macro_use]
extern crate tracing;

mod bilibili;
mod config;
mod core;
mod database;
mod downloader;
mod error;
mod model;

use std::time::Duration;

use config::ARGS;
use once_cell::sync::Lazy;
use tokio::time;

use crate::bilibili::BiliClient;
use crate::config::CONFIG;
use crate::core::command::{process_collection, process_favorite_list};
use crate::core::utils::init_logger;
use crate::database::{database_connection, migrate_database};

#[tokio::main]
async fn main() {
    Lazy::force(&ARGS);
    init_logger(&ARGS.log_level);
    Lazy::force(&CONFIG);
    let mut anchor = chrono::Local::now().date_naive();
    let bili_client = BiliClient::new();
    let connection = database_connection().await.unwrap();
    migrate_database(&connection).await.unwrap();
    loop {
        if let Err(e) = bili_client.is_login().await {
            error!("检查登录状态时遇到错误：{e}，等待下一轮执行");
            time::sleep(Duration::from_secs(CONFIG.interval)).await;
            continue;
        }
        if anchor != chrono::Local::now().date_naive() {
            if let Err(e) = bili_client.check_refresh().await {
                error!("检查刷新 Credential 遇到错误：{e}，等待下一轮执行");
                time::sleep(Duration::from_secs(CONFIG.interval)).await;
                continue;
            }
            anchor = chrono::Local::now().date_naive();
        }
        for (fid, path) in &CONFIG.favorite_list {
            if let Err(e) = process_favorite_list(&bili_client, fid, path, &connection).await {
                error!("处理收藏夹 {fid} 时遇到非预期的错误：{e}");
            }
        }
        info!("所有收藏夹处理完毕");
        for (collection, path) in &CONFIG.collection_list {
            if let Err(e) = process_collection(&bili_client, collection, path, &connection).await {
                error!("处理合集 {collection:?} 时遇到非预期的错误：{e}");
            }
        }
        info!("所有合集处理完毕");
        info!("等待 {} 分钟后进行下一次扫描", CONFIG.interval);
        tokio::time::sleep(std::time::Duration::from_secs(CONFIG.interval)).await;
    }
}
