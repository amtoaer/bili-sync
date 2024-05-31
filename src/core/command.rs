use std::collections::HashMap;
use std::env::{args, var};
use std::path::{Path, PathBuf};
use std::pin::Pin;

use anyhow::{bail, Result};
use entity::{favorite, page, video};
use filenamify::filenamify;
use futures::stream::{FuturesOrdered, FuturesUnordered};
use futures::{pin_mut, Future, StreamExt};
use once_cell::sync::Lazy;
use sea_orm::entity::prelude::*;
use sea_orm::ActiveValue::Set;
use sea_orm::TransactionTrait;
use serde_json::json;
use tokio::fs;
use tokio::sync::{Mutex, Semaphore};

use crate::bilibili::{BestStream, BiliClient, BiliError, Dimension, FavoriteList, PageInfo, Video};
use crate::config::CONFIG;
use crate::core::status::{PageStatus, VideoStatus};
use crate::core::utils::{
    create_video_pages, create_videos, exist_labels, filter_unfilled_videos, handle_favorite_info, total_video_count,
    unhandled_videos_pages, update_pages_model, update_videos_model, ModelWrapper, NFOMode, NFOSerializer, TEMPLATE,
};
use crate::downloader::Downloader;
use crate::error::{DownloadAbortError, ProcessPageError};

pub static SCAN_ONLY: Lazy<bool> = Lazy::new(|| var("SCAN_ONLY").is_ok() || args().any(|arg| arg == "--scan-only"));

/// 处理某个收藏夹，首先刷新收藏夹信息，然后下载收藏夹中未下载成功的视频
pub async fn process_favorite_list(
    bili_client: &BiliClient,
    fid: &str,
    path: &Path,
    connection: &DatabaseConnection,
) -> Result<()> {
    let favorite_model = refresh_favorite_list(bili_client, fid, path, connection).await?;
    let favorite_model = fetch_video_details(bili_client, favorite_model, connection).await?;
    if *SCAN_ONLY {
        warn!("已开启仅扫描模式，跳过视频下载...");
        return Ok(());
    }
    download_unprocessed_videos(bili_client, favorite_model, connection).await
}

/// 获取收藏夹 Model，从收藏夹列表中获取所有新添加的视频，将其写入数据库
pub async fn refresh_favorite_list(
    bili_client: &BiliClient,
    fid: &str,
    path: &Path,
    connection: &DatabaseConnection,
) -> Result<favorite::Model> {
    let bili_favorite_list = FavoriteList::new(bili_client, fid.to_owned());
    let favorite_list_info = bili_favorite_list.get_info().await?;
    let favorite_model = handle_favorite_info(&favorite_list_info, path, connection).await?;
    info!("开始扫描收藏夹: {} - {}...", favorite_model.f_id, favorite_model.name);
    // 每十个视频一组，避免太多的数据库操作
    let video_stream = bili_favorite_list.into_video_stream().chunks(10);
    pin_mut!(video_stream);
    let mut got_count = 0;
    let total_count = total_video_count(&favorite_model, connection).await?;
    while let Some(videos_info) = video_stream.next().await {
        got_count += videos_info.len();
        let exist_labels = exist_labels(&videos_info, &favorite_model, connection).await?;
        // 如果发现有视频的收藏时间和 bvid 和数据库中重合，说明到达了上次处理到的地方，可以直接退出
        let should_break = videos_info
            .iter()
            .any(|v| exist_labels.contains(&(v.bvid.clone(), v.fav_time.naive_utc())));
        // 将视频信息写入数据库
        create_videos(&videos_info, &favorite_model, connection).await?;
        if should_break {
            info!("到达上一次处理的位置，提前中止");
            break;
        }
    }
    let total_count = total_video_count(&favorite_model, connection).await? - total_count;
    info!(
        "扫描收藏夹: {} - {} 完成, 获取了 {} 条视频, 其中有 {} 条新视频",
        favorite_model.f_id, favorite_model.name, got_count, total_count
    );
    Ok(favorite_model)
}

