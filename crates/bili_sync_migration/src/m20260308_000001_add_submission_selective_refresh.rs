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
                    .table(Submission::Table)
                    .add_column(boolean(Submission::SelectiveRefreshEnabled).not_null().default(false))
                    .add_column(big_unsigned_null(Submission::RefreshTtlP5))
                    .add_column(timestamp_null(Submission::LastRefreshedAt))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Submission::Table)
                    .drop_column(Submission::SelectiveRefreshEnabled)
                    .drop_column(Submission::RefreshTtlP5)
                    .drop_column(Submission::LastRefreshedAt)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum Submission {
    Table,
    SelectiveRefreshEnabled,
    RefreshTtlP5,
    LastRefreshedAt,
}
