mod collection;
mod convert;
mod favorite;

use std::collections::HashSet;

use anyhow::Result;
use sea_orm::DatabaseConnection;

use crate::bilibili::{BiliClient, VideoInfo};

pub trait VideoListModel {
    /* 逻辑相关 */

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
    fn video_models_by_info(&self, videos_info: &[VideoInfo]) -> Result<Vec<bili_sync_entity::video::ActiveModel>>;

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
}
