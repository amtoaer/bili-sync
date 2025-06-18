use std::collections::HashSet;
use std::sync::Arc;

use anyhow::Result;
use axum::Router;
use axum::body::Body;
use axum::extract::{Extension, Path, Query};
use axum::response::Response;
use axum::routing::{get, post, put};
use bili_sync_entity::*;
use bili_sync_migration::{Expr, OnConflict};
use reqwest::{Method, StatusCode, header};
use sea_orm::ActiveValue::Set;
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect,
    TransactionTrait,
};
use utoipa::OpenApi;

use super::request::ImageProxyParams;
use crate::adapter::_ActiveModel;
use crate::api::auth::OpenAPIAuth;
use crate::api::error::InnerApiError;
use crate::api::helper::{update_page_download_status, update_video_download_status};
use crate::api::request::{
    FollowedCollectionsRequest, FollowedUppersRequest, UpdateVideoSourceRequest, UpdateVideoStatusRequest,
    UpsertCollectionRequest, UpsertFavoriteRequest, UpsertSubmissionRequest, VideosRequest,
};
use crate::api::response::{
    CollectionWithSubscriptionStatus, CollectionsResponse, FavoriteWithSubscriptionStatus, FavoritesResponse, PageInfo,
    ResetAllVideosResponse, ResetVideoResponse, UpdateVideoStatusResponse, UpperWithSubscriptionStatus, UppersResponse,
    VideoInfo, VideoResponse, VideoSource, VideoSourceDetail, VideoSourcesDetailsResponse, VideoSourcesResponse,
    VideosResponse,
};
use crate::api::wrapper::{ApiError, ApiResponse, ValidatedJson};
use crate::bilibili::{BiliClient, Collection, CollectionItem, FavoriteList, Me, Submission};
use crate::config::{Config, VersionedConfig};
use crate::task::DOWNLOADER_TASK_RUNNING;
use crate::utils::status::{PageStatus, VideoStatus};

#[derive(OpenApi)]
#[openapi(
    paths(
        get_video_sources, get_video_sources_details, update_video_source, get_videos, get_video, reset_video, reset_all_videos, update_video_status,
        get_created_favorites, get_followed_collections, get_followed_uppers,
        upsert_favorite, upsert_collection, upsert_submission
    ),
    modifiers(&OpenAPIAuth),
    security(
        ("Token" = []),
    )
)]
pub struct ApiDoc;

pub fn api_router() -> Router {
    Router::new()
        .route("/api/video-sources", get(get_video_sources))
        .route("/api/video-sources/details", get(get_video_sources_details))
        .route("/api/video-sources/{type}/{id}", put(update_video_source))
        .route("/api/video-sources/collections", post(upsert_collection))
        .route("/api/video-sources/favorites", post(upsert_favorite))
        .route("/api/video-sources/submissions", post(upsert_submission))
        .route("/api/videos", get(get_videos))
        .route("/api/videos/{id}", get(get_video))
        .route("/api/videos/{id}/reset", post(reset_video))
        .route("/api/videos/reset-all", post(reset_all_videos))
        .route("/api/videos/{id}/update-status", post(update_video_status))
        .route("/api/me/favorites", get(get_created_favorites))
        .route("/api/me/collections", get(get_followed_collections))
        .route("/api/me/uppers", get(get_followed_uppers))
        .route("/api/config", get(get_config))
        .route("/api/config", put(update_config))
        .route("/image-proxy", get(image_proxy))
}

