use std::borrow::Cow;
use std::path::Path;
use std::pin::Pin;

use anyhow::{Result, ensure};
use bili_sync_entity::rule::Rule;
use bili_sync_entity::*;
use chrono::Utc;
use futures::Stream;
use sea_orm::ActiveValue::Set;
use sea_orm::entity::prelude::*;
use sea_orm::sea_query::SimpleExpr;
use sea_orm::{DatabaseConnection, Unchanged};

use crate::adapter::{_ActiveModel, VideoSource, VideoSourceEnum};
use crate::bilibili::{BiliClient, Collection, CollectionItem, CollectionType, Credential, VideoInfo};

impl VideoSource for collection::Model {
    fn display_name(&self) -> Cow<'static, str> {
        format!("{}「{}」", CollectionType::from_expected(self.r#type), self.name).into()
    }

    fn filter_expr(&self) -> SimpleExpr {
        video::Column::CollectionId.eq(self.id)
    }

    fn set_relation_id(&self, video_model: &mut video::ActiveModel) {
        video_model.collection_id = Set(Some(self.id));
    }

    fn path(&self) -> &Path {
        Path::new(self.path.as_str())
    }

    fn get_latest_row_at(&self) -> DateTime {
        self.latest_row_at
    }

    fn update_latest_row_at(&self, datetime: DateTime) -> _ActiveModel {
        _ActiveModel::Collection(collection::ActiveModel {
            id: Unchanged(self.id),
            latest_row_at: Set(datetime),
            ..Default::default()
        })
    }

    fn should_take(
        &self,
        _idx: usize,
        _release_datetime: &chrono::DateTime<Utc>,
        _latest_row_at: &chrono::DateTime<Utc>,
    ) -> bool {
        // collection（视频合集/视频列表）返回的内容似乎并非严格按照时间排序，并且不同 collection 的排序方式也不同
        // 为了保证程序正确性，collection 不根据时间提前 break，而是每次都全量拉取
        true
    }

    fn should_filter(
        &self,
        _idx: usize,
        video_info: Result<VideoInfo, anyhow::Error>,
        latest_row_at: &chrono::DateTime<Utc>,
    ) -> Option<VideoInfo> {
        // 由于 collection 的视频无固定时间顺序，should_take 无法提前中断拉取，因此 should_filter 环节需要进行额外过滤
        if let Ok(video_info) = video_info
            && video_info.release_datetime() > latest_row_at
        {
            return Some(video_info);
        }
        None
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
        let collection = Collection::new(
            bili_client,
            CollectionItem {
                sid: self.s_id.to_string(),
                mid: self.m_id.to_string(),
                collection_type: CollectionType::from_expected(self.r#type),
            },
            credential,
        );
        let collection_info = collection.get_info().await?;
        ensure!(
            collection_info.sid == self.s_id
                && collection_info.mid == self.m_id
                && collection_info.collection_type == CollectionType::from_expected(self.r#type),
            "collection info mismatch: {:?} != {:?}",
            collection_info,
            collection.collection
        );
        let updated_model = collection::ActiveModel {
            id: Unchanged(self.id),
            name: Set(collection_info.name),
            ..Default::default()
        }
        .update(connection)
        .await?;
        Ok((updated_model.into(), Box::pin(collection.into_video_stream())))
    }
}
