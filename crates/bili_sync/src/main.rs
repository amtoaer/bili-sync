#[macro_use]
extern crate tracing;

mod adapter;
mod api;
mod bilibili;
mod config;
mod database;
mod downloader;
mod error;
mod notifier;
mod task;
mod utils;
mod workflow;

use std::collections::VecDeque;
use std::fmt::Debug;
use std::future::Future;
use std::sync::Arc;

use anyhow::{Context, Result, bail};
use bilibili::BiliClient;
use parking_lot::RwLock;
use sea_orm::DatabaseConnection;
use task::{http_server, video_downloader};
use tokio::process::Command;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

use crate::api::{LogHelper, MAX_HISTORY_LOGS};
use crate::config::{ARGS, CONFIG_DIR, VersionedConfig};
use crate::database::setup_database;
use crate::utils::init_logger;
use crate::utils::signal::terminate;

#[tokio::main]
async fn main() {
    let (bili_client, connection, log_writer) = match init().await {
        Ok(res) => res,
        Err(e) => {
            error!("初始化失败：{:#}", e);
            return;
        }
    };

    let token = CancellationToken::new();
    let tracker = TaskTracker::new();

    spawn_task(
        "HTTP 服务",
        http_server(connection.clone(), bili_client.clone(), log_writer),
        &tracker,
        token.clone(),
    );

    spawn_task(
        "定时下载",
        video_downloader(connection.clone(), bili_client),
        &tracker,
        token.clone(),
    );

    tracker.close();
    handle_shutdown(connection, tracker, token).await
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
async fn init() -> Result<(Arc<BiliClient>, DatabaseConnection, LogHelper)> {
    let (tx, _rx) = tokio::sync::broadcast::channel(30);
    let log_history = Arc::new(RwLock::new(VecDeque::with_capacity(MAX_HISTORY_LOGS + 1)));
    let log_writer = LogHelper::new(tx, log_history.clone());

    init_logger(&ARGS.log_level, Some(log_writer.clone()));
    info!("欢迎使用 Bili-Sync，当前程序版本：{}", config::version());
    info!("项目地址：https://github.com/amtoaer/bili-sync");

    let ffmpeg_path = ARGS.ffmpeg_path.as_deref().unwrap_or("ffmpeg");
    let ffmpeg_exists = Command::new(ffmpeg_path)
        .arg("-version")
        .output()
        .await
        .map(|output| output.status.success())
        .unwrap_or(false);
    if !ffmpeg_exists {
        bail!("ffmpeg 不存在或无法执行，请确保已正确安装 ffmpeg，并且 {ffmpeg_path} 命令可用");
    }

    let connection = setup_database(&CONFIG_DIR.join("data.sqlite"))
        .await
        .context("数据库初始化失败")?;
    info!("数据库初始化完成");
    VersionedConfig::init(&connection).await.context("配置初始化失败")?;
    info!("配置初始化完成");

    Ok((Arc::new(BiliClient::new()), connection, log_writer))
}

async fn handle_shutdown(connection: DatabaseConnection, tracker: TaskTracker, token: CancellationToken) {
    tokio::select! {
        _ = tracker.wait() => {
            error!("所有任务均已终止..")
        }
        _ = terminate() => {
            info!("接收到终止信号，开始终止任务..");
            token.cancel();
            tracker.wait().await;
            info!("所有任务均已终止..");
        }
    }
    info!("正在关闭数据库连接..");
    match connection.close().await {
        Ok(()) => info!("数据库连接已关闭，程序结束"),
        Err(e) => error!("关闭数据库连接时遇到错误：{:#}，程序异常结束", e),
    }
}