/// 列出所有视频来源
#[utoipa::path(
    get,
    path = "/api/video-sources",
    responses(
        (status = 200, body = ApiResponse<VideoSourcesResponse>),
    )
)]
pub async fn get_video_sources(
    Extension(db): Extension<Arc<DatabaseConnection>>,
) -> Result<ApiResponse<VideoSourcesResponse>, ApiError> {
    let (collection, favorite, submission, mut watch_later) = tokio::try_join!(
        collection::Entity::find()
            .select_only()
            .columns([collection::Column::Id, collection::Column::Name])
            .into_model::<VideoSource>()
            .all(db.as_ref()),
        favorite::Entity::find()
            .select_only()
            .columns([favorite::Column::Id, favorite::Column::Name])
            .into_model::<VideoSource>()
            .all(db.as_ref()),
        submission::Entity::find()
            .select_only()
            .column(submission::Column::Id)
            .column_as(submission::Column::UpperName, "name")
            .into_model::<VideoSource>()
            .all(db.as_ref()),
        watch_later::Entity::find()
            .select_only()
            .column(watch_later::Column::Id)
            .column_as(Expr::value("稍后再看"), "name")
            .into_model::<VideoSource>()
            .all(db.as_ref())
    )?;
    // watch_later 是一个特殊的视频来源，如果不存在则添加一个默认项
    if watch_later.is_empty() {
        watch_later.push(VideoSource {
            id: 1,
            name: "稍后再看".to_string(),
        });
    }
    Ok(ApiResponse::ok(VideoSourcesResponse {
        collection,
        favorite,
        submission,
        watch_later,
    }))
}

/// 列出视频的基本信息，支持根据视频来源筛选、名称查找和分页
#[utoipa::path(
    get,
    path = "/api/videos",
    params(
        VideosRequest,
    ),
    responses(
        (status = 200, body = ApiResponse<VideosResponse>),
    )
)]
pub async fn get_videos(
    Extension(db): Extension<Arc<DatabaseConnection>>,
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
        query = query.filter(video::Column::Name.contains(query_word));
    }
    let total_count = query.clone().count(db.as_ref()).await?;
    let (page, page_size) = if let (Some(page), Some(page_size)) = (params.page, params.page_size) {
        (page, page_size)
    } else {
        (0, 10)
    };
    Ok(ApiResponse::ok(VideosResponse {
        videos: query
            .order_by_desc(video::Column::Id)
            .into_partial_model::<VideoInfo>()
            .paginate(db.as_ref(), page_size)
            .fetch_page(page)
            .await?,
        total_count,
    }))
}

/// 获取视频详细信息，包括关联的所有 page
#[utoipa::path(
    get,
    path = "/api/videos/{id}",
    responses(
        (status = 200, body = ApiResponse<VideoResponse>),
    )
)]
pub async fn get_video(
    Path(id): Path<i32>,
    Extension(db): Extension<Arc<DatabaseConnection>>,
) -> Result<ApiResponse<VideoResponse>, ApiError> {
    let (video_info, pages_info) = tokio::try_join!(
        video::Entity::find_by_id(id)
            .into_partial_model::<VideoInfo>()
            .one(db.as_ref()),
        page::Entity::find()
            .filter(page::Column::VideoId.eq(id))
            .order_by_asc(page::Column::Cid)
            .into_partial_model::<PageInfo>()
            .all(db.as_ref())
    )?;
    let Some(video_info) = video_info else {
        return Err(InnerApiError::NotFound(id).into());
    };
    Ok(ApiResponse::ok(VideoResponse {
        video: video_info,
        pages: pages_info,
    }))
}

