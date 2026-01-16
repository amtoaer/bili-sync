use std::borrow::Borrow;

use itertools::Itertools;
use sea_orm::{Condition, ConnectionTrait, DatabaseTransaction};

use crate::api::request::StatusFilter;
use crate::api::response::{PageInfo, SimplePageInfo, SimpleVideoInfo, VideoInfo};
use crate::utils::status::VideoStatus;

impl StatusFilter {
    pub fn to_video_query(&self) -> Condition {
        let query_builder = VideoStatus::query_builder();
        match self {
            Self::Failed => query_builder.failed(),
            Self::Succeeded => query_builder.succeeded(),
            Self::Waiting => query_builder.waiting(),
        }
    }
}

pub trait VideoRecord {
    fn as_id_status_tuple(&self) -> (i32, u32);
}

pub trait PageRecord {
    fn as_id_status_tuple(&self) -> (i32, u32);
}

impl VideoRecord for VideoInfo {
    fn as_id_status_tuple(&self) -> (i32, u32) {
        (self.id, self.download_status)
    }
}

impl VideoRecord for SimpleVideoInfo {
    fn as_id_status_tuple(&self) -> (i32, u32) {
        (self.id, self.download_status)
    }
}

impl PageRecord for PageInfo {
    fn as_id_status_tuple(&self) -> (i32, u32) {
        (self.id, self.download_status)
    }
}

impl PageRecord for SimplePageInfo {
    fn as_id_status_tuple(&self) -> (i32, u32) {
        (self.id, self.download_status)
    }
}

pub async fn update_video_download_status<T>(
    txn: &DatabaseTransaction,
    videos: &[impl Borrow<T>],
    batch_size: Option<usize>,
) -> Result<(), sea_orm::DbErr>
where
    T: VideoRecord,
{
    if videos.is_empty() {
        return Ok(());
    }
    if let Some(size) = batch_size {
        for chunk in videos.chunks(size) {
            execute_video_update_batch(txn, chunk.iter().map(|v| v.borrow().as_id_status_tuple())).await?;
        }
    } else {
        execute_video_update_batch(txn, videos.iter().map(|v| v.borrow().as_id_status_tuple())).await?;
    }
    Ok(())
}

pub async fn update_page_download_status<T>(
    txn: &DatabaseTransaction,
    pages: &[impl Borrow<T>],
    batch_size: Option<usize>,
) -> Result<(), sea_orm::DbErr>
where
    T: PageRecord,
{
    if pages.is_empty() {
        return Ok(());
    }
    if let Some(size) = batch_size {
        for chunk in pages.chunks(size) {
            execute_page_update_batch(txn, chunk.iter().map(|v| v.borrow().as_id_status_tuple())).await?;
        }
    } else {
        execute_page_update_batch(txn, pages.iter().map(|v| v.borrow().as_id_status_tuple())).await?;
    }
    Ok(())
}

async fn execute_video_update_batch(
    txn: &DatabaseTransaction,
    videos: impl Iterator<Item = (i32, u32)>,
) -> Result<(), sea_orm::DbErr> {
    let values = videos.map(|v| format!("({}, {})", v.0, v.1)).join(", ");
    if values.is_empty() {
        return Ok(());
    }
    let sql = format!(
        "WITH tempdata(id, download_status) AS (VALUES {}) \
        UPDATE video \
        SET download_status = tempdata.download_status \
        FROM tempdata \
        WHERE video.id = tempdata.id",
        values
    );
    txn.execute_unprepared(&sql).await?;
    Ok(())
}

async fn execute_page_update_batch(
    txn: &DatabaseTransaction,
    pages: impl Iterator<Item = (i32, u32)>,
) -> Result<(), sea_orm::DbErr> {
    let values = pages
        .map(|p| format!("({}, {})", p.0, p.1))
        .collect::<Vec<_>>()
        .join(", ");
    if values.is_empty() {
        return Ok(());
    }
    let sql = format!(
        "WITH tempdata(id, download_status) AS (VALUES {}) \
        UPDATE page \
        SET download_status = tempdata.download_status \
        FROM tempdata \
        WHERE page.id = tempdata.id",
        values
    );
    txn.execute_unprepared(&sql).await?;
    Ok(())
}
