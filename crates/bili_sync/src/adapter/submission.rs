use std::path::Path;
use std::pin::Pin;

use anyhow::{Context, Result, ensure};
use bili_sync_entity::*;
use futures::Stream;
use sea_orm::ActiveValue::Set;
use sea_orm::entity::prelude::*;
use sea_orm::sea_query::SimpleExpr;
use sea_orm::{DatabaseConnection, Unchanged};

use crate::adapter::{_ActiveModel, VideoSource, VideoSourceEnum};
use crate::bilibili::{BiliClient, Submission, VideoInfo};

impl VideoSource for submission::Model {
    fn filter_expr(&self) -> SimpleExpr {
        video::Column::SubmissionId.eq(self.id)
    }

    fn set_relation_id(&self, video_model: &mut video::ActiveModel) {
        video_model.submission_id = Set(Some(self.id));
    }

    fn path(&self) -> &Path {
        Path::new(self.path.as_str())
    }

    fn get_latest_row_at(&self) -> DateTime {
        self.latest_row_at
    }

    fn update_latest_row_at(&self, datetime: DateTime) -> _ActiveModel {
        _ActiveModel::Submission(submission::ActiveModel {
            id: Unchanged(self.id),
            latest_row_at: Set(datetime),
            ..Default::default()
        })
    }

    fn log_refresh_video_start(&self) {
        info!("开始扫描「{}」投稿..", self.upper_name);
    }

    fn log_refresh_video_end(&self, count: usize) {
        info!("扫描「{}」投稿完成，获取到 {} 条新视频", self.upper_name, count,);
    }

    fn log_fetch_video_start(&self) {
        info!("开始填充「{}」投稿视频详情..", self.upper_name);
    }

    fn log_fetch_video_end(&self) {
        info!("填充「{}」投稿视频详情完成", self.upper_name);
    }

    fn log_download_video_start(&self) {
        info!("开始下载「{}」投稿视频..", self.upper_name);
    }

    fn log_download_video_end(&self) {
        info!("下载「{}」投稿视频完成", self.upper_name);
    }

    async fn refresh<'a>(
        self,
        bili_client: &'a BiliClient,
        connection: &'a DatabaseConnection,
    ) -> Result<(
        VideoSourceEnum,
        Pin<Box<dyn Stream<Item = Result<VideoInfo>> + Send + 'a>>,
    )> {
        let submission = Submission::new(bili_client, self.upper_id.to_string());
        let upper = submission.get_info().await?;
        ensure!(
            upper.mid == submission.upper_id,
            "submission upper id mismatch: {} != {}",
            upper.mid,
            submission.upper_id
        );
        submission::ActiveModel {
            id: Unchanged(self.id),
            upper_name: Set(upper.name),
            ..Default::default()
        }
        .save(connection)
        .await?;
        Ok((
            submission::Entity::find()
                .filter(submission::Column::Id.eq(self.id))
                .one(connection)
                .await?
                .context("submission not found")?
                .into(),
            Box::pin(submission.into_video_stream()),
        ))
    }
}
