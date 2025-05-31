#[macro_use]
extern crate tracing;

mod adapter;
mod api;
mod bilibili;
mod config;
mod database;
mod downloader;
mod error;
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
    // spawn_task("定时下载", video_downloader(connection), &tracker, token.clone());

    tracker.close();
    handle_shutdown(tracker, token).await
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
