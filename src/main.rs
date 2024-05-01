#[macro_use]
extern crate tracing;

mod bilibili;
mod config;
mod core;
mod database;
mod downloader;
mod error;

use once_cell::sync::Lazy;
use tracing_subscriber::util::SubscriberInitExt;

use crate::bilibili::BiliClient;
use crate::config::CONFIG;
use crate::core::command::{process_favorite_list, SCAN_ONLY};
use crate::database::{database_connection, migrate_database};

#[tokio::main]
async fn main() -> ! {
    let default_log_level = std::env::var("RUST_LOG").unwrap_or("None,bili_sync=info".to_owned());
    tracing_subscriber::fmt::Subscriber::builder()
        .with_env_filter(tracing_subscriber::EnvFilter::builder().parse_lossy(default_log_level))
        .with_timer(tracing_subscriber::fmt::time::ChronoLocal::new(
            "%Y-%m-%d %H:%M:%S%.3f".to_owned(),
        ))
        .finish()
        .try_init()
        .expect("初始化日志失败");
    Lazy::force(&SCAN_ONLY);
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
