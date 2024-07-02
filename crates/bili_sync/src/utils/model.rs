use anyhow::Result;
use bili_sync_entity::*;
use bili_sync_migration::OnConflict;
use sea_orm::entity::prelude::*;
use sea_orm::ActiveValue::Set;

use crate::adapter::{unique_video_columns, VideoListModel};
use crate::bilibili::{PageInfo, VideoInfo};

/// 尝试创建 Video Model，如果发生冲突则忽略
pub async fn create_videos(
    videos_info: &[VideoInfo],
    video_list_model: &dyn VideoListModel,
    connection: &DatabaseConnection,
) -> Result<()> {
    let video_models = videos_info
        .iter()
        .map(|v| video_list_model.video_model_by_info(v))
        .collect::<Vec<_>>();
    video::Entity::insert_many(video_models)
        .on_conflict(OnConflict::columns(unique_video_columns()).do_nothing().to_owned())
        .do_nothing()
        .exec(connection)
        .await?;
    Ok(())
}

/// 创建视频的所有分 P
pub async fn create_video_pages(
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

/// 更新视频 model 的下载状态
pub async fn update_videos_model(videos: Vec<video::ActiveModel>, connection: &DatabaseConnection) -> Result<()> {
    video::Entity::insert_many(videos)
        .on_conflict(
            OnConflict::column(video::Column::Id)
                .update_column(video::Column::DownloadStatus)
                .to_owned(),
        )
        .exec(connection)
        .await?;
    Ok(())
}

/// 更新视频页 model 的下载状态
pub async fn update_pages_model(pages: Vec<page::ActiveModel>, connection: &DatabaseConnection) -> Result<()> {
    let query = page::Entity::insert_many(pages).on_conflict(
        OnConflict::column(page::Column::Id)
            .update_columns([page::Column::DownloadStatus, page::Column::Path])
            .to_owned(),
    );
    query.exec(connection).await?;
    Ok(())
}
