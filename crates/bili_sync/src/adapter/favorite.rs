use std::path::Path;
use std::pin::Pin;

use anyhow::{Context, Result};
use async_trait::async_trait;
use bili_sync_entity::*;
use futures::Stream;
use sea_orm::entity::prelude::*;
use sea_orm::sea_query::{IntoCondition, OnConflict};
use sea_orm::ActiveValue::Set;
use sea_orm::{DatabaseConnection, TransactionTrait, Unchanged};

use crate::adapter::{helper, VideoListModel};
use crate::bilibili::{self, BiliClient, FavoriteList, VideoInfo};
use crate::utils::status::STATUS_COMPLETED;

#[async_trait]
impl VideoListModel for favorite::Model {
    async fn unfilled_videos(&self, connection: &DatabaseConnection) -> Result<Vec<video::Model>> {
        helper::filter_videos(
            video::Column::FavoriteId
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
            video::Column::FavoriteId
                .eq(self.id)
                .and(video::Column::Valid.eq(true))
                .and(video::Column::DownloadStatus.lt(STATUS_COMPLETED))
                .and(video::Column::Category.eq(2))
                .and(video::Column::SinglePage.is_not_null())
                .into_condition(),
            connection,
        )
        .await
    }

    fn video_model_by_info(&self, video_info: &VideoInfo, base_model: Option<video::Model>) -> video::ActiveModel {
        let mut video_model = video_info.to_model(base_model);
        video_model.favorite_id = Set(Some(self.id));
        helper::video_with_path(video_model, &self.path, video_info)
    }

    async fn fetch_videos_detail(
        &self,
        video: bilibili::Video<'_>,
        video_model: video::Model,
        connection: &DatabaseConnection,
    ) -> Result<()> {
        let info: Result<_> = async { Ok((video.get_tags().await?, video.get_pages().await?)) }.await;
        match info {
            Ok((tags, pages_info)) => {
                let txn = connection.begin().await?;
                // 将分页信息写入数据库
                helper::create_video_pages(&pages_info, &video_model, &txn).await?;
                // 将页标记和 tag 写入数据库
                let mut video_active_model: video::ActiveModel = video_model.into();
                video_active_model.single_page = Set(Some(pages_info.len() == 1));
                video_active_model.tags = Set(Some(serde_json::to_value(tags)?));
                video_active_model.save(&txn).await?;
                txn.commit().await?;
            }
            Err(e) => {
                helper::error_fetch_video_detail(e, video_model, connection).await?;
            }
        };
        Ok(())
    }

    fn get_latest_row_at(&self) -> DateTime {
        self.latest_row_at
    }

    async fn update_latest_row_at(&self, datetime: DateTime, connection: &DatabaseConnection) -> Result<()> {
        favorite::ActiveModel {
            id: Unchanged(self.id),
            latest_row_at: Set(datetime),
            ..Default::default()
        }
        .update(connection)
        .await?;
        Ok(())
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

    fn log_refresh_video_start(&self) {
        info!("开始扫描收藏夹: {} - {} 的新视频...", self.f_id, self.name);
    }

    fn log_refresh_video_end(&self, count: usize) {
        info!(
            "扫描收藏夹: {} - {} 的新视频完成，获取了 {} 条新视频",
            self.f_id, self.name, count
        );
    }
}

pub(super) async fn favorite_from<'a>(
    fid: &str,
    path: &Path,
    bili_client: &'a BiliClient,
    connection: &DatabaseConnection,
) -> Result<(Box<dyn VideoListModel>, Pin<Box<dyn Stream<Item = VideoInfo> + 'a>>)> {
    let favorite = FavoriteList::new(bili_client, fid.to_owned());
    let favorite_info = favorite.get_info().await?;
    favorite::Entity::insert(favorite::ActiveModel {
        f_id: Set(favorite_info.id),
        name: Set(favorite_info.title.clone()),
        path: Set(path.to_string_lossy().to_string()),
        ..Default::default()
    })
    .on_conflict(
        OnConflict::column(favorite::Column::FId)
            .update_columns([favorite::Column::Name, favorite::Column::Path])
            .to_owned(),
    )
    .exec(connection)
    .await?;
    Ok((
        Box::new(
            favorite::Entity::find()
                .filter(favorite::Column::FId.eq(favorite_info.id))
                .one(connection)
                .await?
                .context("favorite not found")?,
        ),
        Box::pin(favorite.into_video_stream()),
    ))
}
