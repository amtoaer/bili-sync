//! 视频源实体定义

use sea_orm::entity::prelude::*;
use sea_orm::ActiveModelBehavior;
use strum::{Display, EnumString, EnumIter};

#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumIter, Display, EnumString)]
#[derive(sea_orm::DeriveActiveEnum)]
#[sea_orm(rs_type = "i32", db_type = "Integer")]
pub enum SourceType {
    #[sea_orm(num_value = 1)]
    Bangumi = 1,
}

// 实现 Default trait
impl Default for SourceType {
    fn default() -> Self {
        SourceType::Bangumi
    }
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Default)]
#[sea_orm(table_name = "video_source")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub name: String,
    pub path: String,
    pub r#type: i32,
    pub latest_row_at: DateTime,
    pub season_id: Option<String>,
    pub media_id: Option<String>,
    pub ep_id: Option<String>,
    pub download_all_seasons: Option<bool>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {} 