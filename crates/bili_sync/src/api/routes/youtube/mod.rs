use std::collections::HashSet;
use std::path::PathBuf;

use anyhow::Context;
use axum::Router;
use axum::extract::{Extension, Path, Query};
use axum::routing::{get, post, put};
use bili_sync_entity::{youtube_channel, youtube_video};
use sea_orm::ActiveValue::Set;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, TransactionTrait,
};

use crate::api::error::InnerApiError;
use crate::api::request::{
    DefaultPathRequest, InsertYoutubeChannelRequest, InsertYoutubePlaylistRequest, SaveYoutubeCookieRequest,
    UpdateYoutubeChannelRequest, YoutubeManualSubmitRequest,
};
use crate::api::response::{
    ContentVideoInfo, YoutubeCookieSaveResponse, YoutubeManualSubmitResponse, YoutubePlaylist,
    YoutubePlaylistsResponse, YoutubeSourceDetail, YoutubeSourcesResponse, YoutubeStatusResponse, YoutubeSubscription,
    YoutubeSubscriptionsResponse, YoutubeTaskResponse,
};
use crate::api::wrapper::{ApiError, ApiResponse, ValidatedJson};
use crate::config::{PathSafeTemplate, TEMPLATE, VersionedConfig, default_manual_download_root};
use crate::utils::status::YoutubeVideoStatus;
use crate::youtube;

pub(super) fn router() -> Router {
    Router::new()
        .route("/youtube/status", get(get_youtube_status))
        .route(
            "/youtube/cookie",
            post(save_youtube_cookie).delete(delete_youtube_cookie),
        )
        .route("/youtube/channels", get(get_subscribed_youtube_channels))
        .route("/youtube/playlists", get(get_youtube_playlists))
        .route("/youtube/sources", get(get_youtube_sources))
        .route("/youtube/sources/default-path", get(get_youtube_default_path))
        .route("/youtube/sources/channels", post(insert_youtube_channel))
        .route("/youtube/sources/playlists", post(insert_youtube_playlist))
        .route("/youtube/manual-submit", post(manual_submit_youtube_link))
        .route(
            "/youtube/tasks/{id}",
            get(get_manual_youtube_task).delete(remove_manual_youtube_task),
        )
        .route(
            "/youtube/sources/channels/{id}",
            put(update_youtube_channel).delete(remove_youtube_channel),
        )
}

async fn get_youtube_status() -> Result<ApiResponse<YoutubeStatusResponse>, ApiError> {
    let cookie_path = youtube::cookie_file_path();
    Ok(ApiResponse::ok(YoutubeStatusResponse {
        cookie_configured: cookie_path.is_file(),
        cookie_path: cookie_path.is_file().then(|| cookie_path.display().to_string()),
    }))
}

async fn save_youtube_cookie(
    ValidatedJson(request): ValidatedJson<SaveYoutubeCookieRequest>,
) -> Result<ApiResponse<YoutubeCookieSaveResponse>, ApiError> {
    let cookie_path = youtube::cookie_file_path();
    let parent = cookie_path.parent().context("invalid youtube cookie path")?;
    if request.content.trim().is_empty() {
        return Err(InnerApiError::BadRequest("Cookie 内容不能为空".to_owned()).into());
    }
    tokio::fs::create_dir_all(parent)
        .await
        .with_context(|| format!("failed to create {}", parent.display()))?;
    tokio::fs::write(&cookie_path, request.content)
        .await
        .with_context(|| format!("failed to write {}", cookie_path.display()))?;
    Ok(ApiResponse::ok(YoutubeCookieSaveResponse {
        saved: true,
        path: cookie_path.display().to_string(),
    }))
}

async fn delete_youtube_cookie() -> Result<ApiResponse<bool>, ApiError> {
    let cookie_path = youtube::cookie_file_path();
    if cookie_path.is_file() {
        tokio::fs::remove_file(&cookie_path)
            .await
            .with_context(|| format!("failed to delete {}", cookie_path.display()))?;
    }
    Ok(ApiResponse::ok(true))
}

