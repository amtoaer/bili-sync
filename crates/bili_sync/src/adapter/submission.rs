use std::path::Path;
use std::pin::Pin;

use anyhow::{Result, ensure};
use bili_sync_entity::rule::Rule;
use bili_sync_entity::*;
use futures::Stream;
use sea_orm::ActiveValue::Set;
use sea_orm::entity::prelude::*;
use sea_orm::sea_query::SimpleExpr;
use sea_orm::{DatabaseConnection, Unchanged};

use crate::adapter::{_ActiveModel, VideoSource, VideoSourceEnum};
use crate::bilibili::{BiliClient, Credential, Dynamic, Submission, VideoInfo};

impl VideoSource for submission::Model {
    fn display_name(&self) -> std::borrow::Cow<'static, str> {
        format!("「{}」投稿", self.upper_name).into()
    }

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

    fn should_take(
        &self,
        idx: usize,
        release_datetime: &chrono::DateTime<chrono::Utc>,
        latest_row_at: &chrono::DateTime<chrono::Utc>,
    ) -> bool {
        // 如果使用动态 API，那么可能出现用户置顶了一个很久以前的视频在动态顶部的情况
        // 这种情况应该继续拉取下去，不能因为第一条不满足条件就停止
        // 后续的非置顶内容是正常由新到旧排序的，可以继续使用常规方式处理
        if idx == 0 && self.use_dynamic_api {
            return true;
        }
        release_datetime > latest_row_at
    }

    fn should_filter(
        &self,
        idx: usize,
        video_info: Result<VideoInfo, anyhow::Error>,
        latest_row_at: &chrono::DateTime<chrono::Utc>,
    ) -> Option<VideoInfo> {
        if idx == 0 && self.use_dynamic_api {
            // 同理，动态 API 的第一条内容可能是置顶的老视频，单独做个过滤
            // 其实不过滤也不影响逻辑正确性，因为后续 insert 发生冲突仍然会忽略掉
            // 此处主要是出于性能考虑，减少不必要的数据库操作
            if let Ok(video_info) = video_info
                && video_info.release_datetime() > latest_row_at
            {
                return Some(video_info);
            }
            None
        } else {
            video_info.ok()
        }
    }

    fn rule(&self) -> &Option<Rule> {
        &self.rule
    }

    async fn refresh<'a>(
        self,
        bili_client: &'a BiliClient,
        credential: &'a Credential,
        connection: &'a DatabaseConnection,
    ) -> Result<(
        VideoSourceEnum,
        Pin<Box<dyn Stream<Item = Result<VideoInfo>> + Send + 'a>>,
    )> {
        let submission = Submission::new(bili_client, self.upper_id.to_string(), credential);
        let upper = submission.get_info().await?;
        ensure!(
            upper.mid == submission.upper_id,
            "submission upper id mismatch: {} != {}",
            upper.mid,
            submission.upper_id
        );
        let updated_model = submission::ActiveModel {
            id: Unchanged(self.id),
            upper_name: Set(upper.name),
            ..Default::default()
        }
        .update(connection)
        .await?;
        let video_stream = if self.use_dynamic_api {
            // 必须显式写出 dyn，否则 rust 会自动推导到 impl 从而认为 if else 返回类型不一致
            Box::pin(Dynamic::from(submission).into_video_stream()) as Pin<Box<dyn Stream<Item = _> + Send + 'a>>
        } else {
            Box::pin(submission.into_video_stream())
        };
        Ok((updated_model.into(), video_stream))
    }

    async fn delete_from_db(self, conn: &impl ConnectionTrait) -> Result<()> {
        self.delete(conn).await?;
        Ok(())
    }
}
