use anyhow::{Context, Result};
use bili_sync_entity::*;
use sea_orm::entity::prelude::*;
use sea_orm::sea_query::{OnConflict, SimpleExpr};
use sea_orm::{Set, TransactionTrait};

use crate::adapter::VideoListModel;
use crate::bilibili::{self, BiliError, VideoInfo};
use crate::config::{PathSafeTemplate, TEMPLATE};
use crate::utils::status::STATUS_COMPLETED;

/// 筛选未填充的视频
pub async fn filter_unfilled_videos(
    additional_expr: SimpleExpr,
    conn: &DatabaseConnection,
) -> Result<Vec<video::Model>> {
    video::Entity::find()
        .filter(
            video::Column::Valid
                .eq(true)
                .and(video::Column::DownloadStatus.eq(0))
                .and(video::Column::Category.eq(2))
                .and(video::Column::SinglePage.is_null())
                .and(additional_expr),
        )
        .all(conn)
        .await
        .context("filter unfilled videos failed")
}

pub async fn filter_unhandled_video_pages(
    additional_expr: SimpleExpr,
    connection: &DatabaseConnection,
) -> Result<Vec<(video::Model, Vec<page::Model>)>> {
    video::Entity::find()
        .filter(
            video::Column::Valid
                .eq(true)
                .and(video::Column::DownloadStatus.lt(STATUS_COMPLETED))
                .and(video::Column::Category.eq(2))
                .and(video::Column::SinglePage.is_not_null())
                .and(additional_expr),
        )
        .find_with_related(page::Entity)
        .all(connection)
        .await
        .context("filter unhandled video pages failed")
}

pub async fn fetch_videos_detail(
    video_list_model: &dyn VideoListModel,
    video: bilibili::Video<'_>,
    video_model: video::Model,
    connection: &DatabaseConnection,
) -> Result<()> {
    let info: Result<_> = async { Ok((video.get_tags().await?, video.get_view_info().await?)) }.await;
    match info {
        Ok((tags, view_info)) => {
            let VideoInfo::Detail { pages, .. } = &view_info else {
                unreachable!("view_info must be VideoInfo::View")
            };
            let txn = connection.begin().await?;
            // 将分页信息写入数据库
            let page_models = pages
                .iter()
                .map(move |p| {
                    let (width, height) = match &p.dimension {
                        Some(d) => {
                            if d.rotate == 0 {
                                (Some(d.width), Some(d.height))
                            } else {
                                (Some(d.height), Some(d.width))
                            }
                        }
                        None => (None, None),
                    };
                    page::ActiveModel {
                        video_id: Set(video_model.id),
                        cid: Set(p.cid),
                        pid: Set(p.page),
                        name: Set(p.name.clone()),
                        width: Set(width),
                        height: Set(height),
                        duration: Set(p.duration),
                        image: Set(p.first_frame.clone()),
                        download_status: Set(0),
                        ..Default::default()
                    }
                })
                .collect::<Vec<page::ActiveModel>>();
            for page_chunk in page_models.chunks(50) {
                page::Entity::insert_many(page_chunk.to_vec())
                    .on_conflict(
                        OnConflict::columns([page::Column::VideoId, page::Column::Pid])
                            .do_nothing()
                            .to_owned(),
                    )
                    .do_nothing()
                    .exec(connection)
                    .await?;
            }
            let mut video_active_model = view_info.to_model(Some(video_model));
            video_list_model.set_relation_id(&mut video_active_model);
            if let Some(fmt_args) = view_info.to_fmt_args() {
                video_active_model.path = Set(video_list_model
                    .path()
                    .join(
                        TEMPLATE
                            .path_safe_render("video", &fmt_args)
                            .expect("template render failed"),
                    )
                    .to_string_lossy()
                    .to_string());
            }
            video_active_model.single_page = Set(Some(pages.len() == 1));
            video_active_model.tags = Set(Some(serde_json::to_value(tags)?));
            video_active_model.save(&txn).await?;
            txn.commit().await?;
        }
        Err(e) => {
            error!(
                "获取视频 {} - {} 的详细信息失败，错误为：{}",
                &video_model.bvid, &video_model.name, e
            );
            if let Some(BiliError::RequestFailed(-404, _)) = e.downcast_ref::<BiliError>() {
                let mut video_active_model: bili_sync_entity::video::ActiveModel = video_model.into();
                video_active_model.valid = Set(false);
                video_active_model.save(connection).await?;
            }
        }
    };
    Ok(())
}

/// 尝试创建 Video Model，如果发生冲突则忽略
pub async fn create_videos(
    videos_info: &[VideoInfo],
    video_list_model: &dyn VideoListModel,
    connection: &DatabaseConnection,
) -> Result<()> {
    let video_models = videos_info
        .iter()
        .map(|v| {
            let mut model = v.to_model(None);
            video_list_model.set_relation_id(&mut model);
            model
        })
        .collect::<Vec<_>>();
    video::Entity::insert_many(video_models)
        // 这里想表达的是 on 索引名，但 sea-orm 的 api 似乎只支持列名而不支持索引名，好在留空可以达到相同的目的
        .on_conflict(OnConflict::new().do_nothing().to_owned())
        .do_nothing()
        .exec(connection)
        .await?;
    Ok(())
}

/// 更新视频 model 的下载状态
pub async fn update_videos_model(videos: Vec<video::ActiveModel>, connection: &DatabaseConnection) -> Result<()> {
    video::Entity::insert_many(videos)
        .on_conflict(
            OnConflict::column(video::Column::Id)
                .update_column(video::Column::DownloadStatus)
                .to_owned(),
        )
        .exec(connection)
        .await?;
    Ok(())
}

/// 更新视频页 model 的下载状态
pub async fn update_pages_model(pages: Vec<page::ActiveModel>, connection: &DatabaseConnection) -> Result<()> {
    let query = page::Entity::insert_many(pages).on_conflict(
        OnConflict::column(page::Column::Id)
            .update_columns([page::Column::DownloadStatus, page::Column::Path])
            .to_owned(),
    );
    query.exec(connection).await?;
    Ok(())
}
