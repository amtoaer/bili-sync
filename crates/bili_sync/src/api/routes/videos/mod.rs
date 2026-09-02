use std::collections::HashSet;

use anyhow::{Context, Result};
use axum::extract::{Extension, Path, Query};
use axum::routing::{get, post};
use axum::{Json, Router};
use bili_sync_entity::*;
use chrono::NaiveDateTime;
use sea_orm::ActiveValue::Set;
use sea_orm::sea_query::{Expr, ExprTrait};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseBackend, DatabaseConnection, EntityTrait, IntoActiveModel,
    PaginatorTrait, QueryFilter, QueryOrder, Select, TransactionTrait, TryIntoModel,
};

use crate::api::error::InnerApiError;
use crate::api::helper::{update_page_download_status, update_video_download_status};
use crate::api::request::{
    ResetFilteredVideoStatusRequest, ResetVideoStatusRequest, UpdateFilteredVideoStatusRequest,
    UpdateVideoStatusRequest, VideosRequest,
};
use crate::api::response::{
    ClearAndResetVideoStatusResponse, PageInfo, ResetFilteredVideosResponse, ResetVideoResponse, SimplePageInfo,
    SimpleVideoInfo, UpdateFilteredVideoStatusResponse, UpdateVideoStatusResponse, VideoInfo, VideoResponse,
    VideosResponse,
};
use crate::api::wrapper::{ApiError, ApiResponse, ValidatedJson};
use crate::utils::status::{PageStatus, VideoStatus};
use crate::utils::time::local_to_utc;

pub(super) fn router() -> Router {
    Router::new()
        .route("/videos", get(get_videos))
        .route("/videos/{id}", get(get_video))
        .route(
            "/videos/{id}/clear-and-reset-status",
            post(clear_and_reset_video_status),
        )
        .route("/videos/{id}/reset-status", post(reset_video_status))
        .route("/videos/{id}/update-status", post(update_video_status))
        .route("/videos/reset-status", post(reset_filtered_video_status))
        .route("/videos/update-status", post(update_filtered_video_status))
}

fn apply_created_at_filter(
    db: &DatabaseConnection,
    mut query: Select<video::Entity>,
    created_from: Option<NaiveDateTime>,
    created_to: Option<NaiveDateTime>,
) -> Select<video::Entity> {
    // 两种后端均以 UTC 存储 created_at。SQLite 在 SQL 内用 datetime(?, 'localtime')
    // 转换到本地时间；PostgreSQL 在应用侧按服务器本地时区换算参数后直接比较，语义一致
    // （SQLite 按查询时刻的当前偏移换算，PG 按目标日期的历史偏移换算，DST 过渡窗口内
    // 两后端边界可能相差至多 1 小时，PG 侧更精确）
    if let Some(created_from) = created_from {
        query = match db.get_database_backend() {
            DatabaseBackend::Sqlite => query.filter(
                Expr::cust_with_expr("datetime(?, 'localtime')", Expr::col(video::Column::CreatedAt))
                    .gte(created_from.format("%Y-%m-%d %H:%M:%S").to_string()),
            ),
            DatabaseBackend::Postgres => query.filter(video::Column::CreatedAt.gte(local_to_utc(created_from, false))),
            _ => unreachable!(),
        };
    }
    if let Some(created_to) = created_to {
        query = match db.get_database_backend() {
            DatabaseBackend::Sqlite => query.filter(
                Expr::cust_with_expr("datetime(?, 'localtime')", Expr::col(video::Column::CreatedAt))
                    .lte(created_to.format("%Y-%m-%d %H:%M:%S").to_string()),
            ),
            DatabaseBackend::Postgres => query.filter(video::Column::CreatedAt.lte(local_to_utc(created_to, true))),
            _ => unreachable!(),
        };
    }
    query
}

