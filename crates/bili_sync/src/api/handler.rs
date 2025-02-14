use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use axum::extract::{Extension, Path, Query};
use axum::Json;
use bili_sync_entity::*;
use sea_orm::prelude::Expr;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, QuerySelect, Set,
};

use crate::api::error::ApiError;
use crate::api::payload::{BulkUpdatePayload, UpdateVideoPayload, VideoDetail, VideoInfo};
use crate::utils::status::VideoStatus;

/// 列出所有视频的基本信息
pub async fn list_videos(
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Vec<VideoInfo>>, ApiError> {
    let videos = video::Entity::find()
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

/// 更新单个视频的状态
pub async fn update_video(
    Path(id): Path<i32>,
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Json(payload): Json<UpdateVideoPayload>,
) -> Result<Json<()>, ApiError> {
    // 查找视频记录
    let video_model = video::Entity::find_by_id(id)
        .one(db.as_ref())
        .await?
        .ok_or_else(|| anyhow!("video not found"))?;
    // 构造视频 active model
    let mut active_video: video::ActiveModel = video_model.into();
    active_video.download_status = Set(VideoStatus::from(payload.download_status).into());
    active_video.update(db.as_ref()).await?;
    Ok(Json(()))
}

#[axum::debug_handler]
pub async fn bulk_update_videos(
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Json(payload): Json<BulkUpdatePayload>,
) -> Result<(), ApiError> {
    let target_status: u32 = VideoStatus::from(payload.download_status).into();
    video::Entity::update_many()
        .filter(video::Column::Id.is_in(payload.video_ids))
        .col_expr(video::Column::DownloadStatus, Expr::value(target_status))
        .exec(db.as_ref())
        .await?;
    Ok(())
}