/// 将某个视频与其所有分页的失败状态清空为未下载状态，这样在下次下载任务中会触发重试
#[utoipa::path(
    post,
    path = "/api/videos/{id}/reset",
    responses(
        (status = 200, body = ApiResponse<ResetVideoResponse>),
    )
)]
pub async fn reset_video(
    Path(id): Path<i32>,
    Extension(db): Extension<Arc<DatabaseConnection>>,
) -> Result<ApiResponse<ResetVideoResponse>, ApiError> {
    let (video_info, pages_info) = tokio::try_join!(
        video::Entity::find_by_id(id)
            .into_partial_model::<VideoInfo>()
            .one(db.as_ref()),
        page::Entity::find()
            .filter(page::Column::VideoId.eq(id))
            .order_by_asc(page::Column::Cid)
            .into_partial_model::<PageInfo>()
            .all(db.as_ref())
    )?;
    let Some(mut video_info) = video_info else {
        return Err(InnerApiError::NotFound(id).into());
    };
    let resetted_pages_info = pages_info
        .into_iter()
        .filter_map(|mut page_info| {
            let mut page_status = PageStatus::from(page_info.download_status);
            if page_status.reset_failed() {
                page_info.download_status = page_status.into();
                Some(page_info)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    let mut video_status = VideoStatus::from(video_info.download_status);
    let mut video_resetted = video_status.reset_failed();
    if !resetted_pages_info.is_empty() {
        video_status.set(4, 0); //  将“分P下载”重置为 0
        video_resetted = true;
    }
    let resetted_videos_info = if video_resetted {
        video_info.download_status = video_status.into();
        vec![&video_info]
    } else {
        vec![]
    };
    let resetted = !resetted_videos_info.is_empty() || !resetted_pages_info.is_empty();
    if resetted {
        let txn = db.begin().await?;
        if !resetted_videos_info.is_empty() {
            // 只可能有 1 个元素，所以不用 batch
            update_video_download_status(&txn, &resetted_videos_info, None).await?;
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

/// 重置所有视频和页面的失败状态为未下载状态，这样在下次下载任务中会触发重试
#[utoipa::path(
    post,
    path = "/api/videos/reset-all",
    responses(
        (status = 200, body = ApiResponse<ResetAllVideosResponse>),
    )
)]
pub async fn reset_all_videos(
    Extension(db): Extension<Arc<DatabaseConnection>>,
) -> Result<ApiResponse<ResetAllVideosResponse>, ApiError> {
    // 先查询所有视频和页面数据
    let (all_videos, all_pages) = tokio::try_join!(
        video::Entity::find().into_partial_model::<VideoInfo>().all(db.as_ref()),
        page::Entity::find().into_partial_model::<PageInfo>().all(db.as_ref())
    )?;
    let resetted_pages_info = all_pages
        .into_iter()
        .filter_map(|mut page_info| {
            let mut page_status = PageStatus::from(page_info.download_status);
            if page_status.reset_failed() {
                page_info.download_status = page_status.into();
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
            let mut video_resetted = video_status.reset_failed();
            if video_ids_with_resetted_pages.contains(&video_info.id) {
                video_status.set(4, 0); // 将"分P下载"重置为 0
                video_resetted = true;
            }
            if video_resetted {
                video_info.download_status = video_status.into();
                Some(video_info)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    let resetted = !(resetted_videos_info.is_empty() && resetted_pages_info.is_empty());
    if resetted {
        let txn = db.begin().await?;
        if !resetted_videos_info.is_empty() {
            update_video_download_status(&txn, &resetted_videos_info, Some(500)).await?;
        }
        if !resetted_pages_info.is_empty() {
            update_page_download_status(&txn, &resetted_pages_info, Some(500)).await?;
        }
        txn.commit().await?;
    }
    Ok(ApiResponse::ok(ResetAllVideosResponse {
        resetted,
        resetted_videos_count: resetted_videos_info.len(),
        resetted_pages_count: resetted_pages_info.len(),
    }))
}

/// 更新特定视频及其所含分页的状态位
#[utoipa::path(
    post,
    path = "/api/video/{id}/update-status",
    request_body = UpdateVideoStatusRequest,
    responses(
        (status = 200, body = ApiResponse<UpdateVideoStatusResponse>),
    )
)]
pub async fn update_video_status(
    Path(id): Path<i32>,
    Extension(db): Extension<Arc<DatabaseConnection>>,
    ValidatedJson(request): ValidatedJson<UpdateVideoStatusRequest>,
) -> Result<ApiResponse<UpdateVideoStatusResponse>, ApiError> {
    let (video_info, mut pages_info) = tokio::try_join!(
        video::Entity::find_by_id(id)
            .into_partial_model::<VideoInfo>()
            .one(db.as_ref()),
        page::Entity::find()
            .filter(page::Column::VideoId.eq(id))
            .order_by_asc(page::Column::Cid)
            .into_partial_model::<PageInfo>()
            .all(db.as_ref())
    )?;
    let Some(mut video_info) = video_info else {
        return Err(InnerApiError::NotFound(id).into());
    };
    let mut video_status = VideoStatus::from(video_info.download_status);
    for update in &request.video_updates {
        video_status.set(update.status_index, update.status_value);
    }
    video_info.download_status = video_status.into();
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
            page_info.download_status = page_status.into();
            updated_pages_info.push(page_info);
        }
    }
    let has_video_updates = !request.video_updates.is_empty();
    let has_page_updates = !updated_pages_info.is_empty();
    if has_video_updates || has_page_updates {
        let txn = db.begin().await?;
        if has_video_updates {
            update_video_download_status(&txn, &[&video_info], None).await?;
        }
        if has_page_updates {
            update_page_download_status(&txn, &updated_pages_info, None).await?;
        }
        txn.commit().await?;
    }
    Ok(ApiResponse::ok(UpdateVideoStatusResponse {
        success: has_video_updates || has_page_updates,
        video: video_info,
        pages: pages_info,
    }))
}

/// 获取当前用户创建的收藏夹列表，包含订阅状态
#[utoipa::path(
    get,
    path = "/api/me/favorites",
    responses(
        (status = 200, body = ApiResponse<FavoritesResponse>),
    )
)]
pub async fn get_created_favorites(
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Extension(bili_client): Extension<Arc<BiliClient>>,
) -> Result<ApiResponse<FavoritesResponse>, ApiError> {
    let me = Me::new(bili_client.as_ref());
    let bili_favorites = me.get_created_favorites().await?;

    let favorites = if let Some(bili_favorites) = bili_favorites {
        // b 站收藏夹相关接口使用的所谓 “fid” 其实是该处的 id，即 fid + mid 后两位
        let bili_fids: Vec<_> = bili_favorites.iter().map(|fav| fav.id).collect();

        let subscribed_fids: Vec<i64> = favorite::Entity::find()
            .select_only()
            .column(favorite::Column::FId)
            .filter(favorite::Column::FId.is_in(bili_fids))
            .into_tuple()
            .all(db.as_ref())
            .await?;
        let subscribed_set: HashSet<i64> = subscribed_fids.into_iter().collect();

        bili_favorites
            .into_iter()
            .map(|fav| FavoriteWithSubscriptionStatus {
                title: fav.title,
                media_count: fav.media_count,
                // api 返回的 id 才是真实的 fid
                fid: fav.id,
                mid: fav.mid,
                subscribed: subscribed_set.contains(&fav.id),
            })
            .collect()
    } else {
        vec![]
    };

    Ok(ApiResponse::ok(FavoritesResponse { favorites }))
}

/// 获取当前用户关注的合集列表，包含订阅状态
#[utoipa::path(
    get,
    path = "/api/me/collections",
    params(
        FollowedCollectionsRequest,
    ),
    responses(
        (status = 200, body = ApiResponse<CollectionsResponse>),
    )
)]
pub async fn get_followed_collections(
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Extension(bili_client): Extension<Arc<BiliClient>>,
    Query(params): Query<FollowedCollectionsRequest>,
) -> Result<ApiResponse<CollectionsResponse>, ApiError> {
    let me = Me::new(bili_client.as_ref());
    let (page_num, page_size) = (params.page_num.unwrap_or(1), params.page_size.unwrap_or(50));
    let bili_collections = me.get_followed_collections(page_num, page_size).await?;

    let collections = if let Some(collection_list) = bili_collections.list {
        let bili_sids: Vec<_> = collection_list.iter().map(|col| col.id).collect();

        let subscribed_ids: Vec<i64> = collection::Entity::find()
            .select_only()
            .column(collection::Column::SId)
            .filter(collection::Column::SId.is_in(bili_sids))
            .into_tuple()
            .all(db.as_ref())
            .await?;
        let subscribed_set: HashSet<i64> = subscribed_ids.into_iter().collect();

        collection_list
            .into_iter()
            .map(|col| CollectionWithSubscriptionStatus {
                title: col.title,
                sid: col.id,
                mid: col.mid,
                invalid: col.state == 1,
                subscribed: subscribed_set.contains(&col.id),
            })
            .collect()
    } else {
        vec![]
    };

    Ok(ApiResponse::ok(CollectionsResponse {
        collections,
        total: bili_collections.count,
    }))
}