/// 筛选出所有没有获取到分页信息和 tag 的视频，请求分页信息和 tag 并写入数据库
pub async fn fetch_video_details(
    bili_client: &BiliClient,
    favorite_model: favorite::Model,
    connection: &DatabaseConnection,
) -> Result<favorite::Model> {
    info!(
        "开始获取收藏夹 {} - {} 的视频与分页信息...",
        favorite_model.f_id, favorite_model.name
    );
    let videos_model = filter_unfilled_videos(&favorite_model, connection).await?;
    for video_model in videos_model {
        let bili_video = Video::new(bili_client, video_model.bvid.clone());
        let tags = match bili_video.get_tags().await {
            Ok(tags) => tags,
            Err(e) => {
                error!(
                    "获取视频 {} - {} 的标签失败，错误为：{}",
                    &video_model.bvid, &video_model.name, e
                );
                if let Some(BiliError::RequestFailed(code, _)) = e.downcast_ref::<BiliError>() {
                    if *code == -404 {
                        let mut video_active_model: video::ActiveModel = video_model.into();
                        video_active_model.valid = Set(false);
                        video_active_model.save(connection).await?;
                    }
                }
                continue;
            }
        };
        let pages_info = match bili_video.get_pages().await {
            Ok(pages) => pages,
            Err(e) => {
                error!(
                    "获取视频 {} - {} 的分页信息失败，错误为：{}",
                    &video_model.bvid, &video_model.name, e
                );
                if let Some(BiliError::RequestFailed(code, _)) = e.downcast_ref::<BiliError>() {
                    if *code == -404 {
                        let mut video_active_model: video::ActiveModel = video_model.into();
                        video_active_model.valid = Set(false);
                        video_active_model.save(connection).await?;
                    }
                }
                continue;
            }
        };
        let txn = connection.begin().await?;
        // 将分页信息写入数据库
        create_video_pages(&pages_info, &video_model, &txn).await?;
        // 将页标记和 tag 写入数据库
        let mut video_active_model: video::ActiveModel = video_model.into();
        video_active_model.single_page = Set(Some(pages_info.len() == 1));
        video_active_model.tags = Set(Some(serde_json::to_value(tags).unwrap()));
        video_active_model.save(&txn).await?;
        txn.commit().await?;
    }
    info!(
        "获取收藏夹 {} - {} 的视频与分页信息完成",
        favorite_model.f_id, favorite_model.name
    );
    Ok(favorite_model)
}

/// 下载所有未处理成功的视频
pub async fn download_unprocessed_videos(
    bili_client: &BiliClient,
    favorite_model: favorite::Model,
    connection: &DatabaseConnection,
) -> Result<()> {
    info!(
        "开始下载收藏夹: {} - {} 中所有未处理过的视频...",
        favorite_model.f_id, favorite_model.name
    );
    let unhandled_videos_pages = unhandled_videos_pages(&favorite_model, connection).await?;
    // 对于视频，允许三个同时下载（视频内还有分页、不同分页还有多种下载任务）
    let semaphore = Semaphore::new(3);
    let downloader = Downloader::new(bili_client.client.clone());
    let mut uppers_mutex: HashMap<i64, (Mutex<()>, Mutex<()>)> = HashMap::new();
    for (video_model, _) in &unhandled_videos_pages {
        uppers_mutex.insert(video_model.upper_id, (Mutex::new(()), Mutex::new(())));
    }
    let mut tasks = unhandled_videos_pages
        .into_iter()
        .map(|(video_model, pages_model)| {
            let upper_mutex = uppers_mutex.get(&video_model.upper_id).unwrap();
            download_video_pages(
                bili_client,
                video_model,
                pages_model,
                connection,
                &semaphore,
                &downloader,
                &CONFIG.upper_path,
                upper_mutex,
            )
        })
        .collect::<FuturesUnordered<_>>();
    let mut models = Vec::with_capacity(10);
    while let Some(res) = tasks.next().await {
        match res {
            Ok(model) => {
                models.push(model);
            }
            Err(e) => {
                if e.downcast_ref::<DownloadAbortError>().is_some() {
                    error!("下载视频时触发风控，将终止收藏夹下所有下载任务，等待下一轮执行");
                    break;
                }
            }
        }
        // 满十个就写入数据库
        if models.len() == 10 {
            update_videos_model(std::mem::replace(&mut models, Vec::with_capacity(10)), connection).await?;
        }
    }
    if !models.is_empty() {
        update_videos_model(models, connection).await?;
    }
    info!(
        "下载收藏夹: {} - {} 中未处理过的视频完成",
        favorite_model.f_id, favorite_model.name
    );
    Ok(())
}

