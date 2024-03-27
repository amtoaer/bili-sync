use std::sync::Arc;

use futures_util::{pin_mut, StreamExt};
use sea_orm::entity::prelude::*;

use crate::bilibili::{BiliClient, FavoriteList, Video};
use crate::core::utils::{
    create_video_pages, create_videos, exists_bvids_favtime, filter_videos, handle_favorite_info,
};
use crate::Result;

pub async fn process_favorite(
    bili_client: Arc<BiliClient>,
    fid: i32,
    connection: Arc<DatabaseConnection>,
) -> Result<()> {
    let favorite_list = FavoriteList::new(bili_client.clone(), fid.to_string());
    let info = favorite_list.get_info().await?;
    let favorite_obj = handle_favorite_info(&info, connection.as_ref()).await?;
    println!(
        "Hi there! I'm going to scan this favorite: {:?}",
        favorite_obj
    );
    let video_stream = favorite_list.into_video_stream().chunks(10);
    pin_mut!(video_stream);
    while let Some(videos_info) = video_stream.next().await {
        let exist_bvids_pubtimes =
            exists_bvids_favtime(&videos_info, favorite_obj.id, connection.as_ref()).await?;
        let should_break = videos_info
            .iter()
            // 出现 bvid 和 fav_time 都相同的记录，说明已经到达了上次处理到的位置
            .any(|v| exist_bvids_pubtimes.contains(&(v.bvid.clone(), v.fav_time.naive_utc())));
        create_videos(&videos_info, &favorite_obj, connection.as_ref()).await?;
        let all_unprocessed_videos =
            filter_videos(&videos_info, &favorite_obj, true, true, connection.as_ref()).await?;
        if !all_unprocessed_videos.is_empty() {
            for video in all_unprocessed_videos {
                let bili_video = Video::new(bili_client.clone(), video.bvid.clone());
                let pages = bili_video.get_pages().await?;
                create_video_pages(&pages, &video, connection.as_ref()).await?;
            }
        }
        if should_break {
            break;
        }
    }
    Ok(())
}