#[utoipa::path(
    get,
    path = "/api/me/uppers",
    params(
        FollowedUppersRequest,
    ),
    responses(
        (status = 200, body = ApiResponse<UppersResponse>),
    )
)]
pub async fn get_followed_uppers(
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Extension(bili_client): Extension<Arc<BiliClient>>,
    Query(params): Query<FollowedUppersRequest>,
) -> Result<ApiResponse<UppersResponse>, ApiError> {
    let me = Me::new(bili_client.as_ref());
    let (page_num, page_size) = (params.page_num.unwrap_or(1), params.page_size.unwrap_or(20));
    let bili_uppers = me.get_followed_uppers(page_num, page_size).await?;

    let bili_uid: Vec<_> = bili_uppers.list.iter().map(|upper| upper.mid).collect();

    let subscribed_ids: Vec<i64> = submission::Entity::find()
        .select_only()
        .column(submission::Column::UpperId)
        .filter(submission::Column::UpperId.is_in(bili_uid))
        .into_tuple()
        .all(db.as_ref())
        .await?;
    let subscribed_set: HashSet<i64> = subscribed_ids.into_iter().collect();

    let uppers = bili_uppers
        .list
        .into_iter()
        .map(|upper| UpperWithSubscriptionStatus {
            mid: upper.mid,
            // 官方没有提供字段，但是可以使用这种方式简单判断下
            invalid: upper.uname == "账号已注销" && upper.face == "https://i0.hdslb.com/bfs/face/member/noface.jpg",
            uname: upper.uname,
            face: upper.face,
            sign: upper.sign,
            subscribed: subscribed_set.contains(&upper.mid),
        })
        .collect();

    Ok(ApiResponse::ok(UppersResponse {
        uppers,
        total: bili_uppers.total,
    }))
}

