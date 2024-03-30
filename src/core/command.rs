use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;

use entity::{favorite, page, video};
use futures::stream::FuturesUnordered;
use futures::Future;
use futures_util::{pin_mut, StreamExt};
use log::{error, info};
use once_cell::sync::Lazy;
use sea_orm::entity::prelude::*;
use sea_orm::ActiveValue::Set;
use sea_orm::TryIntoModel;
use serde_json::json;
use tokio::fs;
use tokio::sync::Semaphore;

use super::status::PageStatus;
use super::utils::{unhandled_videos_pages, ModelWrapper, NFOMode, NFOSerializer};
use crate::bilibili::{BestStream, BiliClient, FavoriteList, FilterOption, PageInfo, Video};
use crate::core::utils::{create_video_pages, create_videos, exist_labels, filter_videos, handle_favorite_info};
use crate::downloader::Downloader;
use crate::Result;

static TEMPLATE: Lazy<handlebars::Handlebars> = Lazy::new(|| {
    let mut handlebars = handlebars::Handlebars::new();
    // TODO: 此处开放自定义
    handlebars.register_template_string("video", "{{bvid}}").unwrap();
    handlebars.register_template_string("page", "{{bvid}}").unwrap();
    handlebars
});

/// 处理某个收藏夹，首先刷新收藏夹信息，然后下载收藏夹中未下载成功的视频
pub async fn process_favorite(bili_client: &BiliClient, fid: &str, connection: &DatabaseConnection) -> Result<()> {
    let favorite_model = refresh_favorite(bili_client, fid, connection).await?;
    download_favorite(bili_client, favorite_model, connection).await
}

pub async fn refresh_favorite(
    bili_client: &BiliClient,
    fid: &str,
    connection: &DatabaseConnection,
) -> Result<favorite::Model> {
    let bili_favorite_list = FavoriteList::new(bili_client, fid.to_owned());
    let favorite_list_info = bili_favorite_list.get_info().await?;
    let favorite_model = handle_favorite_info(&favorite_list_info, connection).await?;
    info!("Scan the favorite: {fid}");
    // 每十个视频一组，避免太多的数据库操作
    let video_stream = bili_favorite_list.into_video_stream().chunks(10);
    pin_mut!(video_stream);
    while let Some(videos_info) = video_stream.next().await {
        info!("handle videos: {}", videos_info.len());
        let exist_labels = exist_labels(&videos_info, &favorite_model, connection).await?;
        // 如果发现有视频的收藏时间和 bvid 和数据库中重合，说明到达了上次处理到的地方，可以直接退出
        let should_break = videos_info
            .iter()
            .any(|v| exist_labels.contains(&(v.bvid.clone(), v.fav_time.naive_utc())));
        // 将视频信息写入数据库
        create_videos(&videos_info, &favorite_model, connection).await?;
        // 找到这些视频中没有下载过，也没有 page 的视频
        let unrefreshed_video_models = filter_videos(&videos_info, &favorite_model, true, true, connection).await?;
        if !unrefreshed_video_models.is_empty() {
            for video_model in unrefreshed_video_models {
                let bili_video = Video::new(bili_client, video_model.bvid.clone());
                let tags = bili_video.get_tags().await?;
                let pages_info = bili_video.get_pages().await?;
                // 记录视频的分集信息
                create_video_pages(&pages_info, &video_model, connection).await?;
                let mut video_active_model: video::ActiveModel = video_model.into();
                video_active_model.single_page = Set(Some(pages_info.len() == 1));
                video_active_model.tags = Set(Some(serde_json::to_value(tags).unwrap()));
                // 记录视频是否是单页，以及标签信息
                video_active_model.save(connection).await?;
            }
        }
        if should_break {
            break;
        }
    }
    info!("handle videos done");
    Ok(favorite_model)
}

