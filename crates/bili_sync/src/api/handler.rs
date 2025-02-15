use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use axum::extract::{Extension, Path, Query};
use axum::Json;
use bili_sync_entity::*;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, QuerySelect};

use crate::api::error::ApiError;
use crate::api::payload::{VideoDetail, VideoInfo, VideoListModel};

pub async fn get_video_list_models(
    Extension(db): Extension<Arc<DatabaseConnection>>,
) -> Result<Json<VideoListModel>, ApiError> {
    Ok(Json(VideoListModel {
        collection: collection::Entity::find()
            .all(db.as_ref())
            .await?
            .into_iter()
            .map(Into::into)
            .collect(),
        favorite: favorite::Entity::find()
            .all(db.as_ref())
            .await?
            .into_iter()
            .map(Into::into)
            .collect(),
        submission: submission::Entity::find()
            .all(db.as_ref())
            .await?
            .into_iter()
            .map(Into::into)
            .collect(),
        watch_later: watch_later::Entity::find()
            .all(db.as_ref())
            .await?
            .into_iter()
            .map(Into::into)
            .collect(),
    }))
}

/// 列出所有视频的基本信息
pub async fn list_videos(
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Vec<VideoInfo>>, ApiError> {
    let mut query = video::Entity::find();
    for (query_key, filter_column) in [
        ("collection", video::Column::CollectionId),
        ("favorite", video::Column::FavoriteId),
        ("submission", video::Column::SubmissionId),
        ("watch_later", video::Column::WatchLaterId),
    ] {
        if let Some(value) = params.get(query_key) {
            query = query.filter(filter_column.eq(value));
            break;
        }
    }
    let videos = query
        .order_by_desc(video::Column::Id)
        .offset(params.get("o").and_then(|o| o.parse().ok()).unwrap_or(0))
        .limit(params.get("l").and_then(|l| l.parse().ok()).unwrap_or(30))
        .all(db.as_ref())
        .await?
        .into_iter()
        .map(VideoInfo::from)
        .collect();
    Ok(Json(videos))
}

/// 根据 id 获取视频详细信息，包括关联的所有 page
pub async fn get_video(
    Path(id): Path<i32>,
    Extension(db): Extension<Arc<DatabaseConnection>>,
) -> Result<Json<VideoDetail>, ApiError> {
    let video_model = video::Entity::find_by_id(id)
        .find_with_related(page::Entity)
        .all(db.as_ref())
        .await?;
    let detail = video_model
        .into_iter()
        .next()
        .map(VideoDetail::from)
        .ok_or_else(|| anyhow!("video not found"))?;
    Ok(Json(detail))
}
