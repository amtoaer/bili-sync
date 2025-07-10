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
pub use log_helper::{LogHelper, MAX_HISTORY_LOGS};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use sysinfo::{
    CpuRefreshKind, DiskRefreshKind, Disks, MemoryRefreshKind, ProcessRefreshKind, RefreshKind, System, get_current_pid,
};
use tokio::pin;
use tokio::task::JoinHandle;
use tokio_stream::wrappers::{BroadcastStream, IntervalStream, WatchStream};
use uuid::Uuid;

use crate::api::response::SysInfo;
use crate::utils::task_notifier::{TASK_STATUS_NOTIFIER, TaskStatus};

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
    Tasks(Arc<TaskStatus>),
    SysInfo(Arc<SysInfo>),
}

struct WebSocketHandler {
    sysinfo_subscribers: Arc<DashMap<Uuid, tokio::sync::mpsc::Sender<ServerEvent>>>,
    sysinfo_handles: RwLock<Option<JoinHandle<()>>>,
}

impl WebSocketHandler {
    fn new() -> Self {
        Self {
            sysinfo_subscribers: Arc::new(DashMap::new()),
            sysinfo_handles: RwLock::new(None),
        }
    }

    async fn handle_sender(
        &self,
        mut sender: SplitSink<WebSocket, Message>,
        mut rx: tokio::sync::mpsc::Receiver<ServerEvent>,
    ) {
        while let Some(event) = rx.recv().await {
            match serde_json::to_string(&event) {
                Ok(text) => {
                    if let Err(e) = sender.send(Message::Text(text.into())).await {
                        error!("Failed to send message: {:?}", e);
                        break;
                    }
                }
                Err(e) => {
                    error!("Failed to serialize event: {:?}", e);
                }
            }
        }
    }

    async fn handle_receiver(
        &self,
        mut receiver: SplitStream<WebSocket>,
        tx: tokio::sync::mpsc::Sender<ServerEvent>,
        uuid: Uuid,
        log_writer: LogHelper,
    ) {
        // 日志和任务状态的处理本身就是由 stream 驱动的，可以直接为每个 ws 连接维护独立的任务处理器
        // 系统信息是服务端轮询然后推送的，如果单独维护会导致每个连接都独立轮询系统信息，造成不必要的浪费
        // 因此采用了全局的订阅者管理，所有连接共享同一个系统信息轮询任务
        let (mut log_handle, mut task_handle) = (None, None);
        while let Some(Ok(msg)) = receiver.next().await {
            if let Message::Text(text) = msg {
                match serde_json::from_str::<ClientEvent>(&text) {
                    Ok(ClientEvent::Subscribe(event_type)) => match event_type {
                        EventType::Logs => {
                            if log_handle.as_ref().is_none_or(|h: &JoinHandle<()>| h.is_finished()) {
                                let log_writer_clone = log_writer.clone();
                                let tx_clone = tx.clone();
                                let history = log_writer_clone.log_history.lock();
                                let history_logs: Vec<String> = history.iter().cloned().collect();
                                drop(history);
                                log_handle = Some(tokio::spawn(async move {
                                    let rx = log_writer_clone.sender.subscribe();
                                    let log_stream = futures::stream::iter(history_logs.into_iter())
                                        .chain(BroadcastStream::new(rx).filter_map(async |msg| msg.ok()))
                                        .map(|msg| ServerEvent::Logs(msg));
                                    pin!(log_stream);
                                    while let Some(event) = log_stream.next().await {
                                        if let Err(e) = tx_clone.send(event).await {
                                            error!("Failed to send log event: {:?}", e);
                                            break;
                                        }
                                    }
                                }));
                            }
                        }
                        EventType::Tasks => {
                            if task_handle.as_ref().is_none_or(|h: &JoinHandle<()>| h.is_finished()) {
                                let tx_clone = tx.clone();
                                task_handle = Some(tokio::spawn(async move {
                                    let mut stream = WatchStream::new(TASK_STATUS_NOTIFIER.subscribe())
                                        .map(|status| ServerEvent::Tasks(status));
                                    while let Some(event) = stream.next().await {
                                        if let Err(e) = tx_clone.send(event).await {
                                            error!("Failed to send task status: {:?}", e);
                                            break;
                                        }
                                    }
                                }));
                            }
                        }
                        EventType::SysInfo => self.add_sysinfo_subscriber(uuid, tx.clone()).await,
                    },
                    Ok(ClientEvent::Unsubscribe(event_type)) => match event_type {
                        EventType::Logs => {
                            if let Some(handle) = log_handle.take() {
                                handle.abort();
                            }
                        }
                        EventType::Tasks => {
                            if let Some(handle) = task_handle.take() {
                                handle.abort();
                            }
                        }
                        EventType::SysInfo => {
                            self.remove_sysinfo_subscriber(uuid).await;
                        }
                    },
                    Err(e) => {
                        error!("Failed to parse client message: {:?}", e);
                    }
                }
            }
        }
        if let Some(handle) = log_handle {
            handle.abort();
        }
        if let Some(handle) = task_handle {
            handle.abort();
        }
        self.remove_sysinfo_subscriber(uuid).await;
    }

