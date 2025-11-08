use std::future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use sea_orm::DatabaseConnection;
use serde::Serialize;
use tokio::sync::{OnceCell, watch};
use tokio_cron_scheduler::{Job, JobScheduler};

use crate::adapter::VideoSource;
use crate::bilibili::{self, BiliClient, BiliError};
use crate::config::{Config, TEMPLATE, Trigger, VersionedConfig};
use crate::notifier::NotifierAllExt;
use crate::utils::model::get_enabled_video_sources;
use crate::workflow::process_video_source;

static INSTANCE: OnceCell<DownloadTaskManager> = OnceCell::const_new();

pub struct DownloadTaskManager {
    sched: Arc<JobScheduler>,
    task_context: TaskContext,
}

#[derive(Serialize, Default, Clone, Copy, Debug)]
pub struct TaskStatus {
    is_running: bool,
    last_run: Option<chrono::DateTime<chrono::Local>>,
    last_finish: Option<chrono::DateTime<chrono::Local>>,
    next_run: Option<chrono::DateTime<chrono::Local>>,
}

#[derive(Clone)]
struct TaskContext {
    connection: DatabaseConnection,
    bili_client: Arc<BiliClient>,
    running: Arc<tokio::sync::Mutex<()>>,
    status_tx: watch::Sender<TaskStatus>,
    status_rx: watch::Receiver<TaskStatus>,
    updating: Arc<tokio::sync::Mutex<Option<uuid::Uuid>>>,
}

