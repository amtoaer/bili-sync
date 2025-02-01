mod collection;
mod favorite;
mod submission;
mod watch_later;

use std::path::Path;
use std::pin::Pin;

use anyhow::Result;
use enum_dispatch::enum_dispatch;
use futures::Stream;
use sea_orm::entity::prelude::*;
use sea_orm::sea_query::SimpleExpr;
use sea_orm::DatabaseConnection;

#[rustfmt::skip]
use bili_sync_entity::collection::Model as Collection;
use bili_sync_entity::favorite::Model as Favorite;
use bili_sync_entity::submission::Model as Submission;
use bili_sync_entity::watch_later::Model as WatchLater;

use crate::adapter::collection::collection_from;
use crate::adapter::favorite::favorite_from;
use crate::adapter::submission::submission_from;
use crate::adapter::watch_later::watch_later_from;
use crate::bilibili::{BiliClient, CollectionItem, VideoInfo};

#[enum_dispatch]
pub enum VideoListModelEnum {
    Favorite,
    Collection,
    Submission,
    WatchLater,
}

#[enum_dispatch(VideoListModelEnum)]
pub trait VideoListModel {
    /// 获取特定视频列表的筛选条件
    fn filter_expr(&self) -> SimpleExpr;

    // 为 video_model 设置该视频列表的关联 id
    fn set_relation_id(&self, video_model: &mut bili_sync_entity::video::ActiveModel);

    // 获取视频列表的保存路径
    fn path(&self) -> &Path;

    /// 获取视频 model 中记录的最新时间
    fn get_latest_row_at(&self) -> DateTime;

    /// 更新视频 model 中记录的最新时间，此处返回需要更新的 ActiveModel，接着调用 save 方法执行保存
    /// 不同 VideoListModel 返回的类型不同，为了 VideoListModel 的 object safety 不能使用 impl Trait
    /// Box<dyn ActiveModelTrait> 又提示 ActiveModelTrait 没有 object safety，因此手写一个 Enum 静态分发
    fn update_latest_row_at(&self, datetime: DateTime) -> _ActiveModel;

    /// 开始刷新视频
    fn log_refresh_video_start(&self);

    /// 结束刷新视频
    fn log_refresh_video_end(&self, count: usize);

    /// 开始填充视频
    fn log_fetch_video_start(&self);

    /// 结束填充视频
    fn log_fetch_video_end(&self);

    /// 开始下载视频
    fn log_download_video_start(&self);

    /// 结束下载视频
    fn log_download_video_end(&self);
}

#[derive(Clone, Copy)]
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
) -> Result<(
    VideoListModelEnum,
    Pin<Box<dyn Stream<Item = Result<VideoInfo>> + 'a + Send>>,
)> {
    match args {
        Args::Favorite { fid } => favorite_from(fid, path, bili_client, connection).await,
        Args::Collection { collection_item } => collection_from(collection_item, path, bili_client, connection).await,
        Args::WatchLater => watch_later_from(path, bili_client, connection).await,
        Args::Submission { upper_id } => submission_from(upper_id, path, bili_client, connection).await,
    }
}

pub enum _ActiveModel {
    Favorite(bili_sync_entity::favorite::ActiveModel),
    Collection(bili_sync_entity::collection::ActiveModel),
    Submission(bili_sync_entity::submission::ActiveModel),
    WatchLater(bili_sync_entity::watch_later::ActiveModel),
}

impl _ActiveModel {
    pub async fn save(self, connection: &DatabaseConnection) -> Result<()> {
        match self {
            _ActiveModel::Favorite(model) => {
                model.save(connection).await?;
            }
            _ActiveModel::Collection(model) => {
                model.save(connection).await?;
            }
            _ActiveModel::Submission(model) => {
                model.save(connection).await?;
            }
            _ActiveModel::WatchLater(model) => {
                model.save(connection).await?;
            }
        }
        Ok(())
    }
}