    // 添加订阅者
    async fn add_sysinfo_subscriber(&self, uuid: Uuid, sender: tokio::sync::mpsc::Sender<ServerEvent>) {
        self.sysinfo_subscribers.insert(uuid, sender);
        if self.sysinfo_subscribers.len() > 0
            && self
                .sysinfo_handles
                .read()
                .as_ref()
                .is_none_or(|h: &JoinHandle<()>| h.is_finished())
        {
            let sysinfo_subscribers = self.sysinfo_subscribers.clone();
            let mut write_guard = self.sysinfo_handles.write();
            if write_guard.as_ref().is_some_and(|h: &JoinHandle<()>| !h.is_finished()) {
                return;
            }
            *write_guard = Some(tokio::spawn(async move {
                let mut system = System::new();
                let mut disks = Disks::new();
                let sys_refresh_kind = sys_refresh_kind();
                let disk_refresh_kind = disk_refresh_kind();
                // 对于 linux/mac/windows 平台，该方法永远返回 Some(pid)，expect 基本是安全的
                let self_pid = get_current_pid().expect("Unsupported platform");
                let mut stream =
                    IntervalStream::new(tokio::time::interval(Duration::from_secs(2))).filter_map(move |_| {
                        system.refresh_specifics(sys_refresh_kind);
                        disks.refresh_specifics(true, disk_refresh_kind);
                        let process = match system.process(self_pid) {
                            Some(p) => p,
                            None => return futures::future::ready(None),
                        };
                        futures::future::ready(Some(SysInfo {
                            total_memory: system.total_memory(),
                            used_memory: system.used_memory(),
                            process_memory: process.memory(),
                            used_cpu: system.global_cpu_usage(),
                            process_cpu: process.cpu_usage() / system.cpus().len() as f32,
                            total_disk: disks.iter().map(|d| d.total_space()).sum(),
                            available_disk: disks.iter().map(|d| d.available_space()).sum(),
                        }))
                    });
                while let Some(sys_info) = stream.next().await {
                    let sys_info = Arc::new(sys_info);
                    future::join_all(sysinfo_subscribers.iter().map(async |subscriber| {
                        if let Err(e) = subscriber.send(ServerEvent::SysInfo(sys_info.clone())).await {
                            error!(
                                "Failed to send sysinfo event to subscriber {}: {:?}",
                                subscriber.key(),
                                e
                            );
                        }
                    }))
                    .await;
                }
            }));
        }
    }

    async fn remove_sysinfo_subscriber(&self, uuid: Uuid) {
        self.sysinfo_subscribers.remove(&uuid);
        if self.sysinfo_subscribers.is_empty() {
            if let Some(handle) = self.sysinfo_handles.write().take() {
                handle.abort();
            }
        }
    }
}

async fn handle_socket(socket: WebSocket, log_writer: LogHelper) {
    let (ws_sender, ws_receiver) = socket.split();
    let uuid = Uuid::new_v4();
    let (tx, rx) = tokio::sync::mpsc::channel(100);
    tokio::spawn(WEBSOCKET_HANDLER.handle_sender(ws_sender, rx));
    tokio::spawn(WEBSOCKET_HANDLER.handle_receiver(ws_receiver, tx, uuid, log_writer));
}

fn sys_refresh_kind() -> RefreshKind {
    RefreshKind::nothing()
        .with_cpu(CpuRefreshKind::nothing().with_cpu_usage())
        .with_memory(MemoryRefreshKind::nothing().with_ram())
        .with_processes(ProcessRefreshKind::nothing().with_cpu().with_memory())
}

fn disk_refresh_kind() -> DiskRefreshKind {
    DiskRefreshKind::nothing().with_storage()
}
