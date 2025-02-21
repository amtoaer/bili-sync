use std::path::Path;
use std::pin::Pin;

use anyhow::{Context, Result};
use bili_sync_entity::*;
use futures::Stream;
use sea_orm::ActiveValue::Set;
use sea_orm::entity::prelude::*;
use sea_orm::sea_query::{OnConflict, SimpleExpr};
use sea_orm::{DatabaseConnection, Unchanged};

use crate::adapter::{_ActiveModel, VideoSource, VideoSourceEnum};
use crate::bilibili::{BiliClient, VideoInfo, WatchLater};

impl VideoSource for watch_later::Model {
    fn filter_expr(&self) -> SimpleExpr {
        video::Column::WatchLaterId.eq(self.id)
    }

    fn set_relation_id(&self, video_model: &mut video::ActiveModel) {
        video_model.watch_later_id = Set(Some(self.id));
    }

    fn path(&self) -> &Path {
        Path::new(self.path.as_str())
    }

    fn get_latest_row_at(&self) -> DateTime {
        self.latest_row_at
    }

    fn update_latest_row_at(&self, datetime: DateTime) -> _ActiveModel {
        _ActiveModel::WatchLater(watch_later::ActiveModel {
            id: Unchanged(self.id),
            latest_row_at: Set(datetime),
            ..Default::default()
        })
    }

    fn log_refresh_video_start(&self) {
        info!("开始扫描稍后再看..");
    }

    fn log_refresh_video_end(&self, count: usize) {
        info!("扫描稍后再看完成，获取到 {} 条新视频", count);
    }

    fn log_fetch_video_start(&self) {
        info!("开始填充稍后再看视频详情..");
    }

    fn log_fetch_video_end(&self) {
        info!("填充稍后再看视频详情完成");
    }

    fn log_download_video_start(&self) {
        info!("开始下载稍后再看视频..");
    }

    fn log_download_video_end(&self) {
        info!("下载稍后再看视频完成");
    }
}

pub(super) async fn watch_later_from<'a>(
    path: &Path,
    bili_client: &'a BiliClient,
    connection: &DatabaseConnection,
) -> Result<(
    VideoSourceEnum,
    Pin<Box<dyn Stream<Item = Result<VideoInfo>> + 'a + Send>>,
)> {
    let watch_later = WatchLater::new(bili_client);
    watch_later::Entity::insert(watch_later::ActiveModel {
        id: Set(1),
        path: Set(path.to_string_lossy().to_string()),
        ..Default::default()
    })
    .on_conflict(
        OnConflict::column(watch_later::Column::Id)
            .update_column(watch_later::Column::Path)
            .to_owned(),
    )
    .exec(connection)
    .await?;
    Ok((
        watch_later::Entity::find()
            .filter(watch_later::Column::Id.eq(1))
            .one(connection)
            .await?
            .context("watch_later not found")?
            .into(),
        Box::pin(watch_later.into_video_stream()),
    ))
}
