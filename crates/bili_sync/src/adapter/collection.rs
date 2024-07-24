use std::collections::HashSet;
use std::path::Path;
use std::pin::Pin;

use anyhow::Result;
use async_trait::async_trait;
use bili_sync_entity::*;
use futures::Stream;
use sea_orm::entity::prelude::*;
use sea_orm::sea_query::{IntoCondition, OnConflict};
use sea_orm::ActiveValue::Set;
use sea_orm::{DatabaseConnection, TransactionTrait};

use crate::adapter::{helper, VideoListModel};
use crate::bilibili::{self, BiliClient, Collection, CollectionItem, CollectionType, VideoInfo};
use crate::utils::status::Status;

#[async_trait]
impl VideoListModel for collection::Model {
    async fn video_count(&self, connection: &DatabaseConnection) -> Result<u64> {
        helper::count_videos(video::Column::CollectionId.eq(self.id).into_condition(), connection).await
    }

    async fn unfilled_videos(&self, connection: &DatabaseConnection) -> Result<Vec<video::Model>> {
        helper::filter_videos(
            video::Column::CollectionId
                .eq(self.id)
                .and(video::Column::Valid.eq(true))
                .and(video::Column::DownloadStatus.eq(0))
                .and(video::Column::Category.eq(2))
                .and(video::Column::SinglePage.is_null())
                .into_condition(),
            connection,
        )
        .await
    }

    async fn unhandled_video_pages(
        &self,
        connection: &DatabaseConnection,
    ) -> Result<Vec<(video::Model, Vec<page::Model>)>> {
        helper::filter_videos_with_pages(
            video::Column::CollectionId
                .eq(self.id)
                .and(video::Column::Valid.eq(true))
                .and(video::Column::DownloadStatus.lt(Status::handled()))
                .and(video::Column::Category.eq(2))
                .and(video::Column::SinglePage.is_not_null())
                .into_condition(),
            connection,
        )
        .await
    }

    async fn exist_labels(
        &self,
        videos_info: &[VideoInfo],
        connection: &DatabaseConnection,
    ) -> Result<HashSet<String>> {
        helper::video_keys(videos_info, [video::Column::Bvid, video::Column::Pubtime], connection).await
    }

    fn video_model_by_info(&self, video_info: &VideoInfo, base_model: Option<video::Model>) -> video::ActiveModel {
        let mut video_model = video_info.to_model(base_model);
        video_model.collection_id = Set(Some(self.id));
        helper::video_with_path(video_model, &self.path, video_info)
    }

    async fn fetch_videos_detail(
        &self,
        video: bilibili::Video<'_>,
        video_model: video::Model,
        connection: &DatabaseConnection,
    ) -> Result<()> {
        let info: Result<_> = async { Ok((video.get_tags().await?, video.get_view_info().await?)) }.await;
        match info {
            Ok((tags, view_info)) => {
                let VideoInfo::View { pages, .. } = &view_info else {
                    unreachable!("view_info must be VideoInfo::View")
                };
                let txn = connection.begin().await?;
                // 将分页信息写入数据库
                helper::create_video_pages(pages, &video_model, &txn).await?;
                // 将页标记和 tag 写入数据库
                let mut video_active_model = self.video_model_by_info(&view_info, Some(video_model));
                video_active_model.single_page = Set(Some(pages.len() == 1));
                video_active_model.tags = Set(Some(serde_json::to_value(tags).unwrap()));
                video_active_model.save(&txn).await?;
                txn.commit().await?;
            }
            Err(e) => {
                helper::error_fetch_video_detail(e, video_model, connection).await?;
            }
        };
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

    fn log_refresh_video_start(&self) {
        info!(
            "开始扫描{}: {} - {} 的新视频...",
            CollectionType::from(self.r#type),
            self.s_id,
            self.name
        );
    }

    fn log_refresh_video_end(&self, got_count: usize, new_count: u64) {
        info!(
            "扫描{}: {} - {} 的新视频完成，获取了 {} 条新视频，其中有 {} 条新视频",
            CollectionType::from(self.r#type),
            self.s_id,
            self.name,
            got_count,
            new_count,
        );
    }
}

pub(super) async fn collection_from<'a>(
    collection_item: &'a CollectionItem,
    path: &Path,
    bili_client: &'a BiliClient,
    connection: &DatabaseConnection,
) -> Result<(Box<dyn VideoListModel>, Pin<Box<dyn Stream<Item = VideoInfo> + 'a>>)> {
    let collection = Collection::new(bili_client, collection_item);
    let collection_info = collection.get_info().await?;
    collection::Entity::insert(collection::ActiveModel {
        s_id: Set(collection_info.sid),
        m_id: Set(collection_info.mid),
        r#type: Set(collection_info.collection_type.into()),
        name: Set(collection_info.name.clone()),
        path: Set(path.to_string_lossy().to_string()),
        ..Default::default()
    })
    .on_conflict(
        OnConflict::columns([
            collection::Column::SId,
            collection::Column::MId,
            collection::Column::Type,
        ])
        .update_columns([collection::Column::Name, collection::Column::Path])
        .to_owned(),
    )
    .exec(connection)
    .await?;
    Ok((
        Box::new(
            collection::Entity::find()
                .filter(
                    collection::Column::SId
                        .eq(collection_item.sid.clone())
                        .and(collection::Column::MId.eq(collection_item.mid.clone()))
                        .and(collection::Column::Type.eq(Into::<i32>::into(collection_item.collection_type.clone()))),
                )
                .one(connection)
                .await?
                .unwrap(),
        ),
        Box::pin(collection.into_simple_video_stream()),
    ))
}
