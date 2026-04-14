//! 弹幕增量更新工作流。
//!
//! 与 [`crate::workflow`] 中的"首次下载"流程解耦：这里只负责在视频已下载成功后，
//! 按照 [`crate::config::item::DanmakuUpdatePolicy`] 的策略周期性重新拉取弹幕。
//!
//! 两种入口：
//! - [`refresh_danmaku_incremental`]：扫描所有已启用视频源里的 page，应用策略，逐个刷新。
//! - [`refresh_danmaku_for_video`] / [`refresh_danmaku_for_page`]：手动触发，忽略策略。
//!
//! 流程内会顺带做 **UP 换源检测**：调用一次 `get_view_info` 读最新的 cid/duration/width/height，
//! 与数据库缓存对比，发现变化时更新 page 表对应字段。cid 变化时同时清除 `download_status` 的
//! 弹幕位，强制后续按新 cid 重建。

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use bili_sync_entity::*;
use chrono::{DateTime, TimeZone, Utc};
use sea_orm::ActiveValue::Set;
use sea_orm::entity::prelude::*;

use crate::bilibili::{BiliClient, Dimension, PageInfo as BiliPageInfo, Video, VideoInfo};
use crate::config::Config;
use crate::utils::danmaku_schedule::{Decision, Stage, should_sync_danmaku};

/// 弹幕子任务在 download_status 中的位偏移（与 PageStatus 保持一致）。
const DANMAKU_STATUS_OFFSET: usize = 3;

/// 扫描所有视频源，按 [`DanmakuUpdatePolicy`] 刷新到期的 page 弹幕。
///
/// 策略未启用时直接返回。不会影响任何主下载流程。
pub async fn refresh_danmaku_incremental(
    bili_client: &BiliClient,
    connection: &DatabaseConnection,
    config: &Config,
) -> Result<()> {
    if !config.danmaku_update_policy.enabled {
        return Ok(());
    }
    if config.skip_option.no_danmaku {
        return Ok(());
    }
    info!("开始执行本轮弹幕增量更新..");
    let candidates = load_candidate_videos(connection).await?;
    let now = Utc::now();
    let mut processed = 0usize;
    let mut refreshed = 0usize;
    for (video_model, pages) in candidates {
        let selected = pages
            .into_iter()
            .filter_map(|page| {
                let pubtime = video_model.pubtime.and_utc();
                let last_synced = page.danmaku_last_synced_at.as_deref().and_then(parse_stored_datetime);
                match should_sync_danmaku(
                    &config.danmaku_update_policy,
                    pubtime,
                    last_synced,
                    page.danmaku_sync_generation,
                    now,
                ) {
                    Decision::Sync { next_stage } => Some((page, Some(next_stage))),
                    Decision::Skip => None,
                }
            })
            .collect::<Vec<_>>();
        if selected.is_empty() {
            continue;
        }
        match refresh_video_pages(bili_client, connection, config, &video_model, selected, now).await {
            Ok(n) => {
                refreshed += n;
                processed += 1;
            }
            Err(e) => {
                error!(
                    "刷新视频「{}」({}) 的弹幕失败：{:#}",
                    video_model.name, video_model.bvid, e
                );
            }
        }
    }
    info!("弹幕增量更新结束：处理视频 {} 个，刷新分页 {} 个", processed, refreshed);
    Ok(())
}

