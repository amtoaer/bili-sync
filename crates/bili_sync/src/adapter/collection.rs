use std::path::{Path, PathBuf};
use std::pin::Pin;

use anyhow::{Context, Result};
use async_trait::async_trait;
use bili_sync_entity::*;
use futures::Stream;
use sea_orm::entity::prelude::*;
use sea_orm::sea_query::{OnConflict, SimpleExpr};
use sea_orm::ActiveValue::Set;
use sea_orm::{DatabaseConnection, TransactionTrait, Unchanged};

use crate::adapter::{helper, VideoListModel};
use crate::bilibili::{self, BiliClient, Collection, CollectionItem, CollectionType, VideoInfo};

#[async_trait]
impl VideoListModel for collection::Model {
    fn filter_expr(&self) -> SimpleExpr {
        video::Column::CollectionId.eq(self.id)
    }

    fn set_relation_id(&self, video_model: &mut video::ActiveModel) {
        video_model.collection_id = Set(Some(self.id));
    }

    fn path(&self) -> &Path {
        Path::new(self.path.as_str())
    }

    fn video_model_by_info(&self, video_info: &VideoInfo, base_model: Option<video::Model>) -> video::ActiveModel {
        let mut video_model = video_info.to_model(base_model);
        video_model.collection_id = Set(Some(self.id));
        helper::video_with_path(video_model, &self.path, video_info)
    }

    fn get_latest_row_at(&self) -> DateTime {
        self.latest_row_at
    }

    async fn update_latest_row_at(&self, datetime: DateTime, connection: &DatabaseConnection) -> Result<()> {
        collection::ActiveModel {
            id: Unchanged(self.id),
            latest_row_at: Set(datetime),
            ..Default::default()
        }
        .update(connection)
        .await?;
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

    fn log_refresh_video_end(&self, count: usize) {
        info!(
            "扫描{}: {} - {} 的新视频完成，获取了 {} 条新视频",
            CollectionType::from(self.r#type),
            self.s_id,
            self.name,
            count,
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
                .context("collection not found")?,
        ),
        Box::pin(collection.into_simple_video_stream()),
    ))
}
