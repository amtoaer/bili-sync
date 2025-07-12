use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_index(
                Index::create()
                    .table(Video::Table)
                    .name("video_created_at_index")
                    .col(Video::CreatedAt)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .table(Video::Table)
                    .name("video_created_at_index")
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum Video {
    Table,
    CreatedAt,
}