pub async fn download_favorite(
    bili_client: &BiliClient,
    favorite_model: favorite::Model,
    connection: &DatabaseConnection,
) -> Result<()> {
    info!("start to download!");
    let unhandled_videos_pages = unhandled_videos_pages(&favorite_model, connection).await?;
    // 对于视频，允许五个同时下载（视频内还有分页、不同分页还有多种下载任务）
    let semaphore = Arc::new(Semaphore::new(5));
    let downloader = Downloader::default();
    let mut tasks = FuturesUnordered::new();
    for (video_model, pages) in unhandled_videos_pages {
        tasks.push(download_video_pages(
            bili_client,
            video_model,
            pages,
            connection,
            semaphore.clone(),
            &downloader,
        ));
    }
    // 创建好下载任务，等待下载任务运行即可，任务结束会返回 Result<VideoModel>
    while let Some(res) = tasks.next().await {
        if let Err(e) = res {
            error!("Error: {e}");
        }
    }
    info!("download done.");
    Ok(())
}

pub async fn download_video_pages(
    bili_client: &BiliClient,
    video_model: video::Model,
    pages: Vec<page::Model>,
    connection: &DatabaseConnection,
    semaphore: Arc<Semaphore>,
    downloader: &Downloader,
) -> Result<()> {
    let permit = semaphore.acquire().await;
    if let Err(e) = permit {
        return Err(e.into());
    }
    // 对于视频的分页，允许同时下载三个同时下载（绝大部分是单页视频）
    let child_semaphore = Arc::new(Semaphore::new(5));
    let mut tasks = FuturesUnordered::new();
    for page_model in pages {
        tasks.push(download_page(
            bili_client,
            &video_model,
            page_model,
            connection,
            child_semaphore.clone(),
            downloader,
        ));
    }
    // 同样创建好下载任务等待运行，任务结束会返回 Result<PageModel>
    while let Some(res) = tasks.next().await {
        if let Err(e) = res {
            error!("Error: {e}");
        }
    }
    let is_single_page = video_model.single_page.unwrap();
    // 对于单页视频，page 的下载已经足够
    // 对于多页视频，page 下载仅包含了分集内容，需要额外补上视频的 poster 的 tvshow.nfo
    if !is_single_page {
        let base_path = Path::new(&video_model.path);
        generate_video_nfo(true, &video_model, base_path.join("tvshow.nfo")).await?;
        fetch_video_poster(true, &video_model, downloader, base_path.join("poster.jpg")).await?;
    }
    Ok(())
}

pub async fn download_page(
    bili_client: &BiliClient,
    video_model: &video::Model,
    page_model: page::Model,
    connection: &DatabaseConnection,
    semaphore: Arc<Semaphore>,
    downloader: &Downloader,
) -> Result<page::Model> {
    let permit = semaphore.acquire().await;
    if let Err(e) = permit {
        return Err(e.into());
    }
    let mut status = PageStatus::new(page_model.download_status);
    let seprate_status = status.should_run();
    let is_single_page = video_model.single_page.unwrap();
    let base_path = Path::new(&video_model.path);
    let base_name = TEMPLATE.render(
        "page",
        &json!({
            "bvid": &video_model.bvid,
            "name": &video_model.name,
            "pid": page_model.pid,
        }),
    )?;
    let (poster_path, video_path, nfo_path) = if is_single_page {
        (
            base_path.join(format!("{}-poster.jpg", &base_name)),
            base_path.join(format!("{}.mp4", &base_name)),
            base_path.join(format!("{}.nfo", &base_name)),
        )
    } else {
        (
            base_path
                .join("Season 1")
                .join(format!("{} - S01E{:0>2}-thumb.jpg", &base_name, page_model.pid)),
            base_path
                .join("Season 1")
                .join(format!("{} - S01E{:0>2}.mp4", &base_name, page_model.pid)),
            base_path
                .join("Season 1")
                .join(format!("{} - S01E{:0>2}.nfo", &base_name, page_model.pid)),
        )
    };
    let tasks: Vec<Pin<Box<dyn Future<Output = Result<()>>>>> = vec![
        // 暂时不支持下载字幕
        Box::pin(fetch_page_poster(
            seprate_status[0],
            video_model,
            &page_model,
            downloader,
            poster_path,
        )),
        Box::pin(fetch_page_video(
            seprate_status[1],
            bili_client,
            video_model,
            &page_model,
            downloader,
            video_path,
        )),
        Box::pin(generate_page_nfo(seprate_status[2], video_model, &page_model, nfo_path)),
    ];
    let results = futures::future::join_all(tasks).await;
    status.update_status(&results);
    let mut page_active_model: page::ActiveModel = page_model.into();
    page_active_model.download_status = Set(status.into());
    page_active_model = page_active_model.save(connection).await?;
    Ok(page_active_model.try_into_model().unwrap())
}