/// 列出视频的基本信息，支持根据视频来源筛选、名称查找和分页
pub async fn get_videos(
    Extension(db): Extension<DatabaseConnection>,
    Query(params): Query<VideosRequest>,
) -> Result<ApiResponse<VideosResponse>, ApiError> {
    let mut query = video::Entity::find();
    for (field, column) in [
        (params.collection, video::Column::CollectionId),
        (params.favorite, video::Column::FavoriteId),
        (params.submission, video::Column::SubmissionId),
        (params.watch_later, video::Column::WatchLaterId),
    ] {
        if let Some(id) = field {
            query = query.filter(column.eq(id));
        }
    }
    if let Some(query_word) = params.query {
        query = query.filter(
            video::Column::Name
                .contains(&query_word)
                .or(video::Column::Bvid.contains(query_word)),
        );
    }
    if let Some(status_filter) = params.status_filter {
        query = query.filter(status_filter.to_video_query());
    }
    if let Some(validation_filter) = params.validation_filter {
        query = query.filter(validation_filter.to_video_query());
    }
    query = apply_created_at_filter(&db, query, params.created_from, params.created_to);
    let total_count = query.clone().count(&db).await?;
    let (page, page_size) = if let (Some(page), Some(page_size)) = (params.page, params.page_size) {
        (page, page_size)
    } else {
        (0, 10)
    };
    Ok(ApiResponse::ok(VideosResponse {
        videos: query
            .order_by_desc(video::Column::Id)
            .into_partial_model::<VideoInfo>()
            .paginate(&db, page_size)
            .fetch_page(page)
            .await?,
        total_count,
    }))
}

pub async fn get_video(
    Path(id): Path<i32>,
    Extension(db): Extension<DatabaseConnection>,
) -> Result<ApiResponse<VideoResponse>, ApiError> {
    let (video_info, pages_info) = tokio::try_join!(
        video::Entity::find_by_id(id).into_partial_model::<VideoInfo>().one(&db),
        page::Entity::find()
            .filter(page::Column::VideoId.eq(id))
            .order_by_asc(page::Column::Cid)
            .into_partial_model::<PageInfo>()
            .all(&db)
    )?;
    let Some(video_info) = video_info else {
        return Err(InnerApiError::NotFound(id).into());
    };
    Ok(ApiResponse::ok(VideoResponse {
        video: video_info,
        pages: pages_info,
    }))
}