async fn get_subscribed_youtube_channels(
    Extension(db): Extension<DatabaseConnection>,
) -> Result<ApiResponse<YoutubeSubscriptionsResponse>, ApiError> {
    if !youtube::cookie_configured() {
        return Err(InnerApiError::BadRequest("请先在设置页粘贴并保存 YouTube Cookie".to_owned()).into());
    }

    let subscriptions = youtube::list_subscriptions()
        .await
        .context("failed to fetch youtube subscriptions")?;
    let subscribed_ids: HashSet<String> = youtube_channel::Entity::find()
        .filter(youtube_channel::Column::SourceType.eq(youtube::SOURCE_TYPE_CHANNEL))
        .select_only()
        .column(youtube_channel::Column::ChannelId)
        .into_tuple::<String>()
        .all(&db)
        .await?
        .into_iter()
        .collect::<HashSet<_>>();

    let channels = subscriptions
        .into_iter()
        .map(|channel| YoutubeSubscription {
            subscribed: subscribed_ids.contains(&channel.channel_id),
            channel_id: channel.channel_id,
            name: channel.name,
            url: channel.url,
            thumbnail: channel.thumbnail,
        })
        .collect::<Vec<_>>();

    Ok(ApiResponse::ok(YoutubeSubscriptionsResponse {
        total: channels.len(),
        channels,
    }))
}

async fn get_youtube_playlists(
    Extension(db): Extension<DatabaseConnection>,
) -> Result<ApiResponse<YoutubePlaylistsResponse>, ApiError> {
    if !youtube::cookie_configured() {
        return Err(InnerApiError::BadRequest("请先在设置页粘贴并保存 YouTube Cookie".to_owned()).into());
    }

    let playlists = youtube::list_playlists()
        .await
        .context("failed to fetch youtube playlists")?;
    let added_ids: HashSet<String> = youtube_channel::Entity::find()
        .filter(youtube_channel::Column::SourceType.eq(youtube::SOURCE_TYPE_PLAYLIST))
        .select_only()
        .column(youtube_channel::Column::ChannelId)
        .into_tuple::<String>()
        .all(&db)
        .await?
        .into_iter()
        .collect::<HashSet<_>>();

    let playlists = playlists
        .into_iter()
        .map(|playlist| YoutubePlaylist {
            added: added_ids.contains(&playlist.playlist_id),
            playlist_id: playlist.playlist_id,
            name: playlist.name,
            url: playlist.url,
            thumbnail: playlist.thumbnail,
            owner_name: playlist.owner_name,
            video_count: playlist.video_count,
        })
        .collect::<Vec<_>>();

    Ok(ApiResponse::ok(YoutubePlaylistsResponse {
        total: playlists.len(),
        playlists,
    }))
}

async fn get_youtube_sources(
    Extension(db): Extension<DatabaseConnection>,
) -> Result<ApiResponse<YoutubeSourcesResponse>, ApiError> {
    let sources = youtube_channel::Entity::find()
        .filter(
            youtube_channel::Column::SourceType
                .eq(youtube::SOURCE_TYPE_CHANNEL)
                .or(youtube_channel::Column::SourceType.eq(youtube::SOURCE_TYPE_PLAYLIST)),
        )
        .select_only()
        .columns([
            youtube_channel::Column::Id,
            youtube_channel::Column::SourceType,
            youtube_channel::Column::ChannelId,
            youtube_channel::Column::Name,
            youtube_channel::Column::Url,
            youtube_channel::Column::Thumbnail,
            youtube_channel::Column::Path,
            youtube_channel::Column::LatestPublishedAt,
            youtube_channel::Column::Enabled,
        ])
        .order_by_asc(youtube_channel::Column::SourceType)
        .order_by_asc(youtube_channel::Column::Name)
        .into_model::<YoutubeSourceDetail>()
        .all(&db)
        .await?;
    Ok(ApiResponse::ok(YoutubeSourcesResponse { sources }))
}

async fn get_youtube_default_path(Query(params): Query<DefaultPathRequest>) -> Result<ApiResponse<String>, ApiError> {
    Ok(ApiResponse::ok(render_default_source_path(&params.name)?))
}

