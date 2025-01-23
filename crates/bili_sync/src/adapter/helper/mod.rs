use std::path::Path;

use anyhow::Result;
use bili_sync_entity::*;
use sea_orm::entity::prelude::*;
use sea_orm::sea_query::OnConflict;
use sea_orm::ActiveValue::Set;
use sea_orm::DatabaseTransaction;

use crate::bilibili::{BiliError, PageInfo, VideoInfo};
use crate::config::{PathSafeTemplate, TEMPLATE};

/// 返回设置了 path 的视频
pub(super) fn video_with_path(
    mut video_model: video::ActiveModel,
    base_path: &str,
    video_info: &VideoInfo,
) -> video::ActiveModel {
    if let Some(fmt_args) = &video_info.to_fmt_args() {
        video_model.path = Set(Path::new(base_path)
            .join(
                TEMPLATE
                    .path_safe_render("video", fmt_args)
                    .expect("template render failed"),
            )
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
    connection: &DatabaseTransaction,
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
    for page_chunk in page_models.chunks(50) {
        page::Entity::insert_many(page_chunk.to_vec())
            .on_conflict(
                OnConflict::columns([page::Column::VideoId, page::Column::Pid])
                    .do_nothing()
                    .to_owned(),
            )
            .do_nothing()
            .exec(connection)
            .await?;
    }
    Ok(())
}
