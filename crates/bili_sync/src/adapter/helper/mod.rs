use std::collections::HashSet;
use std::path::Path;

use anyhow::Result;
use bili_sync_entity::*;
use sea_orm::entity::prelude::*;
use sea_orm::sea_query::{OnConflict, SimpleExpr};
use sea_orm::ActiveValue::Set;
use sea_orm::{Condition, QuerySelect};

use crate::bilibili::{BiliError, PageInfo, VideoInfo};
use crate::config::TEMPLATE;
use crate::utils::filenamify::filenamify;
use crate::utils::id_time_key;

/// 使用 condition 筛选视频，返回视频数量
pub(super) async fn count_videos(condition: Condition, conn: &DatabaseConnection) -> Result<u64> {
    Ok(video::Entity::find().filter(condition).count(conn).await?)
}

/// 使用 condition 筛选视频，返回视频列表
pub(super) async fn filter_videos(condition: Condition, conn: &DatabaseConnection) -> Result<Vec<video::Model>> {
    Ok(video::Entity::find().filter(condition).all(conn).await?)
}

/// 使用 condition 筛选视频，返回视频列表和相关的分 P 列表
pub(super) async fn filter_videos_with_pages(
    condition: Condition,
    conn: &DatabaseConnection,
) -> Result<Vec<(video::Model, Vec<page::Model>)>> {
    Ok(video::Entity::find()
        .filter(condition)
        .find_with_related(page::Entity)
        .all(conn)
        .await?)
}

/// 返回 videos_info 存在于视频表里那部分对应的 key
pub(super) async fn video_keys(
    expr: SimpleExpr,
    videos_info: &[VideoInfo],
    columns: [video::Column; 2],
    conn: &DatabaseConnection,
) -> Result<HashSet<String>> {
    Ok(video::Entity::find()
        .filter(
            video::Column::Bvid
                .is_in(videos_info.iter().map(|v| v.bvid().to_string()))
                .and(expr),
        )
        .select_only()
        .columns(columns)
        .into_tuple()
        .all(conn)
        .await?
        .into_iter()
        .map(|(bvid, time)| id_time_key(&bvid, &time))
        .collect())
}

/// 返回设置了 path 的视频
pub(super) fn video_with_path(
    mut video_model: video::ActiveModel,
    base_path: &str,
    video_info: &VideoInfo,
) -> video::ActiveModel {
    if let Some(fmt_args) = &video_info.to_fmt_args() {
        video_model.path = Set(Path::new(base_path)
            .join(filenamify(
                TEMPLATE
                    .render("video", fmt_args)
                    .unwrap_or_else(|_| video_info.bvid().to_string()),
            ))
            .to_string_lossy()
            .to_string());
    }
    video_model
}

/// 处理获取视频详细信息失败的情况
pub(super) async fn error_fetch_video_detail(
    e: anyhow::Error,
    video_model: bili_sync_entity::video::Model,
    connection: &DatabaseConnection,
) -> Result<()> {
    error!(
        "获取视频 {} - {} 的详细信息失败，错误为：{}",
        &video_model.bvid, &video_model.name, e
    );
    if let Some(BiliError::RequestFailed(-404, _)) = e.downcast_ref::<BiliError>() {
        let mut video_active_model: bili_sync_entity::video::ActiveModel = video_model.into();
        video_active_model.valid = Set(false);
        video_active_model.save(connection).await?;
    }
    Ok(())
}

/// 创建视频的所有分 P
pub(crate) async fn create_video_pages(
    pages_info: &[PageInfo],
    video_model: &video::Model,
    connection: &impl ConnectionTrait,
) -> Result<()> {
    let page_models = pages_info
        .iter()
        .map(move |p| {
            let (width, height) = match &p.dimension {
                Some(d) => {
                    if d.rotate == 0 {
                        (Some(d.width), Some(d.height))
                    } else {
                        (Some(d.height), Some(d.width))
                    }
                }
                None => (None, None),
            };
            page::ActiveModel {
                video_id: Set(video_model.id),
                cid: Set(p.cid),
                pid: Set(p.page),
                name: Set(p.name.clone()),
                width: Set(width),
                height: Set(height),
                duration: Set(p.duration),
                image: Set(p.first_frame.clone()),
                download_status: Set(0),
                ..Default::default()
            }
        })
        .collect::<Vec<page::ActiveModel>>();
    page::Entity::insert_many(page_models)
        .on_conflict(
            OnConflict::columns([page::Column::VideoId, page::Column::Pid])
                .do_nothing()
                .to_owned(),
        )
        .do_nothing()
        .exec(connection)
        .await?;
    Ok(())
}
