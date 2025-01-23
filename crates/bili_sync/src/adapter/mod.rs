mod collection;
mod favorite;
mod helper;
mod submission;
mod watch_later;

use std::path::Path;
use std::pin::Pin;

use anyhow::Result;
use async_trait::async_trait;
use futures::Stream;
use sea_orm::entity::prelude::*;
use sea_orm::sea_query::SimpleExpr;
use sea_orm::DatabaseConnection;

use crate::adapter::collection::collection_from;
use crate::adapter::favorite::favorite_from;
use crate::adapter::submission::submission_from;
use crate::adapter::watch_later::watch_later_from;
use crate::bilibili::{self, BiliClient, CollectionItem, VideoInfo};

pub enum Args<'a> {
    Favorite { fid: &'a str },
    Collection { collection_item: &'a CollectionItem },
    WatchLater,
    Submission { upper_id: &'a str },
}

pub async fn video_list_from<'a>(
    args: Args<'a>,
    path: &Path,
    bili_client: &'a BiliClient,
    connection: &DatabaseConnection,
) -> Result<(Box<dyn VideoListModel>, Pin<Box<dyn Stream<Item = VideoInfo> + 'a>>)> {
    match args {
        Args::Favorite { fid } => favorite_from(fid, path, bili_client, connection).await,
        Args::Collection { collection_item } => collection_from(collection_item, path, bili_client, connection).await,
        Args::WatchLater => watch_later_from(path, bili_client, connection).await,
        Args::Submission { upper_id } => submission_from(upper_id, path, bili_client, connection).await,
    }
}

#[async_trait]
pub trait VideoListModel {
    fn filter_expr(&self) -> SimpleExpr;

    fn set_relation_id(&self, video_model: &mut bili_sync_entity::video::ActiveModel);

    fn path(&self) -> &Path;

    /// 视频信息对应的视频 model
    fn video_model_by_info(
        &self,
        video_info: &VideoInfo,
        base_model: Option<bili_sync_entity::video::Model>,
    ) -> bili_sync_entity::video::ActiveModel;

    /// 获取视频 model 中记录的最新时间
    fn get_latest_row_at(&self) -> DateTime;

    /// 更新视频 model 中记录的最新时间
    async fn update_latest_row_at(&self, datetime: DateTime, connection: &DatabaseConnection) -> Result<()>;

    /// 开始获取视频
    fn log_fetch_video_start(&self);

    /// 结束获取视频
    fn log_fetch_video_end(&self);

    /// 开始下载视频
    fn log_download_video_start(&self);

    /// 结束下载视频
    fn log_download_video_end(&self);

    /// 开始刷新视频
    fn log_refresh_video_start(&self);

    /// 结束刷新视频
    fn log_refresh_video_end(&self, count: usize);
}
