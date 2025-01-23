use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::pin::Pin;

use anyhow::{bail, Context, Result};
use bili_sync_entity::*;
use futures::stream::{FuturesOrdered, FuturesUnordered};
use futures::{Future, Stream, StreamExt};
use sea_orm::entity::prelude::*;
use sea_orm::ActiveValue::Set;
use sea_orm::TransactionTrait;
use tokio::fs;
use tokio::sync::{Mutex, Semaphore};

use crate::adapter::{video_list_from, Args, VideoListModel};
use crate::bilibili::{BestStream, BiliClient, BiliError, Dimension, PageInfo, Video, VideoInfo};
use crate::config::{PathSafeTemplate, ARGS, CONFIG, TEMPLATE};
use crate::downloader::Downloader;
use crate::error::{DownloadAbortError, ProcessPageError};
use crate::utils::format_arg::{page_format_args, video_format_args};
use crate::utils::model::{
    create_pages, create_videos, filter_unfilled_videos, filter_unhandled_video_pages, update_pages_model,
    update_videos_model,
};
use crate::utils::nfo::{ModelWrapper, NFOMode, NFOSerializer};
use crate::utils::status::{PageStatus, VideoStatus};

/// 完整地处理某个视频列表
pub async fn process_video_list(
    args: Args<'_>,
    bili_client: &BiliClient,
    path: &Path,
    connection: &DatabaseConnection,
) -> Result<()> {
    // 从参数中获取视频列表的 Model 与视频流
    let (video_list_model, video_streams) = video_list_from(args, path, bili_client, connection).await?;
    // 从视频流中获取新视频的简要信息，写入数据库
    refresh_video_list(video_list_model.as_ref(), video_streams, connection).await?;
    // 单独请求视频详情接口，获取视频的详情信息与所有的分页，写入数据库
    fetch_video_details(bili_client, video_list_model.as_ref(), connection).await?;
    if ARGS.scan_only {
        warn!("已开启仅扫描模式，跳过视频下载...");
    } else {
        // 从数据库中查找所有未下载的视频与分页，下载并处理
        download_unprocessed_videos(bili_client, video_list_model.as_ref(), connection).await?;
    }
    Ok(())
}

/// 请求接口，获取视频列表中所有新添加的视频信息，将其写入数据库
pub async fn refresh_video_list<'a>(
    video_list_model: &dyn VideoListModel,
    video_streams: Pin<Box<dyn Stream<Item = VideoInfo> + 'a>>,
    connection: &DatabaseConnection,
) -> Result<()> {
    video_list_model.log_refresh_video_start();
    let latest_row_at = video_list_model.get_latest_row_at().and_utc();
    let mut max_datetime = latest_row_at;
    let mut video_streams = video_streams
        .take_while(|v| {
            // 虽然 video_streams 是从新到旧的，但由于此处是分页请求，极端情况下可能发生访问完第一页时插入了两整页视频的情况
            // 此时获取到的第二页视频比第一页的还要新，因此为了确保正确，理应对每一页的第一个视频进行时间比较
            // 但在 streams 的抽象下，无法判断具体是在哪里分页的，所以暂且对每个视频都进行比较，应该不会有太大性能损失
            let release_datetime = v.release_datetime();
            if release_datetime > &max_datetime {
                max_datetime = *release_datetime;
            }
            futures::future::ready(release_datetime > &latest_row_at)
        })
        .chunks(10);
    let mut count = 0;
    while let Some(videos_info) = video_streams.next().await {
        count += videos_info.len();
        create_videos(videos_info, video_list_model, connection).await?;
    }
    if max_datetime != latest_row_at {
        video_list_model
            .update_latest_row_at(max_datetime.naive_utc())
            .save(connection)
            .await?;
    }
    video_list_model.log_refresh_video_end(count);
    Ok(())
}

