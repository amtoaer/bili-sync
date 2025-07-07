use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

use axum::response::Sse;
use axum::response::sse::{Event, KeepAlive};
use axum::routing::get;
use axum::{Extension, Router};
use bili_sync_entity::*;
use futures::StreamExt;
use sea_orm::entity::prelude::*;
use sea_orm::{FromQueryResult, Statement};
use sysinfo::{
    CpuRefreshKind, DiskRefreshKind, Disks, MemoryRefreshKind, ProcessRefreshKind, RefreshKind, System, get_current_pid,
};
use tokio_stream::wrappers::IntervalStream;

use crate::api::response::{DashBoardResponse, DayCountPair, SysInfoResponse};
use crate::api::wrapper::{ApiError, ApiResponse};

pub(super) fn router() -> Router {
    Router::new()
        .route("/dashboard", get(get_dashboard))
        .route("/dashboard/sysinfo", get(get_sysinfo))
}

async fn get_dashboard(
    Extension(db): Extension<Arc<DatabaseConnection>>,
) -> Result<ApiResponse<DashBoardResponse>, ApiError> {
    let (enabled_favorites, enabled_collections, enabled_submissions, enabled_watch_later, videos_by_day) = tokio::try_join!(
        favorite::Entity::find()
            .filter(favorite::Column::Enabled.eq(true))
            .count(db.as_ref()),
        collection::Entity::find()
            .filter(collection::Column::Enabled.eq(true))
            .count(db.as_ref()),
        submission::Entity::find()
            .filter(submission::Column::Enabled.eq(true))
            .count(db.as_ref()),
        watch_later::Entity::find()
            .filter(watch_later::Column::Enabled.eq(true))
            .count(db.as_ref()),
        DayCountPair::find_by_statement(Statement::from_string(
            db.get_database_backend(),
            // 用 SeaORM 太复杂了，直接写个裸 SQL
            "
SELECT
	dates.day AS day,
	COUNT(video.id) AS cnt
FROM
	(
		SELECT
			STRFTIME(
				'%Y-%m-%d',
			DATE('now', '-' || n || ' days', 'localtime')) AS day
		FROM
			(
				SELECT
					0 AS n UNION ALL
				SELECT
					1 UNION ALL
				SELECT
					2 UNION ALL
				SELECT
					3 UNION ALL
				SELECT
					4 UNION ALL
				SELECT
					5 UNION ALL
				SELECT
			6)) AS dates
	LEFT JOIN video ON STRFTIME('%Y-%m-%d', video.created_at, 'localtime') = dates.day
GROUP BY
	dates.day
ORDER BY
	dates.day;
    "
        ))
        .all(db.as_ref()),
    )?;
    return Ok(ApiResponse::ok(DashBoardResponse {
        enabled_favorites,
        enabled_collections,
        enabled_submissions,
        enable_watch_later: enabled_watch_later > 0,
        videos_by_day,
    }));
}

async fn get_sysinfo() -> Sse<impl futures::Stream<Item = Result<Event, Infallible>>> {
    let sys_refresh_kind = sys_refresh_kind();
    let disk_refresh_kind = disk_refresh_kind();
    let mut system = System::new();
    let mut disks = Disks::new();
    // safety: this functions always returns Ok on Linux/MacOS/Windows
    let self_pid = get_current_pid().unwrap();
    let stream = IntervalStream::new(tokio::time::interval(Duration::from_secs(2)))
        .map(move |_| {
            system.refresh_specifics(sys_refresh_kind);
            disks.refresh_specifics(true, disk_refresh_kind);
            let process = match system.process(self_pid) {
                Some(p) => p,
                None => return None,
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
            serde_json::to_string(&info).ok()
        })
        .take_while(|info| futures::future::ready(info.is_some()))
        // safety: after `take_while`, `info` is always Some
        .map(|info| Ok(Event::default().data(info.unwrap())));
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
