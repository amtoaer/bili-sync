mod collection;
mod favorite;
mod helper;
mod watch_later;

use std::collections::HashSet;
use std::path::Path;
use std::pin::Pin;

use anyhow::Result;
use async_trait::async_trait;
use collection::collection_from;
use favorite::favorite_from;
use futures::Stream;
use sea_orm::entity::prelude::*;
use sea_orm::DatabaseConnection;
use watch_later::watch_later_from;

use crate::bilibili::{self, BiliClient, CollectionItem, VideoInfo};

pub enum Args<'a> {
    Favorite { fid: &'a str },
    Collection { collection_item: &'a CollectionItem },
    WatchLater,
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
    }
}

#[async_trait]
pub trait VideoListModel {
    /// 与视频列表关联的视频总数
    async fn video_count(&self, connection: &DatabaseConnection) -> Result<u64>;

    /// 未填充的视频
    async fn unfilled_videos(&self, connection: &DatabaseConnection) -> Result<Vec<bili_sync_entity::video::Model>>;

    /// 未处理的视频和分页
    async fn unhandled_video_pages(
        &self,
        connection: &DatabaseConnection,
    ) -> Result<Vec<(bili_sync_entity::video::Model, Vec<bili_sync_entity::page::Model>)>>;

    /// 该批次视频的存在标记
    async fn exist_labels(&self, videos_info: &[VideoInfo], connection: &DatabaseConnection)
        -> Result<HashSet<String>>;

    /// 视频信息对应的视频 model
    fn video_model_by_info(
        &self,
        video_info: &VideoInfo,
        base_model: Option<bili_sync_entity::video::Model>,
    ) -> bili_sync_entity::video::ActiveModel;

    /// 视频 model 中缺失的信息
    async fn fetch_videos_detail(
        &self,
        video: bilibili::Video<'_>,
        video_model: bili_sync_entity::video::Model,
        connection: &DatabaseConnection,
    ) -> Result<()>;

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
    fn log_refresh_video_end(&self, got_count: usize, new_count: u64);
}