/// 筛选出所有未获取到全部信息的视频，尝试补充其详细信息
pub async fn fetch_video_details(
    bili_client: &BiliClient,
    video_list_model: &dyn VideoListModel,
    connection: &DatabaseConnection,
) -> Result<()> {
    video_list_model.log_fetch_video_start();
    let videos_model = filter_unfilled_videos(video_list_model.filter_expr(), connection).await?;
    for video_model in videos_model {
        let video = Video::new(bili_client, video_model.bvid.clone());
        let info: Result<_> = async { Ok((video.get_tags().await?, video.get_view_info().await?)) }.await;
        match info {
            Err(e) => {
                error!(
                    "获取视频 {} - {} 的详细信息失败，错误为：{}",
                    &video_model.bvid, &video_model.name, e
                );
                if let Some(BiliError::RequestFailed(-404, _)) = e.downcast_ref::<BiliError>() {
                    let mut video_active_model: bili_sync_entity::video::ActiveModel = video_model.into();
                    video_active_model.valid = Set(false);
                    video_active_model.save(connection).await?;
                }
            }
            Ok((tags, mut view_info)) => {
                let VideoInfo::Detail { pages, .. } = &mut view_info else {
                    unreachable!()
                };
                let pages = std::mem::take(pages);
                let pages_len = pages.len();
                let txn = connection.begin().await?;
                // 将分页信息写入数据库
                create_pages(pages, &video_model, &txn).await?;
                let mut video_active_model = view_info.into_detail_model(video_model);
                video_list_model.set_relation_id(&mut video_active_model);
                video_active_model.single_page = Set(Some(pages_len == 1));
                video_active_model.tags = Set(Some(serde_json::to_value(tags)?));
                video_active_model.save(&txn).await?;
                txn.commit().await?;
            }
        };
    }
    video_list_model.log_fetch_video_end();
    Ok(())
}

