pub mod entities;

pub use entities::*;

use chrono::{DateTime, NaiveDateTime, Utc};
use sea_orm::sea_query::SimpleExpr;

pub trait VideoSourceTrait {
    /// 获取特定视频列表的筛选条件
    fn filter_expr(&self) -> SimpleExpr;

    // 为 video_model 设置该视频列表的关联 id
    fn set_relation_id(&self, video_model: &mut video::ActiveModel);

    /// 获取视频 model 中记录的最新时间
    fn get_latest_row_at(&self) -> NaiveDateTime;

    /// 更新视频 model 中记录的最新时间
    fn update_latest_row_at(&self, latest_row_at: NaiveDateTime) -> video_source::ActiveModel;

    // 判断是否应该继续拉取视频
    fn should_take(&self, release_datetime: &DateTime<Utc>, latest_row_at: &DateTime<Utc>) -> bool;

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