pub async fn fetch_page_poster(
    should_run: bool,
    video_model: &video::Model,
    page_model: &page::Model,
    downloader: &Downloader,
    poster_path: PathBuf,
) -> Result<()> {
    if !should_run {
        return Ok(());
    }
    // 如果单页没有封面，就使用视频的封面
    let url = match &page_model.image {
        Some(url) => url.as_str(),
        None => video_model.cover.as_str(),
    };
    downloader.fetch(url, &poster_path).await
}

pub async fn fetch_page_video(
    should_run: bool,
    bili_client: &BiliClient,
    video_model: &video::Model,
    page_model: &page::Model,
    downloader: &Downloader,
    page_path: PathBuf,
) -> Result<()> {
    if !should_run {
        return Ok(());
    }
    let bili_video = Video::new(bili_client, video_model.bvid.clone());
    let streams = bili_video
        .get_page_analyzer(&PageInfo {
            cid: page_model.cid,
            ..Default::default()
        })
        .await?
        .best_stream(&FilterOption::default())?;
    match streams {
        BestStream::Mixed(mix_stream) => {
            downloader.fetch(mix_stream.url(), &page_path).await?;
        }
        BestStream::VideoAudio {
            video: video_stream,
            audio: None,
        } => {
            downloader.fetch(video_stream.url(), &page_path).await?;
        }
        BestStream::VideoAudio {
            video: video_stream,
            audio: Some(audio_stream),
        } => {
            let (tmp_video_path, tmp_audio_path) = (
                page_path.with_extension("tmp_video"),
                page_path.with_extension("tmp_audio"),
            );
            downloader.fetch(video_stream.url(), &tmp_video_path).await?;
            downloader.fetch(audio_stream.url(), &tmp_audio_path).await?;
            downloader.merge(&tmp_video_path, &tmp_audio_path, &page_path).await?;
        }
    }
    Ok(())
}

pub async fn generate_page_nfo(
    should_run: bool,
    video_model: &video::Model,
    page_model: &page::Model,
    nfo_path: PathBuf,
) -> Result<()> {
    if !should_run {
        return Ok(());
    }
    let single_page = video_model.single_page.unwrap();
    let nfo_serializer = if single_page {
        NFOSerializer(ModelWrapper::Video(video_model), NFOMode::MOVIE)
    } else {
        NFOSerializer(ModelWrapper::Page(page_model), NFOMode::EPOSODE)
    };
    generate_nfo(nfo_serializer, nfo_path).await
}

pub async fn fetch_video_poster(
    should_run: bool,
    video_model: &video::Model,
    downloader: &Downloader,
    poster_path: PathBuf,
) -> Result<()> {
    if !should_run {
        return Ok(());
    }
    downloader.fetch(&video_model.cover, &poster_path).await
}

pub async fn generate_video_nfo(should_run: bool, video_model: &video::Model, nfo_path: PathBuf) -> Result<()> {
    if !should_run {
        return Ok(());
    }
    let nfo_serializer = NFOSerializer(ModelWrapper::Video(video_model), NFOMode::TVSHOW);
    generate_nfo(nfo_serializer, nfo_path).await
}

/// 创建 nfo_path 的父目录，然后写入 nfo 文件
async fn generate_nfo(serializer: NFOSerializer<'_>, nfo_path: PathBuf) -> Result<()> {
    if let Some(parent) = nfo_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(nfo_path, serializer.generate_nfo().await?.as_bytes()).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_usage() {
        assert_eq!(
            TEMPLATE.render("video", &json!({"bvid": "BV1b5411h7g7"})).unwrap(),
            "BV1b5411h7g7"
        );
    }
}
