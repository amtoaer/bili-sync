mod log_helper;

use std::sync::{Arc, LazyLock};
use std::time::Duration;

use axum::extract::WebSocketUpgrade;
use axum::extract::ws::{Message, WebSocket};
use axum::response::IntoResponse;
use axum::routing::any;
use axum::{Extension, Router};
use dashmap::DashMap;
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt, future};
use itertools::Itertools;
pub use log_helper::{LogHelper, MAX_HISTORY_LOGS};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use sysinfo::{
    CpuRefreshKind, DiskRefreshKind, Disks, MemoryRefreshKind, Pid, ProcessRefreshKind, ProcessesToUpdate, System,
    get_current_pid,
};
use tokio::sync::mpsc;
use tokio::{pin, select};
use tokio_stream::wrappers::{BroadcastStream, WatchStream};
use tokio_util::future::FutureExt;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::api::response::SysInfo;
use crate::task::{DownloadTaskManager, TaskStatus};

static WEBSOCKET_HANDLER: LazyLock<WebSocketHandler> = LazyLock::new(WebSocketHandler::new);

pub(super) fn router() -> Router {
    Router::new().route("/ws", any(websocket_handler))
}

async fn websocket_handler(ws: WebSocketUpgrade, Extension(log_writer): Extension<LogHelper>) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, log_writer))
}

// 事件类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum EventType {
    Logs,
    Tasks,
    SysInfo,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
enum ClientEvent {
    Subscribe(EventType),
    Unsubscribe(EventType),
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
enum ServerEvent {
    Logs(String),
    Tasks(TaskStatus),
    SysInfo(SysInfo),
}

struct WebSocketHandler {
    sysinfo_subscribers: Arc<DashMap<Uuid, mpsc::Sender<ServerEvent>>>,
    sysinfo_cancel: RwLock<Option<CancellationToken>>,
}

impl WebSocketHandler {
    fn new() -> Self {
        Self {
            sysinfo_subscribers: Arc::new(DashMap::new()),
            sysinfo_cancel: RwLock::new(None),
        }
    }

    /// 向客户端推送信息
    async fn handle_sender(&self, mut sender: SplitSink<WebSocket, Message>, mut rx: mpsc::Receiver<ServerEvent>) {
        while let Some(event) = rx.recv().await {
            let text = match serde_json::to_string(&event) {
                Ok(text) => text,
                Err(e) => {
                    error!("Failed to serialize event: {:?}", e);
                    continue;
                }
            };
            if let Err(e) = sender.send(Message::Text(text.into())).await {
                error!("Failed to send message: {:?}", e);
                break;
            }
        }
    }

    /// 从客户端接收信息
    async fn handle_receiver(
        &self,
        mut receiver: SplitStream<WebSocket>,
        tx: mpsc::Sender<ServerEvent>,
        uuid: Uuid,
        log_writer: LogHelper,
    ) {
        // 日志和任务状态的处理本身就是由 stream 驱动的，可以直接为每个 ws 连接维护独立的任务处理器
        // 系统信息是服务端轮询然后推送的，如果单独维护会导致每个连接都独立轮询系统信息，造成不必要的浪费
        // 因此采用了全局的订阅者管理，所有连接共享同一个系统信息轮询任务
        let (mut log_cancel, mut task_cancel) = (None, None);
        while let Some(Ok(msg)) = receiver.next().await {
            let Message::Text(text) = msg else {
                continue;
            };
            let client_event = match serde_json::from_str::<ClientEvent>(&text) {
                Ok(event) => event,
                Err(e) => {
                    error!("Failed to parse client message: {:?}, error: {:?}", text, e);
                    continue;
                }
            };
            match client_event {
                ClientEvent::Subscribe(EventType::Logs) => {
                    if log_cancel.is_none() {
                        log_cancel = Some(self.new_log_handler(tx.clone(), &log_writer));
                    }
                }
                ClientEvent::Unsubscribe(EventType::Logs) => {
                    if let Some(cancel) = log_cancel.take() {
                        cancel.cancel();
                    }
                }
                ClientEvent::Subscribe(EventType::Tasks) => {
                    if task_cancel.is_none() {
                        task_cancel = Some(self.new_task_handler(tx.clone()));
                    }
                }
                ClientEvent::Unsubscribe(EventType::Tasks) => {
                    if let Some(cancel) = task_cancel.take() {
                        cancel.cancel();
                    }
                }
                ClientEvent::Subscribe(EventType::SysInfo) => {
                    self.add_sysinfo_subscriber(uuid, tx.clone());
                }
                ClientEvent::Unsubscribe(EventType::SysInfo) => {
                    self.remove_sysinfo_subscriber(uuid);
                }
            }
        }
        // 连接关闭，清除仍然残留的任务
        if let Some(cancel) = log_cancel {
            cancel.cancel();
        }
        if let Some(cancel) = task_cancel {
            cancel.cancel();
        }
        self.remove_sysinfo_subscriber(uuid);
    }

