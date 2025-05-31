use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 创建 source_favorite 表
        manager
            .create_table(
                Table::create()
                    .table(SourceFavorite::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(SourceFavorite::Id)
                        .unsigned()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(SourceFavorite::FId).unique_key().unsigned().not_null())
                    .col(ColumnDef::new(SourceFavorite::Path).string().not_null())
                    .col(ColumnDef::new(SourceFavorite::Description).string().not_null())
                    .col(ColumnDef::new(SourceFavorite::Enabled).small_unsigned().default(0).not_null())
                    .col(
                        ColumnDef::new(SourceFavorite::CreatedAt)
                            .timestamp()
                            .default(Expr::current_timestamp())
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        // 创建 source_collection 表及索引
        manager
            .create_table(
                Table::create()
                    .table(SourceCollection::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(SourceCollection::Id)
                            .unsigned()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(SourceCollection::SId).unsigned().not_null())
                    .col(ColumnDef::new(SourceCollection::MId).unsigned().not_null())
                    .col(ColumnDef::new(SourceCollection::Type).small_unsigned().not_null())
                    .col(ColumnDef::new(SourceCollection::Path).string().not_null())
                    .col(ColumnDef::new(SourceCollection::Description).string().not_null())
                    .col(ColumnDef::new(SourceCollection::Enabled).small_unsigned().default(0).not_null())
                    .col(
                        ColumnDef::new(SourceCollection::CreatedAt)
                            .timestamp()
                            .default(Expr::current_timestamp())
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(SourceCollection::Table)
                    .name("idx_source_collection_sid_mid_type")
                    .col(SourceCollection::SId)
                    .col(SourceCollection::MId)
                    .col(SourceCollection::Type)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // 创建 source_submission 表（注意：存在重复 path 字段需要修正）
        manager
            .create_table(
                Table::create()
                    .table(SourceSubmission::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(SourceSubmission::Id)
                            .unsigned()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(SourceSubmission::UpperId).unsigned().unique_key().not_null())
                    .col(ColumnDef::new(SourceSubmission::Path).string().not_null())
                    .col(ColumnDef::new(SourceSubmission::Description).string().not_null())
                    .col(ColumnDef::new(SourceSubmission::Enabled).small_unsigned().default(0).not_null())
                    .col(
                        ColumnDef::new(SourceSubmission::CreatedAt)
                            .timestamp()
                            .default(Expr::current_timestamp())
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        // 创建 source_watch_later 表
        manager
            .create_table(
                Table::create()
                    .table(SourceWatchLater::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(SourceWatchLater::Id)
                            .unsigned()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(SourceWatchLater::Path).string().not_null())
                    .col(ColumnDef::new(SourceWatchLater::Description).string().not_null())
                    .col(ColumnDef::new(SourceWatchLater::Enabled).small_unsigned().default(0).not_null())
                    .col(
                        ColumnDef::new(SourceWatchLater::CreatedAt)
                            .timestamp()
                            .default(Expr::current_timestamp())
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 逆序删除表结构
        manager
            .drop_table(Table::drop().table(SourceWatchLater::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(SourceSubmission::Table).to_owned())
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .table(SourceCollection::Table)
                    .name("idx_source_collection_sid_mid_type")
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(Table::drop().table(SourceCollection::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(SourceFavorite::Table).to_owned())
            .await
    }
}

// 表字段枚举定义
#[derive(DeriveIden)]
enum SourceFavorite {
    Table,
    Id,
    FId,
    Path,
    Description,
    Enabled,
    CreatedAt,
}

#[derive(DeriveIden)]
enum SourceCollection {
    Table,
    Id,
    SId,
    MId,
    Type,
    Path,
    Description,
    Enabled,
    CreatedAt,
}

#[derive(DeriveIden)]
enum SourceSubmission {
    Table,
    Id,
    UpperId,
    Path,
    Description,
    Enabled,
    CreatedAt,
}

#[derive(DeriveIden)]
enum SourceWatchLater {
    Table,
    Id,
    Path,
    Description,
    Enabled,
    CreatedAt,
}