pub async fn reset_video_status(
    Path(id): Path<i32>,
    Extension(db): Extension<DatabaseConnection>,
    Json(request): Json<ResetVideoStatusRequest>,
) -> Result<ApiResponse<ResetVideoResponse>, ApiError> {
    let (video_info, pages_info) = tokio::try_join!(
        video::Entity::find_by_id(id).into_partial_model::<VideoInfo>().one(&db),
        page::Entity::find()
            .filter(page::Column::VideoId.eq(id))
            .order_by_asc(page::Column::Cid)
            .into_partial_model::<PageInfo>()
            .all(&db)
    )?;
    let Some(mut video_info) = video_info else {
        return Err(InnerApiError::NotFound(id).into());
    };
    let resetted_pages_info = pages_info
        .into_iter()
        .filter_map(|mut page_info| {
            let mut page_status = PageStatus::from(page_info.download_status);
            if (request.force && page_status.force_reset_failed()) || page_status.reset_failed() {
                page_info.download_status = i64::from(page_status);
                Some(page_info)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    let mut video_status = VideoStatus::from(video_info.download_status);
    let mut video_resetted = (request.force && video_status.force_reset_failed()) || video_status.reset_failed();
    if !resetted_pages_info.is_empty() {
        video_status.set(4, 0); //  将“分页下载”重置为 0
        video_resetted = true;
    }
    let resetted_videos_info = if video_resetted {
        video_info.download_status = i64::from(video_status);
        vec![&video_info]
    } else {
        vec![]
    };
    let resetted = !resetted_videos_info.is_empty() || !resetted_pages_info.is_empty();
    if resetted {
        let txn = db.begin().await?;
        if !resetted_videos_info.is_empty() {
            // 只可能有 1 个元素，所以不用 batch
            update_video_download_status::<VideoInfo>(&txn, &resetted_videos_info, None).await?;
        }
        if !resetted_pages_info.is_empty() {
            update_page_download_status(&txn, &resetted_pages_info, Some(500)).await?;
        }
        txn.commit().await?;
    }
    Ok(ApiResponse::ok(ResetVideoResponse {
        resetted,
        video: video_info,
        pages: resetted_pages_info,
    }))
}

pub async fn clear_and_reset_video_status(
    Path(id): Path<i32>,
    Extension(db): Extension<DatabaseConnection>,
) -> Result<ApiResponse<ClearAndResetVideoStatusResponse>, ApiError> {
    let video_info = video::Entity::find_by_id(id).one(&db).await?;
    let Some(video_info) = video_info else {
        return Err(InnerApiError::NotFound(id).into());
    };
    let txn = db.begin().await?;
    let mut video_info = video_info.into_active_model();
    video_info.single_page = Set(None);
    video_info.download_status = Set(0);
    video_info.valid = Set(true);
    let video_info = video_info.update(&txn).await?;
    page::Entity::delete_many()
        .filter(page::Column::VideoId.eq(id))
        .exec(&txn)
        .await?;
    txn.commit().await?;
    let video_info = video_info.try_into_model()?;
    let warning = if video_info.path.is_empty() {
        None
    } else {
        tokio::fs::remove_dir_all(&video_info.path)
            .await
            .context(format!("删除本地路径「{}」失败", video_info.path))
            .err()
            .map(|e| format!("{:#}", e))
    };
    Ok(ApiResponse::ok(ClearAndResetVideoStatusResponse {
        warning,
        video: VideoInfo {
            id: video_info.id,
            bvid: video_info.bvid,
            name: video_info.name,
            upper_name: video_info.upper_name,
            valid: video_info.valid,
            should_download: video_info.should_download,
            download_status: video_info.download_status,
            collection_id: video_info.collection_id,
            favorite_id: video_info.favorite_id,
            submission_id: video_info.submission_id,
            watch_later_id: video_info.watch_later_id,
        },
    }))
}

pub async fn reset_filtered_video_status(
    Extension(db): Extension<DatabaseConnection>,
    Json(request): Json<ResetFilteredVideoStatusRequest>,
) -> Result<ApiResponse<ResetFilteredVideosResponse>, ApiError> {
    let mut query = video::Entity::find();
    for (field, column) in [
        (request.collection, video::Column::CollectionId),
        (request.favorite, video::Column::FavoriteId),
        (request.submission, video::Column::SubmissionId),
        (request.watch_later, video::Column::WatchLaterId),
    ] {
        if let Some(id) = field {
            query = query.filter(column.eq(id));
        }
    }
    if let Some(query_word) = request.query {
        query = query.filter(
            video::Column::Name
                .contains(&query_word)
                .or(video::Column::Bvid.contains(query_word)),
        );
    }
    if let Some(status_filter) = request.status_filter {
        query = query.filter(status_filter.to_video_query());
    }
    if let Some(validation_filter) = request.validation_filter {
        query = query.filter(validation_filter.to_video_query());
    }
    query = apply_created_at_filter(&db, query, request.created_from, request.created_to);
    let all_videos = query.into_partial_model::<SimpleVideoInfo>().all(&db).await?;
    let all_pages = page::Entity::find()
        .filter(page::Column::VideoId.is_in(all_videos.iter().map(|v| v.id)))
        .into_partial_model::<SimplePageInfo>()
        .all(&db)
        .await?;
    let resetted_pages_info = all_pages
        .into_iter()
        .filter_map(|mut page_info| {
            let mut page_status = PageStatus::from(page_info.download_status);
            if (request.force && page_status.force_reset_failed()) || page_status.reset_failed() {
                page_info.download_status = i64::from(page_status);
                Some(page_info)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    let video_ids_with_resetted_pages: HashSet<i32> = resetted_pages_info.iter().map(|page| page.video_id).collect();
    let resetted_videos_info = all_videos
        .into_iter()
        .filter_map(|mut video_info| {
            let mut video_status = VideoStatus::from(video_info.download_status);
            let mut video_resetted =
                (request.force && video_status.force_reset_failed()) || video_status.reset_failed();
            if video_ids_with_resetted_pages.contains(&video_info.id) {
                video_status.set(4, 0); // 将"分页下载"重置为 0
                video_resetted = true;
            }
            if video_resetted {
                video_info.download_status = i64::from(video_status);
                Some(video_info)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    let has_video_updates = !resetted_videos_info.is_empty();
    let has_page_updates = !resetted_pages_info.is_empty();
    if has_video_updates || has_page_updates {
        let txn = db.begin().await?;
        if has_video_updates {
            update_video_download_status(&txn, &resetted_videos_info, Some(500)).await?;
        }
        if has_page_updates {
            update_page_download_status(&txn, &resetted_pages_info, Some(500)).await?;
        }
        txn.commit().await?;
    }
    Ok(ApiResponse::ok(ResetFilteredVideosResponse {
        resetted: has_video_updates || has_page_updates,
        resetted_videos_count: resetted_videos_info.len(),
        resetted_pages_count: resetted_pages_info.len(),
    }))
}

pub async fn update_video_status(
    Path(id): Path<i32>,
    Extension(db): Extension<DatabaseConnection>,
    ValidatedJson(request): ValidatedJson<UpdateVideoStatusRequest>,
) -> Result<ApiResponse<UpdateVideoStatusResponse>, ApiError> {
    let (video_info, mut pages_info) = tokio::try_join!(
        video::Entity::find_by_id(id).into_partial_model::<VideoInfo>().one(&db),
        page::Entity::find()
            .filter(page::Column::VideoId.eq(id))
            .order_by_asc(page::Column::Cid)
            .into_partial_model::<PageInfo>()
            .all(&db)
    )?;
    let Some(mut video_info) = video_info else {
        return Err(InnerApiError::NotFound(id).into());
    };
    let mut video_status = VideoStatus::from(video_info.download_status);
    for update in &request.video_updates {
        video_status.set(update.status_index, update.status_value);
    }
    video_info.download_status = i64::from(video_status);
    let mut updated_pages_info = Vec::new();
    let mut page_id_map = pages_info
        .iter_mut()
        .map(|page| (page.id, page))
        .collect::<std::collections::HashMap<_, _>>();
    for page_update in &request.page_updates {
        if let Some(page_info) = page_id_map.remove(&page_update.page_id) {
            let mut page_status = PageStatus::from(page_info.download_status);
            for update in &page_update.updates {
                page_status.set(update.status_index, update.status_value);
            }
            page_info.download_status = i64::from(page_status);
            updated_pages_info.push(page_info);
        }
    }
    let has_video_updates = !request.video_updates.is_empty();
    let has_page_updates = !updated_pages_info.is_empty();
    if has_video_updates || has_page_updates {
        let txn = db.begin().await?;
        if has_video_updates {
            update_video_download_status::<VideoInfo>(&txn, &[&video_info], None).await?;
        }
        if has_page_updates {
            update_page_download_status::<PageInfo>(&txn, &updated_pages_info, None).await?;
        }
        txn.commit().await?;
    }
    Ok(ApiResponse::ok(UpdateVideoStatusResponse {
        success: has_video_updates || has_page_updates,
        video: video_info,
        pages: pages_info,
    }))
}

pub async fn update_filtered_video_status(
    Extension(db): Extension<DatabaseConnection>,
    ValidatedJson(request): ValidatedJson<UpdateFilteredVideoStatusRequest>,
) -> Result<ApiResponse<UpdateFilteredVideoStatusResponse>, ApiError> {
    let mut query = video::Entity::find();
    for (field, column) in [
        (request.collection, video::Column::CollectionId),
        (request.favorite, video::Column::FavoriteId),
        (request.submission, video::Column::SubmissionId),
        (request.watch_later, video::Column::WatchLaterId),
    ] {
        if let Some(id) = field {
            query = query.filter(column.eq(id));
        }
    }
    if let Some(query_word) = request.query {
        query = query.filter(
            video::Column::Name
                .contains(&query_word)
                .or(video::Column::Bvid.contains(query_word)),
        );
    }
    if let Some(status_filter) = request.status_filter {
        query = query.filter(status_filter.to_video_query());
    }
    if let Some(validation_filter) = request.validation_filter {
        query = query.filter(validation_filter.to_video_query());
    }
    query = apply_created_at_filter(&db, query, request.created_from, request.created_to);
    let mut all_videos = query.into_partial_model::<SimpleVideoInfo>().all(&db).await?;
    let mut all_pages = page::Entity::find()
        .filter(page::Column::VideoId.is_in(all_videos.iter().map(|v| v.id)))
        .into_partial_model::<SimplePageInfo>()
        .all(&db)
        .await?;
    for video_info in all_videos.iter_mut() {
        let mut video_status = VideoStatus::from(video_info.download_status);
        for update in &request.video_updates {
            video_status.set(update.status_index, update.status_value);
        }
        video_info.download_status = i64::from(video_status);
    }
    for page_info in all_pages.iter_mut() {
        let mut page_status = PageStatus::from(page_info.download_status);
        for update in &request.page_updates {
            page_status.set(update.status_index, update.status_value);
        }
        page_info.download_status = i64::from(page_status);
    }
    let has_video_updates = !all_videos.is_empty();
    let has_page_updates = !all_pages.is_empty();
    if has_video_updates || has_page_updates {
        let txn = db.begin().await?;
        if has_video_updates {
            update_video_download_status(&txn, &all_videos, Some(500)).await?;
        }
        if has_page_updates {
            update_page_download_status(&txn, &all_pages, Some(500)).await?;
        }
        txn.commit().await?;
    }
    Ok(ApiResponse::ok(UpdateFilteredVideoStatusResponse {
        success: has_video_updates || has_page_updates,
        updated_videos_count: all_videos.len(),
        updated_pages_count: all_pages.len(),
    }))
}
