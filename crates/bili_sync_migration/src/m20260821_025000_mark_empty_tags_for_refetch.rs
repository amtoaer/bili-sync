use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::DatabaseBackend;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // PostgreSQL 的 jsonb 列比较需要显式类型转换
        match manager.get_database_backend() {
            DatabaseBackend::Sqlite => {
                manager
                    .get_connection()
                    .execute_unprepared("UPDATE video SET tags = NULL WHERE tags = '[]'")
                    .await?;
            }
            DatabaseBackend::Postgres => {
                manager
                    .get_connection()
                    .execute_unprepared("UPDATE video SET tags = NULL WHERE tags = '[]'::jsonb")
                    .await?;
            }
            _ => unreachable!(),
        }
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
