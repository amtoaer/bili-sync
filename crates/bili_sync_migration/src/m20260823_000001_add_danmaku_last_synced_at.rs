use sea_orm_migration::prelude::*;
use sea_orm_migration::schema::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Page::Table)
                    .add_column(timestamp_null(Page::DanmakuLastSyncedAt))
                    .to_owned(),
            )
            .await?;
        manager
            .get_connection()
            .execute_unprepared(
                "UPDATE page SET danmaku_last_synced_at = CURRENT_TIMESTAMP WHERE ((download_status >> 9) & 7) = 7",
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Page::Table)
                    .drop_column(Page::DanmakuLastSyncedAt)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum Page {
    Table,
    DanmakuLastSyncedAt,
}
