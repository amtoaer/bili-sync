use std::path::Path;
use std::pin::Pin;

use anyhow::{Context, Result};
use bili_sync_entity::*;
use futures::Stream;
use sea_orm::ActiveValue::Set;
use sea_orm::entity::prelude::*;
use sea_orm::sea_query::{OnConflict, SimpleExpr};
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
}

pub(super) async fn submission_from<'a>(
    upper_id: &str,
    path: &Path,
    bili_client: &'a BiliClient,
    connection: &DatabaseConnection,
) -> Result<(
    VideoSourceEnum,
    Pin<Box<dyn Stream<Item = Result<VideoInfo>> + 'a + Send>>,
)> {
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
        submission::Entity::find()
            .filter(submission::Column::UpperId.eq(upper.mid))
            .one(connection)
            .await?
            .context("submission not found")?
            .into(),
        Box::pin(submission.into_video_stream()),
    ))
}
