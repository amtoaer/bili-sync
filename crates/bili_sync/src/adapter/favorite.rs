use std::path::Path;
use std::pin::Pin;

use anyhow::{Context, Result};
use bili_sync_entity::*;
use futures::Stream;
use sea_orm::entity::prelude::*;
use sea_orm::sea_query::{OnConflict, SimpleExpr};
use sea_orm::ActiveValue::Set;
use sea_orm::{DatabaseConnection, Unchanged};

use crate::adapter::{VideoListModel, VideoListModelEnum, _ActiveModel};
use crate::bilibili::{BiliClient, FavoriteList, VideoInfo};

impl VideoListModel for favorite::Model {
    fn filter_expr(&self) -> SimpleExpr {
        video::Column::FavoriteId.eq(self.id)
    }

    fn set_relation_id(&self, video_model: &mut video::ActiveModel) {
        video_model.favorite_id = Set(Some(self.id));
    }

    fn path(&self) -> &Path {
        Path::new(self.path.as_str())
    }

    fn get_latest_row_at(&self) -> DateTime {
        self.latest_row_at
    }

    fn update_latest_row_at(&self, datetime: DateTime) -> _ActiveModel {
        _ActiveModel::Favorite(favorite::ActiveModel {
            id: Unchanged(self.id),
            latest_row_at: Set(datetime),
            ..Default::default()
        })
    }

    fn log_refresh_video_start(&self) {
        info!("开始扫描收藏夹「{}」..", self.name);
    }

    fn log_refresh_video_end(&self, count: usize) {
        info!("扫描收藏夹「{}」完成，获取到 {} 条新视频", self.name, count);
    }

    fn log_fetch_video_start(&self) {
        info!("开始填充收藏夹「{}」视频详情..", self.name);
    }

    fn log_fetch_video_end(&self) {
        info!("填充收藏夹「{}」视频详情完成", self.name);
    }

    fn log_download_video_start(&self) {
        info!("开始下载收藏夹「{}」视频..", self.name);
    }

    fn log_download_video_end(&self) {
        info!("下载收藏夹「{}」视频完成", self.name);
    }
}

pub(super) async fn favorite_from<'a>(
    fid: &str,
    path: &Path,
    bili_client: &'a BiliClient,
    connection: &DatabaseConnection,
) -> Result<(VideoListModelEnum, Pin<Box<dyn Stream<Item = Result<VideoInfo>> + 'a>>)> {
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
        favorite::Entity::find()
            .filter(favorite::Column::FId.eq(favorite_info.id))
            .one(connection)
            .await?
            .context("favorite not found")?
            .into(),
        Box::pin(favorite.into_video_stream()),
    ))
}
