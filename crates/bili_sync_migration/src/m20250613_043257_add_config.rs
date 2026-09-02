use sea_orm_migration::prelude::*;

use crate::utc_now_default;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Config::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Config::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Config::Data).text().not_null())
                    .col(
                        ColumnDef::new(Config::CreatedAt)
                            .timestamp()
                            .default(utc_now_default(manager))
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(Config::Table).to_owned()).await
    }
}

#[derive(DeriveIden)]
enum Config {
    Table,
    Id,
    Data,
    CreatedAt,
}