    /// 添加全局系统信息订阅者
    fn add_sysinfo_subscriber(&self, uuid: Uuid, sender: mpsc::Sender<ServerEvent>) {
        self.sysinfo_subscribers.insert(uuid, sender);
        if self.sysinfo_cancel.read().is_none() {
            let mut sys_info_cancel = self.sysinfo_cancel.write();
            if sys_info_cancel.is_some() {
                return;
            }
            *sys_info_cancel = Some(self.new_sysinfo_handler(self.sysinfo_subscribers.clone()));
        }
    }

    /// 移除全局系统信息订阅者
    fn remove_sysinfo_subscriber(&self, uuid: Uuid) {
        self.sysinfo_subscribers.remove(&uuid);
        if self.sysinfo_subscribers.is_empty()
            && let Some(token) = self.sysinfo_cancel.write().take()
        {
            token.cancel();
        }
    }

    /// 创建异步日志推送任务，返回任务的取消令牌
    fn new_log_handler(&self, tx: mpsc::Sender<ServerEvent>, log_writer: &LogHelper) -> CancellationToken {
        let cancel_token = CancellationToken::new();
        // 读取历史日志
        let history = log_writer.log_history.read();
        let history_logs = history.iter().cloned().collect::<Vec<String>>();
        drop(history);
        // 获取日志广播接收器
        let log_rx = log_writer.sender.subscribe();
        tokio::spawn(
            async move {
                // 合并历史日志和实时日志流
                let log_stream = futures::stream::iter(history_logs)
                    .chain(BroadcastStream::new(log_rx).filter_map(async |msg| msg.ok()))
                    .map(ServerEvent::Logs);
                pin!(log_stream);
                while let Some(event) = log_stream.next().await {
                    if let Err(e) = tx.send(event).await {
                        error!("Failed to send log event: {:?}", e);
                        break;
                    }
                }
            }
            .with_cancellation_token_owned(cancel_token.clone()),
        );
        cancel_token
    }

    /// 创建异步任务状态推送任务，返回任务的取消令牌
    fn new_task_handler(&self, tx: mpsc::Sender<ServerEvent>) -> CancellationToken {
        let cancel_token = CancellationToken::new();
        tokio::spawn(
            async move {
                let mut stream = WatchStream::new(DownloadTaskManager::get().subscribe()).map(ServerEvent::Tasks);
                while let Some(event) = stream.next().await {
                    if let Err(e) = tx.send(event).await {
                        error!("Failed to send task status: {:?}", e);
                        break;
                    }
                }
            }
            .with_cancellation_token_owned(cancel_token.clone()),
        );
        cancel_token
    }

