use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        manager
            .create_table(
                Table::create()
                    .table(Collection::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Collection::Id)
                            .unsigned()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Collection::SId).unsigned().not_null())
                    .col(ColumnDef::new(Collection::MId).unsigned().not_null())
                    .col(ColumnDef::new(Collection::Name).string().not_null())
                    .col(ColumnDef::new(Collection::Type).small_unsigned().not_null())
                    .col(ColumnDef::new(Collection::Path).string().not_null())
                    .col(
                        ColumnDef::new(Collection::CreatedAt)
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
                    .table(Collection::Table)
                    .name("idx_collection_sid_mid_type")
                    .col(Collection::SId)
                    .col(Collection::MId)
                    .col(Collection::Type)
                    .unique()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .table(Video::Table)
                    .name("idx_video_favorite_id_bvid")
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Video::Table)
                    .add_column(ColumnDef::new(Video::CollectionId).unsigned().null())
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Video::Table)
                    .add_column(ColumnDef::new(Video::TempFavoriteId).unsigned().null())
                    .to_owned(),
            )
            .await?;
        db.execute_unprepared("UPDATE video SET temp_favorite_id = favorite_id")
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Video::Table)
                    .drop_column(Video::FavoriteId)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Video::Table)
                    .rename_column(Video::TempFavoriteId, Video::FavoriteId)
                    .to_owned(),
            )
            .await?;
        // 在唯一索引中，NULL 不等于 NULL，所以需要使用 ifnull 函数排除空的情况
        db.execute_unprepared("CREATE UNIQUE INDEX `idx_video_cid_fid_bvid` ON `video` (ifnull(`collection_id`, -1), ifnull(`favorite_id`, -1), `bvid`)")
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        manager
            .drop_index(
                Index::drop()
                    .table(Video::Table)
                    .name("idx_video_cid_fid_bvid")
                    .to_owned(),
            )
            .await?;
        db.execute_unprepared("DELETE FROM video WHERE favorite_id IS NULL")
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Video::Table)
                    // 向存在记录的表中添加非空列时，必须提供默认值
                    .add_column(ColumnDef::new(Video::TempFavoriteId).unsigned().not_null().default(0))
                    .to_owned(),
            )
            .await?;
        db.execute_unprepared("UPDATE video SET temp_favorite_id = favorite_id")
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Video::Table)
                    .drop_column(Video::FavoriteId)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Video::Table)
                    .rename_column(Video::TempFavoriteId, Video::FavoriteId)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Video::Table)
                    .drop_column(Video::CollectionId)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .table(Video::Table)
                    .name("idx_video_favorite_id_bvid")
                    .col(Video::FavoriteId)
                    .col(Video::Bvid)
                    .unique()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(Table::drop().table(Collection::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Collection {
    Table,
    Id,
    SId,
    MId,
    Name,
    Type,
    Path,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Video {
    Table,
    FavoriteId,
    TempFavoriteId,
    CollectionId,
    Bvid,
}
