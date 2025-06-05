use std::borrow::Borrow;

use sea_orm::{ConnectionTrait, DatabaseTransaction};

use crate::api::response::{PageInfo, VideoInfo};

pub async fn update_video_download_status(
    txn: &DatabaseTransaction,
    videos: &[impl Borrow<VideoInfo>],
    batch_size: Option<usize>,
) -> Result<(), sea_orm::DbErr> {
    if videos.is_empty() {
        return Ok(());
    }
    let videos = videos.iter().map(|v| v.borrow()).collect::<Vec<_>>();
    if let Some(size) = batch_size {
        for chunk in videos.chunks(size) {
            execute_video_update_batch(txn, chunk).await?;
        }
    } else {
        execute_video_update_batch(txn, &videos).await?;
    }
    Ok(())
}

pub async fn update_page_download_status(
    txn: &DatabaseTransaction,
    pages: &[impl Borrow<PageInfo>],
    batch_size: Option<usize>,
) -> Result<(), sea_orm::DbErr> {
    if pages.is_empty() {
        return Ok(());
    }
    let pages = pages.iter().map(|v| v.borrow()).collect::<Vec<_>>();
    if let Some(size) = batch_size {
        for chunk in pages.chunks(size) {
            execute_page_update_batch(txn, chunk).await?;
        }
    } else {
        execute_page_update_batch(txn, &pages).await?;
    }
    Ok(())
}

async fn execute_video_update_batch(txn: &DatabaseTransaction, videos: &[&VideoInfo]) -> Result<(), sea_orm::DbErr> {
    if videos.is_empty() {
        return Ok(());
    }
    let sql = format!(
        "WITH tempdata(id, download_status) AS (VALUES {}) \
        UPDATE video \
        SET download_status = tempdata.download_status \
        FROM tempdata \
        WHERE video.id = tempdata.id",
        videos
            .iter()
            .map(|v| format!("({}, {})", v.id, v.download_status))
            .collect::<Vec<_>>()
            .join(", ")
    );
    txn.execute_unprepared(&sql).await?;
    Ok(())
}

async fn execute_page_update_batch(txn: &DatabaseTransaction, pages: &[&PageInfo]) -> Result<(), sea_orm::DbErr> {
    if pages.is_empty() {
        return Ok(());
    }
    let sql = format!(
        "WITH tempdata(id, download_status) AS (VALUES {}) \
        UPDATE page \
        SET download_status = tempdata.download_status \
        FROM tempdata \
        WHERE page.id = tempdata.id",
        pages
            .iter()
            .map(|p| format!("({}, {})", p.id, p.download_status))
            .collect::<Vec<_>>()
            .join(", ")
    );
    txn.execute_unprepared(&sql).await?;
    Ok(())
}