/// 候选视频：有效 + 有路径（至少下载过） + 至少存在一个 page 的 download_status 弹幕位已成功，
/// 且**所属源仍处于启用状态**。
///
/// 与项目里其他流程保持一致：disabled 源被视为"用户主动暂停处理"，弹幕增量也不再触碰它的内容，
/// 避免后台默默地继续请求 B 站接口和改写本地 ASS 文件。
async fn load_candidate_videos(connection: &DatabaseConnection) -> Result<Vec<(video::Model, Vec<page::Model>)>> {
    use sea_orm::{Condition, QuerySelect};

    // 一次性取齐四类启用源的 id 集合
    let favorite_ids: Vec<i32> = favorite::Entity::find()
        .filter(favorite::Column::Enabled.eq(true))
        .select_only()
        .column(favorite::Column::Id)
        .into_tuple()
        .all(connection)
        .await
        .context("load enabled favorite ids failed")?;
    let collection_ids: Vec<i32> = collection::Entity::find()
        .filter(collection::Column::Enabled.eq(true))
        .select_only()
        .column(collection::Column::Id)
        .into_tuple()
        .all(connection)
        .await
        .context("load enabled collection ids failed")?;
    let submission_ids: Vec<i32> = submission::Entity::find()
        .filter(submission::Column::Enabled.eq(true))
        .select_only()
        .column(submission::Column::Id)
        .into_tuple()
        .all(connection)
        .await
        .context("load enabled submission ids failed")?;
    let watch_later_ids: Vec<i32> = watch_later::Entity::find()
        .filter(watch_later::Column::Enabled.eq(true))
        .select_only()
        .column(watch_later::Column::Id)
        .into_tuple()
        .all(connection)
        .await
        .context("load enabled watch_later ids failed")?;

    // 至少一个外键命中启用集合，才纳入候选；全部为空时直接 early-return 避免无意义查询。
    if favorite_ids.is_empty() && collection_ids.is_empty() && submission_ids.is_empty() && watch_later_ids.is_empty() {
        return Ok(Vec::new());
    }
    let mut source_filter = Condition::any();
    if !favorite_ids.is_empty() {
        source_filter = source_filter.add(video::Column::FavoriteId.is_in(favorite_ids));
    }
    if !collection_ids.is_empty() {
        source_filter = source_filter.add(video::Column::CollectionId.is_in(collection_ids));
    }
    if !submission_ids.is_empty() {
        source_filter = source_filter.add(video::Column::SubmissionId.is_in(submission_ids));
    }
    if !watch_later_ids.is_empty() {
        source_filter = source_filter.add(video::Column::WatchLaterId.is_in(watch_later_ids));
    }

    video::Entity::find()
        .filter(
            Condition::all()
                .add(video::Column::Valid.eq(true))
                .add(video::Column::Path.ne(""))
                .add(source_filter),
        )
        .find_with_related(page::Entity)
        .all(connection)
        .await
        .context("load candidate videos for danmaku refresh failed")
        .map(|rows| {
            rows.into_iter()
                .map(|(v, pages)| {
                    // 只保留弹幕任务已经成功过的 page；从未成功过的交给主流程处理
                    let filtered = pages
                        .into_iter()
                        .filter(|p| danmaku_subtask_completed(p.download_status))
                        .collect::<Vec<_>>();
                    (v, filtered)
                })
                .filter(|(_, pages)| !pages.is_empty())
                .collect()
        })
}

/// 检查 download_status 中弹幕子任务是否为 STATUS_OK（值为 7）。
fn danmaku_subtask_completed(status: u32) -> bool {
    let slot = (status >> (DANMAKU_STATUS_OFFSET * 3)) & 0b111;
    slot == crate::utils::status::STATUS_OK
}

/// UP 主换源（cid 变化）后，将 page 的所有非弹幕子任务（封面/视频/NFO/字幕）位重置为
/// `STATUS_NOT_STARTED`，让主下载流程下一轮重新拉取 MP4/SRT/封面等本地资产。
///
/// **保留弹幕位为 STATUS_OK**：因为本次刷新已经用新 cid 写入了正确的 ASS 文件。
/// 同时清掉 STATUS_COMPLETED 高位，让 page 重新进入"未完成"状态。
fn reset_non_danmaku_subtasks(status: u32) -> u32 {
    let mut new_status = status;
    for offset in 0..5 {
        if offset == DANMAKU_STATUS_OFFSET {
            continue;
        }
        new_status &= !(0b111 << (offset * 3));
    }
    new_status & !(1 << 31) // 清完成标记
}

