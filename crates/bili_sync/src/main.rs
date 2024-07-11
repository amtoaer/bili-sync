#[macro_use]
extern crate tracing;

mod adapter;
mod bilibili;
mod config;
mod database;
mod downloader;
mod error;
mod utils;
mod workflow;
use std::time::Duration;

use once_cell::sync::Lazy;
use tokio::time;

use crate::adapter::Args;
use crate::bilibili::BiliClient;
use crate::config::{ARGS, CONFIG};
use crate::database::{database_connection, migrate_database};
use crate::utils::init_logger;
use crate::workflow::process_video_list;

#[tokio::main]
async fn main() {
    init_logger(&ARGS.log_level);
    Lazy::force(&CONFIG);
    migrate_database().await.expect("数据库迁移失败");
    let connection = database_connection().await.expect("获取数据库连接失败");
    let mut anchor = chrono::Local::now().date_naive();
    let bili_client = BiliClient::new();
    let watch_later_config = &CONFIG.watch_later;
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
            if let Err(e) = process_video_list(Args::Favorite { fid }, &bili_client, path, &connection).await {
                error!("处理收藏夹 {fid} 时遇到非预期的错误：{e}");
            }
        }
        info!("所有收藏夹处理完毕");
        for (collection_item, path) in &CONFIG.collection_list {
            if let Err(e) =
                process_video_list(Args::Collection { collection_item }, &bili_client, path, &connection).await
            {
                error!("处理合集 {collection_item:?} 时遇到非预期的错误：{e}");
            }
        }
        info!("所有合集处理完毕");
        if watch_later_config.enabled {
            if let Err(e) =
                process_video_list(Args::WatchLater, &bili_client, &watch_later_config.path, &connection).await
            {
                error!("处理稍后再看时遇到非预期的错误：{e}");
            }
        }
        info!("稍后再看处理完毕");
        info!("本轮任务执行完毕，等待下一轮执行");
        tokio::time::sleep(std::time::Duration::from_secs(CONFIG.interval)).await;
    }
}