    /// 创建异步系统信息推送任务，返回任务的取消令牌
    fn new_sysinfo_handler(
        &self,
        sysinfo_subscribers: Arc<DashMap<Uuid, mpsc::Sender<ServerEvent>>>,
    ) -> CancellationToken {
        let cancel_token = CancellationToken::new();
        let cancel_token_clone = cancel_token.clone();
        tokio::spawn(async move {
            let (tx, mut rx) = mpsc::channel(10);
            let (tick_tx, mut tick_rx) = mpsc::channel(3);
            // 在阻塞线程中轮询系统信息，防止阻塞异步运行时
            tokio::task::spawn_blocking(move || {
                // 对于 linux/mac/windows 平台，该方法永远返回 Some(pid)，expect 基本是安全的
                let self_pid = get_current_pid().expect("Unsupported platform");
                let mut system = System::new();
                let mut disks = Disks::new();
                while tick_rx.blocking_recv().is_some() {
                    system.refresh_needed(self_pid);
                    disks.refresh_needed(self_pid);
                    let process = match system.process(self_pid) {
                        Some(p) => p,
                        None => continue,
                    };
                    let (available, total) = disks
                        .iter()
                        .filter(|d| {
                            d.available_space() > 0
                                && d.total_space() > 0
                                // 简单过滤一些虚拟文件系统
                                && !["overlay", "tmpfs", "sysfs", "proc"]
                                    .contains(&d.file_system().to_string_lossy().as_ref())
                        })
                        .unique_by(|d| d.name())
                        .fold((0, 0), |(mut available, mut total), d| {
                            available += d.available_space();
                            total += d.total_space();
                            (available, total)
                        });
                    let sys_info = SysInfo {
                        timestamp: chrono::Utc::now().timestamp_millis(),
                        total_memory: system.total_memory(),
                        used_memory: system.used_memory(),
                        process_memory: process.memory(),
                        used_cpu: system.global_cpu_usage(),
                        process_cpu: process.cpu_usage() / system.cpus().len() as f32,
                        total_disk: total,
                        available_disk: available,
                    };
                    if tx.blocking_send(sys_info).is_err() {
                        break;
                    }
                }
            });
            // 异步部分负责获取由阻塞线程发送过来的系统信息，并推送给所有订阅者
            // 收到取消信号时，设置标志位，确保阻塞线程正常退出
            let mut interval = tokio::time::interval(Duration::from_secs(2));
            loop {
                select! {
                    _ = cancel_token_clone.cancelled() => {
                        drop(tick_tx);
                        break;
                    }
                    _ = interval.tick() => {
                        let _ = tick_tx.send(()).await;
                    }
                    Some(sys_info) = rx.recv() => {
                        future::join_all(sysinfo_subscribers.iter().map(async |subscriber| {
                            if let Err(e) = subscriber.send(ServerEvent::SysInfo(sys_info)).await {
                                error!(
                                    "Failed to send sysinfo event to subscriber {}: {:?}",
                                    subscriber.key(),
                                    e
                                );
                            }
                        }))
                        .await;
                    }
                }
            }
        });
        cancel_token
    }
}

async fn handle_socket(socket: WebSocket, log_writer: LogHelper) {
    let (ws_sender, ws_receiver) = socket.split();
    let uuid = Uuid::new_v4();
    let (tx, rx) = tokio::sync::mpsc::channel(100);
    tokio::spawn(WEBSOCKET_HANDLER.handle_sender(ws_sender, rx));
    tokio::spawn(WEBSOCKET_HANDLER.handle_receiver(ws_receiver, tx, uuid, log_writer));
}

trait SysInfoExt {
    fn refresh_needed(&mut self, self_pid: Pid);
}

impl SysInfoExt for System {
    fn refresh_needed(&mut self, self_pid: Pid) {
        self.refresh_memory_specifics(MemoryRefreshKind::nothing().with_ram());
        self.refresh_cpu_specifics(CpuRefreshKind::nothing().with_cpu_usage());
        self.refresh_processes_specifics(
            ProcessesToUpdate::Some(&[self_pid]),
            true,
            ProcessRefreshKind::nothing().with_cpu().with_memory(),
        );
    }
}

impl SysInfoExt for Disks {
    fn refresh_needed(&mut self, _self_pid: Pid) {
        self.refresh_specifics(true, DiskRefreshKind::nothing().with_storage());
    }
}
