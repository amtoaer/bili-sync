use std::sync::Arc;

use anyhow::Result;
use axum::Router;
use axum::extract::{Extension, Path};
use axum::routing::{get, post, put};
use bili_sync_entity::*;
use bili_sync_migration::Expr;
use sea_orm::ActiveValue::Set;
use sea_orm::{DatabaseConnection, EntityTrait, QuerySelect};

use crate::adapter::_ActiveModel;
use crate::api::error::InnerApiError;
use crate::api::request::{
    InsertCollectionRequest, InsertFavoriteRequest, InsertSubmissionRequest, UpdateVideoSourceRequest,
};
use crate::api::response::{VideoSource, VideoSourceDetail, VideoSourcesDetailsResponse, VideoSourcesResponse};
use crate::api::wrapper::{ApiError, ApiResponse, ValidatedJson};
use crate::bilibili::{BiliClient, Collection, CollectionItem, FavoriteList, Submission};

pub(super) fn router() -> Router {
    Router::new()
        .route("/video-sources", get(get_video_sources))
        .route("/video-sources/details", get(get_video_sources_details))
        .route("/video-sources/{type}/{id}", put(update_video_source))
        .route("/video-sources/favorites", post(insert_favorite))
        .route("/video-sources/collections", post(insert_collection))
        .route("/video-sources/submissions", post(insert_submission))
}

/// 列出所有视频来源
pub async fn get_video_sources(
    Extension(db): Extension<DatabaseConnection>,
) -> Result<ApiResponse<VideoSourcesResponse>, ApiError> {
    let (collection, favorite, submission, mut watch_later) = tokio::try_join!(
        collection::Entity::find()
            .select_only()
            .columns([collection::Column::Id, collection::Column::Name])
            .into_model::<VideoSource>()
            .all(&db),
        favorite::Entity::find()
            .select_only()
            .columns([favorite::Column::Id, favorite::Column::Name])
            .into_model::<VideoSource>()
            .all(&db),
        submission::Entity::find()
            .select_only()
            .column(submission::Column::Id)
            .column_as(submission::Column::UpperName, "name")
            .into_model::<VideoSource>()
            .all(&db),
        watch_later::Entity::find()
            .select_only()
            .column(watch_later::Column::Id)
            .column_as(Expr::value("稍后再看"), "name")
            .into_model::<VideoSource>()
            .all(&db)
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

/// 获取视频来源详情
pub async fn get_video_sources_details(
    Extension(db): Extension<DatabaseConnection>,
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
            .all(&db),
        favorite::Entity::find()
            .select_only()
            .columns([
                favorite::Column::Id,
                favorite::Column::Name,
                favorite::Column::Path,
                favorite::Column::Enabled
            ])
            .into_model::<VideoSourceDetail>()
            .all(&db),
        submission::Entity::find()
            .select_only()
            .column(submission::Column::Id)
            .column_as(submission::Column::UpperName, "name")
            .columns([submission::Column::Path, submission::Column::Enabled])
            .into_model::<VideoSourceDetail>()
            .all(&db),
        watch_later::Entity::find()
            .select_only()
            .column(watch_later::Column::Id)
            .column_as(Expr::value("稍后再看"), "name")
            .columns([watch_later::Column::Path, watch_later::Column::Enabled])
            .into_model::<VideoSourceDetail>()
            .all(&db)
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

/// 更新视频来源
pub async fn update_video_source(
    Path((source_type, id)): Path<(String, i32)>,
    Extension(db): Extension<DatabaseConnection>,
    ValidatedJson(request): ValidatedJson<UpdateVideoSourceRequest>,
) -> Result<ApiResponse<bool>, ApiError> {
    let active_model = match source_type.as_str() {
        "collections" => collection::Entity::find_by_id(id).one(&db).await?.map(|model| {
            let mut active_model: collection::ActiveModel = model.into();
            active_model.path = Set(request.path);
            active_model.enabled = Set(request.enabled);
            _ActiveModel::Collection(active_model)
        }),
        "favorites" => favorite::Entity::find_by_id(id).one(&db).await?.map(|model| {
            let mut active_model: favorite::ActiveModel = model.into();
            active_model.path = Set(request.path);
            active_model.enabled = Set(request.enabled);
            _ActiveModel::Favorite(active_model)
        }),
        "submissions" => submission::Entity::find_by_id(id).one(&db).await?.map(|model| {
            let mut active_model: submission::ActiveModel = model.into();
            active_model.path = Set(request.path);
            active_model.enabled = Set(request.enabled);
            _ActiveModel::Submission(active_model)
        }),
        "watch_later" => match watch_later::Entity::find_by_id(id).one(&db).await? {
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
    active_model.save(&db).await?;
    Ok(ApiResponse::ok(true))
}

/// 新增收藏夹订阅
pub async fn insert_favorite(
    Extension(db): Extension<DatabaseConnection>,
    Extension(bili_client): Extension<Arc<BiliClient>>,
    ValidatedJson(request): ValidatedJson<InsertFavoriteRequest>,
) -> Result<ApiResponse<bool>, ApiError> {
    let favorite = FavoriteList::new(bili_client.as_ref(), request.fid.to_string());
    let favorite_info = favorite.get_info().await?;
    favorite::Entity::insert(favorite::ActiveModel {
        f_id: Set(favorite_info.id),
        name: Set(favorite_info.title.clone()),
        path: Set(request.path),
        enabled: Set(true),
        ..Default::default()
    })
    .exec(&db)
    .await?;
    Ok(ApiResponse::ok(true))
}

/// 新增合集/列表订阅
pub async fn insert_collection(
    Extension(db): Extension<DatabaseConnection>,
    Extension(bili_client): Extension<Arc<BiliClient>>,
    ValidatedJson(request): ValidatedJson<InsertCollectionRequest>,
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
        enabled: Set(true),
        ..Default::default()
    })
    .exec(&db)
    .await?;

    Ok(ApiResponse::ok(true))
}

/// 新增投稿订阅
pub async fn insert_submission(
    Extension(db): Extension<DatabaseConnection>,
    Extension(bili_client): Extension<Arc<BiliClient>>,
    ValidatedJson(request): ValidatedJson<InsertSubmissionRequest>,
) -> Result<ApiResponse<bool>, ApiError> {
    let submission = Submission::new(bili_client.as_ref(), request.upper_id.to_string());
    let upper = submission.get_info().await?;
    submission::Entity::insert(submission::ActiveModel {
        upper_id: Set(upper.mid.parse()?),
        upper_name: Set(upper.name),
        path: Set(request.path),
        enabled: Set(true),
        ..Default::default()
    })
    .exec(&db)
    .await?;
    Ok(ApiResponse::ok(true))
}
