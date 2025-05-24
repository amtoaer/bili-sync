use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::pin::Pin;

use anyhow::{Context, Result, anyhow, bail};
use bili_sync_entity::*;
use futures::stream::{FuturesOrdered, FuturesUnordered};
use futures::{Future, Stream, StreamExt, TryStreamExt};
use sea_orm::ActiveValue::Set;
use sea_orm::TransactionTrait;
use sea_orm::entity::prelude::*;
use tokio::fs;
use tokio::sync::Semaphore;

use crate::adapter::{Args, VideoSource, VideoSourceEnum, video_source_from};
use crate::bilibili::{BestStream, BiliClient, BiliError, Dimension, PageInfo, Video, VideoInfo};
use crate::config::{ARGS, CONFIG, PathSafeTemplate, TEMPLATE};
use crate::downloader::Downloader;
use crate::error::{DownloadAbortError, ExecutionStatus, ProcessPageError};
use crate::utils::format_arg::{page_format_args, video_format_args};
use crate::utils::model::{
    create_pages, create_videos, filter_unfilled_videos, filter_unhandled_video_pages, update_pages_model,
    update_videos_model,
};
use crate::utils::nfo::{ModelWrapper, NFOMode, NFOSerializer};
use crate::utils::status::{PageStatus, STATUS_OK, VideoStatus};

/// 完整地处理某个视频来源
pub async fn process_video_source(
    args: Args<'_>,
    bili_client: &BiliClient,
    path: &Path,
    connection: &DatabaseConnection,
) -> Result<()> {
    // 记录当前处理的参数和路径
    if let Args::Bangumi { season_id: _, media_id: _, ep_id: _ } = args {
        // 获取番剧标题，从路径中提取
        let title = path.file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| "未知番剧".to_string());
        info!("处理番剧下载: {}", title);
    }
    
    // 从参数中获取视频列表的 Model 与视频流
    let (video_source, video_streams) = video_source_from(args, path, bili_client, connection).await?;
    // 从视频流中获取新视频的简要信息，写入数据库
    refresh_video_source(&video_source, video_streams, connection).await?;
    // 单独请求视频详情接口，获取视频的详情信息与所有的分页，写入数据库
    fetch_video_details(bili_client, &video_source, connection).await?;
    if ARGS.scan_only {
        warn!("已开启仅扫描模式，跳过视频下载..");
    } else {
        // 从数据库中查找所有未下载的视频与分页，下载并处理
        download_unprocessed_videos(bili_client, &video_source, connection).await?;
    }
    Ok(())
}

/// 请求接口，获取视频列表中所有新添加的视频信息，将其写入数据库
pub async fn refresh_video_source<'a>(
    video_source: &VideoSourceEnum,
    video_streams: Pin<Box<dyn Stream<Item = Result<VideoInfo>> + 'a + Send>>,
    connection: &DatabaseConnection,
) -> Result<()> {
    video_source.log_refresh_video_start();
    let latest_row_at = video_source.get_latest_row_at().and_utc();
    let mut max_datetime = latest_row_at;
    let mut error = Ok(());
    let mut video_streams = video_streams
        .take_while(|res| {
            match res {
                Err(e) => {
                    error = Err(anyhow!(e.to_string()));
                    futures::future::ready(false)
                }
                Ok(v) => {
                    // 虽然 video_streams 是从新到旧的，但由于此处是分页请求，极端情况下可能发生访问完第一页时插入了两整页视频的情况
                    // 此时获取到的第二页视频比第一页的还要新，因此为了确保正确，理应对每一页的第一个视频进行时间比较
                    // 但在 streams 的抽象下，无法判断具体是在哪里分页的，所以暂且对每个视频都进行比较，应该不会有太大性能损失
                    let release_datetime = v.release_datetime();
                    if release_datetime > &max_datetime {
                        max_datetime = *release_datetime;
                    }
                    futures::future::ready(video_source.should_take(release_datetime, &latest_row_at))
                }
            }
        })
        .filter_map(|res| futures::future::ready(res.ok()))
        .chunks(10);
    let mut count = 0;
    while let Some(videos_info) = video_streams.next().await {
        count += videos_info.len();
        create_videos(videos_info, video_source, connection).await?;
    }
    // 如果获取视频分页过程中发生了错误，直接在此处返回，不更新 latest_row_at
    error?;
    if max_datetime != latest_row_at {
        video_source
            .update_latest_row_at(max_datetime.naive_utc())
            .save(connection)
            .await?;
    }
    video_source.log_refresh_video_end(count);
    Ok(())
}