#[utoipa::path(
    post,
    path = "/api/video-sources/favorites",
    request_body = UpsertFavoriteRequest,
    responses(
        (status = 200, body = ApiResponse<bool>),
    )
)]
pub async fn upsert_favorite(
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Extension(bili_client): Extension<Arc<BiliClient>>,
    ValidatedJson(request): ValidatedJson<UpsertFavoriteRequest>,
) -> Result<ApiResponse<bool>, ApiError> {
    let favorite = FavoriteList::new(bili_client.as_ref(), request.fid.to_string());
    let favorite_info = favorite.get_info().await?;
    favorite::Entity::insert(favorite::ActiveModel {
        f_id: Set(favorite_info.id),
        name: Set(favorite_info.title.clone()),
        path: Set(request.path),
        ..Default::default()
    })
    .on_conflict(
        OnConflict::column(favorite::Column::FId)
            .update_columns([favorite::Column::Name, favorite::Column::Path])
            .to_owned(),
    )
    .exec(db.as_ref())
    .await?;

    Ok(ApiResponse::ok(true))
}

#[utoipa::path(
    post,
    path = "/api/video-sources/collections",
    request_body = UpsertCollectionRequest,
    responses(
        (status = 200, body = ApiResponse<bool>),
    )
)]
pub async fn upsert_collection(
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Extension(bili_client): Extension<Arc<BiliClient>>,
    ValidatedJson(request): ValidatedJson<UpsertCollectionRequest>,
) -> Result<ApiResponse<bool>, ApiError> {
    let collection = Collection::new(
        bili_client.as_ref(),
        CollectionItem {
            sid: request.sid.to_string(),
            mid: request.mid.to_string(),
            collection_type: request.collection_type,
        },
    );
    let collection_info = collection.get_info().await?;

    collection::Entity::insert(collection::ActiveModel {
        s_id: Set(collection_info.sid),
        m_id: Set(collection_info.mid),
        r#type: Set(collection_info.collection_type.into()),
        name: Set(collection_info.name.clone()),
        path: Set(request.path),
        ..Default::default()
    })
    .on_conflict(
        OnConflict::columns([
            collection::Column::SId,
            collection::Column::MId,
            collection::Column::Type,
        ])
        .update_columns([collection::Column::Name, collection::Column::Path])
        .to_owned(),
    )
    .exec(db.as_ref())
    .await?;

    Ok(ApiResponse::ok(true))
}

