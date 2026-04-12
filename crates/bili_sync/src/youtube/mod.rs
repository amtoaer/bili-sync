mod helper;
mod rss;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use bili_sync_entity::{youtube_channel, youtube_video};
pub use helper::{Playlist, ResolvedSource, ResolvedSourceKind, Subscription};
use sea_orm::ActiveValue::Set;
use sea_orm::entity::prelude::*;
use sea_orm::sea_query::OnConflict;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder};

use crate::config::{CONFIG_DIR, Config};
use crate::utils::compact_log_text;
use crate::utils::status::{STATUS_COMPLETED, STATUS_MAX_RETRY, STATUS_OK, YoutubeVideoStatus};

pub const SOURCE_TYPE_CHANNEL: &str = "channel";
pub const SOURCE_TYPE_PLAYLIST: &str = "playlist";
pub const SOURCE_TYPE_MANUAL: &str = "manual";

pub fn cookie_file_path() -> PathBuf {
    CONFIG_DIR.join("youtube").join("cookies.txt")
}

pub fn cookie_configured() -> bool {
    cookie_file_path().is_file()
}

fn optional_cookie_path() -> Option<PathBuf> {
    let cookie_path = cookie_file_path();
    cookie_path.is_file().then_some(cookie_path)
}

pub async fn list_subscriptions() -> Result<Vec<Subscription>> {
    let cookie_path = cookie_file_path();
    helper::list_subscriptions(&cookie_path).await
}

pub async fn list_playlists() -> Result<Vec<Playlist>> {
    let cookie_path = cookie_file_path();
    helper::list_playlists(&cookie_path).await
}

pub async fn resolve_url(url: &str) -> Result<ResolvedSource> {
    let cookie_path = optional_cookie_path();
    helper::resolve_source(url, cookie_path.as_deref()).await
}

pub async fn process_enabled_sources(connection: &DatabaseConnection, config: &Config) -> Result<()> {
    let sources = enabled_sources(connection).await?;

    if sources.is_empty() {
        return Ok(());
    }

    for source in sources {
        if let Err(error) = process_source(&source, connection, config).await {
            error!("处理 YouTube 源「{}」失败：{:#}", source.name, error);
        }
    }

    Ok(())
}

async fn process_source(
    source: &youtube_channel::Model,
    connection: &DatabaseConnection,
    config: &Config,
) -> Result<()> {
    match source.source_type.as_str() {
        SOURCE_TYPE_CHANNEL => process_channel(source, connection, config).await,
        SOURCE_TYPE_PLAYLIST => process_playlist(source, connection, config).await,
        other => {
            warn!("未知的 YouTube 源类型「{}」，跳过：{}", other, source.name);
            Ok(())
        }
    }
}

async fn process_channel(
    source: &youtube_channel::Model,
    connection: &DatabaseConnection,
    config: &Config,
) -> Result<()> {
    info!("开始扫描 YouTube 频道「{}」..", source.name);

    let feed_entries = rss::fetch_channel_videos(&source.channel_id)
        .await
        .with_context(|| format!("failed to fetch youtube rss for {}", source.channel_id))?;

    let mut newest_published_at = source.latest_published_at;
    let mut new_videos = Vec::new();

    for entry in feed_entries.iter().rev() {
        let published_at = entry.published_at.naive_utc();
        if source
            .latest_published_at
            .is_some_and(|latest_published_at| published_at <= latest_published_at)
        {
            continue;
        }
        newest_published_at = Some(newest_published_at.map_or(published_at, |latest| latest.max(published_at)));
        new_videos.push(youtube_video::ActiveModel {
            youtube_channel_id: Set(source.id),
            video_id: Set(entry.video_id.clone()),
            title: Set(entry.title.clone()),
            url: Set(entry.url.clone()),
            description: Set(entry.description.clone()),
            uploader: Set(entry.uploader.clone()),
            thumbnail: Set(entry.thumbnail.clone()),
            published_at: Set(published_at),
            download_status: Set(u32::from(YoutubeVideoStatus::default())),
            valid: Set(true),
            should_download: Set(true),
            path: Set(None),
            ..Default::default()
        });
    }

    if !new_videos.is_empty() {
        let discovered_count = new_videos.len();
        youtube_video::Entity::insert_many(new_videos)
            .on_conflict(
                OnConflict::columns([youtube_video::Column::YoutubeChannelId, youtube_video::Column::VideoId])
                    .do_nothing()
                    .to_owned(),
            )
            .exec(connection)
            .await
            .context("failed to insert youtube channel videos")?;
        info!(
            "扫描 YouTube 频道「{}」完成，本轮发现 {} 条候选视频",
            source.name, discovered_count
        );
    }

    update_source_latest_published_at(source, newest_published_at, connection).await?;
    process_pending_videos(source, connection, config).await?;

    info!("处理 YouTube 频道「{}」完成", source.name);
    Ok(())
}

