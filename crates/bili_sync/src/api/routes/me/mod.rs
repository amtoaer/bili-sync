use std::collections::HashSet;
use std::sync::Arc;

use anyhow::Result;
use axum::Router;
use axum::extract::{Extension, Query};
use axum::routing::get;
use bili_sync_entity::*;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QuerySelect};

use crate::api::request::{FollowedCollectionsRequest, FollowedUppersRequest};
use crate::api::response::{
    CollectionWithSubscriptionStatus, CollectionsResponse, FavoriteWithSubscriptionStatus, FavoritesResponse,
    UpperWithSubscriptionStatus, UppersResponse,
};
use crate::api::wrapper::{ApiError, ApiResponse};
use crate::bilibili::{BiliClient, Me};
use crate::config::VersionedConfig;

pub(super) fn router() -> Router {
    Router::new()
        .route("/me/favorites", get(get_created_favorites))
        .route("/me/collections", get(get_followed_collections))
        .route("/me/uppers", get(get_followed_uppers))
}

/// 获取当前用户创建的收藏夹
pub async fn get_created_favorites(
    Extension(db): Extension<DatabaseConnection>,
    Extension(bili_client): Extension<Arc<BiliClient>>,
) -> Result<ApiResponse<FavoritesResponse>, ApiError> {
    let config = VersionedConfig::get().read();
    let me = Me::new(bili_client.as_ref(), &config.credential);
    let bili_favorites = me.get_created_favorites().await?;

    let favorites = if let Some(bili_favorites) = bili_favorites {
        // b 站收藏夹相关接口使用的所谓“fid”其实是该处的 id，即 fid + mid 后两位
        let bili_fids: Vec<_> = bili_favorites.iter().map(|fav| fav.id).collect();

        let subscribed_fids: Vec<i64> = favorite::Entity::find()
            .select_only()
            .column(favorite::Column::FId)
            .filter(favorite::Column::FId.is_in(bili_fids))
            .into_tuple()
            .all(&db)
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

/// 获取当前用户收藏的合集
pub async fn get_followed_collections(
    Extension(db): Extension<DatabaseConnection>,
    Extension(bili_client): Extension<Arc<BiliClient>>,
    Query(params): Query<FollowedCollectionsRequest>,
) -> Result<ApiResponse<CollectionsResponse>, ApiError> {
    let config = VersionedConfig::get().read();
    let me = Me::new(bili_client.as_ref(), &config.credential);
    let (page_num, page_size) = (params.page_num.unwrap_or(1), params.page_size.unwrap_or(50));
    let bili_collections = me.get_followed_collections(page_num, page_size).await?;

    let collections = if let Some(collection_list) = bili_collections.list {
        let bili_sids: Vec<_> = collection_list.iter().map(|col| col.id).collect();

        let subscribed_ids: Vec<i64> = collection::Entity::find()
            .select_only()
            .column(collection::Column::SId)
            .filter(collection::Column::SId.is_in(bili_sids))
            .into_tuple()
            .all(&db)
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

/// 获取当前用户关注的 UP 主
pub async fn get_followed_uppers(
    Extension(db): Extension<DatabaseConnection>,
    Extension(bili_client): Extension<Arc<BiliClient>>,
    Query(params): Query<FollowedUppersRequest>,
) -> Result<ApiResponse<UppersResponse>, ApiError> {
    let config = VersionedConfig::get().read();
    let me = Me::new(bili_client.as_ref(), &config.credential);
    let (page_num, page_size) = (params.page_num.unwrap_or(1), params.page_size.unwrap_or(20));
    let bili_uppers = me.get_followed_uppers(page_num, page_size).await?;

    let bili_uid: Vec<_> = bili_uppers.list.iter().map(|upper| upper.mid).collect();

    let subscribed_ids: Vec<i64> = submission::Entity::find()
        .select_only()
        .column(submission::Column::UpperId)
        .filter(submission::Column::UpperId.is_in(bili_uid))
        .into_tuple()
        .all(&db)
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
