use axum::routing::get;
use axum::{Extension, Router};
use bili_sync_entity::*;
use sea_orm::entity::prelude::*;
use sea_orm::{DatabaseBackend, FromQueryResult, Statement};

use crate::api::response::{DashBoardResponse, DayCountPair};
use crate::api::wrapper::{ApiError, ApiResponse};
use crate::utils::time::last_seven_local_days;

pub(super) fn router() -> Router {
    Router::new().route("/dashboard", get(get_dashboard))
}

async fn get_dashboard(
    Extension(db): Extension<DatabaseConnection>,
) -> Result<ApiResponse<DashBoardResponse>, ApiError> {
    let (enabled_favorites, enabled_collections, enabled_submissions, enabled_watch_later, videos_by_day) = tokio::try_join!(
        favorite::Entity::find()
            .filter(favorite::Column::Enabled.eq(true))
            .count(&db),
        collection::Entity::find()
            .filter(collection::Column::Enabled.eq(true))
            .count(&db),
        submission::Entity::find()
            .filter(submission::Column::Enabled.eq(true))
            .count(&db),
        watch_later::Entity::find()
            .filter(watch_later::Column::Enabled.eq(true))
            .count(&db),
        DayCountPair::find_by_statement(Statement::from_string(
            db.get_database_backend(),
            dashboard_day_count_sql(db.get_database_backend()),
        ))
        .all(&db),
    )?;
    Ok(ApiResponse::ok(DashBoardResponse {
        enabled_favorites,
        enabled_collections,
        enabled_submissions,
        enable_watch_later: enabled_watch_later > 0,
        videos_by_day,
    }))
}

/// 按天统计最近 7 天新增视频数。两种后端均以 UTC 存储 created_at：
/// SQLite 在 SQL 内换算本地时间，PostgreSQL 的日期边界在应用侧按服务器
/// 本地时区计算后内联进 SQL（与 SQLite 的 localtime 语义一致；SQLite 按当前
/// 偏移换算、PG 按历史偏移换算，DST 过渡窗口内两后端日界可能相差至多 1 小时，
/// PG 侧按日历日语义更精确）
pub(crate) fn dashboard_day_count_sql(backend: DatabaseBackend) -> String {
    match backend {
        DatabaseBackend::Sqlite => "
SELECT
    dates.day AS day,
    COUNT(video.id) AS cnt
FROM
    (
        SELECT
            STRFTIME('%Y-%m-%d', DATE('now', '-' || n || ' days', 'localtime')) AS day,
            DATETIME(DATE('now', '-' || n || ' days', 'localtime'), 'utc') AS start_utc_datetime,
            DATETIME(DATE('now', '-' || n || ' days', '+1 day', 'localtime'), 'utc') AS end_utc_datetime
        FROM
            (
                SELECT 0 AS n UNION ALL SELECT 1 UNION ALL SELECT 2 UNION ALL SELECT 3 UNION ALL SELECT 4 UNION ALL SELECT 5 UNION ALL SELECT 6
            )
    ) AS dates
LEFT JOIN
    video ON video.created_at >= dates.start_utc_datetime AND video.created_at < dates.end_utc_datetime
GROUP BY
    dates.day
ORDER BY
    dates.day;
    "
        .to_string(),
        DatabaseBackend::Postgres => {
            let values = last_seven_local_days()
                .iter()
                .map(|(day, start, end)| {
                    format!(
                        "('{day}', '{}'::timestamp, '{}'::timestamp)",
                        start.format("%Y-%m-%d %H:%M:%S"),
                        end.format("%Y-%m-%d %H:%M:%S"),
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            format!(
                "SELECT
    dates.day AS day,
    COUNT(video.id) AS cnt
FROM
    (
        SELECT * FROM (VALUES {values}) AS v(day, start_utc_datetime, end_utc_datetime)
    ) AS dates
LEFT JOIN
    video ON video.created_at >= dates.start_utc_datetime AND video.created_at < dates.end_utc_datetime
GROUP BY
    dates.day
ORDER BY
    dates.day;"
            )
        }
        _ => unreachable!(),
    }
}