/// 当某个 page 的 cid 变了之后，需要让其所属 video 重新进入 `filter_unhandled_video_pages`
/// 的候选集。两个条件：
/// 1. 清掉 STATUS_COMPLETED 高位（否则 `lt(STATUS_COMPLETED)` 过滤会把它直接排除）。
/// 2. 把视频层"分页下载"子任务（offset 4）位归零，让 `should_run` 重新返回 true。
fn reset_video_for_page_redownload(status: u32) -> u32 {
    const PAGE_DOWNLOAD_OFFSET: usize = 4;
    let cleared = status & !(0b111 << (PAGE_DOWNLOAD_OFFSET * 3));
    cleared & !(1 << 31)
}

/// 对某个视频下选中的 page 做一次弹幕刷新：拉 view_info 检测换源 → 逐个重抓弹幕 → 更新元数据。
///
/// 返回本次成功刷新的 page 数量。
async fn refresh_video_pages(
    bili_client: &BiliClient,
    connection: &DatabaseConnection,
    config: &Config,
    video_model: &video::Model,
    selected: Vec<(page::Model, Option<Stage>)>,
    now: DateTime<Utc>,
) -> Result<usize> {
    let bili_video = Video::new(bili_client, video_model.bvid.as_str(), &config.credential);
    // 拉一次 view_info，拿到最新的 cid/duration/dimension；失败则本轮跳过该视频
    let view_info = bili_video
        .get_view_info()
        .await
        .with_context(|| format!("刷新视频 {} 时获取 view_info 失败", video_model.bvid))?;
    let VideoInfo::Detail { pages: fresh_pages, .. } = view_info else {
        bail!("view_info 返回了非 Detail 类型，无法刷新弹幕");
    };
    let mut success = 0usize;
    for (db_page, next_stage) in selected {
        let fresh = fresh_pages.iter().find(|p| p.page == db_page.pid);
        let Some(fresh) = fresh else {
            warn!(
                "视频「{}」({}) 的分页 pid={} 在新拉取的 view_info 中不存在，跳过",
                video_model.name, video_model.bvid, db_page.pid
            );
            continue;
        };
        if let Err(e) = refresh_one_page(
            &bili_video,
            connection,
            config,
            video_model,
            db_page,
            fresh,
            next_stage,
            now,
        )
        .await
        {
            error!(
                "刷新视频「{}」({}) 分页 pid={} 弹幕失败：{:#}",
                video_model.name, video_model.bvid, fresh.page, e
            );
            continue;
        }
        success += 1;
    }
    Ok(success)
}

