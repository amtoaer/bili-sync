use std::sync::{Arc, LazyLock};

use serde::Serialize;
use tokio::sync::MutexGuard;

use crate::config::VersionedConfig;

pub static TASK_STATUS_NOTIFIER: LazyLock<TaskStatusNotifier> = LazyLock::new(TaskStatusNotifier::new);

#[derive(Serialize)]
pub struct TaskStatus {
    is_running: bool,
    last_run: Option<chrono::DateTime<chrono::Local>>,
    last_finish: Option<chrono::DateTime<chrono::Local>>,
    next_run: Option<chrono::DateTime<chrono::Local>>,
}

pub struct TaskStatusNotifier {
    mutex: tokio::sync::Mutex<()>,
    tx: tokio::sync::watch::Sender<Arc<TaskStatus>>,
    rx: tokio::sync::watch::Receiver<Arc<TaskStatus>>,
}

impl Default for TaskStatus {
    fn default() -> Self {
        Self {
            is_running: false,
            last_run: None,
            last_finish: None,
            next_run: None,
        }
    }
}

impl TaskStatusNotifier {
    pub fn new() -> Self {
        let (tx, rx) = tokio::sync::watch::channel(Arc::new(TaskStatus::default()));
        Self {
            mutex: tokio::sync::Mutex::const_new(()),
            tx,
            rx,
        }
    }

    pub async fn start_running(&self) -> MutexGuard<()> {
        let lock = self.mutex.lock().await;
        let _ = self.tx.send(Arc::new(TaskStatus {
            is_running: true,
            last_run: Some(chrono::Local::now()),
            last_finish: None,
            next_run: None,
        }));
        lock
    }

    pub fn finish_running(&self, _lock: MutexGuard<()>) {
        let last_status = self.tx.borrow();
        let last_run = last_status.last_run.clone();
        drop(last_status);
        let config = VersionedConfig::get().load();
        let now = chrono::Local::now();

        let _ = self.tx.send(Arc::new(TaskStatus {
            is_running: false,
            last_run,
            last_finish: Some(now),
            next_run: now.checked_add_signed(chrono::Duration::seconds(config.interval as i64)),
        }));
    }

    /// 精确探测任务执行状态，保证如果读取到“未运行”，那么在锁释放之前任务不会被执行
    pub fn detect_running(&self) -> Option<MutexGuard<'_, ()>> {
        self.mutex.try_lock().ok()
    }

    pub fn subscribe(&self) -> tokio::sync::watch::Receiver<Arc<TaskStatus>> {
        self.rx.clone()
    }
}
