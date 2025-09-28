use std::borrow::Cow;
use std::path::Path;
use std::pin::Pin;

use anyhow::{Context, Result, ensure};
use bili_sync_entity::rule::Rule;
use bili_sync_entity::*;
use futures::Stream;
use sea_orm::ActiveValue::Set;
use sea_orm::entity::prelude::*;
use sea_orm::sea_query::SimpleExpr;
use sea_orm::{DatabaseConnection, Unchanged};

use crate::adapter::{_ActiveModel, VideoSource, VideoSourceEnum};
use crate::bilibili::{BiliClient, FavoriteList, VideoInfo};

impl VideoSource for favorite::Model {
    fn display_name(&self) -> Cow<'static, str> {
        format!("收藏夹「{}」", self.name).into()
    }

    fn filter_expr(&self) -> SimpleExpr {
        video::Column::FavoriteId.eq(self.id)
    }

    fn set_relation_id(&self, video_model: &mut video::ActiveModel) {
        video_model.favorite_id = Set(Some(self.id));
    }

    fn path(&self) -> &Path {
        Path::new(self.path.as_str())
    }

    fn get_latest_row_at(&self) -> DateTime {
        self.latest_row_at
    }

    fn update_latest_row_at(&self, datetime: DateTime) -> _ActiveModel {
        _ActiveModel::Favorite(favorite::ActiveModel {
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
        connection: &'a DatabaseConnection,
    ) -> Result<(
        VideoSourceEnum,
        Pin<Box<dyn Stream<Item = Result<VideoInfo>> + Send + 'a>>,
    )> {
        let favorite = FavoriteList::new(bili_client, self.f_id.to_string());
        let favorite_info = favorite.get_info().await?;
        ensure!(
            favorite_info.id == self.f_id,
            "favorite id mismatch: {} != {}",
            favorite_info.id,
            self.f_id
        );
        favorite::ActiveModel {
            id: Unchanged(self.id),
            name: Set(favorite_info.title.clone()),
            ..Default::default()
        }
        .save(connection)
        .await?;
        Ok((
            favorite::Entity::find()
                .filter(favorite::Column::Id.eq(self.id))
                .one(connection)
                .await?
                .context("favorite not found")?
                .into(),
            Box::pin(favorite.into_video_stream()),
        ))
    }
}
