use std::sync::Arc;

use sea_orm::DatabaseConnection;
use tokio::time;

use crate::bilibili::{self, BiliClient};
use crate::config::config_template_owned;
use crate::workflow::process_video_source;

/// 启动周期下载视频的任务
pub async fn video_downloader(connection: Arc<DatabaseConnection>, bili_client: Arc<BiliClient>) {
    let mut anchor = chrono::Local::now().date_naive();
    loop {
        info!("开始执行本轮视频下载任务..");
        let config_template = config_template_owned();
        let config = &config_template.config;
        let video_sources = config.as_video_sources();
        'inner: {
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
                if let Err(e) = bili_client.check_refresh().await {
                    error!("检查刷新 Credential 遇到错误：{:#}，等待下一轮执行", e);
                    break 'inner;
                }
                anchor = chrono::Local::now().date_naive();
            }
            for (args, path) in &video_sources {
                if let Err(e) = process_video_source(*args, &bili_client, path, &connection).await {
                    error!("处理过程遇到错误：{:#}", e);
                }
            }
            info!("本轮任务执行完毕，等待下一轮执行");
        }
        time::sleep(time::Duration::from_secs(config.interval)).await;
    }
}
