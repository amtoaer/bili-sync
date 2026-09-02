pub use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::DatabaseBackend;

/// created_at 的数据库默认值：SQLite 的 CURRENT_TIMESTAMP 返回 UTC 文本；
/// PostgreSQL 上 CURRENT_TIMESTAMP 是 timestamptz，写入 timestamp 列时按会话
/// 时区转墙钟，需显式换算出 UTC 以保证两后端存储语义一致
pub fn utc_now_default(manager: &SchemaManager) -> SimpleExpr {
    if manager.get_database_backend() == DatabaseBackend::Postgres {
        Expr::cust("(CURRENT_TIMESTAMP AT TIME ZONE 'UTC')::timestamp")
    } else {
        Expr::current_timestamp().into()
    }
}

mod m20240322_000001_create_table;
mod m20240505_130850_add_collection;
mod m20240709_130914_watch_later;
mod m20240724_161008_submission;
mod m20250122_062926_add_latest_row_at;
mod m20250612_090826_add_enabled;
mod m20250613_043257_add_config;
mod m20250712_080013_add_video_created_at_index;
mod m20250903_094454_add_rule_and_should_download;
mod m20251009_123713_add_use_dynamic_api;
mod m20260324_055217_add_staff;
mod m20260712_123205_add_filter_option;
mod m20260821_025000_mark_empty_tags_for_refetch;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20240322_000001_create_table::Migration),
            Box::new(m20240505_130850_add_collection::Migration),
            Box::new(m20240709_130914_watch_later::Migration),
            Box::new(m20240724_161008_submission::Migration),
            Box::new(m20250122_062926_add_latest_row_at::Migration),
            Box::new(m20250612_090826_add_enabled::Migration),
            Box::new(m20250613_043257_add_config::Migration),
            Box::new(m20250712_080013_add_video_created_at_index::Migration),
            Box::new(m20250903_094454_add_rule_and_should_download::Migration),
            Box::new(m20251009_123713_add_use_dynamic_api::Migration),
            Box::new(m20260324_055217_add_staff::Migration),
            Box::new(m20260712_123205_add_filter_option::Migration),
            Box::new(m20260821_025000_mark_empty_tags_for_refetch::Migration),
        ]
    }
}
