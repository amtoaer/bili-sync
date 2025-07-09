mod mpsc;

use std::convert::Infallible;
use std::time::Duration;

use axum::response::Sse;
use axum::response::sse::{Event, KeepAlive};
use axum::routing::get;
use axum::{Extension, Router};
use futures::{Stream, StreamExt};
pub use mpsc::{MAX_HISTORY_LOGS, MpscWriter};
use sysinfo::{
    CpuRefreshKind, DiskRefreshKind, Disks, MemoryRefreshKind, ProcessRefreshKind, RefreshKind, System, get_current_pid,
};
use tokio_stream::wrappers::{BroadcastStream, IntervalStream, WatchStream};

use crate::api::response::SysInfoResponse;
use crate::utils::task_notifier::TASK_STATUS_NOTIFIER;

pub(super) fn router() -> Router {
    Router::new()
        .route("/sse/logs", get(logs))
        .route("/sse/tasks", get(get_tasks))
        .route("/sse/sysinfo", get(get_sysinfo))
}

async fn get_tasks() -> Sse<impl futures::Stream<Item = Result<Event, Infallible>>> {
    let stream = WatchStream::new(TASK_STATUS_NOTIFIER.subscribe()).filter_map(|status| async move {
        match serde_json::to_string(&status) {
            Ok(status) => Some(Ok(Event::default().data(status))),
            Err(_) => None,
        }
    });
    Sse::new(stream).keep_alive(KeepAlive::default())
}

async fn logs(Extension(log_writer): Extension<MpscWriter>) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let history = log_writer.log_history.lock();
    let rx = log_writer.sender.subscribe();
    let history_logs: Vec<String> = history.iter().cloned().collect();
    drop(history);

    let history_stream = { futures::stream::iter(history_logs.into_iter().map(|msg| Ok(Event::default().data(msg)))) };

    let stream = BroadcastStream::new(rx).filter_map(async |msg| match msg {
        Ok(log_message) => Some(Ok(Event::default().data(log_message))),
        Err(e) => {
            error!("Log stream error: {:?}", e);
            None
        }
    });
    Sse::new(history_stream.chain(stream)).keep_alive(KeepAlive::default())
}

async fn get_sysinfo() -> Sse<impl futures::Stream<Item = Result<Event, Infallible>>> {
    let sys_refresh_kind = sys_refresh_kind();
    let disk_refresh_kind = disk_refresh_kind();
    let mut system = System::new();
    let mut disks = Disks::new();
    // safety: this functions always returns Ok on Linux/MacOS/Windows
    let self_pid = get_current_pid().unwrap();
    let stream = IntervalStream::new(tokio::time::interval(Duration::from_secs(2))).filter_map(move |_| {
        system.refresh_specifics(sys_refresh_kind);
        disks.refresh_specifics(true, disk_refresh_kind);
        let process = match system.process(self_pid) {
            Some(p) => p,
            None => return futures::future::ready(None),
        };
        let info = SysInfoResponse {
            total_memory: system.total_memory(),
            used_memory: system.used_memory(),
            process_memory: process.memory(),
            used_cpu: system.global_cpu_usage(),
            process_cpu: process.cpu_usage() / system.cpus().len() as f32,
            total_disk: disks.iter().map(|d| d.total_space()).sum(),
            available_disk: disks.iter().map(|d| d.available_space()).sum(),
        };
        match serde_json::to_string(&info) {
            Ok(json) => futures::future::ready(Some(Ok(Event::default().data(json)))),
            Err(_) => {
                error!("Failed to serialize system info");
                futures::future::ready(None)
            }
        }
    });
    Sse::new(stream).keep_alive(KeepAlive::default())
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
