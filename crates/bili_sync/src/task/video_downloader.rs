use std::path::PathBuf;
use std::sync::Arc;

use sea_orm::DatabaseConnection;
use tokio::time;

use crate::adapter::Args;
use crate::bilibili::{self, BiliClient};
use crate::config::CONFIG;
use crate::workflow::process_video_source;

/// 启动周期下载视频的任务
pub async fn video_downloader(connection: Arc<DatabaseConnection>) {
    let mut anchor = chrono::Local::now().date_naive();
    let bili_client = BiliClient::new();
    let params = collect_task_params();
    loop {
        'inner: {
            match bili_client.wbi_img().await.map(|wbi_img| wbi_img.into()) {
                Ok(Some(mixin_key)) => bilibili::set_global_mixin_key(mixin_key),
                Ok(_) => {
                    error!("解析 mixin key 失败，等待下一轮执行");
                    break 'inner;
                }
                Err(e) => {
                    error!("获取 mixin key 遇到错误：{e}，等待下一轮执行");
                    break 'inner;
                }
            };
            if anchor != chrono::Local::now().date_naive() {
                if let Err(e) = bili_client.check_refresh().await {
                    error!("检查刷新 Credential 遇到错误：{e}，等待下一轮执行");
                    break 'inner;
                }
                anchor = chrono::Local::now().date_naive();
            }
            for (args, path) in &params {
                if let Err(e) = process_video_source(*args, &bili_client, path, &connection).await {
                    error!("处理过程遇到错误：{e}");
                }
            }
            info!("本轮任务执行完毕，等待下一轮执行");
        }
        time::sleep(time::Duration::from_secs(CONFIG.interval)).await;
    }
}

/// 构造下载视频任务执行所需的参数（下载类型和保存路径）
fn collect_task_params() -> Vec<(Args<'static>, &'static PathBuf)> {
    let mut params = Vec::new();
    CONFIG
        .favorite_list
        .iter()
        .for_each(|(fid, path)| params.push((Args::Favorite { fid }, path)));
    CONFIG
        .collection_list
        .iter()
        .for_each(|(collection_item, path)| params.push((Args::Collection { collection_item }, path)));
    if CONFIG.watch_later.enabled {
        params.push((Args::WatchLater, &CONFIG.watch_later.path));
    }
    CONFIG
        .submission_list
        .iter()
        .for_each(|(upper_id, path)| params.push((Args::Submission { upper_id }, path)));
    params
}
