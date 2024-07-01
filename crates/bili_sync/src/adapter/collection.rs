use std::collections::HashSet;
use std::path::Path;

use anyhow::Result;
use bili_sync_entity::*;
use filenamify::filenamify;
use sea_orm::entity::prelude::*;
use sea_orm::ActiveValue::Set;
use sea_orm::{DatabaseConnection, QuerySelect, TransactionTrait};

use super::VideoListModel;
use crate::bilibili::{BiliClient, BiliError, CollectionType, Video, VideoInfo};
use crate::core::status::Status;
use crate::core::utils::{create_video_pages, id_time_key, TEMPLATE};

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
            .map(|v| {
                let mut video_model = video::ActiveModel {
                    collection_id: Set(Some(self.id)),
                    ..v.to_model()
                };
                if let Some(fmt_args) = &v.to_fmt_args() {
                    video_model.path = Set(Path::new(&self.path)
                        .join(filenamify(
                            TEMPLATE
                                .render("video", fmt_args)
                                .unwrap_or_else(|_| v.bvid().to_string()),
                        ))
                        .to_string_lossy()
                        .to_string());
                }
                video_model
            })
            .collect())
    }

    async fn fetch_videos_detail(
        &self,
        bili_clent: &BiliClient,
        videos_model: Vec<video::Model>,
        connection: &DatabaseConnection,
    ) -> Result<()> {
        for video_model in videos_model {
            let video = Video::new(bili_clent, video_model.bvid.clone());
            let info: Result<_> = async { Ok((video.get_tags().await?, video.get_view_info().await?)) }.await;
            match info {
                Ok((tags, view_info)) => {
                    let VideoInfo::View { pages, .. } = &view_info else {
                        unreachable!("view_info must be VideoInfo::View")
                    };
                    let txn = connection.begin().await?;
                    // 将分页信息写入数据库
                    create_video_pages(pages, &video_model, &txn).await?;
                    // 将页标记和 tag 写入数据库
                    let mut video_active_model = view_info.to_model();
                    video_active_model.single_page = Set(Some(pages.len() == 1));
                    video_active_model.tags = Set(Some(serde_json::to_value(tags).unwrap()));
                    video_active_model.save(&txn).await?;
                    txn.commit().await?;
                }
                Err(e) => {
                    error!(
                        "获取视频 {} - {} 的详细信息失败，错误为：{}",
                        &video_model.bvid, &video_model.name, e
                    );
                    if let Some(BiliError::RequestFailed(-404, _)) = e.downcast_ref::<BiliError>() {
                        let mut video_active_model: video::ActiveModel = video_model.into();
                        video_active_model.valid = Set(false);
                        video_active_model.save(connection).await?;
                    }
                    continue;
                }
            };
        }
        Ok(())
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
