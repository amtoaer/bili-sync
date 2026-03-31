//! `SeaORM` Entity.

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Default)]
#[sea_orm(table_name = "youtube_video")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub youtube_channel_id: i32,
    pub video_id: String,
    pub title: String,
    pub url: String,
    pub description: String,
    pub uploader: String,
    pub thumbnail: Option<String>,
    pub published_at: DateTime,
    pub download_status: u32,
    pub valid: bool,
    pub should_download: bool,
    pub path: Option<String>,
    pub created_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::youtube_channel::Entity",
        from = "Column::YoutubeChannelId",
        to = "super::youtube_channel::Column::Id"
    )]
    YoutubeChannel,
}

impl Related<super::youtube_channel::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::YoutubeChannel.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