/// 筛选出所有未获取到全部信息的视频，尝试补充其详细信息
pub async fn fetch_video_details(
    bili_client: &BiliClient,
    video_source: &VideoSourceEnum,
    connection: &DatabaseConnection,
) -> Result<()> {
    video_source.log_fetch_video_start();
    let videos_model = filter_unfilled_videos(video_source.filter_expr(), connection).await?;
    for video_model in videos_model {
        let video = Video::new(bili_client, video_model.bvid.clone());
        let info: Result<_> = async { Ok((video.get_tags().await?, video.get_view_info().await?)) }.await;
        match info {
            Err(e) => {
                error!(
                    "获取视频 {} - {} 的详细信息失败，错误为：{:#}",
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
                video_source.set_relation_id(&mut video_active_model);
                video_active_model.single_page = Set(Some(pages_len == 1));
                video_active_model.tags = Set(Some(serde_json::to_value(tags)?));
                video_active_model.save(&txn).await?;
                txn.commit().await?;
            }
        };
    }
    video_source.log_fetch_video_end();
    Ok(())
}

/// 下载所有未处理成功的视频
pub async fn download_unprocessed_videos(
    bili_client: &BiliClient,
    video_source: &VideoSourceEnum,
    connection: &DatabaseConnection,
) -> Result<()> {
    video_source.log_download_video_start();
    let semaphore = Semaphore::new(CONFIG.concurrent_limit.video);
    let downloader = Downloader::new(bili_client.client.clone());
    let unhandled_videos_pages = filter_unhandled_video_pages(video_source.filter_expr(), connection).await?;
    
    // 添加详细日志，记录找到的未处理视频数量
    info!("找到 {} 个未处理完成的视频", unhandled_videos_pages.len());
    
    let mut assigned_upper = HashSet::new();
    let tasks = unhandled_videos_pages
        .into_iter()
        .map(|(video_model, pages_model)| {
            let should_download_upper = !assigned_upper.contains(&video_model.upper_id);
            assigned_upper.insert(video_model.upper_id);
            info!("下载视频: {}", video_model.name);
            download_video_pages(
                bili_client,
                video_source,
                video_model,
                pages_model,
                connection,
                &semaphore,
                &downloader,
                should_download_upper,
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
        error!("下载触发风控，已终止所有任务，等待下一轮执行");
    }
    video_source.log_download_video_end();
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn download_video_pages(
    bili_client: &BiliClient,
    video_source: &VideoSourceEnum,
    video_model: video::Model,
    pages: Vec<page::Model>,
    connection: &DatabaseConnection,
    semaphore: &Semaphore,
    downloader: &Downloader,
    should_download_upper: bool,
) -> Result<video::ActiveModel> {
    let _permit = semaphore.acquire().await.context("acquire semaphore failed")?;
    let mut status = VideoStatus::from(video_model.download_status);
    let separate_status = status.should_run();
    
    // 添加判断：检查是否为番剧类型
    let is_bangumi = match video_source {
        VideoSourceEnum::BangumiSource(_) => true,
        _ => false,
    };
    
    // 添加日志，帮助排查，降级为debug级别
    debug!("视频「{}」是否为番剧: {}", &video_model.name, is_bangumi);
    debug!("视频source_type: {:?}", video_model.source_type);
    
    // 获取番剧源和季度信息
    let (base_path, season_folder) = if is_bangumi {
        let bangumi_source = match video_source {
            VideoSourceEnum::BangumiSource(source) => source,
            _ => unreachable!(),
        };
        
        let path = bangumi_source.path().to_path_buf();
        
        // 如果启用了下载所有季度，则根据season_id创建子文件夹
        if bangumi_source.download_all_seasons && video_model.season_id.is_some() {
            let season_id = video_model.season_id.as_ref().unwrap();
            
            // 从API获取季度标题
            let season_title = match get_season_title_from_api(bili_client, season_id).await {
                Some(title) => title,
                None => season_id.clone(), // 如果找不到季度名称，就使用season_id
            };
            
            debug!("番剧「{}」使用季度文件夹: {}", &video_model.name, season_title);
            (path.join(&season_title), Some(season_title))
        } else {
            // 不启用下载所有季度时，直接使用指定路径
            debug!("番剧「{}」使用直接路径: {}", &video_model.name, path.display());
            (path, None)
        }
    } else {
        // 非番剧使用原来的逻辑
        let path = video_source
        .path()
        .join(TEMPLATE.path_safe_render("video", &video_format_args(&video_model))?);
        debug!("非番剧「{}」使用子文件夹路径: {}", &video_model.name, path.display());
        (path, None)
    };
    
    // 确保季度文件夹存在
    if let Some(season_folder_name) = &season_folder {
        let season_path = video_source.path().join(season_folder_name);
        if !season_path.exists() {
            fs::create_dir_all(&season_path).await?;
            info!("创建季度文件夹: {}", season_path.display());
        }
    }
    
    let upper_id = video_model.upper_id.to_string();
    let base_upper_path = &CONFIG
        .upper_path
        .join(upper_id.chars().next().context("upper_id is empty")?.to_string())
        .join(upper_id);
    let is_single_page = video_model.single_page.context("single_page is null")?;
    // 对于单页视频，page 的下载已经足够
    // 对于多页视频，page 下载仅包含了分集内容，需要额外补上视频的 poster 的 tvshow.nfo
    let tasks: Vec<Pin<Box<dyn Future<Output = Result<ExecutionStatus>> + Send>>> = vec![
        // 下载视频封面
        Box::pin(fetch_video_poster(
            separate_status[0] && !is_single_page,
            &video_model,
            downloader,
            base_path.join("poster.jpg"),
            base_path.join("fanart.jpg"),
        )),
        // 生成视频信息的 nfo
        Box::pin(generate_video_nfo(
            separate_status[1] && !is_single_page,
            &video_model,
            base_path.join("tvshow.nfo"),
        )),
        // 下载 Up 主头像
        Box::pin(fetch_upper_face(
            separate_status[2] && should_download_upper,
            &video_model,
            downloader,
            base_upper_path.join("folder.jpg"),
        )),
        // 生成 Up 主信息的 nfo
        Box::pin(generate_upper_nfo(
            separate_status[3] && should_download_upper,
            &video_model,
            base_upper_path.join("person.nfo"),
        )),
        // 分发并执行分 P 下载的任务
        Box::pin(dispatch_download_page(
            separate_status[4],
            bili_client,
            &video_model,
            pages,
            connection,
            downloader,
            &base_path,
        )),
    ];
    let tasks: FuturesOrdered<_> = tasks.into_iter().collect();
    let results: Vec<ExecutionStatus> = tasks.collect::<Vec<_>>().await.into_iter().map(Into::into).collect();
    status.update_status(&results);
    results
        .iter()
        .take(4)
        .zip(["封面", "详情", "作者头像", "作者详情"])
        .for_each(|(res, task_name)| match res {
            ExecutionStatus::Skipped => info!("处理视频「{}」{}已成功过，跳过", &video_model.name, task_name),
            ExecutionStatus::Succeeded => info!("处理视频「{}」{}成功", &video_model.name, task_name),
            ExecutionStatus::Ignored(e) => {
                info!(
                    "处理视频「{}」{}出现常见错误，已忽略: {:#}",
                    &video_model.name, task_name, e
                )
            }
            ExecutionStatus::Failed(e) | ExecutionStatus::FixedFailed(_, e) => {
                // 对于404错误，降级为debug日志
                if let Some(BiliError::RequestFailed(-404, _)) = e.downcast_ref::<BiliError>() {
                    debug!("处理视频「{}」{}失败(404): {:#}", &video_model.name, task_name, e);
                } else {
                    error!("处理视频「{}」{}失败: {:#}", &video_model.name, task_name, e);
                }
            }
        });
    if let ExecutionStatus::Failed(e) = results.into_iter().nth(4).context("page download result not found")? {
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
) -> Result<ExecutionStatus> {
    if !should_run {
        return Ok(ExecutionStatus::Skipped);
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
    let (mut download_aborted, mut target_status) = (false, STATUS_OK);
    let mut stream = tasks
        .take_while(|res| {
            match res {
                Ok(model) => {
                    // 该视频的所有分页的下载状态都会在此返回，需要根据这些状态确认视频层"分 P 下载"子任务的状态
                    // 在过去的实现中，此处仅仅根据 page_download_status 的最高标志位来判断，如果最高标志位是 true 则认为完成
                    // 这样会导致即使分页中有失败到 MAX_RETRY 的情况，视频层的分 P 下载状态也会被认为是 Succeeded，不够准确
                    // 新版本实现会将此处取值为所有子任务状态的最小值，这样只有所有分页的子任务全部成功时才会认为视频层的分 P 下载状态是 Succeeded
                    let page_download_status = model.download_status.try_as_ref().expect("download_status must be set");
                    let separate_status: [u32; 5] = PageStatus::from(*page_download_status).into();
                    for status in separate_status {
                        target_status = target_status.min(status);
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
        error!("下载视频「{}」的分页时触发风控，将异常向上传递..", &video_model.name);
        bail!(DownloadAbortError());
    }
    if target_status != STATUS_OK {
        return Ok(ExecutionStatus::FixedFailed(target_status, ProcessPageError().into()));
    }
    Ok(ExecutionStatus::Succeeded)
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
    let mut status = PageStatus::from(page_model.download_status);
    let separate_status = status.should_run();
    let is_single_page = video_model.single_page.context("single_page is null")?;
    
    // 检查是否为番剧
    let is_bangumi = match video_model.source_type {
        Some(1) => true,  // source_type = 1 表示为番剧
        _ => false,
    };
    
    // 添加日志，帮助排查
    debug!("分集「{}」是否为番剧: {}", &page_model.name, is_bangumi);
    debug!("分集所属视频source_type: {:?}", video_model.source_type);
    debug!("使用基础路径: {}", base_path.display());
    
    let base_name = TEMPLATE.path_safe_render("page", &page_format_args(video_model, &page_model))?;
    let (poster_path, video_path, nfo_path, danmaku_path, fanart_path, subtitle_path) = if is_single_page {
        (
            base_path.join(format!("{}-poster.jpg", &base_name)),
            base_path.join(format!("{}.mp4", &base_name)),
            base_path.join(format!("{}.nfo", &base_name)),
            base_path.join(format!("{}.zh-CN.default.ass", &base_name)),
            Some(base_path.join(format!("{}-fanart.jpg", &base_name))),
            base_path.join(format!("{}.srt", &base_name)),
        )
    } else if is_bangumi {
        // 番剧直接使用基础路径，不创建子文件夹结构
        (
            base_path.join(format!("{}-thumb.jpg", &base_name)),
            base_path.join(format!("{}.mp4", &base_name)),
            base_path.join(format!("{}.nfo", &base_name)),
            base_path.join(format!("{}.zh-CN.default.ass", &base_name)),
            None,
            base_path.join(format!("{}.srt", &base_name)),
        )
    } else {
        // 非番剧使用原来的逻辑
        // 使用自定义文件夹结构模板
        let folder_structure = TEMPLATE.path_safe_render("folder_structure", &page_format_args(video_model, &page_model))?;
        // 分割路径，确保所有父目录都存在
        let folder_path = base_path.join(&folder_structure);
        if let Some(parent) = folder_path.parent() {
            fs::create_dir_all(parent).await?;
        }
        
        (
            folder_path.with_extension("thumb.jpg"),
            folder_path.with_extension("mp4"),
            folder_path.with_extension("nfo"),
            folder_path.with_extension("zh-CN.default.ass"),
            // 对于多页视频，会在上一步 fetch_video_poster 中获取剧集的 fanart，无需在此处下载单集的
            None,
            folder_path.with_extension("srt"),
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
    let tasks: Vec<Pin<Box<dyn Future<Output = Result<ExecutionStatus>> + Send>>> = vec![
        Box::pin(fetch_page_poster(
            separate_status[0],
            video_model,
            &page_model,
            downloader,
            poster_path,
            fanart_path,
        )),
        Box::pin(fetch_page_video(
            separate_status[1],
            bili_client,
            video_model,
            downloader,
            &page_info,
            &video_path,
        )),
        Box::pin(generate_page_nfo(
            separate_status[2],
            video_model,
            &page_model,
            nfo_path,
        )),
        Box::pin(fetch_page_danmaku(
            separate_status[3],
            bili_client,
            video_model,
            &page_info,
            danmaku_path,
        )),
        Box::pin(fetch_page_subtitle(
            separate_status[4],
            bili_client,
            video_model,
            &page_info,
            &subtitle_path,
        )),
    ];
    let tasks: FuturesOrdered<_> = tasks.into_iter().collect();
    let results: Vec<ExecutionStatus> = tasks.collect::<Vec<_>>().await.into_iter().map(Into::into).collect();
    status.update_status(&results);
    results
        .iter()
        .zip(["封面", "视频", "详情", "弹幕", "字幕"])
        .for_each(|(res, task_name)| match res {
            ExecutionStatus::Skipped => info!(
                "处理视频「{}」第 {} 页{}已成功过，跳过",
                &video_model.name, page_model.pid, task_name
            ),
            ExecutionStatus::Succeeded => info!(
                "处理视频「{}」第 {} 页{}成功",
                &video_model.name, page_model.pid, task_name
            ),
            ExecutionStatus::Ignored(e) => {
                info!(
                    "处理视频「{}」第 {} 页{}出现常见错误，已忽略: {:#}",
                    &video_model.name, page_model.pid, task_name, e
                )
            }
            ExecutionStatus::Failed(e) | ExecutionStatus::FixedFailed(_, e) => {
                // 对于404错误，降级为debug日志
                if let Some(BiliError::RequestFailed(-404, _)) = e.downcast_ref::<BiliError>() {
                    debug!(
                        "处理视频「{}」第 {} 页{}失败(404): {:#}",
                        &video_model.name, page_model.pid, task_name, e
                    );
                } else {
                    error!(
                "处理视频「{}」第 {} 页{}失败: {:#}",
                &video_model.name, page_model.pid, task_name, e
                    );
                }
            },
        });
    // 如果下载视频时触发风控，直接返回 DownloadAbortError
    if let ExecutionStatus::Failed(e) = results.into_iter().nth(1).context("video download result not found")? {
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
) -> Result<ExecutionStatus> {
    if !should_run {
        return Ok(ExecutionStatus::Skipped);
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
    Ok(ExecutionStatus::Succeeded)
}

pub async fn fetch_page_video(
    should_run: bool,
    bili_client: &BiliClient,
    video_model: &video::Model,
    downloader: &Downloader,
    page_info: &PageInfo,
    page_path: &Path,
) -> Result<ExecutionStatus> {
    if !should_run {
        return Ok(ExecutionStatus::Skipped);
    }
    
    let bili_video = Video::new(bili_client, video_model.bvid.clone());
    
    // 获取视频流信息
    let streams = match bili_video.get_page_analyzer(page_info).await {
        Ok(mut analyzer) => {
            match analyzer.best_stream(&CONFIG.filter_option) {
                Ok(stream) => {
                    stream
                },
                Err(e) => {
                    // 对于404错误，降级为debug日志，不需要打扰用户
                    if let Some(BiliError::RequestFailed(-404, _)) = e.downcast_ref::<BiliError>() {
                        debug!("选择最佳流失败(404): {:#}", e);
                    } else {
                    error!("选择最佳流失败: {:#}", e);
                    }
                    return Err(e);
                }
            }
        },
        Err(e) => {
            // 对于404错误，降级为debug日志，不需要打扰用户
            if let Some(BiliError::RequestFailed(-404, _)) = e.downcast_ref::<BiliError>() {
                debug!("获取视频分析器失败(404): {:#}", e);
            } else {
            error!("获取视频分析器失败: {:#}", e);
            }
            return Err(e);
        }
    };
    
    // 创建保存目录
    if let Some(parent) = page_path.parent() {
        if !parent.exists() {
            tokio::fs::create_dir_all(parent).await?;
        }
    }
    
    // 获取多线程下载配置
    let parallel_config = &CONFIG.concurrent_limit.parallel_download;
    let use_parallel = parallel_config.enabled;
    let threads = parallel_config.threads;
    
    // 记录开始时间
    let start_time = std::time::Instant::now();
    let mut total_bytes = 0u64;
    
    // 根据流类型进行不同处理
    match streams {
        BestStream::Mixed(mix_stream) => {
            if use_parallel {
                match downloader.fetch_with_fallback_parallel(&mix_stream.urls(), page_path, threads).await {
                    Ok(_) => {
                        // 获取文件大小
                        if let Ok(metadata) = tokio::fs::metadata(page_path).await {
                            total_bytes = metadata.len();
                        }
                    },
                    Err(e) => {
                        // 对于404错误，降级为debug日志
                        if let Some(BiliError::RequestFailed(-404, _)) = e.downcast_ref::<BiliError>() {
                            debug!("下载失败(404): {:#}", e);
                        } else {
                            error!("下载失败: {:#}", e);
                        }
                        return Err(e);
                    }
                }
            } else {
                match downloader.fetch_with_fallback(&mix_stream.urls(), page_path).await {
                    Ok(_) => {
                        // 获取文件大小
                        if let Ok(metadata) = tokio::fs::metadata(page_path).await {
                            total_bytes = metadata.len();
                        }
                    },
                    Err(e) => {
                        // 对于404错误，降级为debug日志
                        if let Some(BiliError::RequestFailed(-404, _)) = e.downcast_ref::<BiliError>() {
                            debug!("下载失败(404): {:#}", e);
                        } else {
                            error!("下载失败: {:#}", e);
                        }
                        return Err(e);
                    }
                }
            }
        },
        BestStream::VideoAudio {
            video: video_stream,
            audio: None,
        } => {
            if use_parallel {
                match downloader.fetch_with_fallback_parallel(&video_stream.urls(), page_path, threads).await {
                    Ok(_) => {
                        // 获取文件大小
                        if let Ok(metadata) = tokio::fs::metadata(page_path).await {
                            total_bytes = metadata.len();
                        }
                    },
                    Err(e) => {
                        // 对于404错误，降级为debug日志
                        if let Some(BiliError::RequestFailed(-404, _)) = e.downcast_ref::<BiliError>() {
                            debug!("下载失败(404): {:#}", e);
                        } else {
                            error!("下载失败: {:#}", e);
                        }
                        return Err(e);
                    }
                }
            } else {
                match downloader.fetch_with_fallback(&video_stream.urls(), page_path).await {
                    Ok(_) => {
                        // 获取文件大小
                        if let Ok(metadata) = tokio::fs::metadata(page_path).await {
                            total_bytes = metadata.len();
                        }
                    },
                    Err(e) => {
                        // 对于404错误，降级为debug日志
                        if let Some(BiliError::RequestFailed(-404, _)) = e.downcast_ref::<BiliError>() {
                            debug!("下载失败(404): {:#}", e);
                        } else {
                            error!("下载失败: {:#}", e);
                        }
                        return Err(e);
                    }
                }
            }
        },
        BestStream::VideoAudio {
            video: video_stream,
            audio: Some(audio_stream),
        } => {
            let (tmp_video_path, tmp_audio_path) = (
                page_path.with_extension("tmp_video"),
                page_path.with_extension("tmp_audio"),
            );
            
            let mut video_size = 0u64;
            let mut audio_size = 0u64;
            
            if use_parallel {
                if let Err(e) = downloader.fetch_with_fallback_parallel(&video_stream.urls(), &tmp_video_path, threads).await {
                    error!("视频流下载失败: {:#}", e);
                    return Err(e);
                } else if let Ok(metadata) = tokio::fs::metadata(&tmp_video_path).await {
                    video_size = metadata.len();
                }
                
                if let Err(e) = downloader.fetch_with_fallback_parallel(&audio_stream.urls(), &tmp_audio_path, threads).await {
                    error!("音频流下载失败: {:#}", e);
                    let _ = fs::remove_file(&tmp_video_path).await;
                    return Err(e);
                } else if let Ok(metadata) = tokio::fs::metadata(&tmp_audio_path).await {
                    audio_size = metadata.len();
                }
            } else {
                if let Err(e) = downloader.fetch_with_fallback(&video_stream.urls(), &tmp_video_path).await {
                    error!("视频流下载失败: {:#}", e);
                    return Err(e);
                } else if let Ok(metadata) = tokio::fs::metadata(&tmp_video_path).await {
                    video_size = metadata.len();
                }
                
                if let Err(e) = downloader.fetch_with_fallback(&audio_stream.urls(), &tmp_audio_path).await {
                    error!("音频流下载失败: {:#}", e);
                    let _ = fs::remove_file(&tmp_video_path).await;
                    return Err(e);
                } else if let Ok(metadata) = tokio::fs::metadata(&tmp_audio_path).await {
                    audio_size = metadata.len();
                }
            }
            
            let res = downloader.merge(&tmp_video_path, &tmp_audio_path, page_path).await;
            let _ = fs::remove_file(tmp_video_path).await;
            let _ = fs::remove_file(tmp_audio_path).await;
            
            if let Err(e) = res {
                error!("音视频合并失败: {:#}", e);
                return Err(e);
            }
            
            // 获取合并后文件大小
            if let Ok(metadata) = tokio::fs::metadata(page_path).await {
                total_bytes = metadata.len();
            } else {
                // 如果无法获取合并后的文件大小，使用视频和音频大小之和
                total_bytes = video_size + audio_size;
            }
        }
    }
    
    // 计算下载速度
    let elapsed = start_time.elapsed();
    let elapsed_secs = elapsed.as_secs_f64();
    
    if elapsed_secs > 0.0 && total_bytes > 0 {
        // 计算速度 (字节/秒)
        let speed_bps = total_bytes as f64 / elapsed_secs;
        
        // 转换为更友好的单位
        let (speed, unit) = if speed_bps >= 1_000_000.0 {
            (speed_bps / 1_000_000.0, "MB/s")
        } else if speed_bps >= 1_000.0 {
            (speed_bps / 1_000.0, "KB/s")
        } else {
            (speed_bps, "B/s")
        };
        
        // 记录下载速度信息
        info!(
            "视频下载完成，总大小: {:.2} MB，耗时: {:.2} 秒，平均速度: {:.2} {}",
            total_bytes as f64 / 1_000_000.0,
            elapsed_secs,
            speed,
            unit
        );
    }
    
    Ok(ExecutionStatus::Succeeded)
}

pub async fn fetch_page_danmaku(
    should_run: bool,
    bili_client: &BiliClient,
    video_model: &video::Model,
    page_info: &PageInfo,
    danmaku_path: PathBuf,
) -> Result<ExecutionStatus> {
    if !should_run {
        return Ok(ExecutionStatus::Skipped);
    }
    let bili_video = Video::new(bili_client, video_model.bvid.clone());
    bili_video
        .get_danmaku_writer(page_info)
        .await?
        .write(danmaku_path)
        .await?;
    Ok(ExecutionStatus::Succeeded)
}

pub async fn fetch_page_subtitle(
    should_run: bool,
    bili_client: &BiliClient,
    video_model: &video::Model,
    page_info: &PageInfo,
    subtitle_path: &Path,
) -> Result<ExecutionStatus> {
    if !should_run {
        return Ok(ExecutionStatus::Skipped);
    }
    let bili_video = Video::new(bili_client, video_model.bvid.clone());
    let subtitles = bili_video.get_subtitles(page_info).await?;
    let tasks = subtitles
        .into_iter()
        .map(|subtitle| async move {
            let path = subtitle_path.with_extension(format!("{}.srt", subtitle.lan));
            tokio::fs::write(path, subtitle.body.to_string()).await
        })
        .collect::<FuturesUnordered<_>>();
    tasks.try_collect::<Vec<()>>().await?;
    Ok(ExecutionStatus::Succeeded)
}

pub async fn generate_page_nfo(
    should_run: bool,
    video_model: &video::Model,
    page_model: &page::Model,
    nfo_path: PathBuf,
) -> Result<ExecutionStatus> {
    if !should_run {
        return Ok(ExecutionStatus::Skipped);
    }
    let single_page = video_model.single_page.context("single_page is null")?;
    let nfo_serializer = if single_page {
        NFOSerializer(ModelWrapper::Video(video_model), NFOMode::MOVIE)
    } else {
        NFOSerializer(ModelWrapper::Page(page_model), NFOMode::EPOSODE)
    };
    generate_nfo(nfo_serializer, nfo_path).await?;
    Ok(ExecutionStatus::Succeeded)
}

pub async fn fetch_video_poster(
    should_run: bool,
    video_model: &video::Model,
    downloader: &Downloader,
    poster_path: PathBuf,
    fanart_path: PathBuf,
) -> Result<ExecutionStatus> {
    if !should_run {
        return Ok(ExecutionStatus::Skipped);
    }
    downloader.fetch(&video_model.cover, &poster_path).await?;
    fs::copy(&poster_path, &fanart_path).await?;
    Ok(ExecutionStatus::Succeeded)
}

pub async fn fetch_upper_face(
    should_run: bool,
    video_model: &video::Model,
    downloader: &Downloader,
    upper_face_path: PathBuf,
) -> Result<ExecutionStatus> {
    if !should_run {
        return Ok(ExecutionStatus::Skipped);
    }
    downloader.fetch(&video_model.upper_face, &upper_face_path).await?;
    Ok(ExecutionStatus::Succeeded)
}

pub async fn generate_upper_nfo(
    should_run: bool,
    video_model: &video::Model,
    nfo_path: PathBuf,
) -> Result<ExecutionStatus> {
    if !should_run {
        return Ok(ExecutionStatus::Skipped);
    }
    let nfo_serializer = NFOSerializer(ModelWrapper::Video(video_model), NFOMode::UPPER);
    generate_nfo(nfo_serializer, nfo_path).await?;
    Ok(ExecutionStatus::Succeeded)
}

pub async fn generate_video_nfo(
    should_run: bool,
    video_model: &video::Model,
    nfo_path: PathBuf,
) -> Result<ExecutionStatus> {
    if !should_run {
        return Ok(ExecutionStatus::Skipped);
    }
    let nfo_serializer = NFOSerializer(ModelWrapper::Video(video_model), NFOMode::TVSHOW);
    generate_nfo(nfo_serializer, nfo_path).await?;
    Ok(ExecutionStatus::Succeeded)
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

async fn get_season_title_from_api(bili_client: &BiliClient, season_id: &str) -> Option<String> {
    use crate::bilibili::bangumi::Bangumi;
    
    // 记录日志
    debug!("通过API获取season_id: {} 的季度标题", season_id);
    
    // 创建Bangumi实例
    let bangumi = Bangumi::new(bili_client, None, Some(season_id.to_string()), None);
    
    // 尝试获取季度信息
    match bangumi.get_season_info().await {
        Ok(season_info) => {
            // 获取完整标题用于日志
            let title = season_info["title"].as_str().unwrap_or_default();
            
            // 从seasons数组中查找当前season_id对应的简短季度标题
            if let Some(seasons) = season_info["seasons"].as_array() {
                for season in seasons {
                    if let (Some(id), Some(season_title)) = (
                        season["season_id"].as_u64().map(|id| id.to_string()),
                        season["season_title"].as_str()
                    ) {
                        if id == season_id && !season_title.is_empty() {
                            debug!("成功获取到番剧「{}」的季度标题: {}", title, season_title);
                            return Some(season_title.to_string());
                        }
                    }
                }
            }
            
            // 如果在seasons数组中没找到，尝试使用根级别的season_title
            if let Some(season_title) = season_info["season_title"].as_str() {
                // 尝试从完整标题中提取季度部分（通常在末尾）
                if let Some(idx) = season_title.rfind(' ') {
                    let short_title = &season_title[idx + 1..];
                    if short_title.contains("季") {
                        debug!("从完整标题「{}」提取季度标题: {}", title, short_title);
                        return Some(short_title.to_string());
                    }
                }
                
                debug!("使用番剧「{}」的默认季度标题: {}", title, season_title);
                return Some(season_title.to_string());
            }
            
            // 如果没有获取到季度标题，尝试使用备用方法
            warn!("未能从API获取到season_id: {} 的季度标题", season_id);
            None
        },
        Err(e) => {
            // 记录错误日志
            warn!("获取season_id: {} 的季度标题失败: {}", season_id, e);
            None
        }
    }
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