async fn insert_youtube_channel(
    Extension(db): Extension<DatabaseConnection>,
    ValidatedJson(request): ValidatedJson<InsertYoutubeChannelRequest>,
) -> Result<ApiResponse<bool>, ApiError> {
    insert_source(
        &db,
        youtube::SOURCE_TYPE_CHANNEL,
        request.channel_id,
        request.name,
        request.url,
        request.thumbnail,
        request.path,
    )
    .await?;

    Ok(ApiResponse::ok(true))
}

async fn insert_youtube_playlist(
    Extension(db): Extension<DatabaseConnection>,
    ValidatedJson(request): ValidatedJson<InsertYoutubePlaylistRequest>,
) -> Result<ApiResponse<bool>, ApiError> {
    insert_source(
        &db,
        youtube::SOURCE_TYPE_PLAYLIST,
        request.playlist_id,
        request.name,
        request.url,
        request.thumbnail,
        request.path,
    )
    .await?;

    Ok(ApiResponse::ok(true))
}

async fn manual_submit_youtube_link(
    Extension(db): Extension<DatabaseConnection>,
    ValidatedJson(request): ValidatedJson<YoutubeManualSubmitRequest>,
) -> Result<ApiResponse<YoutubeManualSubmitResponse>, ApiError> {
    let url = request.url.trim();
    let custom_path = request
        .path
        .as_deref()
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(ToOwned::to_owned);
    let submit_url = url.to_owned();
    let db = db.clone();
    tokio::spawn(async move {
        if let Err(error) = process_manual_submit(db, submit_url.clone(), custom_path).await {
            error!("YouTube 手动提交链接失败（{}）：{:#?}", submit_url, error);
        }
    });

    Ok(ApiResponse::ok(YoutubeManualSubmitResponse {
        queued: true,
        url: url.to_owned(),
    }))
}

async fn get_manual_youtube_task(
    Path(id): Path<i32>,
    Extension(db): Extension<DatabaseConnection>,
) -> Result<ApiResponse<YoutubeTaskResponse>, ApiError> {
    let (video, source) = youtube_video::Entity::find_by_id(id)
        .find_also_related(youtube_channel::Entity)
        .one(&db)
        .await?
        .ok_or_else(|| InnerApiError::NotFound(id))?;
    let Some(source) = source else {
        return Err(InnerApiError::NotFound(id).into());
    };
    if source.source_type != youtube::SOURCE_TYPE_MANUAL {
        return Err(InnerApiError::NotFound(id).into());
    }
    Ok(ApiResponse::ok(YoutubeTaskResponse {
        video: build_manual_youtube_content_video(video, source),
    }))
}

async fn remove_manual_youtube_task(
    Path(id): Path<i32>,
    Extension(db): Extension<DatabaseConnection>,
) -> Result<ApiResponse<bool>, ApiError> {
    let Some(video) = youtube_video::Entity::find_by_id(id).one(&db).await? else {
        return Err(InnerApiError::NotFound(id).into());
    };

    let Some(source) = youtube_channel::Entity::find_by_id(video.youtube_channel_id)
        .one(&db)
        .await?
    else {
        return Err(InnerApiError::NotFound(id).into());
    };
    if source.source_type != youtube::SOURCE_TYPE_MANUAL {
        return Err(InnerApiError::NotFound(id).into());
    }

    let txn = db.begin().await?;
    youtube_video::Entity::delete_by_id(id).exec(&txn).await?;
    let remaining_videos = youtube_video::Entity::find()
        .filter(youtube_video::Column::YoutubeChannelId.eq(source.id))
        .count(&txn)
        .await?;
    if remaining_videos == 0 {
        youtube_channel::Entity::delete_by_id(source.id).exec(&txn).await?;
    }
    txn.commit().await?;

    Ok(ApiResponse::ok(true))
}

async fn update_youtube_channel(
    Path(id): Path<i32>,
    Extension(db): Extension<DatabaseConnection>,
    ValidatedJson(request): ValidatedJson<UpdateYoutubeChannelRequest>,
) -> Result<ApiResponse<bool>, ApiError> {
    let Some(model) = youtube_channel::Entity::find_by_id(id).one(&db).await? else {
        return Err(InnerApiError::NotFound(id).into());
    };

    let mut active_model: youtube_channel::ActiveModel = model.into();
    active_model.path = Set(request.path);
    active_model.enabled = Set(request.enabled);
    active_model.update(&db).await?;

    Ok(ApiResponse::ok(true))
}

