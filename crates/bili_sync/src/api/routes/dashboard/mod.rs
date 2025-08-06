use axum::routing::get;
use axum::{Extension, Router};
use bili_sync_entity::*;
use sea_orm::entity::prelude::*;
use sea_orm::{FromQueryResult, Statement};

use crate::api::response::{DashBoardResponse, DayCountPair};
use crate::api::wrapper::{ApiError, ApiResponse};

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
            // 用 SeaORM 太复杂了，直接写个裸 SQL
            "
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
        ))
        .all(&db),
    )?;
    return Ok(ApiResponse::ok(DashBoardResponse {
        enabled_favorites,
        enabled_collections,
        enabled_submissions,
        enable_watch_later: enabled_watch_later > 0,
        videos_by_day,
    }));
}