/// `next_stage` 语义：
/// - `Some(stage)`：调度路径，使用决策函数算好的阶段（可能是 `Frozen`）。
/// - `None`：手动触发路径，按 page 当前年龄计算阶段，不允许冻结（cap 在 `Cold`），
///   避免用户手动刷新已成熟视频时被回退成 `Fresh`，也避免活跃视频被意外冻结。
async fn refresh_one_page(
    bili_video: &Video<'_>,
    connection: &DatabaseConnection,
    config: &Config,
    video_model: &video::Model,
    db_page: page::Model,
    fresh: &BiliPageInfo,
    next_stage: Option<Stage>,
    now: DateTime<Utc>,
) -> Result<()> {
    let pubtime = video_model.pubtime.and_utc();
    let resolved_stage = next_stage.unwrap_or_else(|| {
        crate::utils::danmaku_schedule::stage_for_age(
            &config.danmaku_update_policy,
            pubtime,
            now,
            /* allow_freeze */ false,
        )
    });
    let danmaku_path = resolve_danmaku_path(video_model, &db_page)?;
    let (fresh_width, fresh_height) = extract_dimension(fresh.dimension.as_ref());
    let cid_changed = db_page.cid != fresh.cid;
    let duration_changed = db_page.duration != fresh.duration;
    let dimension_changed = fresh_width != db_page.width || fresh_height != db_page.height;

    if cid_changed {
        warn!(
            "检测到视频「{}」({}) 分页 pid={} 的 cid 发生变化 ({} -> {})，可能是 UP 主换源，已重置弹幕状态",
            video_model.name, video_model.bvid, fresh.page, db_page.cid, fresh.cid
        );
    }

    // 使用最新的 PageInfo 构造弹幕请求：保证换源后的新 duration 被用于分段数
    let page_info_for_danmaku = BiliPageInfo {
        cid: fresh.cid,
        page: fresh.page,
        name: db_page.name.clone(),
        duration: fresh.duration,
        first_frame: fresh.first_frame.clone(),
        dimension: fresh.dimension.as_ref().map(|d| Dimension {
            width: d.width,
            height: d.height,
            rotate: d.rotate,
        }),
    };

    // 原子写入：先写到 .tmp，再 rename，避免播放器读到半截 ASS
    let tmp_path = make_tmp_path(&danmaku_path);
    bili_video
        .get_danmaku_writer(&page_info_for_danmaku)
        .await?
        .write(tmp_path.clone(), &config.danmaku_option)
        .await?;
    tokio::fs::rename(&tmp_path, &danmaku_path)
        .await
        .with_context(|| format!("重命名弹幕文件 {:?} -> {:?} 失败", tmp_path, danmaku_path))?;

    // 写回数据库
    let now_str = now.naive_utc().to_string();
    let mut active: page::ActiveModel = db_page.clone().into();
    active.danmaku_last_synced_at = Set(Some(now_str));
    active.danmaku_sync_generation = Set(resolved_stage.as_generation());
    active.danmaku_cid_snapshot = Set(Some(fresh.cid));
    if cid_changed {
        // cid 变化 = UP 主把这页换成了不同内容（不是简单修正）。本地的 MP4/SRT/封面/NFO 都还指向
        // 旧 cid 的内容，必须让主下载流程重抓一次。这里：
        // 1. 清掉 page 的非弹幕子任务位（弹幕已经用新 cid 写盘，保留 OK，避免下一轮 incremental 又跑一次）。
        // 2. 同时清掉所属 video 的"分页下载"子任务 + STATUS_COMPLETED 高位，让 video 重新被
        //    filter_unhandled_video_pages 选中。否则 page 标记是"未完成"也没用，video 高位拦着。
        active.cid = Set(fresh.cid);
        active.download_status = Set(reset_non_danmaku_subtasks(db_page.download_status));
        let new_video_status = reset_video_for_page_redownload(video_model.download_status);
        if new_video_status != video_model.download_status {
            let mut video_active: video::ActiveModel = video_model.clone().into();
            video_active.download_status = Set(new_video_status);
            video_active
                .update(connection)
                .await
                .context("cid 变化后重置 video.download_status 失败")?;
        }
    }
    if duration_changed {
        active.duration = Set(fresh.duration);
    }
    if dimension_changed {
        active.width = Set(fresh_width);
        active.height = Set(fresh_height);
    }
    active.update(connection).await.context("更新 page 弹幕同步状态失败")?;
    info!(
        "视频「{}」({}) 分页 pid={} 弹幕已刷新 -> stage={:?}",
        video_model.name, video_model.bvid, fresh.page, resolved_stage
    );
    Ok(())
}

/// 依据 [`Dimension::rotate`] 得到数据库里应保存的 (width, height)。
fn extract_dimension(d: Option<&Dimension>) -> (Option<u32>, Option<u32>) {
    match d {
        Some(d) if d.rotate == 0 => (Some(d.width), Some(d.height)),
        Some(d) => (Some(d.height), Some(d.width)),
        None => (None, None),
    }
}