async fn remove_youtube_channel(
    Path(id): Path<i32>,
    Extension(db): Extension<DatabaseConnection>,
) -> Result<ApiResponse<bool>, ApiError> {
    let delete_result = youtube_channel::Entity::delete_by_id(id).exec(&db).await?;
    if delete_result.rows_affected == 0 {
        return Err(InnerApiError::NotFound(id).into());
    }
    Ok(ApiResponse::ok(true))
}

async fn insert_source(
    db: &DatabaseConnection,
    source_type: &str,
    source_id: String,
    name: String,
    url: String,
    thumbnail: Option<String>,
    path: String,
) -> Result<(), ApiError> {
    let exists = youtube_channel::Entity::find()
        .filter(youtube_channel::Column::SourceType.eq(source_type))
        .filter(youtube_channel::Column::ChannelId.eq(&source_id))
        .one(db)
        .await?;
    if exists.is_some() {
        return Err(InnerApiError::BadRequest(format!("该 YouTube {}已添加", source_type_display(source_type))).into());
    }

    youtube_channel::Entity::insert(youtube_channel::ActiveModel {
        source_type: Set(source_type.to_owned()),
        channel_id: Set(source_id),
        name: Set(name),
        url: Set(url),
        thumbnail: Set(thumbnail),
        path: Set(path),
        enabled: Set(true),
        latest_published_at: Set(None),
        ..Default::default()
    })
    .exec(db)
    .await?;

    Ok(())
}

fn render_default_source_path(name: &str) -> Result<String, ApiError> {
    let template = TEMPLATE.read();
    Ok(template.path_safe_render("youtube_channel_default_path", &serde_json::json!({ "name": name }))?)
}

fn source_type_display(source_type: &str) -> &'static str {
    match source_type {
        youtube::SOURCE_TYPE_CHANNEL => "频道",
        youtube::SOURCE_TYPE_PLAYLIST => "播放列表",
        _ => "视频源",
    }
}

async fn process_manual_submit(
    db: DatabaseConnection,
    url: String,
    custom_path: Option<String>,
) -> Result<(), ApiError> {
    let resolved = youtube::resolve_url(&url)
        .await
        .map_err(|error| InnerApiError::BadRequest(format!("{:#}", error)))?;

    match resolved.kind {
        youtube::ResolvedSourceKind::Channel => {
            let path = custom_path.unwrap_or(render_default_source_path(&resolved.name)?);
            insert_source(
                &db,
                youtube::SOURCE_TYPE_CHANNEL,
                resolved.source_id,
                resolved.name,
                resolved.url,
                resolved.thumbnail,
                path,
            )
            .await?;
        }
        youtube::ResolvedSourceKind::Playlist => {
            let path = custom_path.unwrap_or(render_default_source_path(&resolved.name)?);
            insert_source(
                &db,
                youtube::SOURCE_TYPE_PLAYLIST,
                resolved.source_id,
                resolved.name,
                resolved.url,
                resolved.thumbnail,
                path,
            )
            .await?;
        }
        youtube::ResolvedSourceKind::Video => {
            let download_path = if let Some(path) = custom_path {
                let path = PathBuf::from(path);
                if !path.is_absolute() {
                    return Err(InnerApiError::BadRequest("YouTube 手动下载路径必须是绝对路径".to_owned()).into());
                }
                path
            } else {
                default_manual_download_root()
            };
            let config = VersionedConfig::get().snapshot();
            let source = upsert_manual_source(&db, &resolved, &download_path).await?;
            let video = upsert_manual_video(&db, source.id, &resolved).await?;
            youtube::process_video(&source, &video, &db, &config)
                .await
                .map_err(|error| InnerApiError::BadRequest(format!("{:#}", error)))?;
        }
    }

    Ok(())
}

