mod collection;
mod favorite;

use std::collections::HashSet;
use std::path::Path;
use std::pin::Pin;

use anyhow::Result;
use async_trait::async_trait;
use bili_sync_migration::IntoIden;
pub use collection::collection_from;
pub use favorite::favorite_from;
use futures::Stream;
use sea_orm::DatabaseConnection;

use crate::bilibili::{BiliClient, CollectionItem, VideoInfo};

pub enum Args<'a> {
    Favorite { fid: &'a str },
    Collection { collection_item: &'a CollectionItem },
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
    }
}

pub const fn unique_video_columns() -> impl IntoIterator<Item = impl IntoIden> {
    [
        bili_sync_entity::video::Column::CollectionId,
        bili_sync_entity::video::Column::FavoriteId,
        bili_sync_entity::video::Column::Bvid,
    ]
}

#[async_trait]
pub trait VideoListModel {
    /* 逻辑相关 */

    async fn video_count(&self, connection: &DatabaseConnection) -> Result<u64>;

    /// 获取未填充的视频
    async fn unfilled_videos(&self, connection: &DatabaseConnection) -> Result<Vec<bili_sync_entity::video::Model>>;

    /// 获取未处理的视频和分页
    async fn unhandled_video_pages(
        &self,
        connection: &DatabaseConnection,
    ) -> Result<Vec<(bili_sync_entity::video::Model, Vec<bili_sync_entity::page::Model>)>>;

    /// 获取该批次视频的存在标记
    async fn exist_labels(&self, videos_info: &[VideoInfo], connection: &DatabaseConnection)
        -> Result<HashSet<String>>;

    /// 获取视频信息对应的视频 model
    fn video_model_by_info(&self, video_info: &VideoInfo) -> bili_sync_entity::video::ActiveModel;

    /// 获取视频 model 中缺失的信息
    async fn fetch_videos_detail(
        &self,
        bili_client: &BiliClient,
        videos_model: Vec<bili_sync_entity::video::Model>,
        connection: &DatabaseConnection,
    ) -> Result<()>;

    /* 日志相关 */
    fn log_fetch_video_start(&self);

    fn log_fetch_video_end(&self);

    fn log_download_video_start(&self);

    fn log_download_video_end(&self);

    fn log_refresh_video_start(&self);

    fn log_refresh_video_end(&self, got_count: usize, new_count: u64);
}
