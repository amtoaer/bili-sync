use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use axum::extract::{Extension, Path, Query};
use axum::Json;
use bili_sync_entity::*;
use bili_sync_migration::Expr;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect};

use crate::api::error::ApiError;
use crate::api::payload::{PageInfo, VideoDetail, VideoInfo, VideoList, VideoListModel, VideoListModelItem};

/// 列出所有视频列表
pub async fn get_video_list_models(
    Extension(db): Extension<Arc<DatabaseConnection>>,
) -> Result<Json<VideoListModel>, ApiError> {
    Ok(Json(VideoListModel {
        collection: collection::Entity::find()
            .select_only()
            .columns([collection::Column::Id, collection::Column::Name])
            .into_model::<VideoListModelItem>()
            .all(db.as_ref())
            .await?,
        favorite: favorite::Entity::find()
            .select_only()
            .columns([favorite::Column::Id, favorite::Column::Name])
            .into_model::<VideoListModelItem>()
            .all(db.as_ref())
            .await?,
        submission: submission::Entity::find()
            .select_only()
            .column(submission::Column::Id)
            .column_as(submission::Column::UpperName, "name")
            .into_model::<VideoListModelItem>()
            .all(db.as_ref())
            .await?,
        watch_later: watch_later::Entity::find()
            .select_only()
            .column(watch_later::Column::Id)
            .column_as(Expr::value("稍后再看"), "name")
            .into_model::<VideoListModelItem>()
            .all(db.as_ref())
            .await?,
    }))
}

/// 列出所有视频的基本信息（支持根据视频列表筛选，支持分页）
pub async fn list_videos(
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<VideoList>, ApiError> {
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
    if let Some(query_word) = params.get("q") {
        query = query.filter(video::Column::Name.contains(query_word));
    }
    let total_count = query.clone().count(db.as_ref()).await?;
    let (page, page_size) = if let (Some(page), Some(page_size)) = (params.get("page"), params.get("page_size")) {
        (page.parse::<u64>()?, page_size.parse::<u64>()?)
    } else {
        (1, 10)
    };
    Ok(Json(VideoList {
        videos: query
            .order_by_desc(video::Column::Id)
            .into_partial_model::<VideoInfo>()
            .paginate(db.as_ref(), page_size)
            .fetch_page(page)
            .await?,
        total_count,
    }))
}

/// 根据 id 获取视频详细信息，包括关联的所有 page
pub async fn get_video(
    Path(id): Path<i32>,
    Extension(db): Extension<Arc<DatabaseConnection>>,
) -> Result<Json<VideoDetail>, ApiError> {
    let video_info = video::Entity::find_by_id(id)
        .into_partial_model::<VideoInfo>()
        .one(db.as_ref())
        .await?;
    let Some(video_info) = video_info else {
        return Err(anyhow!("视频不存在").into());
    };
    let pages = page::Entity::find()
        .filter(page::Column::VideoId.eq(id))
        .order_by_asc(page::Column::Pid)
        .into_partial_model::<PageInfo>()
        .all(db.as_ref())
        .await?;
    Ok(Json(VideoDetail {
        video: video_info,
        pages,
    }))
}
