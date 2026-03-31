//! `SeaORM` Entity.

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Default)]
#[sea_orm(table_name = "youtube_channel")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub source_type: String,
    pub channel_id: String,
    pub name: String,
    pub url: String,
    pub thumbnail: Option<String>,
    pub path: String,
    pub latest_published_at: Option<DateTime>,
    pub enabled: bool,
    pub created_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::youtube_video::Entity")]
    YoutubeVideo,
}

impl Related<super::youtube_video::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::YoutubeVideo.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