/// 暂时这样做，后面提取成上下文
#[allow(clippy::too_many_arguments)]
pub async fn download_video_pages(
    bili_client: &BiliClient,
    video_model: video::Model,
    pages: Vec<page::Model>,
    connection: &DatabaseConnection,
    semaphore: &Semaphore,
    downloader: &Downloader,
    upper_path: &Path,
    upper_mutex: &(Mutex<()>, Mutex<()>),
) -> Result<video::ActiveModel> {
    let permit = semaphore.acquire().await;
    if let Err(e) = permit {
        bail!(e);
    }
    let mut status = VideoStatus::new(video_model.download_status);
    let seprate_status = status.should_run();
    let base_path = Path::new(&video_model.path);
    let upper_id = video_model.upper_id.to_string();
    let base_upper_path = upper_path
        .join(upper_id.chars().next().unwrap().to_string())
        .join(upper_id);
    let is_single_page = video_model.single_page.unwrap();
    // 对于单页视频，page 的下载已经足够
    // 对于多页视频，page 下载仅包含了分集内容，需要额外补上视频的 poster 的 tvshow.nfo
    let tasks: Vec<Pin<Box<dyn Future<Output = Result<()>>>>> = vec![
        // 下载视频封面
        Box::pin(fetch_video_poster(
            seprate_status[0] && !is_single_page,
            &video_model,
            downloader,
            base_path.join("poster.jpg"),
            base_path.join("fanart.jpg"),
        )),
        // 生成视频信息的 nfo
        Box::pin(generate_video_nfo(
            seprate_status[1] && !is_single_page,
            &video_model,
            base_path.join("tvshow.nfo"),
        )),
        // 下载 Up 主头像
        Box::pin(fetch_upper_face(
            seprate_status[2],
            &video_model,
            downloader,
            &upper_mutex.0,
            base_upper_path.join("folder.jpg"),
        )),
        // 生成 Up 主信息的 nfo
        Box::pin(generate_upper_nfo(
            seprate_status[3],
            &video_model,
            &upper_mutex.1,
            base_upper_path.join("person.nfo"),
        )),
        // 分发并执行分 P 下载的任务
        Box::pin(dispatch_download_page(
            seprate_status[4],
            bili_client,
            &video_model,
            pages,
            connection,
            downloader,
        )),
    ];
    let tasks: FuturesOrdered<_> = tasks.into_iter().collect();
    let results: Vec<Result<()>> = tasks.collect().await;
    status.update_status(&results);
    results
        .iter()
        .take(4)
        .zip(["封面", "视频 nfo", "up 主头像", "up 主 nfo"])
        .for_each(|(res, task_name)| match res {
            Ok(_) => info!(
                "处理视频 {} - {} 的 {} 成功",
                &video_model.bvid, &video_model.name, task_name
            ),
            Err(e) => error!(
                "处理视频 {} - {} 的 {} 失败: {}",
                &video_model.bvid, &video_model.name, task_name, e
            ),
        });
    if let Err(e) = results.into_iter().nth(4).unwrap() {
        if e.downcast_ref::<DownloadAbortError>().is_some() {
            return Err(e);
        }
    }
    let mut video_active_model: video::ActiveModel = video_model.into();
    video_active_model.download_status = Set(status.into());
    Ok(video_active_model)
}

pub async fn dispatch_download_page(
    should_run: bool,
    bili_client: &BiliClient,
    video_model: &video::Model,
    pages: Vec<page::Model>,
    connection: &DatabaseConnection,
    downloader: &Downloader,
) -> Result<()> {
    if !should_run {
        return Ok(());
    }
    // 对于视频的分页，允许两个同时下载（绝大部分是单页视频）
    let child_semaphore = Semaphore::new(2);
    let mut tasks = pages
        .into_iter()
        .map(|page_model| download_page(bili_client, video_model, page_model, &child_semaphore, downloader))
        .collect::<FuturesUnordered<_>>();
    let mut models = Vec::with_capacity(10);
    let (mut should_error, mut is_break) = (false, false);
    while let Some(res) = tasks.next().await {
        match res {
            Ok(model) => {
                if let Set(status) = model.download_status {
                    let status = PageStatus::new(status);
                    if status.should_run().iter().any(|v| *v) {
                        // 有一个分页没变成终止状态（即下载成功或者重试次数达到限制），就应该向上层传递 Error
                        should_error = true;
                    }
                }
                models.push(model);
            }
            Err(e) => {
                if e.downcast_ref::<DownloadAbortError>().is_some() {
                    should_error = true;
                    is_break = true;
                    break;
                }
            }
        }
        if models.len() == 10 {
            update_pages_model(std::mem::replace(&mut models, Vec::with_capacity(10)), connection).await?;
        }
    }
    if !models.is_empty() {
        update_pages_model(models, connection).await?;
    }
    if should_error {
        if is_break {
            error!(
                "下载视频 {} - {} 的分页时触发风控，将异常向上传递...",
                &video_model.bvid, &video_model.name
            );
            bail!(DownloadAbortError());
        } else {
            error!(
                "下载视频 {} - {} 的分页时出现了错误，将在下一轮尝试重新处理",
                &video_model.bvid, &video_model.name
            );
            bail!(ProcessPageError());
        }
    }
    Ok(())
}