/// 订阅UP主投稿
#[utoipa::path(
    post,
    path = "/api/video-sources/submissions",
    request_body = UpsertSubmissionRequest,
    responses(
        (status = 200, body = ApiResponse<bool>),
    )
)]
pub async fn upsert_submission(
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Extension(bili_client): Extension<Arc<BiliClient>>,
    ValidatedJson(request): ValidatedJson<UpsertSubmissionRequest>,
) -> Result<ApiResponse<bool>, ApiError> {
    let submission = Submission::new(bili_client.as_ref(), request.upper_id.to_string());
    let upper = submission.get_info().await?;

    submission::Entity::insert(submission::ActiveModel {
        upper_id: Set(upper.mid.parse()?),
        upper_name: Set(upper.name),
        path: Set(request.path),
        ..Default::default()
    })
    .on_conflict(
        OnConflict::column(submission::Column::UpperId)
            .update_columns([submission::Column::UpperName, submission::Column::Path])
            .to_owned(),
    )
    .exec(db.as_ref())
    .await?;

    Ok(ApiResponse::ok(true))
}

/// 获取所有视频源的详细信息，包括 path 和 enabled 状态
#[utoipa::path(
    get,
    path = "/api/video-sources/details",
    responses(
        (status = 200, body = ApiResponse<VideoSourcesDetailsResponse>),
    )
)]
pub async fn get_video_sources_details(
    Extension(db): Extension<Arc<DatabaseConnection>>,
) -> Result<ApiResponse<VideoSourcesDetailsResponse>, ApiError> {
    let (collections, favorites, submissions, mut watch_later) = tokio::try_join!(
        collection::Entity::find()
            .select_only()
            .columns([
                collection::Column::Id,
                collection::Column::Name,
                collection::Column::Path,
                collection::Column::Enabled
            ])
            .into_model::<VideoSourceDetail>()
            .all(db.as_ref()),
        favorite::Entity::find()
            .select_only()
            .columns([
                favorite::Column::Id,
                favorite::Column::Name,
                favorite::Column::Path,
                favorite::Column::Enabled
            ])
            .into_model::<VideoSourceDetail>()
            .all(db.as_ref()),
        submission::Entity::find()
            .select_only()
            .column(submission::Column::Id)
            .column_as(submission::Column::UpperName, "name")
            .columns([submission::Column::Path, submission::Column::Enabled])
            .into_model::<VideoSourceDetail>()
            .all(db.as_ref()),
        watch_later::Entity::find()
            .select_only()
            .column(watch_later::Column::Id)
            .column_as(Expr::value("稍后再看"), "name")
            .columns([watch_later::Column::Path, watch_later::Column::Enabled])
            .into_model::<VideoSourceDetail>()
            .all(db.as_ref())
    )?;
    if watch_later.is_empty() {
        watch_later.push(VideoSourceDetail {
            id: 1,
            name: "稍后再看".to_string(),
            path: String::new(),
            enabled: false,
        })
    }
    Ok(ApiResponse::ok(VideoSourcesDetailsResponse {
        collections,
        favorites,
        submissions,
        watch_later,
    }))
}

