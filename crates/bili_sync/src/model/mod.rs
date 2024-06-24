mod collection;
mod favorite;

use anyhow::Result;
use sea_orm::DatabaseConnection;

pub trait VideoListModel {
    /* 逻辑相关 */

    /// 获取未填充的视频
    async fn unfilled_videos(&self, connection: &DatabaseConnection) -> Result<Vec<bili_sync_entity::video::Model>>;

    /// 获取未处理的视频和分页
    async fn unhandled_video_pages(
        &self,
        connection: &DatabaseConnection,
    ) -> Result<Vec<(bili_sync_entity::video::Model, Vec<bili_sync_entity::page::Model>)>>;

    fn log_fetch_video_start(&self);

    fn log_fetch_video_end(&self);

    fn log_download_video_start(&self);

    fn log_download_video_end(&self);
}
