#[macro_use]
extern crate tracing;

mod adapter;
mod api;
mod bilibili;
mod config;
mod database;
mod downloader;
mod error;
mod initialization;
mod task;
mod utils;
mod workflow;

use std::fmt::Debug;
use std::future::Future;
use std::sync::Arc;

use once_cell::sync::Lazy;
use task::{http_server, video_downloader};
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

use crate::bilibili::bangumi::Bangumi;
use crate::config::{ARGS, CONFIG};
use crate::database::setup_database;
use crate::utils::init_logger;
use crate::utils::signal::terminate;

#[tokio::main]
async fn main() {
    init();
    
    let connection = Arc::new(setup_database().await);
    let token = CancellationToken::new();
    let tracker = TaskTracker::new();

    spawn_task("HTTP 服务", http_server(connection.clone()), &tracker, token.clone());
    spawn_task("定时下载", video_downloader(connection), &tracker, token.clone());

    tracker.close();
    handle_shutdown(tracker, token).await
}

/// 调试番剧API
#[allow(dead_code)]
async fn debug_bangumi_api(season_id: &str) {
    info!("调试番剧API，season_id: {}", season_id);
    
    // 创建客户端
    let bili_client = bilibili::BiliClient::new(String::new());
    
    // 获取番剧信息
    let bangumi = Bangumi::new(&bili_client, None, Some(season_id.to_string()), None);
    
    match bangumi.get_season_info().await {
        Ok(season_info) => {
            let title = season_info["title"].as_str().unwrap_or_default().to_string();
            info!("番剧详情获取成功！");
            info!("标题: {}", title);
            info!("封面: {}", season_info["cover"].as_str().unwrap_or_default());
        
            // 获取分集信息
            match bangumi.get_episodes().await {
                Ok(episodes) => {
                    info!("获取到 {} 集番剧", episodes.len());
                    for (i, ep) in episodes.iter().enumerate() {
                        info!("第{}集: {} (EP{}) BV号: {}", i+1, ep.title, ep.id, ep.bvid);
                    }
                    
                    // 尝试手动下载第一集
                    if !episodes.is_empty() {
                        let first_ep = &episodes[0];
                        test_download_episode(&bili_client, first_ep, &title).await;
                    }
                },
                Err(e) => {
                    error!("获取番剧分集失败: {:?}", e);
                }
            }
        },
        Err(e) => {
            error!("获取番剧详情失败: {:?}", e);
        }
    }
}

/// 测试下载一集番剧
#[allow(dead_code)]
async fn test_download_episode(_client: &bilibili::BiliClient, episode: &crate::bilibili::bangumi::BangumiEpisode, title: &str) {
    info!("测试下载番剧: {} - {}", title, episode.title);
    
    // 设置下载路径
    let download_path = std::path::Path::new("D:/Downloads/假面骑士");
    
    // 创建目录
    if let Err(e) = tokio::fs::create_dir_all(download_path).await {
        error!("创建下载目录失败: {:?}", e);
        return;
    }
    
    // 创建一个测试文件，验证目录是否可写
    let test_file_path = download_path.join("测试文件.txt");
    if let Err(e) = tokio::fs::write(&test_file_path, "这是一个测试文件").await {
        error!("写入测试文件失败: {:?}", e);
    } else {
        info!("成功写入测试文件: {}", test_file_path.display());
    }
    
    // 手动下载视频
    let video_path = download_path.join(format!("{} - {}.mp4", title, episode.title));
    info!("视频下载路径: {}", video_path.display());
    
    // 手动构建请求并下载
    info!("视频将被下载到: {}", download_path.display());
    info!("你可以手动使用BBDown工具来下载：BBDown av{} --config \"D:/配置目录\"", episode.aid);
}

fn spawn_task(
    task_name: &'static str,
    task: impl Future<Output = impl Debug> + Send + 'static,
    tracker: &TaskTracker,
    token: CancellationToken,
) {
    tracker.spawn(async move {
        tokio::select! {
            res = task => {
                error!("「{}」异常结束，返回结果为：「{:?}」，取消其它仍在执行的任务..", task_name, res);
                token.cancel();
            },
            _ = token.cancelled() => {
                info!("「{}」接收到取消信号，终止运行..", task_name);
            }
        }
    });
}

/// 初始化日志系统，打印欢迎信息，加载配置文件
fn init() {
    init_logger(&ARGS.log_level);
    info!("欢迎使用 Bili-Sync，当前程序版本：{}", config::version());
    info!("项目地址：https://github.com/amtoaer/bili-sync");
    Lazy::force(&CONFIG);
}

async fn handle_shutdown(tracker: TaskTracker, token: CancellationToken) {
    tokio::select! {
        _ = tracker.wait() => {
            error!("所有任务均已终止，程序退出")
        }
        _ = terminate() => {
            info!("接收到终止信号，正在终止任务..");
            token.cancel();
            tracker.wait().await;
            info!("所有任务均已终止，程序退出");
        }
    }
}