/// 下载所有未处理成功的视频
pub async fn download_unprocessed_videos(
    bili_client: &BiliClient,
    video_list_model: &dyn VideoListModel,
    connection: &DatabaseConnection,
) -> Result<()> {
    video_list_model.log_download_video_start();
    let semaphore = Semaphore::new(CONFIG.concurrent_limit.video);
    let downloader = Downloader::new(bili_client.client.clone());
    let mut uppers_mutex: HashMap<i64, (Mutex<()>, Mutex<()>)> = HashMap::new();
    let unhandled_videos_pages = filter_unhandled_video_pages(video_list_model.filter_expr(), connection).await?;
    for (video_model, _) in &unhandled_videos_pages {
        uppers_mutex
            .entry(video_model.upper_id)
            .or_insert_with(|| (Mutex::new(()), Mutex::new(())));
    }
    let tasks = unhandled_videos_pages
        .into_iter()
        .map(|(video_model, pages_model)| {
            let upper_mutex = uppers_mutex.get(&video_model.upper_id).expect("upper mutex not found");
            download_video_pages(
                bili_client,
                video_list_model,
                video_model,
                pages_model,
                connection,
                &semaphore,
                &downloader,
                upper_mutex,
            )
        })
        .collect::<FuturesUnordered<_>>();
    let mut download_aborted = false;
    let mut stream = tasks
        // 触发风控时设置 download_aborted 标记并终止流
        .take_while(|res| {
            if res
                .as_ref()
                .is_err_and(|e| e.downcast_ref::<DownloadAbortError>().is_some())
            {
                download_aborted = true;
            }
            futures::future::ready(!download_aborted)
        })
        // 过滤掉没有触发风控的普通 Err，只保留正确返回的 Model
        .filter_map(|res| futures::future::ready(res.ok()))
        // 将成功返回的 Model 按十个一组合并
        .chunks(10);
    while let Some(models) = stream.next().await {
        update_videos_model(models, connection).await?;
    }
    if download_aborted {
        error!("下载视频时触发风控，终止收藏夹下所有下载任务，等待下一轮执行");
    }
    video_list_model.log_download_video_end();
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn download_video_pages(
    bili_client: &BiliClient,
    video_list_model: &dyn VideoListModel,
    video_model: video::Model,
    pages: Vec<page::Model>,
    connection: &DatabaseConnection,
    semaphore: &Semaphore,
    downloader: &Downloader,
    upper_mutex: &(Mutex<()>, Mutex<()>),
) -> Result<video::ActiveModel> {
    let _permit = semaphore.acquire().await.context("acquire semaphore failed")?;
    let mut status = VideoStatus::new(video_model.download_status);
    let seprate_status = status.should_run();
    let base_path = video_list_model
        .path()
        .join(TEMPLATE.path_safe_render("video", &video_format_args(&video_model))?);
    let upper_id = video_model.upper_id.to_string();
    let base_upper_path = &CONFIG
        .upper_path
        .join(upper_id.chars().next().context("upper_id is empty")?.to_string())
        .join(upper_id);
    let is_single_page = video_model.single_page.context("single_page is null")?;
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
            &base_path,
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
    if let Err(e) = results.into_iter().nth(4).context("page download result not found")? {
        if e.downcast_ref::<DownloadAbortError>().is_some() {
            return Err(e);
        }
    }
    let mut video_active_model: video::ActiveModel = video_model.into();
    video_active_model.download_status = Set(status.into());
    video_active_model.path = Set(base_path.to_string_lossy().to_string());
    Ok(video_active_model)
}

/// 分发并执行分页下载任务，当且仅当所有分页成功下载或达到最大重试次数时返回 Ok，否则根据失败原因返回对应的错误
pub async fn dispatch_download_page(
    should_run: bool,
    bili_client: &BiliClient,
    video_model: &video::Model,
    pages: Vec<page::Model>,
    connection: &DatabaseConnection,
    downloader: &Downloader,
    base_path: &Path,
) -> Result<()> {
    if !should_run {
        return Ok(());
    }
    let child_semaphore = Semaphore::new(CONFIG.concurrent_limit.page);
    let tasks = pages
        .into_iter()
        .map(|page_model| {
            download_page(
                bili_client,
                video_model,
                page_model,
                &child_semaphore,
                downloader,
                base_path,
            )
        })
        .collect::<FuturesUnordered<_>>();
    let (mut download_aborted, mut error_occurred) = (false, false);
    let mut stream = tasks
        .take_while(|res| {
            match res {
                Ok(model) => {
                    // 当前函数返回的是所有分页的下载状态，只要有任何一个分页返回新的下载状态标识位是 false，当前函数就应该认为是失败的
                    if model
                        .download_status
                        .try_as_ref()
                        .is_none_or(|status| !PageStatus::new(*status).get_completed())
                    {
                        error_occurred = true;
                    }
                }
                Err(e) => {
                    if e.downcast_ref::<DownloadAbortError>().is_some() {
                        download_aborted = true;
                    }
                }
            }
            // 仅在发生风控时终止流，其它情况继续执行
            futures::future::ready(!download_aborted)
        })
        .filter_map(|res| futures::future::ready(res.ok()))
        .chunks(10);
    while let Some(models) = stream.next().await {
        update_pages_model(models, connection).await?;
    }
    if download_aborted {
        error!(
            "下载视频 {} - {} 的分页时触发风控，将异常向上传递...",
            &video_model.bvid, &video_model.name
        );
        bail!(DownloadAbortError());
    }
    if error_occurred {
        error!(
            "下载视频 {} - {} 的分页时出现了错误，将在下一轮尝试重新处理",
            &video_model.bvid, &video_model.name
        );
        bail!(ProcessPageError());
    }
    Ok(())
}

/// 下载某个分页，未发生风控且正常运行时返回 Ok(Page::ActiveModel)，其中 status 字段存储了新的下载状态，发生风控时返回 DownloadAbortError
pub async fn download_page(
    bili_client: &BiliClient,
    video_model: &video::Model,
    page_model: page::Model,
    semaphore: &Semaphore,
    downloader: &Downloader,
    base_path: &Path,
) -> Result<page::ActiveModel> {
    let _permit = semaphore.acquire().await.context("acquire semaphore failed")?;
    let mut status = PageStatus::new(page_model.download_status);
    let seprate_status = status.should_run();
    let is_single_page = video_model.single_page.context("single_page is null")?;
    let base_name = TEMPLATE.path_safe_render("page", &page_format_args(video_model, &page_model))?;
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
    let dimension = match (page_model.width, page_model.height) {
        (Some(width), Some(height)) => Some(Dimension {
            width,
            height,
            rotate: 0,
        }),
        _ => None,
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
            &video_path,
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
    // 如果下载视频时触发风控，直接返回 DownloadAbortError
    if let Err(e) = results.into_iter().nth(1).context("video download result not found")? {
        if let Ok(BiliError::RiskControlOccurred) = e.downcast::<BiliError>() {
            bail!(DownloadAbortError());
        }
    }
    let mut page_active_model: page::ActiveModel = page_model.into();
    page_active_model.download_status = Set(status.into());
    page_active_model.path = Set(Some(video_path.to_string_lossy().to_string()));
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
    let single_page = video_model.single_page.context("single_page is null")?;
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
    page_path: &Path,
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
        BestStream::Mixed(mix_stream) => downloader.fetch(mix_stream.url(), page_path).await,
        BestStream::VideoAudio {
            video: video_stream,
            audio: None,
        } => downloader.fetch(video_stream.url(), page_path).await,
        BestStream::VideoAudio {
            video: video_stream,
            audio: Some(audio_stream),
        } => {
            let (tmp_video_path, tmp_audio_path) = (
                page_path.with_extension("tmp_video"),
                page_path.with_extension("tmp_audio"),
            );
            let res = async {
                downloader.fetch(video_stream.url(), &tmp_video_path).await?;
                downloader.fetch(audio_stream.url(), &tmp_audio_path).await?;
                downloader.merge(&tmp_video_path, &tmp_audio_path, page_path).await
            }
            .await;
            let _ = fs::remove_file(tmp_video_path).await;
            let _ = fs::remove_file(tmp_audio_path).await;
            res
        }
    }
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
        .await
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
    let single_page = video_model.single_page.context("single_page is null")?;
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
    use serde_json::json;

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
        let _ = template.path_safe_register("video", "test{{bvid}}test");
        let _ = template.path_safe_register("test_truncate", "哈哈，{{ truncate title 30 }}");
        let _ = template.path_safe_register("test_path_unix", "{{ truncate title 7 }}/test/a");
        let _ = template.path_safe_register("test_path_windows", r"{{ truncate title 7 }}\\test\\a");
        #[cfg(not(windows))]
        {
            assert_eq!(
                template
                    .path_safe_render("test_path_unix", &json!({"title": "关注/永雏塔菲喵"}))
                    .unwrap(),
                "关注_永雏塔菲/test/a"
            );
            assert_eq!(
                template
                    .path_safe_render("test_path_windows", &json!({"title": "关注/永雏塔菲喵"}))
                    .unwrap(),
                "关注_永雏塔菲_test_a"
            );
        }
        #[cfg(windows)]
        {
            assert_eq!(
                template
                    .path_safe_render("test_path_unix", &json!({"title": "关注/永雏塔菲喵"}))
                    .unwrap(),
                "关注_永雏塔菲_test_a"
            );
            assert_eq!(
                template
                    .path_safe_render("test_path_windows", &json!({"title": "关注/永雏塔菲喵"}))
                    .unwrap(),
                r"关注_永雏塔菲\\test\\a"
            );
        }
        assert_eq!(
            template
                .path_safe_render("video", &json!({"bvid": "BV1b5411h7g7"}))
                .unwrap(),
            "testBV1b5411h7g7test"
        );
        assert_eq!(
            template
                .path_safe_render(
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
