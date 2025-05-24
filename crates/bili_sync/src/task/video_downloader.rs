use std::sync::Arc;

use sea_orm::DatabaseConnection;
use tokio::time;

use crate::adapter::{
    init_favorite_sources,
    init_collection_sources,
    init_submission_sources,
    init_watch_later_source
};
use crate::bilibili::{self, BiliClient};
use crate::config::CONFIG;
use crate::workflow::process_video_source;
use crate::initialization;

/// 启动周期下载视频的任务
pub async fn video_downloader(connection: Arc<DatabaseConnection>) {
    let mut anchor = chrono::Local::now().date_naive();
    let bili_client = BiliClient::new(String::new());
    
    // 在启动时初始化所有视频源
    if let Err(e) = initialization::init_sources(&CONFIG, &connection).await {
        error!("初始化番剧源失败: {}", e);
    } else {
        info!("初始化番剧源成功");
    }
    
    if let Err(e) = init_favorite_sources(&connection, &CONFIG.favorite_list).await {
        error!("初始化收藏夹源失败: {:#}", e);
    } else {
        info!("初始化收藏夹源成功");
    }
    
    if let Err(e) = init_collection_sources(&connection, &CONFIG.collection_list).await {
        error!("初始化合集源失败: {:#}", e);
    } else {
        info!("初始化合集源成功");
    }
    
    if let Err(e) = init_submission_sources(&connection, &CONFIG.submission_list).await {
        error!("初始化UP主投稿源失败: {:#}", e);
    } else {
        info!("初始化UP主投稿源成功");
    }
    
    if let Err(e) = init_watch_later_source(&connection, &CONFIG.watch_later).await {
        error!("初始化稍后观看源失败: {:#}", e);
    } else {
        info!("初始化稍后观看源成功");
    }
    
    loop {
        // 尝试获取最新的配置（如果配置已被重载）
        // 注意：由于我们使用Lazy，全局CONFIG并不会自动更新
        // 所以这段代码实际上没有效果，但为未来可能的改进保留
        let config = crate::config::reload_config();
        
        // 重新初始化所有视频源
        // 注意：需要确保源初始化是幂等的（可以重复执行而不会有副作用）
        if let Err(e) = initialization::init_sources(&config, &connection).await {
            error!("重新初始化番剧源失败: {}", e);
        }
        
        if let Err(e) = init_favorite_sources(&connection, &config.favorite_list).await {
            error!("重新初始化收藏夹源失败: {:#}", e);
        }
        
        if let Err(e) = init_collection_sources(&connection, &config.collection_list).await {
            error!("重新初始化合集源失败: {:#}", e);
        }
        
        if let Err(e) = init_submission_sources(&connection, &config.submission_list).await {
            error!("重新初始化UP主投稿源失败: {:#}", e);
        }
        
        if let Err(e) = init_watch_later_source(&connection, &config.watch_later).await {
            error!("重新初始化稍后观看源失败: {:#}", e);
        }
        
        let video_sources = config.as_video_sources();
        
        info!("开始执行本轮视频下载任务..");
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
