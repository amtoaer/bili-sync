use std::pin::Pin;
use std::sync::Arc;

use entity::{favorite, page, video};
use futures::stream::FuturesUnordered;
use futures::Future;
use futures_util::{pin_mut, StreamExt};
use log::{error, info};
use sea_orm::entity::prelude::*;
use sea_orm::ActiveValue::Set;
use sea_orm::TryIntoModel;
use tokio::sync::Semaphore;

use super::status::Status;
use super::utils::unhandled_videos_pages;
use crate::bilibili::{BiliClient, FavoriteList, Video};
use crate::core::utils::{
    create_video_pages, create_videos, exist_labels, filter_videos, handle_favorite_info,
};
use crate::downloader::Downloader;
use crate::Result;

pub async fn process_favorite(
    bili_client: Arc<BiliClient>,
    fid: &str,
    connection: Arc<DatabaseConnection>,
) -> Result<()> {
    let favorite_model = refresh_favorite(bili_client.clone(), fid, connection.clone()).await?;
    download_favorite(bili_client.clone(), favorite_model, connection.clone()).await?;
    Ok(())
}

pub async fn refresh_favorite(
    bili_client: Arc<BiliClient>,
    fid: &str,
    connection: Arc<DatabaseConnection>,
) -> Result<favorite::Model> {
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
    Ok(favorite_model)
}

#[allow(unused_variables)]
pub async fn download_favorite(
    bili_client: Arc<BiliClient>,
    favorite_model: favorite::Model,
    connection: Arc<DatabaseConnection>,
) -> Result<()> {
    let unhandled_videos_pages =
        unhandled_videos_pages(&favorite_model, connection.as_ref()).await?;
    let semaphore = Arc::new(Semaphore::new(3));
    let downloader = Arc::new(Downloader::default());
    let mut tasks = FuturesUnordered::new();
    for (video_model, pages) in unhandled_videos_pages {
        tasks.push(Box::pin(download_video_pages(
            bili_client.clone(),
            video_model,
            pages,
            connection.clone(),
            semaphore.clone(),
        )));
    }
    while let Some(res) = tasks.next().await {
        if let Err(e) = res {
            error!("Error: {e}");
        }
    }
    Ok(())
}

pub async fn download_video_pages(
    bili_client: Arc<BiliClient>,
    video_model: video::Model,
    pages: Vec<page::Model>,
    connection: Arc<DatabaseConnection>,
    semaphore: Arc<Semaphore>,
) -> Result<()> {
    let permit = semaphore.acquire().await;
    if let Err(e) = permit {
        return Err(e.into());
    }
    let child_semaphore = Arc::new(Semaphore::new(5));
    let mut tasks = FuturesUnordered::new();
    for page_model in pages {
        tasks.push(Box::pin(download_page(
            bili_client.clone(),
            &video_model,
            page_model,
            connection.clone(),
            child_semaphore.clone(),
        )));
    }
    while let Some(res) = tasks.next().await {
        if let Err(e) = res {
            error!("Error: {e}");
        }
    }
    Ok(())
}

pub async fn download_page(
    bili_client: Arc<BiliClient>,
    video_model: &video::Model,
    page_model: page::Model,
    connection: Arc<DatabaseConnection>,
    semaphore: Arc<Semaphore>,
) -> Result<page::Model> {
    let permit = semaphore.acquire().await;
    if let Err(e) = permit {
        return Err(e.into());
    }
    let mut status = Status::new(page_model.download_status);
    let seprate_status = status.should_run();
    let tasks: Vec<Pin<Box<dyn Future<Output = Result<()>>>>> = vec![
        // 暂时不支持下载字幕
        Box::pin(download_poster(
            seprate_status[0],
            &bili_client,
            video_model,
            &page_model,
        )),
        Box::pin(download_upper(
            seprate_status[1],
            &bili_client,
            video_model,
            &page_model,
        )),
        Box::pin(download_video(
            seprate_status[2],
            &bili_client,
            video_model,
            &page_model,
        )),
        Box::pin(generate_nfo(
            seprate_status[3],
            &bili_client,
            video_model,
            &page_model,
        )),
    ];
    let results = futures::future::join_all(tasks).await;
    status.update_status(&results);
    let mut page_active_model: page::ActiveModel = page_model.into();
    page_active_model.download_status = Set(status.into());
    page_active_model = page_active_model.save(connection.as_ref()).await?;
    Ok(page_active_model.try_into_model().unwrap())
}

#[allow(unused_variables)]
pub async fn download_poster(
    should_run: bool,
    bili_client: &Arc<BiliClient>,
    video_model: &video::Model,
    page_model: &page::Model,
) -> Result<()> {
    Ok(())
}
#[allow(unused_variables)]
pub async fn download_upper(
    should_run: bool,
    bili_client: &Arc<BiliClient>,
    video_model: &video::Model,
    page_model: &page::Model,
) -> Result<()> {
    Ok(())
}
#[allow(unused_variables)]
pub async fn download_video(
    should_run: bool,
    bili_client: &Arc<BiliClient>,
    video_model: &video::Model,
    page_model: &page::Model,
) -> Result<()> {
    Ok(())
}
#[allow(unused_variables)]
pub async fn generate_nfo(
    should_run: bool,
    bili_client: &Arc<BiliClient>,
    video_model: &video::Model,
    page_model: &page::Model,
) -> Result<()> {
    Ok(())
}
