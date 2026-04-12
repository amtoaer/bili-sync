use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use anyhow::{Context, Result, bail};
use bili_sync_entity::upper_vec::Upper;
use bili_sync_entity::{page, video, watch_later};
use futures::StreamExt;
use futures::stream::FuturesUnordered;
use regex::Regex;
use sea_orm::ActiveValue::Set;
use sea_orm::TryIntoModel;
use tokio::sync::Semaphore;

use crate::adapter::{VideoSource, VideoSourceEnum};
use crate::bilibili::{BiliClient, BiliError, Video, VideoInfo};
use crate::config::{PathSafeTemplate, TEMPLATE, VersionedConfig, default_manual_download_root};
use crate::downloader::Downloader;
use crate::error::ExecutionStatus;
use crate::utils::compact_log_text;
use crate::utils::download_context::DownloadContext;
use crate::utils::format_arg::video_format_args;
use crate::utils::status::{STATUS_OK, VideoStatus};
use crate::workflow::{download_page, fetch_upper_face, fetch_video_poster, generate_upper_nfo, generate_video_nfo};

static BVID_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(BV[0-9A-Za-z]{10})").expect("failed to compile bvid regex"));

/// 尝试从输入字符串中提取 bvid，支持直接粘贴 bvid 或包含 bvid 的视频链接
pub fn extract_bvid(input: &str) -> Option<String> {
    let capture = BVID_REGEX.captures(input.trim())?;
    let mut bvid = capture.get(1)?.as_str().to_string();
    bvid.replace_range(..2, "BV");
    Some(bvid)
}

/// 将用户输入解析为 bvid，支持：直接 bvid、包含 bvid 的链接、b23.tv 短链跳转后的目标链接
pub async fn resolve_bvid(input: &str, client: &reqwest::Client) -> Result<String> {
    if let Some(bvid) = extract_bvid(input) {
        return Ok(bvid);
    }
    let input = input.trim();
    if input.starts_with("http://") || input.starts_with("https://") {
        let response = client
            .get(input)
            .send()
            .await
            .with_context(|| format!("访问链接「{}」失败", input))?;
        if let Some(bvid) = extract_bvid(response.url().as_str()) {
            return Ok(bvid);
        }
    }
    bail!("无法从输入内容中解析 BV 号，请检查输入是否正确")
}

/// 手动下载单个视频（支持多 P）
pub async fn download_video_by_bvid(
    connection: &sea_orm::DatabaseConnection,
    bili_client: &BiliClient,
    bvid: &str,
    download_path: Option<&str>,
) -> Result<()> {
    let config = VersionedConfig::get().snapshot();
    config.check().context("配置检查失败")?;

    let mixin_key = bili_client
        .wbi_img(&config.credential)
        .await
        .context("获取 wbi_img 失败")?
        .into_mixin_key()
        .context("解析 mixin key 失败")?;
    crate::bilibili::set_global_mixin_key(mixin_key);
    let bili_client = bili_client.snapshot()?;
    let template = TEMPLATE.snapshot();

    let output_root = download_path
        .map(PathBuf::from)
        .unwrap_or_else(default_manual_download_root);
    if !output_root.is_absolute() {
        bail!("手动下载路径必须是绝对路径");
    }
    let video_source = VideoSourceEnum::from(watch_later::Model {
        id: 0,
        path: output_root.to_string_lossy().to_string(),
        created_at: String::new(),
        latest_row_at: chrono::Utc::now().naive_utc(),
        rule: None,
        enabled: false,
    });
    video_source.create_dir_all().await?;

    let video = Video::new(&bili_client, bvid, &config.credential);
    let (tags, mut view_info) = tokio::try_join!(video.get_tags(), video.get_view_info())?;
    let pages = match &mut view_info {
        VideoInfo::Detail { pages, .. } => std::mem::take(pages),
        _ => bail!("视频详情解析失败"),
    };
    let base_model = video::Model {
        bvid: bvid.to_string(),
        category: 2,
        valid: true,
        favtime: chrono::Utc::now().naive_utc(),
        ..Default::default()
    };
    let mut video_active_model = view_info.into_detail_model(base_model, config.try_upower_anyway);
    video_active_model.single_page = Set(Some(pages.len() == 1));
    video_active_model.tags = Set(Some(tags.into()));
    video_active_model.should_download = Set(true);
    let video_model = video_active_model.try_into_model()?;
    if !video_model.valid {
        bail!("该视频当前不可下载（可能是番剧跳转、失效或受充电专享限制）");
    }

    let page_models = pages
        .into_iter()
        .enumerate()
        .map(|(idx, page_info)| {
            let (width, height) = match page_info.dimension {
                Some(d) if d.rotate == 0 => (Some(d.width), Some(d.height)),
                Some(d) => (Some(d.height), Some(d.width)),
                None => (None, None),
            };
            page::Model {
                id: idx as i32 + 1,
                video_id: video_model.id,
                cid: page_info.cid,
                pid: page_info.page,
                name: page_info.name,
                width,
                height,
                duration: page_info.duration,
                path: None,
                image: page_info.first_frame,
                download_status: 0,
                created_at: String::new(),
            }
        })
        .collect::<Vec<_>>();

    info!("开始执行手动下载任务：{} ({})", video_model.name, video_model.bvid);
    download_single_video(
        &video_model,
        page_models,
        &video_source,
        connection,
        &bili_client,
        &template,
        &config,
    )
    .await?;
    info!("手动下载任务完成：{} ({})", video_model.name, video_model.bvid);
    Ok(())
}