impl DownloadTaskManager {
    pub async fn init(
        connection: DatabaseConnection,
        bili_client: Arc<BiliClient>,
    ) -> Result<&'static DownloadTaskManager> {
        INSTANCE
            .get_or_try_init(|| DownloadTaskManager::new(connection, bili_client))
            .await
    }

    pub fn get() -> &'static DownloadTaskManager {
        INSTANCE.get().expect("DownloadTaskManager is not initialized")
    }

    pub fn subscribe(&self) -> watch::Receiver<TaskStatus> {
        self.task_context.status_rx.clone()
    }

    pub async fn oneshot(&self) -> Result<()> {
        let task_context = self.task_context.clone();
        let _ = self
            .sched
            .add(Job::new_one_shot_async(Duration::from_secs(0), move |uuid, l| {
                DownloadTaskManager::download_video_task(uuid, l, task_context.clone())
            })?)
            .await?;
        Ok(())
    }

    pub(self) async fn start(&self) -> Result<()> {
        let _ = self.sched.start().await?;
        Ok(())
    }

    async fn new(connection: DatabaseConnection, bili_client: Arc<BiliClient>) -> Result<Self> {
        let sched = Arc::new(JobScheduler::new().await?);
        let (status_tx, status_rx) = watch::channel(TaskStatus::default());
        let (running, updating) = (
            Arc::new(tokio::sync::Mutex::new(())),
            Arc::new(tokio::sync::Mutex::new(None)),
        );
        // 固定每天凌晨 1 点更新凭据
        let (connection_clone, bili_client_clone, running_clone) =
            (connection.clone(), bili_client.clone(), running.clone());
        sched
            .add(Job::new_async_tz(
                "0 0 1 * * *",
                chrono::Local,
                move |_uuid, mut _l| {
                    DownloadTaskManager::check_and_refresh_credential_task(
                        connection_clone.clone(),
                        bili_client_clone.clone(),
                        running_clone.clone(),
                    )
                },
            )?)
            .await?;
        let task_context = TaskContext {
            connection: connection.clone(),
            bili_client: bili_client.clone(),
            running: running.clone(),
            status_tx: status_tx.clone(),
            status_rx: status_rx.clone(),
            updating: updating.clone(),
        };
        // 根据 interval 策略分发不同触发机制的视频下载任务，并记录任务 ID
        let mut rx = VersionedConfig::get().subscribe();
        let initial_config = rx.borrow_and_update().clone();
        let task_context_clone = task_context.clone();
        let job_run = move |uuid, l| DownloadTaskManager::download_video_task(uuid, l, task_context_clone.clone());
        let job = match &initial_config.interval {
            Trigger::Interval(interval) => Job::new_repeated_async(Duration::from_secs(*interval), job_run)?,
            Trigger::Cron(cron) => Job::new_async_tz(cron, chrono::Local, job_run)?,
        };
        let download_task_id = sched.add(job).await?;
        *updating.lock().await = Some(download_task_id);
        // 发起一个一次性的任务，更新一下下次运行的时间
        let task_context_clone = task_context.clone();
        sched
            .add(Job::new_one_shot_async(Duration::from_secs(0), move |_uuid, mut l| {
                let task_context = task_context_clone.clone();
                Box::pin(async move {
                    let old_status = task_context.status_rx.borrow().clone();
                    let next_run = l
                        .next_tick_for_job(download_task_id)
                        .await
                        .ok()
                        .flatten()
                        .map(|dt| dt.with_timezone(&chrono::Local));
                    let _ = task_context.status_tx.send(TaskStatus { next_run, ..old_status });
                })
            })?)
            .await?;
        // 监听配置变更，动态更新视频下载任务
        let task_context_clone = task_context.clone();
        let sched_clone = sched.clone();
        tokio::spawn(async move {
            while rx.changed().await.is_ok() {
                let new_config = rx.borrow().clone();
                let task_context = task_context_clone.clone();
                // 先把旧的视频下载任务删掉
                let mut task_id_guard = task_context_clone.updating.lock().await;
                if let Some(old_task_id) = *task_id_guard {
                    sched_clone.remove(&old_task_id).await?;
                }
                // 再使用新的配置创建新的视频下载任务，并添加
                let job_run = move |uuid, l| DownloadTaskManager::download_video_task(uuid, l, task_context.clone());
                let job = match &new_config.interval {
                    Trigger::Interval(interval) => Job::new_repeated_async(Duration::from_secs(*interval), job_run)?,
                    Trigger::Cron(cron) => Job::new_async_tz(cron, chrono::Local, job_run)?,
                };
                let new_task_id = sched_clone.add(job).await?;
                *task_id_guard = Some(new_task_id);
                // 发起一个一次性的任务，更新一下下次运行的时间
                let task_context = task_context_clone.clone();
                sched_clone
                    .add(Job::new_one_shot_async(Duration::from_secs(0), move |_uuid, mut l| {
                        let task_context_clone = task_context.clone();
                        Box::pin(async move {
                            let old_status = task_context_clone.status_rx.borrow().clone();
                            let next_run = l
                                .next_tick_for_job(new_task_id)
                                .await
                                .ok()
                                .flatten()
                                .map(|dt| dt.with_timezone(&chrono::Local));
                            let _ = task_context_clone.status_tx.send(TaskStatus { next_run, ..old_status });
                        })
                    })?)
                    .await?;
            }
            Result::<(), anyhow::Error>::Ok(())
        });
        Ok(Self { sched, task_context })
    }

    fn check_and_refresh_credential_task(
        connection: DatabaseConnection,
        bili_client: Arc<BiliClient>,
        running: Arc<tokio::sync::Mutex<()>>,
    ) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        Box::pin(async move {
            let _lock = running.lock();
            if cfg!(debug_assertions) {
                info!("检测到调试模式，跳过本次凭据检查与刷新任务的执行..");
                return;
            }
            let config = VersionedConfig::get().read();
            info!("开始执行本轮凭据检查与刷新任务..");
            match check_and_refresh_credential(connection, &bili_client, &config).await {
                Ok(_) => info!("本轮凭据检查与刷新任务执行完毕"),
                Err(e) => {
                    let error_msg = format!("本轮凭据检查与刷新任务执行遇到错误：{:#}", e);
                    error!("{error_msg}");
                    let _ = config
                        .notifiers
                        .notify_all(bili_client.inner_client(), &error_msg)
                        .await;
                }
            }
        })
    }

    fn download_video_task(
        current_task_uuid: uuid::Uuid,
        mut l: JobScheduler,
        cx: TaskContext,
    ) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        Box::pin(async move {
            let Ok(_lock) = cx.running.try_lock() else {
                warn!("上一次视频下载任务尚未结束，跳过本次执行..");
                return;
            };
            let _ = cx.status_tx.send(TaskStatus {
                is_running: true,
                last_run: Some(chrono::Local::now()),
                last_finish: None,
                next_run: None,
            });
            if cfg!(debug_assertions) {
                info!("检测到调试模式，跳过本次视频下载任务的执行..");
            } else {
                info!("开始执行本轮视频下载任务..");
                let mut config = VersionedConfig::get().snapshot();
                match download_all_video_sources(&cx.connection, &cx.bili_client, &mut config).await {
                    Ok(_) => info!("本轮视频下载任务执行完毕"),
                    Err(e) => {
                        let error_msg = format!("本轮视频下载任务执行遇到错误：{:#}", e);
                        error!("{error_msg}");
                        let _ = config
                            .notifiers
                            .notify_all(cx.bili_client.inner_client(), &error_msg)
                            .await;
                    }
                }
            }
            // 注意此处尽量从 updating 中读取 uuid，因为当前任务可能是不存在 next_tick 的 oneshot 任务
            let task_uuid = (*cx.updating.lock().await).unwrap_or(current_task_uuid);
            let next_run = l
                .next_tick_for_job(task_uuid)
                .await
                .ok()
                .flatten()
                .map(|dt| dt.with_timezone(&chrono::Local));
            let last_status = cx.status_rx.borrow().clone();
            let _ = cx.status_tx.send(TaskStatus {
                is_running: false,
                last_run: last_status.last_run,
                last_finish: Some(chrono::Local::now()),
                next_run,
            });
        })
    }
}