/// 更新视频源的 path 和 enabled 状态
#[utoipa::path(
    put,
    path = "/api/video-sources/{type}/{id}",
    request_body = UpdateVideoSourceRequest,
    responses(
        (status = 200, body = ApiResponse<bool>),
    )
)]
pub async fn update_video_source(
    Path((source_type, id)): Path<(String, i32)>,
    Extension(db): Extension<Arc<DatabaseConnection>>,
    ValidatedJson(request): ValidatedJson<UpdateVideoSourceRequest>,
) -> Result<ApiResponse<bool>, ApiError> {
    let active_model = match source_type.as_str() {
        "collections" => collection::Entity::find_by_id(id).one(db.as_ref()).await?.map(|model| {
            let mut active_model: collection::ActiveModel = model.into();
            active_model.path = Set(request.path);
            active_model.enabled = Set(request.enabled);
            _ActiveModel::Collection(active_model)
        }),
        "favorites" => favorite::Entity::find_by_id(id).one(db.as_ref()).await?.map(|model| {
            let mut active_model: favorite::ActiveModel = model.into();
            active_model.path = Set(request.path);
            active_model.enabled = Set(request.enabled);
            _ActiveModel::Favorite(active_model)
        }),
        "submissions" => submission::Entity::find_by_id(id).one(db.as_ref()).await?.map(|model| {
            let mut active_model: submission::ActiveModel = model.into();
            active_model.path = Set(request.path);
            active_model.enabled = Set(request.enabled);
            _ActiveModel::Submission(active_model)
        }),
        "watch_later" => match watch_later::Entity::find_by_id(id).one(db.as_ref()).await? {
            // 稍后再看需要做特殊处理，get 时如果稍后再看不存在返回的是 id 为 1 的假记录
            // 因此此处可能是更新也可能是插入，做个额外的处理
            Some(model) => {
                // 如果有记录，使用 id 对应的记录更新
                let mut active_model: watch_later::ActiveModel = model.into();
                active_model.path = Set(request.path);
                active_model.enabled = Set(request.enabled);
                Some(_ActiveModel::WatchLater(active_model))
            }
            None => {
                if id != 1 {
                    None
                } else {
                    // 如果没有记录且 id 为 1，插入一个新的稍后再看记录
                    Some(_ActiveModel::WatchLater(watch_later::ActiveModel {
                        id: Set(1),
                        path: Set(request.path),
                        enabled: Set(request.enabled),
                        ..Default::default()
                    }))
                }
            }
        },
        _ => return Err(InnerApiError::BadRequest("Invalid video source type".to_string()).into()),
    };
    let Some(active_model) = active_model else {
        return Err(InnerApiError::NotFound(id).into());
    };
    active_model.save(db.as_ref()).await?;
    Ok(ApiResponse::ok(true))
}

#[utoipa::path(
    get,
    path = "/api/config",
    responses(
        (status = 200, body = ApiResponse<Config>),
    )
)]
pub async fn get_config() -> Result<ApiResponse<Arc<Config>>, ApiError> {
    Ok(ApiResponse::ok(VersionedConfig::get().load_full()))
}

#[utoipa::path(
    put,
    path = "/api/config",
    request_body = Config,
    responses(
        (status = 200, body = ApiResponse<Config>),
    )
)]
pub async fn update_config(
    Extension(db): Extension<Arc<DatabaseConnection>>,
    ValidatedJson(config): ValidatedJson<Config>,
) -> Result<ApiResponse<Arc<Config>>, ApiError> {
    let Ok(_lock) = DOWNLOADER_TASK_RUNNING.try_lock() else {
        // 简单避免一下可能的不一致现象
        return Err(InnerApiError::BadRequest("下载任务正在运行，无法修改配置".to_string()).into());
    };
    config.check()?;
    let new_config = VersionedConfig::get().update(config, db.as_ref()).await?;
    drop(_lock);
    Ok(ApiResponse::ok(new_config))
}

/// B 站的图片会检查 referer，需要做个转发伪造一下，否则直接返回 403
pub async fn image_proxy(
    Extension(bili_client): Extension<Arc<BiliClient>>,
    Query(params): Query<ImageProxyParams>,
) -> Response {
    let resp = bili_client.client.request(Method::GET, &params.url, None).send().await;
    let whitelist = [
        header::CONTENT_TYPE,
        header::CONTENT_LENGTH,
        header::CACHE_CONTROL,
        header::EXPIRES,
        header::LAST_MODIFIED,
        header::ETAG,
        header::CONTENT_DISPOSITION,
        header::CONTENT_ENCODING,
        header::ACCEPT_RANGES,
        header::ACCESS_CONTROL_ALLOW_ORIGIN,
    ]
    .into_iter()
    .collect::<HashSet<_>>();

    let builder = Response::builder();

    let response = match resp {
        Err(e) => builder.status(StatusCode::BAD_GATEWAY).body(Body::new(e.to_string())),
        Ok(res) => {
            let mut response = builder.status(res.status());
            for (k, v) in res.headers() {
                if whitelist.contains(k) {
                    response = response.header(k, v);
                }
            }
            let streams = res.bytes_stream();
            response.body(Body::from_stream(streams))
        }
    };
    //safety: all previously configured headers are taken from a valid response, ensuring the response is safe to use
    response.unwrap()
}