async fn download_single_video(
    video_model: &video::Model,
    page_models: Vec<page::Model>,
    video_source: &VideoSourceEnum,
    connection: &sea_orm::DatabaseConnection,
    bili_client: &BiliClient,
    template: &handlebars::Handlebars<'_>,
    config: &crate::config::Config,
) -> Result<()> {
    let status = VideoStatus::default();
    let video_log_name = compact_log_text(&video_model.name, 48);
    let separate_status = status.should_run();
    let base_path = video_source
        .path()
        .join(template.path_safe_render("video", &video_format_args(video_model, &config.time_format))?);
    tokio::fs::create_dir_all(&base_path).await?;
    let base_path = dunce::canonicalize(base_path).context("canonicalize video path failed")?;
    let is_single_page = video_model.single_page.context("single_page is null")?;
    let uppers_with_path = video_model
        .uppers()
        .map(|u| {
            let id_string = u.mid.to_string();
            (
                u,
                config
                    .upper_path
                    .join(id_string.chars().next().unwrap_or_default().to_string())
                    .join(id_string),
            )
        })
        .collect::<Vec<(Upper<i64, &str>, PathBuf)>>();
    let downloader = Downloader::new(bili_client.client.clone());
    let cx = DownloadContext::new(bili_client, video_source, template, connection, &downloader, config);
    let (res_1, res_2, res_3, res_4, res_5) = tokio::join!(
        fetch_video_poster(
            separate_status[0] && !is_single_page && !config.skip_option.no_poster,
            video_model,
            base_path.join("poster.jpg"),
            base_path.join("fanart.jpg"),
            cx
        ),
        generate_video_nfo(
            separate_status[1] && !is_single_page && !config.skip_option.no_video_nfo,
            video_model,
            base_path.join("tvshow.nfo"),
            cx
        ),
        fetch_upper_face(
            separate_status[2] && !config.skip_option.no_upper,
            &uppers_with_path,
            cx
        ),
        generate_upper_nfo(
            separate_status[3] && !config.skip_option.no_upper,
            video_model,
            &uppers_with_path,
            cx,
        ),
        dispatch_manual_download_page(separate_status[4], video_model, page_models, &base_path, cx)
    );
    let results = [res_1.into(), res_2.into(), res_3.into(), res_4.into(), res_5.into()];
    results
        .iter()
        .take(4)
        .zip(["封面", "详情", "作者头像", "作者详情"])
        .for_each(|(res, task_name)| match res {
            ExecutionStatus::Skipped => info!("手动下载视频「{}」{}已成功过，跳过", video_log_name, task_name),
            ExecutionStatus::Succeeded => info!("手动下载视频「{}」{}成功", video_log_name, task_name),
            ExecutionStatus::Ignored(e) => {
                error!(
                    "手动下载视频「{}」{}出现常见错误，已忽略：{:#}",
                    video_log_name, task_name, e
                )
            }
            ExecutionStatus::Failed(e) => error!("手动下载视频「{}」{}失败：{:#}", video_log_name, task_name, e),
            ExecutionStatus::Fixed(_) => unreachable!(),
        });
    for result in results {
        if let ExecutionStatus::Failed(e) = result
            && let Ok(e) = e.downcast::<BiliError>()
            && e.is_risk_control_related()
        {
            bail!(e);
        }
    }
    Ok(())
}

async fn dispatch_manual_download_page(
    should_run: bool,
    video_model: &video::Model,
    page_models: Vec<page::Model>,
    base_path: &Path,
    cx: DownloadContext<'_>,
) -> Result<ExecutionStatus> {
    if !should_run {
        return Ok(ExecutionStatus::Skipped);
    }
    let child_semaphore = Semaphore::new(cx.config.concurrent_limit.page);
    let mut tasks = page_models
        .into_iter()
        .map(|page_model| download_page(video_model, page_model, &child_semaphore, base_path, cx))
        .collect::<FuturesUnordered<_>>();
    let mut target_status = STATUS_OK;
    while let Some(res) = tasks.next().await {
        let model = res?;
        let separate_status: [u32; 5] = crate::utils::status::PageStatus::from(
            *model.download_status.try_as_ref().expect("download_status must be set"),
        )
        .into();
        for status in separate_status {
            target_status = target_status.min(status);
        }
    }
    Ok(ExecutionStatus::Fixed(target_status))
}
