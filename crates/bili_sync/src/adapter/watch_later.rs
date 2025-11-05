use std::path::Path;
use std::pin::Pin;

use anyhow::Result;
use bili_sync_entity::rule::Rule;
use bili_sync_entity::*;
use futures::Stream;
use sea_orm::ActiveValue::Set;
use sea_orm::entity::prelude::*;
use sea_orm::sea_query::SimpleExpr;
use sea_orm::{DatabaseConnection, Unchanged};

use crate::adapter::{_ActiveModel, VideoSource, VideoSourceEnum};
use crate::bilibili::{BiliClient, Credential, VideoInfo, WatchLater};

impl VideoSource for watch_later::Model {
    fn display_name(&self) -> std::borrow::Cow<'static, str> {
        "稍后再看".into()
    }

    fn filter_expr(&self) -> SimpleExpr {
        video::Column::WatchLaterId.eq(self.id)
    }

    fn set_relation_id(&self, video_model: &mut video::ActiveModel) {
        video_model.watch_later_id = Set(Some(self.id));
    }

    fn path(&self) -> &Path {
        Path::new(self.path.as_str())
    }

    fn get_latest_row_at(&self) -> DateTime {
        self.latest_row_at
    }

    fn update_latest_row_at(&self, datetime: DateTime) -> _ActiveModel {
        _ActiveModel::WatchLater(watch_later::ActiveModel {
            id: Unchanged(self.id),
            latest_row_at: Set(datetime),
            ..Default::default()
        })
    }

    fn rule(&self) -> &Option<Rule> {
        &self.rule
    }

    async fn refresh<'a>(
        self,
        bili_client: &'a BiliClient,
        credential: &'a Credential,
        _connection: &'a DatabaseConnection,
    ) -> Result<(
        VideoSourceEnum,
        Pin<Box<dyn Stream<Item = Result<VideoInfo>> + Send + 'a>>,
    )> {
        let watch_later = WatchLater::new(bili_client, credential);
        Ok((self.into(), Box::pin(watch_later.into_video_stream())))
    }
}
