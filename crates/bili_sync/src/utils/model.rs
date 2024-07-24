use anyhow::Result;
use bili_sync_entity::*;
use sea_orm::entity::prelude::*;
use sea_orm::sea_query::OnConflict;

use crate::adapter::VideoListModel;
use crate::bilibili::VideoInfo;

/// 尝试创建 Video Model，如果发生冲突则忽略
pub async fn create_videos(
    videos_info: &[VideoInfo],
    video_list_model: &dyn VideoListModel,
    connection: &DatabaseConnection,
) -> Result<()> {
    let video_models = videos_info
        .iter()
        .map(|v| video_list_model.video_model_by_info(v, None))
        .collect::<Vec<_>>();
    video::Entity::insert_many(video_models)
        // 这里想表达的是 on 索引名，但 sea-orm 的 api 似乎只支持列名而不支持索引名，好在留空可以达到相同的目的
        .on_conflict(OnConflict::new().do_nothing().to_owned())
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
