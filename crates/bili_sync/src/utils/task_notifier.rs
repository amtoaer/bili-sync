use std::sync::LazyLock;

use serde::Serialize;
use tokio::sync::{MutexGuard, watch};

pub static TASK_STATUS_NOTIFIER: LazyLock<TaskStatusNotifier> = LazyLock::new(TaskStatusNotifier::new);

#[derive(Serialize, Default, Clone, Copy)]
pub struct TaskStatus {
    is_running: bool,
    last_run: Option<chrono::DateTime<chrono::Local>>,
    last_finish: Option<chrono::DateTime<chrono::Local>>,
    next_run: Option<chrono::DateTime<chrono::Local>>,
}

pub struct TaskStatusNotifier {
    mutex: tokio::sync::Mutex<()>,
    tx: watch::Sender<TaskStatus>,
    rx: watch::Receiver<TaskStatus>,
}

impl TaskStatusNotifier {
    pub fn new() -> Self {
        let (tx, rx) = watch::channel(TaskStatus::default());
        Self {
            mutex: tokio::sync::Mutex::const_new(()),
            tx,
            rx,
        }
    }

    pub async fn start_running(&'_ self) -> MutexGuard<'_, ()> {
        let lock = self.mutex.lock().await;
        let _ = self.tx.send(TaskStatus {
            is_running: true,
            last_run: Some(chrono::Local::now()),
            last_finish: None,
            next_run: None,
        });
        lock
    }

    pub fn finish_running(&self, _lock: MutexGuard<()>, interval: i64) {
        let last_status = self.tx.borrow();
        let last_run = last_status.last_run;
        drop(last_status);
        let now = chrono::Local::now();
        let _ = self.tx.send(TaskStatus {
            is_running: false,
            last_run,
            last_finish: Some(now),
            next_run: now.checked_add_signed(chrono::Duration::seconds(interval)),
        });
    }

    pub fn subscribe(&self) -> tokio::sync::watch::Receiver<TaskStatus> {
        self.rx.clone()
    }
}
