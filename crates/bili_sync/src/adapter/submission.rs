use std::collections::HashSet;
use std::path::Path;
use std::pin::Pin;

use anyhow::Result;
use async_trait::async_trait;
use bili_sync_entity::*;
use futures::Stream;
use sea_orm::entity::prelude::*;
use sea_orm::sea_query::{IntoCondition, OnConflict};
use sea_orm::ActiveValue::Set;
use sea_orm::{DatabaseConnection, TransactionTrait};

use crate::adapter::helper::video_with_path;
use crate::adapter::{helper, VideoListModel};
use crate::bilibili::{self, BiliClient, Submission, VideoInfo};
use crate::utils::status::Status;

#[async_trait]
impl VideoListModel for submission::Model {
    async fn video_count(&self, connection: &DatabaseConnection) -> Result<u64> {
        helper::count_videos(video::Column::SubmissionId.eq(self.id).into_condition(), connection).await
    }

    async fn unfilled_videos(&self, connection: &DatabaseConnection) -> Result<Vec<video::Model>> {
        helper::filter_videos(
            video::Column::SubmissionId
                .eq(self.id)
                .and(video::Column::Valid.eq(true))
                .and(video::Column::DownloadStatus.eq(0))
                .and(video::Column::Category.eq(2))
                .and(video::Column::SinglePage.is_null())
                .into_condition(),
            connection,
        )
        .await
    }

    async fn unhandled_video_pages(
        &self,
        connection: &DatabaseConnection,
    ) -> Result<Vec<(video::Model, Vec<page::Model>)>> {
        helper::filter_videos_with_pages(
            video::Column::SubmissionId
                .eq(self.id)
                .and(video::Column::Valid.eq(true))
                .and(video::Column::DownloadStatus.lt(Status::handled()))
                .and(video::Column::Category.eq(2))
                .and(video::Column::SinglePage.is_not_null())
                .into_condition(),
            connection,
        )
        .await
    }

    async fn exist_labels(
        &self,
        videos_info: &[VideoInfo],
        connection: &DatabaseConnection,
    ) -> Result<HashSet<String>> {
        helper::video_keys(
            video::Column::SubmissionId.eq(self.id),
            videos_info,
            [video::Column::Bvid, video::Column::Ctime],
            connection,
        )
        .await
    }

    fn video_model_by_info(&self, video_info: &VideoInfo, base_model: Option<video::Model>) -> video::ActiveModel {
        let mut video_model = video_info.to_model(base_model);
        video_model.submission_id = Set(Some(self.id));
        video_with_path(video_model, &self.path, video_info)
    }

    async fn fetch_videos_detail(
        &self,
        video: bilibili::Video<'_>,
        video_model: video::Model,
        connection: &DatabaseConnection,
    ) -> Result<()> {
        let info: Result<_> = async { Ok((video.get_tags().await?, video.get_view_info().await?)) }.await;
        match info {
            Ok((tags, view_info)) => {
                let VideoInfo::View { pages, .. } = &view_info else {
                    unreachable!("view_info must be VideoInfo::View")
                };
                let txn = connection.begin().await?;
                // 将分页信息写入数据库
                helper::create_video_pages(pages, &video_model, &txn).await?;
                // 将页标记和 tag 写入数据库
                let mut video_active_model = self.video_model_by_info(&view_info, Some(video_model));
                video_active_model.single_page = Set(Some(pages.len() == 1));
                video_active_model.tags = Set(Some(serde_json::to_value(tags).unwrap()));
                video_active_model.save(&txn).await?;
                txn.commit().await?;
            }
            Err(e) => {
                helper::error_fetch_video_detail(e, video_model, connection).await?;
            }
        };
        Ok(())
    }

    fn log_fetch_video_start(&self) {
        info!(
            "开始获取 UP 主 {} - {} 投稿的视频与分页信息...",
            self.upper_id, self.upper_name
        );
    }

    fn log_fetch_video_end(&self) {
        info!(
            "获取 UP 主 {} - {} 投稿的视频与分页信息完成",
            self.upper_id, self.upper_name
        );
    }

    fn log_download_video_start(&self) {
        info!(
            "开始下载 UP 主 {} - {} 投稿的所有未处理过的视频...",
            self.upper_id, self.upper_name
        );
    }

    fn log_download_video_end(&self) {
        info!(
            "下载 UP 主 {} - {} 投稿的所有未处理过的视频完成",
            self.upper_id, self.upper_name
        );
    }

    fn log_refresh_video_start(&self) {
        info!("开始扫描 UP 主 {} - {} 投稿的新视频...", self.upper_id, self.upper_name);
    }

    fn log_refresh_video_end(&self, got_count: usize, new_count: u64) {
        info!(
            "扫描 UP 主 {} - {} 投稿的新视频完成，获取了 {} 条新视频，其中有 {} 条新视频",
            self.upper_id, self.upper_name, got_count, new_count,
        );
    }
}

pub(super) async fn submission_from<'a>(
    upper_id: &str,
    path: &Path,
    bili_client: &'a BiliClient,
    connection: &DatabaseConnection,
) -> Result<(Box<dyn VideoListModel>, Pin<Box<dyn Stream<Item = VideoInfo> + 'a>>)> {
    let submission = Submission::new(bili_client, upper_id.to_owned());
    let upper = submission.get_info().await?;
    submission::Entity::insert(submission::ActiveModel {
        upper_id: Set(upper.mid.parse()?),
        upper_name: Set(upper.name),
        path: Set(path.to_string_lossy().to_string()),
        ..Default::default()
    })
    .on_conflict(
        OnConflict::column(submission::Column::UpperId)
            .update_columns([submission::Column::UpperName, submission::Column::Path])
            .to_owned(),
    )
    .exec(connection)
    .await?;
    Ok((
        Box::new(
            submission::Entity::find()
                .filter(submission::Column::UpperId.eq(upper.mid))
                .one(connection)
                .await?
                .unwrap(),
        ),
        Box::pin(submission.into_video_stream()),
    ))
}