/// 根据 page_model.path 推断出弹幕 ASS 文件应写入的路径。
///
/// 与 [`crate::workflow::download_page`] 的拼接规则保持一致：
/// - 单页视频： `{base_path}/{base_name}.zh-CN.default.ass`
/// - 多页视频： `{base_path}/Season 1/{base_name} - S01E{pid}.zh-CN.default.ass`
fn resolve_danmaku_path(video_model: &video::Model, page_model: &page::Model) -> Result<PathBuf> {
    let is_single_page = video_model.single_page.context("single_page is null")?;
    let old_video_path = page_model
        .path
        .as_deref()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("page 未记录下载路径，无法推断弹幕位置"))?;
    let old_video_path = Path::new(old_video_path);
    let old_video_filename = old_video_path
        .file_name()
        .context("invalid page path format")?
        .to_string_lossy();
    if is_single_page {
        let base_path = old_video_path.parent().context("invalid page path format")?;
        let base_name = old_video_filename.trim_end_matches(".mp4");
        Ok(base_path.join(format!("{}.zh-CN.default.ass", base_name)))
    } else {
        let base_path = old_video_path
            .parent()
            .and_then(|p| p.parent())
            .context("invalid page path format")?;
        let base_name = old_video_filename
            .rsplit_once(" - ")
            .context("invalid page path format")?
            .0;
        Ok(base_path
            .join("Season 1")
            .join(format!("{} - S01E{:0>2}.zh-CN.default.ass", base_name, page_model.pid)))
    }
}

fn make_tmp_path(target: &Path) -> PathBuf {
    let mut s = target.as_os_str().to_os_string();
    s.push(".tmp");
    PathBuf::from(s)
}

/// 解析数据库中 `danmaku_last_synced_at` 字符串（NaiveDateTime::to_string 的格式，例如 "2026-04-13 10:20:30"）。
fn parse_stored_datetime(s: &str) -> Option<DateTime<Utc>> {
    // NaiveDateTime::to_string() 产出 "YYYY-MM-DD HH:MM:SS[.fraction]"
    chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f")
        .or_else(|_| chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S"))
        .ok()
        .map(|naive| Utc.from_utc_datetime(&naive))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::status::STATUS_OK;

    #[test]
    fn danmaku_completed_detects_ok() {
        let with_danmaku_ok: u32 = STATUS_OK << 9;
        assert!(danmaku_subtask_completed(with_danmaku_ok));
        let without: u32 = STATUS_OK << 6; // 视频信息位，不是弹幕
        assert!(!danmaku_subtask_completed(without));
    }

    #[test]
    fn reset_non_danmaku_subtasks_keeps_only_danmaku_ok() {
        // 五个子任务都 OK + 完成位
        let all_ok_completed: u32 = (1u32 << 31) | (0..5).map(|i| STATUS_OK << (i * 3)).fold(0u32, |a, b| a | b);
        let reset = reset_non_danmaku_subtasks(all_ok_completed);
        // 弹幕位保留
        assert_eq!((reset >> 9) & 0b111, STATUS_OK);
        // 其它四个位都被清零
        for i in [0usize, 1, 2, 4] {
            assert_eq!((reset >> (i * 3)) & 0b111, 0);
        }
        // 完成位被清掉
        assert_eq!(reset >> 31, 0);
    }

    #[test]
    fn reset_video_for_page_redownload_clears_subtask_4_and_completed_bit() {
        // 五个子任务都 OK + 完成位
        let video_done: u32 = (1u32 << 31) | (0..5).map(|i| STATUS_OK << (i * 3)).fold(0u32, |a, b| a | b);
        let reset = reset_video_for_page_redownload(video_done);
        // offset 4（分页下载子任务）被清零
        assert_eq!((reset >> 12) & 0b111, 0);
        // 其它子任务保留
        for i in [0usize, 1, 2, 3] {
            assert_eq!((reset >> (i * 3)) & 0b111, STATUS_OK);
        }
        // 完成位被清掉
        assert_eq!(reset >> 31, 0);
    }

    #[test]
    fn parse_stored_datetime_roundtrip() {
        let now = chrono::Utc.with_ymd_and_hms(2026, 4, 13, 10, 20, 30).unwrap();
        let s = now.naive_utc().to_string();
        let parsed = parse_stored_datetime(&s).expect("parse ok");
        assert_eq!(parsed, now);
    }
}
