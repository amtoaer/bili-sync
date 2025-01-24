use anyhow::{Context, Result};
use bili_sync_entity::*;
use sea_orm::entity::prelude::*;
use sea_orm::sea_query::{OnConflict, SimpleExpr};
use sea_orm::DatabaseTransaction;

use crate::adapter::{VideoListModel, VideoListModelEnum};
use crate::bilibili::{PageInfo, VideoInfo};
use crate::utils::status::STATUS_COMPLETED;

/// 筛选未填充的视频
pub async fn filter_unfilled_videos(
    additional_expr: SimpleExpr,
    conn: &DatabaseConnection,
) -> Result<Vec<video::Model>> {
    video::Entity::find()
        .filter(
            video::Column::Valid
                .eq(true)
                .and(video::Column::DownloadStatus.eq(0))
                .and(video::Column::Category.eq(2))
                .and(video::Column::SinglePage.is_null())
                .and(additional_expr),
        )
        .all(conn)
        .await
        .context("filter unfilled videos failed")
}

/// 筛选未处理完成的视频和视频页
pub async fn filter_unhandled_video_pages(
    additional_expr: SimpleExpr,
    connection: &DatabaseConnection,
) -> Result<Vec<(video::Model, Vec<page::Model>)>> {
    video::Entity::find()
        .filter(
            video::Column::Valid
                .eq(true)
                .and(video::Column::DownloadStatus.lt(STATUS_COMPLETED))
                .and(video::Column::Category.eq(2))
                .and(video::Column::SinglePage.is_not_null())
                .and(additional_expr),
        )
        .find_with_related(page::Entity)
        .all(connection)
        .await
        .context("filter unhandled video pages failed")
}

/// 尝试创建 Video Model，如果发生冲突则忽略
pub async fn create_videos(
    videos_info: Vec<VideoInfo>,
    video_list_model: &VideoListModelEnum,
    connection: &DatabaseConnection,
) -> Result<()> {
    let video_models = videos_info
        .into_iter()
        .map(|v| {
            let mut model = v.into_simple_model();
            video_list_model.set_relation_id(&mut model);
            model
        })
        .collect::<Vec<_>>();
    video::Entity::insert_many(video_models)
        // 这里想表达的是 on 索引名，但 sea-orm 的 api 似乎只支持列名而不支持索引名，好在留空可以达到相同的目的
        .on_conflict(OnConflict::new().do_nothing().to_owned())
        .do_nothing()
        .exec(connection)
        .await?;
    Ok(())
}

/// 尝试创建 Page Model，如果发生冲突则忽略
pub async fn create_pages(
    pages_info: Vec<PageInfo>,
    video_model: &bili_sync_entity::video::Model,
    connection: &DatabaseTransaction,
) -> Result<()> {
    let page_models = pages_info
        .into_iter()
        .map(|p| p.into_active_model(video_model))
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