pub async fn download_page(
    bili_client: &BiliClient,
    video_model: &video::Model,
    page_model: page::Model,
    semaphore: &Semaphore,
    downloader: &Downloader,
) -> Result<page::ActiveModel> {
    let permit = semaphore.acquire().await;
    if let Err(e) = permit {
        return Err(e.into());
    }
    let mut status = PageStatus::new(page_model.download_status);
    let seprate_status = status.should_run();
    let is_single_page = video_model.single_page.unwrap();
    let base_path = Path::new(&video_model.path);
    let base_name = filenamify(TEMPLATE.render(
        "page",
        &json!({
            "bvid": &video_model.bvid,
            "title": &video_model.name,
            "upper_name": &video_model.upper_name,
            "upper_mid": &video_model.upper_id,
            "ptitle": &page_model.name,
            "pid": page_model.pid,
        }),
    )?);
    let (poster_path, video_path, nfo_path, danmaku_path, fanart_path) = if is_single_page {
        (
            base_path.join(format!("{}-poster.jpg", &base_name)),
            base_path.join(format!("{}.mp4", &base_name)),
            base_path.join(format!("{}.nfo", &base_name)),
            base_path.join(format!("{}.zh-CN.default.ass", &base_name)),
            Some(base_path.join(format!("{}-fanart.jpg", &base_name))),
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
            base_path
                .join("Season 1")
                .join(format!("{} - S01E{:0>2}.zh-CN.default.ass", &base_name, page_model.pid)),
            // 对于多页视频，会在上一步 fetch_video_poster 中获取剧集的 fanart，无需在此处下载单集的
            None,
        )
    };
    let dimension = if page_model.width.is_some() && page_model.height.is_some() {
        Some(Dimension {
            width: page_model.width.unwrap(),
            height: page_model.height.unwrap(),
            rotate: 0,
        })
    } else {
        None
    };
    let page_info = PageInfo {
        cid: page_model.cid,
        duration: page_model.duration,
        dimension,
        ..Default::default()
    };
    let tasks: Vec<Pin<Box<dyn Future<Output = Result<()>>>>> = vec![
        Box::pin(fetch_page_poster(
            seprate_status[0],
            video_model,
            &page_model,
            downloader,
            poster_path,
            fanart_path,
        )),
        Box::pin(fetch_page_video(
            seprate_status[1],
            bili_client,
            video_model,
            downloader,
            &page_info,
            video_path.clone(),
        )),
        Box::pin(generate_page_nfo(seprate_status[2], video_model, &page_model, nfo_path)),
        Box::pin(fetch_page_danmaku(
            seprate_status[3],
            bili_client,
            video_model,
            &page_info,
            danmaku_path,
        )),
    ];
    let tasks: FuturesOrdered<_> = tasks.into_iter().collect();
    let results: Vec<Result<()>> = tasks.collect().await;
    status.update_status(&results);
    results
        .iter()
        .zip(["封面", "视频", "视频 nfo", "弹幕"])
        .for_each(|(res, task_name)| match res {
            Ok(_) => info!(
                "处理视频 {} - {} 第 {} 页的 {} 成功",
                &video_model.bvid, &video_model.name, page_model.pid, task_name
            ),
            Err(e) => error!(
                "处理视频 {} - {} 第 {} 页的 {} 失败: {}",
                &video_model.bvid, &video_model.name, page_model.pid, task_name, e
            ),
        });
    // 查看下载视频的状态，该状态会影响上层是否 break
    if let Err(e) = results.into_iter().nth(1).unwrap() {
        if let Ok(BiliError::RiskControlOccurred) = e.downcast::<BiliError>() {
            bail!(DownloadAbortError());
        }
    }
    let mut page_active_model: page::ActiveModel = page_model.into();
    page_active_model.download_status = Set(status.into());
    page_active_model.path = Set(Some(video_path.to_str().unwrap().to_string()));
    Ok(page_active_model)
}