async fn process_playlist(
    source: &youtube_channel::Model,
    connection: &DatabaseConnection,
    config: &Config,
) -> Result<()> {
    info!("开始扫描 YouTube 播放列表「{}」..", source.name);

    let cookie_path = optional_cookie_path();
    let entries = helper::list_playlist_videos(&source.url, cookie_path.as_deref())
        .await
        .with_context(|| format!("failed to fetch youtube playlist videos for {}", source.url))?;

    let mut newest_published_at = source.latest_published_at;
    let mut new_videos = Vec::new();

    for entry in entries {
        let published_at = entry
            .published_at
            .and_then(|timestamp| chrono::DateTime::from_timestamp(timestamp, 0))
            .map(|datetime| datetime.naive_utc())
            .unwrap_or_else(|| chrono::Utc::now().naive_utc());
        newest_published_at = Some(newest_published_at.map_or(published_at, |latest| latest.max(published_at)));
        new_videos.push(youtube_video::ActiveModel {
            youtube_channel_id: Set(source.id),
            video_id: Set(entry.video_id),
            title: Set(entry.title),
            url: Set(entry.url),
            description: Set(entry.description),
            uploader: Set(entry.uploader),
            thumbnail: Set(entry.thumbnail),
            published_at: Set(published_at),
            download_status: Set(u32::from(YoutubeVideoStatus::default())),
            valid: Set(true),
            should_download: Set(true),
            path: Set(None),
            ..Default::default()
        });
    }

    if !new_videos.is_empty() {
        let discovered_count = new_videos.len();
        youtube_video::Entity::insert_many(new_videos)
            .on_conflict(
                OnConflict::columns([youtube_video::Column::YoutubeChannelId, youtube_video::Column::VideoId])
                    .do_nothing()
                    .to_owned(),
            )
            .exec(connection)
            .await
            .context("failed to insert youtube playlist videos")?;
        info!(
            "扫描 YouTube 播放列表「{}」完成，本轮发现 {} 条候选视频",
            source.name, discovered_count
        );
    }

    update_source_latest_published_at(source, newest_published_at, connection).await?;
    process_pending_videos(source, connection, config).await?;

    info!("处理 YouTube 播放列表「{}」完成", source.name);
    Ok(())
}

async fn update_source_latest_published_at(
    source: &youtube_channel::Model,
    newest_published_at: Option<DateTime>,
    connection: &DatabaseConnection,
) -> Result<()> {
    if newest_published_at != source.latest_published_at {
        let mut active_model: youtube_channel::ActiveModel = source.clone().into();
        active_model.latest_published_at = Set(newest_published_at);
        active_model
            .update(connection)
            .await
            .context("failed to update youtube source latest_published_at")?;
    }
    Ok(())
}

async fn process_pending_videos(
    source: &youtube_channel::Model,
    connection: &DatabaseConnection,
    config: &Config,
) -> Result<()> {
    let pending_videos = youtube_video::Entity::find()
        .filter(youtube_video::Column::YoutubeChannelId.eq(source.id))
        .filter(youtube_video::Column::Valid.eq(true))
        .filter(youtube_video::Column::ShouldDownload.eq(true))
        .filter(youtube_video::Column::DownloadStatus.lt(STATUS_COMPLETED))
        .order_by_asc(youtube_video::Column::PublishedAt)
        .all(connection)
        .await
        .context("failed to query pending youtube videos")?;

    for video in pending_videos {
        process_video(source, &video, connection, config).await?;
    }

    Ok(())
}

pub(crate) async fn process_video(
    source: &youtube_channel::Model,
    video: &youtube_video::Model,
    connection: &DatabaseConnection,
    config: &Config,
) -> Result<()> {
    let cookie_path = optional_cookie_path();
    let video_log_title = compact_log_text(&video.title, 48);

    info!("开始处理 YouTube 视频「{}」", video_log_title);
    match helper::download_video(
        &video.url,
        Path::new(&source.path),
        cookie_path.as_deref(),
        &config.youtube.skip_option,
        config.youtube.video_format,
    )
    .await
    {
        Ok(result) => {
            let mut status = YoutubeVideoStatus::from(video.download_status);
            let mut raw_status: [u32; 4] = status.into();
            raw_status[0] = STATUS_OK;
            raw_status[1] = STATUS_OK;
            raw_status[2] = STATUS_OK;
            raw_status[3] = STATUS_OK;
            status = YoutubeVideoStatus::from(raw_status);

            let mut active_model: youtube_video::ActiveModel = video.clone().into();
            active_model.download_status = Set(status.into());
            active_model.path = Set(Some(result.output_dir));
            active_model
                .update(connection)
                .await
                .context("failed to persist youtube video status")?;
            info!(
                "YouTube 视频「{}」处理完成，文件：{}",
                video_log_title,
                helper::compact_download_file_log(&result.video_file)
            );
        }
        Err(error) => {
            let mut raw_status: [u32; 4] = YoutubeVideoStatus::from(video.download_status).into();
            if raw_status[1] < STATUS_MAX_RETRY {
                raw_status[1] += 1;
            }
            if raw_status[1] >= STATUS_MAX_RETRY {
                for item in &mut raw_status {
                    if *item == 0 {
                        *item = STATUS_MAX_RETRY;
                    }
                }
            }

            let mut active_model: youtube_video::ActiveModel = video.clone().into();
            active_model.download_status = Set(YoutubeVideoStatus::from(raw_status).into());
            active_model
                .update(connection)
                .await
                .context("failed to persist failed youtube video status")?;
            error!("处理 YouTube 视频「{}」失败：{:#}", video_log_title, error);
        }
    }

    Ok(())
}

pub async fn has_enabled_sources(connection: &DatabaseConnection) -> Result<bool> {
    Ok(!enabled_sources(connection).await?.is_empty())
}

async fn enabled_sources(connection: &DatabaseConnection) -> Result<Vec<youtube_channel::Model>> {
    youtube_channel::Entity::find()
        .filter(youtube_channel::Column::Enabled.eq(true))
        .order_by_asc(youtube_channel::Column::SourceType)
        .order_by_asc(youtube_channel::Column::Name)
        .all(connection)
        .await
        .context("failed to query enabled youtube sources")
}
