mod collection;
mod favorite;
mod submission;
mod watch_later;

use std::borrow::Cow;
use std::path::Path;
use std::pin::Pin;

use anyhow::Result;
use chrono::Utc;
use enum_dispatch::enum_dispatch;
use futures::Stream;
use sea_orm::ActiveValue::Set;
use sea_orm::DatabaseConnection;
use sea_orm::entity::prelude::*;
use sea_orm::sea_query::SimpleExpr;

#[rustfmt::skip]
use bili_sync_entity::collection::Model as Collection;
use bili_sync_entity::favorite::Model as Favorite;
use bili_sync_entity::rule::Rule;
use bili_sync_entity::submission::Model as Submission;
use bili_sync_entity::watch_later::Model as WatchLater;

use crate::bilibili::{BiliClient, VideoInfo};

#[enum_dispatch]
pub enum VideoSourceEnum {
    Favorite,
    Collection,
    Submission,
    WatchLater,
}

#[enum_dispatch(VideoSourceEnum)]
pub trait VideoSource {
    /// 获取视频源的名称
    fn display_name(&self) -> Cow<'static, str>;

    /// 获取特定视频列表的筛选条件
    fn filter_expr(&self) -> SimpleExpr;

    // 为 video_model 设置该视频列表的关联 id
    fn set_relation_id(&self, video_model: &mut bili_sync_entity::video::ActiveModel);

    // 获取视频列表的保存路径
    fn path(&self) -> &Path;

    /// 获取视频 model 中记录的最新时间
    fn get_latest_row_at(&self) -> DateTime;

    /// 更新视频 model 中记录的最新时间，此处返回需要更新的 ActiveModel，接着调用 save 方法执行保存
    /// 不同 VideoSource 返回的类型不同，为了 VideoSource 的 object safety 不能使用 impl Trait
    /// Box<dyn ActiveModelTrait> 又提示 ActiveModelTrait 没有 object safety，因此手写一个 Enum 静态分发
    fn update_latest_row_at(&self, datetime: DateTime) -> _ActiveModel;

    // 判断是否应该继续拉取视频
    fn should_take(&self, release_datetime: &chrono::DateTime<Utc>, latest_row_at: &chrono::DateTime<Utc>) -> bool {
        release_datetime > latest_row_at
    }

    fn should_filter(
        &self,
        video_info: Result<VideoInfo, anyhow::Error>,
        _latest_row_at: &chrono::DateTime<Utc>,
    ) -> Option<VideoInfo> {
        // 视频按照时间顺序拉取，should_take 已经获取了所有需要处理的视频，should_filter 无需额外处理
        video_info.ok()
    }

    fn rule(&self) -> Option<&Rule>;

    fn log_refresh_video_start(&self) {
        info!("开始扫描{}..", self.display_name());
    }

    fn log_refresh_video_end(&self, count: usize) {
        info!("扫描{}完成，获取到 {} 条新视频", self.display_name(), count);
    }

    fn log_fetch_video_start(&self) {
        info!("开始填充{}视频详情..", self.display_name());
    }

    fn log_fetch_video_end(&self) {
        info!("填充{}视频详情完成", self.display_name());
    }

    fn log_download_video_start(&self) {
        info!("开始下载{}视频..", self.display_name());
    }

    fn log_download_video_end(&self) {
        info!("下载{}视频完成", self.display_name());
    }

    async fn refresh<'a>(
        self,
        bili_client: &'a BiliClient,
        connection: &'a DatabaseConnection,
    ) -> Result<(
        VideoSourceEnum,
        Pin<Box<dyn Stream<Item = Result<VideoInfo>> + Send + 'a>>,
    )>;
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
            _ActiveModel::WatchLater(mut model) => {
                if model.id.is_not_set() {
                    model.id = Set(1);
                    model.insert(connection).await?;
                } else {
                    model.save(connection).await?;
                }
            }
        }
        Ok(())
    }
}