pub async fn fetch_page_poster(
    should_run: bool,
    video_model: &video::Model,
    page_model: &page::Model,
    downloader: &Downloader,
    poster_path: PathBuf,
    fanart_path: Option<PathBuf>,
) -> Result<()> {
    if !should_run {
        return Ok(());
    }
    let single_page = video_model.single_page.unwrap();
    let url = if single_page {
        // 单页视频直接用视频的封面
        video_model.cover.as_str()
    } else {
        // 多页视频，如果单页没有封面，就使用视频的封面
        match &page_model.image {
            Some(url) => url.as_str(),
            None => video_model.cover.as_str(),
        }
    };
    downloader.fetch(url, &poster_path).await?;
    if let Some(fanart_path) = fanart_path {
        fs::copy(&poster_path, &fanart_path).await?;
    }
    Ok(())
}

pub async fn fetch_page_video(
    should_run: bool,
    bili_client: &BiliClient,
    video_model: &video::Model,
    downloader: &Downloader,
    page_info: &PageInfo,
    page_path: PathBuf,
) -> Result<()> {
    if !should_run {
        return Ok(());
    }
    let bili_video = Video::new(bili_client, video_model.bvid.clone());
    let streams = bili_video
        .get_page_analyzer(page_info)
        .await?
        .best_stream(&CONFIG.filter_option)?;
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

pub async fn fetch_page_danmaku(
    should_run: bool,
    bili_client: &BiliClient,
    video_model: &video::Model,
    page_info: &PageInfo,
    danmaku_path: PathBuf,
) -> Result<()> {
    if !should_run {
        return Ok(());
    }
    let bili_video = Video::new(bili_client, video_model.bvid.clone());
    bili_video
        .get_danmaku_writer(page_info)
        .await?
        .write(danmaku_path)
        .await?;
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
    fanart_path: PathBuf,
) -> Result<()> {
    if !should_run {
        return Ok(());
    }
    downloader.fetch(&video_model.cover, &poster_path).await?;
    fs::copy(&poster_path, &fanart_path).await?;
    Ok(())
}

pub async fn fetch_upper_face(
    should_run: bool,
    video_model: &video::Model,
    downloader: &Downloader,
    upper_face_mutex: &Mutex<()>,
    upper_face_path: PathBuf,
) -> Result<()> {
    if !should_run {
        return Ok(());
    }
    // 这个锁只是为了避免多个视频同时下载同一个 up 主的头像，不携带实际内容
    let _ = upper_face_mutex.lock().await;
    if !upper_face_path.exists() {
        return downloader.fetch(&video_model.upper_face, &upper_face_path).await;
    }
    Ok(())
}

pub async fn generate_upper_nfo(
    should_run: bool,
    video_model: &video::Model,
    upper_nfo_mutex: &Mutex<()>,
    nfo_path: PathBuf,
) -> Result<()> {
    if !should_run {
        return Ok(());
    }
    let _ = upper_nfo_mutex.lock().await;
    if !nfo_path.exists() {
        let nfo_serializer = NFOSerializer(ModelWrapper::Video(video_model), NFOMode::UPPER);
        return generate_nfo(nfo_serializer, nfo_path).await;
    }
    Ok(())
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
    fs::write(
        nfo_path,
        serializer.generate_nfo(&CONFIG.nfo_time_type).await?.as_bytes(),
    )
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use handlebars::handlebars_helper;

    use super::*;

    #[test]
    fn test_template_usage() {
        let mut template = handlebars::Handlebars::new();
        handlebars_helper!(truncate: |s: String, len: usize| {
            if s.chars().count() > len {
                s.chars().take(len).collect::<String>()
            } else {
                s.to_string()
            }
        });
        template.register_helper("truncate", Box::new(truncate));
        let _ = template.register_template_string("video", "test{{bvid}}test");
        let _ = template.register_template_string("test_truncate", "哈哈，{{ truncate title 30 }}");
        assert_eq!(
            template.render("video", &json!({"bvid": "BV1b5411h7g7"})).unwrap(),
            "testBV1b5411h7g7test"
        );
        assert_eq!(
            template
                .render(
                    "test_truncate",
                    &json!({"title": "你说得对，但是 Rust 是由 Mozilla 自主研发的一款全新的编译期格斗游戏。\
                    编译将发生在一个被称作「Cargo」的构建系统中。在这里，被引用的指针将被授予「生命周期」之力，导引对象安全。\
                    你将扮演一位名为「Rustacean」的神秘角色, 在与「Rustc」的搏斗中邂逅各种骨骼惊奇的傲娇报错。\
                    征服她们、通过编译同时，逐步发掘「C++」程序崩溃的真相。"})
                )
                .unwrap(),
            "哈哈，你说得对，但是 Rust 是由 Mozilla 自主研发的一"
        );
    }
}