async fn upsert_manual_source(
    db: &DatabaseConnection,
    resolved: &youtube::ResolvedSource,
    download_path: &PathBuf,
) -> Result<youtube_channel::Model, ApiError> {
    let existing = youtube_channel::Entity::find()
        .filter(youtube_channel::Column::SourceType.eq(youtube::SOURCE_TYPE_MANUAL))
        .filter(youtube_channel::Column::ChannelId.eq(&resolved.source_id))
        .one(db)
        .await?;

    let path = download_path.to_string_lossy().to_string();
    match existing {
        Some(model) => {
            let mut active_model: youtube_channel::ActiveModel = model.clone().into();
            active_model.name = Set("YouTube 手动提交".to_owned());
            active_model.url = Set(resolved.url.clone());
            active_model.thumbnail = Set(resolved.thumbnail.clone());
            active_model.path = Set(path);
            active_model.enabled = Set(false);
            Ok(active_model.update(db).await?)
        }
        None => {
            let insert_result = youtube_channel::Entity::insert(youtube_channel::ActiveModel {
                source_type: Set(youtube::SOURCE_TYPE_MANUAL.to_owned()),
                channel_id: Set(resolved.source_id.clone()),
                name: Set("YouTube 手动提交".to_owned()),
                url: Set(resolved.url.clone()),
                thumbnail: Set(resolved.thumbnail.clone()),
                path: Set(path),
                enabled: Set(false),
                latest_published_at: Set(None),
                ..Default::default()
            })
            .exec(db)
            .await?;
            youtube_channel::Entity::find_by_id(insert_result.last_insert_id)
                .one(db)
                .await?
                .ok_or_else(|| InnerApiError::BadRequest("创建 YouTube 手动任务来源失败".to_owned()).into())
        }
    }
}

async fn upsert_manual_video(
    db: &DatabaseConnection,
    youtube_channel_id: i32,
    resolved: &youtube::ResolvedSource,
) -> Result<youtube_video::Model, ApiError> {
    let existing = youtube_video::Entity::find()
        .filter(youtube_video::Column::YoutubeChannelId.eq(youtube_channel_id))
        .filter(youtube_video::Column::VideoId.eq(&resolved.source_id))
        .one(db)
        .await?;

    match existing {
        Some(model) => {
            let mut active_model: youtube_video::ActiveModel = model.clone().into();
            active_model.title = Set(resolved.name.clone());
            active_model.url = Set(resolved.url.clone());
            active_model.uploader = Set(resolved.owner_name.clone().unwrap_or_else(|| "YouTube".to_owned()));
            active_model.thumbnail = Set(resolved.thumbnail.clone());
            active_model.valid = Set(true);
            active_model.should_download = Set(true);
            Ok(active_model.update(db).await?)
        }
        None => {
            let insert_result = youtube_video::Entity::insert(youtube_video::ActiveModel {
                youtube_channel_id: Set(youtube_channel_id),
                video_id: Set(resolved.source_id.clone()),
                title: Set(resolved.name.clone()),
                url: Set(resolved.url.clone()),
                description: Set(String::new()),
                uploader: Set(resolved.owner_name.clone().unwrap_or_else(|| "YouTube".to_owned())),
                thumbnail: Set(resolved.thumbnail.clone()),
                published_at: Set(chrono::Utc::now().naive_utc()),
                download_status: Set(u32::from(YoutubeVideoStatus::default())),
                valid: Set(true),
                should_download: Set(true),
                path: Set(None),
                ..Default::default()
            })
            .exec(db)
            .await?;
            youtube_video::Entity::find_by_id(insert_result.last_insert_id)
                .one(db)
                .await?
                .ok_or_else(|| InnerApiError::BadRequest("创建 YouTube 手动任务失败".to_owned()).into())
        }
    }
}

pub(crate) fn build_manual_youtube_content_video(
    video: youtube_video::Model,
    source: youtube_channel::Model,
) -> ContentVideoInfo {
    ContentVideoInfo {
        key: format!("youtube:{}", video.id),
        id: video.id,
        platform: "youtube".to_owned(),
        bvid: None,
        name: video.title,
        upper_name: video.uploader,
        valid: video.valid,
        should_download: video.should_download,
        download_status: <[u32; 4]>::from(YoutubeVideoStatus::from(video.download_status)).to_vec(),
        collection_id: None,
        favorite_id: None,
        submission_id: None,
        watch_later_id: None,
        source_type: Some("youtube_manual".to_owned()),
        source_name: Some(source.name),
        external_url: Some(video.url),
    }
}
