use anyhow::Result;
use bili_sync_entity::*;
use sea_orm::entity::prelude::*;
use sea_orm::DatabaseConnection;

use super::VideoListModel;
use crate::core::status::Status;

impl VideoListModel for favorite::Model {
    async fn unfilled_videos(&self, connection: &DatabaseConnection) -> Result<Vec<video::Model>> {
        Ok(video::Entity::find()
            .filter(
                video::Column::FavoriteId
                    .eq(self.id)
                    .and(video::Column::Valid.eq(true))
                    .and(video::Column::DownloadStatus.eq(0))
                    .and(video::Column::Category.eq(2))
                    .and(video::Column::SinglePage.is_null()),
            )
            .all(connection)
            .await?)
    }

    async fn unhandled_video_pages(
        &self,
        connection: &DatabaseConnection,
    ) -> Result<Vec<(video::Model, Vec<page::Model>)>> {
        Ok(video::Entity::find()
            .filter(
                video::Column::FavoriteId
                    .eq(self.id)
                    .and(video::Column::Valid.eq(true))
                    .and(video::Column::DownloadStatus.lt(Status::handled()))
                    .and(video::Column::Category.eq(2))
                    .and(video::Column::SinglePage.is_not_null()),
            )
            .find_with_related(page::Entity)
            .all(connection)
            .await?)
    }

    fn log_fetch_video_start(&self) {
        info!("开始获取收藏夹 {} - {} 的视频与分页信息...", self.f_id, self.name);
    }

    fn log_fetch_video_end(&self) {
        info!("获取收藏夹 {} - {} 的视频与分页信息完成", self.f_id, self.name);
    }

    fn log_download_video_start(&self) {
        info!("开始下载收藏夹: {} - {} 中所有未处理过的视频...", self.f_id, self.name);
    }

    fn log_download_video_end(&self) {
        info!("下载收藏夹: {} - {} 中未处理过的视频完成", self.f_id, self.name);
    }
}
