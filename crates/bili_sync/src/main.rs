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

use std::io;
use std::path::PathBuf;

use once_cell::sync::Lazy;
use sea_orm::DatabaseConnection;
use tokio::{signal, time};

use crate::adapter::Args;
use crate::bilibili::BiliClient;
use crate::config::{ARGS, CONFIG};
use crate::database::{database_connection, migrate_database};
use crate::utils::init_logger;
use crate::workflow::process_video_list;

#[tokio::main]
async fn main() {
    init();
    let connection = setup_database().await;
    let bili_client = BiliClient::new();
    let params = collect_task_params();
    let task = spawn_periodic_task(bili_client, params, connection);
    handle_shutdown(task).await;
}

/// 初始化日志系统，加载命令行参数和配置文件
fn init() {
    Lazy::force(&ARGS);
    init_logger(&ARGS.log_level);
    Lazy::force(&CONFIG);
}

/// 迁移数据库并获取数据库连接
async fn setup_database() -> DatabaseConnection {
    migrate_database().await.expect("数据库迁移失败");
    database_connection().await.expect("获取数据库连接失败")
}

/// 收集任务执行所需的参数（下载类型和保存路径）
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

/// 启动周期下载的任务
fn spawn_periodic_task(
    bili_client: BiliClient,
    params: Vec<(Args<'static>, &'static PathBuf)>,
    connection: DatabaseConnection,
) -> tokio::task::JoinHandle<()> {
    let mut anchor = chrono::Local::now().date_naive();
    tokio::spawn(async move {
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
                    if let Err(e) = process_video_list(*args, &bili_client, path, &connection).await {
                        error!("处理过程遇到错误：{e}");
                    }
                }
                info!("本轮任务执行完毕，等待下一轮执行");
            }
            time::sleep(time::Duration::from_secs(CONFIG.interval)).await;
        }
    })
}

/// 处理终止信号
async fn handle_shutdown(task: tokio::task::JoinHandle<()>) {
    let _ = terminate().await;
    info!("接收到终止信号，正在终止任务..");
    task.abort();
    match task.await {
        Err(e) if !e.is_cancelled() => error!("任务终止时遇到错误：{}", e),
        _ => {
            info!("任务成功终止，退出程序..");
        }
    }
}

#[cfg(target_family = "windows")]
async fn terminate() -> io::Result<()> {
    signal::ctrl_c().await
}

/// ctrl + c 发送的是 SIGINT 信号，docker stop 发送的是 SIGTERM 信号，都需要处理
#[cfg(target_family = "unix")]
async fn terminate() -> io::Result<()> {
    use tokio::select;

    let mut term = signal::unix::signal(signal::unix::SignalKind::terminate())?;
    let mut int = signal::unix::signal(signal::unix::SignalKind::interrupt())?;
    select! {
        _ = term.recv() => Ok(()),
        _ = int.recv() => Ok(()),
    }
}