/// 启动周期下载视频的任务
pub async fn video_downloader(connection: DatabaseConnection, bili_client: Arc<BiliClient>) -> Result<()> {
    let task_manager = DownloadTaskManager::init(connection, bili_client).await?;
    let _ = task_manager.start().await;
    future::pending::<()>().await;
    Ok(())
}

async fn check_and_refresh_credential(
    connection: DatabaseConnection,
    bili_client: &BiliClient,
    config: &Config,
) -> Result<()> {
    if let Some(new_credential) = bili_client
        .check_refresh(&config.credential)
        .await
        .context("检查刷新 Credential 失败")?
    {
        VersionedConfig::get()
            .update_credential(new_credential, &connection)
            .await
            .context("更新 Credential 失败")?;
    }
    Ok(())
}

async fn download_all_video_sources(
    connection: &DatabaseConnection,
    bili_client: &BiliClient,
    config: &mut Arc<Config>,
) -> Result<()> {
    config.check().context("配置检查失败")?;
    let mixin_key = bili_client
        .wbi_img(&config.credential)
        .await
        .context("获取 wbi_img 失败")?
        .into_mixin_key()
        .context("解析 mixin key 失败")?;
    bilibili::set_global_mixin_key(mixin_key);
    let template = TEMPLATE.snapshot();
    let bili_client = bili_client.snapshot()?;
    let video_sources = get_enabled_video_sources(connection)
        .await
        .context("获取视频源列表失败")?;
    if video_sources.is_empty() {
        bail!("没有可用的视频源");
    }
    for video_source in video_sources {
        let display_name = video_source.display_name();
        if let Err(e) = process_video_source(video_source, &bili_client, connection, &template, config).await {
            let error_msg = format!("处理 {} 时遇到错误：{:#}，跳过该视频源", display_name, e);
            error!("{error_msg}");
            let _ = config
                .notifiers
                .notify_all(bili_client.inner_client(), &error_msg)
                .await;
            if let Ok(e) = e.downcast::<BiliError>()
                && e.is_risk_control_related()
            {
                warn!("检测到风控，终止此轮视频下载任务..");
                break;
            }
        }
    }
    Ok(())
}
