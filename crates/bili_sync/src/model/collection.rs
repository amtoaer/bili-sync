use std::collections::HashSet;

use anyhow::Result;
use bili_sync_entity::*;
use sea_orm::entity::prelude::*;
use sea_orm::ActiveValue::Set;
use sea_orm::{DatabaseConnection, QuerySelect};

use super::VideoListModel;
use crate::bilibili::{CollectionType, VideoInfo};
use crate::core::status::Status;
use crate::core::utils::id_time_key;

impl VideoListModel for collection::Model {
    async fn unfilled_videos(&self, connection: &DatabaseConnection) -> Result<Vec<video::Model>> {
        Ok(video::Entity::find()
            .filter(
                video::Column::CollectionId
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
                video::Column::CollectionId
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

    async fn exist_labels(
        &self,
        videos_info: &[VideoInfo],
        connection: &DatabaseConnection,
    ) -> Result<HashSet<String>> {
        let bvids = videos_info.iter().map(|v| v.bvid().to_string()).collect::<Vec<_>>();
        Ok(video::Entity::find()
            .filter(
                video::Column::CollectionId
                    .eq(self.id)
                    .and(video::Column::Bvid.is_in(bvids)),
            )
            .select_only()
            .columns([video::Column::Bvid, video::Column::Favtime])
            .into_tuple()
            .all(connection)
            .await?
            .into_iter()
            .map(|(bvid, time)| id_time_key(&bvid, &time))
            .collect::<HashSet<_>>())
    }

    fn video_models_by_info(&self, videos_info: &[VideoInfo]) -> Result<Vec<video::ActiveModel>> {
        Ok(videos_info
            .iter()
            .map(|v| video::ActiveModel {
                collection_id: Set(Some(self.id)),
                ..v.to_model()
            })
            .collect())
    }

    fn log_fetch_video_start(&self) {
        info!(
            "开始获取{} {} - {} 的视频与分页信息...",
            CollectionType::from(self.r#type),
            self.s_id,
            self.name
        );
    }

    fn log_fetch_video_end(&self) {
        info!(
            "获取{} {} - {} 的视频与分页信息完成",
            CollectionType::from(self.r#type),
            self.s_id,
            self.name
        );
    }

    fn log_download_video_start(&self) {
        info!(
            "开始下载{}: {} - {} 中所有未处理过的视频...",
            CollectionType::from(self.r#type),
            self.s_id,
            self.name
        );
    }

    fn log_download_video_end(&self) {
        info!(
            "下载{}: {} - {} 中未处理过的视频完成",
            CollectionType::from(self.r#type),
            self.s_id,
            self.name
        );
    }
}
