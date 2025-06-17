use std::sync::Arc;

use sea_orm::DatabaseConnection;
use tokio::time;

use crate::bilibili::{self, BiliClient};
use crate::config::VersionedConfig;
use crate::utils::model::get_enabled_video_sources;
use crate::workflow::process_video_source;

pub static DOWNLOADER_TASK_RUNNING: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// 启动周期下载视频的任务
pub async fn video_downloader(connection: Arc<DatabaseConnection>, bili_client: Arc<BiliClient>) {
    let mut anchor = chrono::Local::now().date_naive();
    loop {
        info!("开始执行本轮视频下载任务..");
        let _lock = DOWNLOADER_TASK_RUNNING.lock().await;
        let config = VersionedConfig::get().load_full();
        'inner: {
            if let Err(e) = config.check() {
                error!("配置检查失败，跳过本轮执行：\n{:#}", e);
                break 'inner;
            }
            match bili_client.wbi_img().await.map(|wbi_img| wbi_img.into()) {
                Ok(Some(mixin_key)) => bilibili::set_global_mixin_key(mixin_key),
                Ok(_) => {
                    error!("解析 mixin key 失败，等待下一轮执行");
                    break 'inner;
                }
                Err(e) => {
                    error!("获取 mixin key 遇到错误：{:#}，等待下一轮执行", e);
                    break 'inner;
                }
            };
            if anchor != chrono::Local::now().date_naive() {
                if let Err(e) = bili_client.check_refresh(&connection).await {
                    error!("检查刷新 Credential 遇到错误：{:#}，等待下一轮执行", e);
                    break 'inner;
                }
                anchor = chrono::Local::now().date_naive();
            }
            let Ok(video_sources) = get_enabled_video_sources(&connection).await else {
                error!("获取视频源列表失败，等待下一轮执行");
                break 'inner;
            };
            if video_sources.is_empty() {
                info!("没有可用的视频源，等待下一轮执行");
                break 'inner;
            }
            for video_source in video_sources {
                if let Err(e) = process_video_source(video_source, &bili_client, &connection).await {
                    error!("处理 {} 时遇到错误：{:#}，等待下一轮执行", "test", e);
                }
            }
            info!("本轮任务执行完毕，等待下一轮执行");
        }
        drop(_lock);
        time::sleep(time::Duration::from_secs(config.interval)).await;
    }
}
