use std::sync::Arc;

use entity::video;
use futures_util::{pin_mut, StreamExt};
use log::info;
use sea_orm::entity::prelude::*;
use sea_orm::ActiveValue::Set;

use crate::bilibili::{BiliClient, FavoriteList, Video};
use crate::core::utils::{
    create_video_pages, create_videos, exist_labels, filter_videos, handle_favorite_info,
};
use crate::Result;

pub async fn process_favorite(
    bili_client: Arc<BiliClient>,
    fid: &str,
    connection: Arc<DatabaseConnection>,
) -> Result<()> {
    refresh_favorite(bili_client.clone(), fid, connection.clone()).await?;
    download_favorite(bili_client.clone(), fid, connection.clone()).await?;
    Ok(())
}

pub async fn refresh_favorite(
    bili_client: Arc<BiliClient>,
    fid: &str,
    connection: Arc<DatabaseConnection>,
) -> Result<()> {
    let bili_favorite_list = FavoriteList::new(bili_client.clone(), fid.to_owned());
    let favorite_list_info = bili_favorite_list.get_info().await?;
    let favorite_model = handle_favorite_info(&favorite_list_info, connection.as_ref()).await?;
    info!("Scan the favorite: {fid}");
    let video_stream = bili_favorite_list.into_video_stream().chunks(10);
    pin_mut!(video_stream);
    while let Some(videos_info) = video_stream.next().await {
        info!("handle videos: {}", videos_info.len());
        let exist_labels = exist_labels(&videos_info, &favorite_model, connection.as_ref()).await?;
        let should_break = videos_info
            .iter()
            .any(|v| exist_labels.contains(&(v.bvid.clone(), v.fav_time.naive_utc())));
        create_videos(&videos_info, &favorite_model, connection.as_ref()).await?;
        let unrefreshed_video_models = filter_videos(
            &videos_info,
            &favorite_model,
            true,
            true,
            connection.as_ref(),
        )
        .await?;
        if !unrefreshed_video_models.is_empty() {
            for video_model in unrefreshed_video_models {
                let bili_video = Video::new(bili_client.clone(), video_model.bvid.clone());
                let tags = bili_video.get_tags().await?;
                let pages_info = bili_video.get_pages().await?;
                create_video_pages(&pages_info, &video_model, connection.as_ref()).await?;
                let mut video_active_model: video::ActiveModel = video_model.into();
                video_active_model.single_page = Set(Some(pages_info.len() == 1));
                video_active_model.tags = Set(Some(serde_json::to_value(tags).unwrap()));
                video_active_model.save(connection.as_ref()).await?;
            }
        }
        if should_break {
            break;
        }
    }
    Ok(())
}

#[allow(unused_variables)]
pub async fn download_favorite(
    bili_client: Arc<BiliClient>,
    fid: &str,
    connection: Arc<DatabaseConnection>,
) -> Result<()> {
    todo!();
}
