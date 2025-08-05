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

use std::collections::VecDeque;
use std::fmt::Debug;
use std::future::Future;
use std::sync::Arc;

use bilibili::BiliClient;
use parking_lot::Mutex;
use sea_orm::DatabaseConnection;
use task::{http_server, video_downloader};
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

use crate::api::{LogHelper, MAX_HISTORY_LOGS};
use crate::config::{ARGS, VersionedConfig};
use crate::database::setup_database;
use crate::utils::init_logger;
use crate::utils::signal::terminate;

#[tokio::main]
async fn main() {
    let (connection, log_writer) = init().await;
    let bili_client = Arc::new(BiliClient::new());

    let token = CancellationToken::new();
    let tracker = TaskTracker::new();

    spawn_task(
        "HTTP 服务",
        http_server(connection.clone(), bili_client.clone(), log_writer),
        &tracker,
        token.clone(),
    );
    if !cfg!(debug_assertions) {
        spawn_task(
            "定时下载",
            video_downloader(connection, bili_client),
            &tracker,
            token.clone(),
        );
    }

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

/// 初始化日志系统、打印欢迎信息，初始化数据库连接和全局配置
async fn init() -> (Arc<DatabaseConnection>, LogHelper) {
    let (tx, _rx) = tokio::sync::broadcast::channel(30);
    let log_history = Arc::new(Mutex::new(VecDeque::with_capacity(MAX_HISTORY_LOGS + 1)));
    let log_writer = LogHelper::new(tx, log_history.clone());

    init_logger(&ARGS.log_level, Some(log_writer.clone()));
    info!("欢迎使用 Bili-Sync，当前程序版本：{}", config::version());
    info!("项目地址：https://github.com/amtoaer/bili-sync");
    let connection = Arc::new(
        match setup_database().await {
            Ok(result) => result,
            Err(error) => error!("数据库初始化失败：{}", error.to_string())
        }
    );
    info!("数据库初始化完成");
    VersionedConfig::init(&connection).await.expect("配置初始化失败");
    info!("配置初始化完成");

    (connection, log_writer)
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
