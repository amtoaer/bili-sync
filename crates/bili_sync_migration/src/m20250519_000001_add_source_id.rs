use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        
        // 创建 video_source 表
        manager
            .create_table(
                Table::create()
                    .table(VideoSource::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(VideoSource::Id)
                            .unsigned()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(VideoSource::Name).string().not_null())
                    .col(ColumnDef::new(VideoSource::Path).string().not_null())
                    .col(ColumnDef::new(VideoSource::Type).integer().not_null())
                    .col(ColumnDef::new(VideoSource::LatestRowAt).timestamp().not_null().default("1970-01-01 00:00:00"))
                    .col(ColumnDef::new(VideoSource::SeasonId).string().null())
                    .col(ColumnDef::new(VideoSource::MediaId).string().null())
                    .col(ColumnDef::new(VideoSource::EpId).string().null())
                    .col(
                        ColumnDef::new(VideoSource::CreatedAt)
                            .timestamp()
                            .default(Expr::current_timestamp())
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;
            
        // 修改 video 表，添加相关列
        // 使用 SQL 语句直接删除索引，加上 IF EXISTS 子句
        db.execute_unprepared("DROP INDEX IF EXISTS idx_video_unique")
            .await?;
        db.execute_unprepared("DROP INDEX IF EXISTS idx_video_cid_fid_bvid")
            .await?;
        db.execute_unprepared("DROP INDEX IF EXISTS idx_video_favorite_id_bvid")
            .await?;
        
        // SQLite 不支持在单个 ALTER TABLE 语句中添加多个列，所以分开执行
        manager
            .alter_table(
                Table::alter()
                    .table(Video::Table)
                    .add_column(ColumnDef::new(Video::SourceId).unsigned().null())
                    .to_owned(),
            )
            .await?;
            
        manager
            .alter_table(
                Table::alter()
                    .table(Video::Table)
                    .add_column(ColumnDef::new(Video::SourceType).integer().null())
                    .to_owned(),
            )
            .await?;
            
        manager
            .alter_table(
                Table::alter()
                    .table(Video::Table)
                    .add_column(ColumnDef::new(Video::SeasonId).string().null())
                    .to_owned(),
            )
            .await?;
            
        manager
            .alter_table(
                Table::alter()
                    .table(Video::Table)
                    .add_column(ColumnDef::new(Video::MediaId).string().null())
                    .to_owned(),
            )
            .await?;
            
        manager
            .alter_table(
                Table::alter()
                    .table(Video::Table)
                    .add_column(ColumnDef::new(Video::EpId).string().null())
                    .to_owned(),
            )
            .await?;
            
        // 更新唯一索引
        db.execute_unprepared("CREATE UNIQUE INDEX `idx_video_unique` ON `video` (ifnull(`collection_id`, -1), ifnull(`favorite_id`, -1), ifnull(`watch_later_id`, -1), ifnull(`submission_id`, -1), ifnull(`source_id`, -1), `bvid`)")
            .await?;
            
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        
        // 删除索引，再删除列
        db.execute_unprepared("DROP INDEX IF EXISTS idx_video_unique")
            .await?;
        
        // 删除所有使用 source_id 的记录
        db.execute_unprepared("DELETE FROM video WHERE source_id IS NOT NULL")
            .await?;
        
        // 删除 source_id 和 source_type 列及番剧相关列 - 分开执行
        manager
            .alter_table(
                Table::alter()
                    .table(Video::Table)
                    .drop_column(Video::SourceId)
                    .to_owned(),
            )
            .await?;
            
        manager
            .alter_table(
                Table::alter()
                    .table(Video::Table)
                    .drop_column(Video::SourceType)
                    .to_owned(),
            )
            .await?;
            
        manager
            .alter_table(
                Table::alter()
                    .table(Video::Table)
                    .drop_column(Video::SeasonId)
                    .to_owned(),
            )
            .await?;
            
        manager
            .alter_table(
                Table::alter()
                    .table(Video::Table)
                    .drop_column(Video::MediaId)
                    .to_owned(),
            )
            .await?;
            
        manager
            .alter_table(
                Table::alter()
                    .table(Video::Table)
                    .drop_column(Video::EpId)
                    .to_owned(),
            )
            .await?;
        
        // 恢复不包含 source_id 的索引
        db.execute_unprepared("CREATE UNIQUE INDEX `idx_video_unique` ON `video` (ifnull(`collection_id`, -1), ifnull(`favorite_id`, -1), ifnull(`watch_later_id`, -1), ifnull(`submission_id`, -1), `bvid`)")
            .await?;
        
        // 删除 video_source 表
        manager
            .drop_table(Table::drop().table(VideoSource::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum VideoSource {
    Table,
    Id,
    Name,
    Path,
    Type,
    LatestRowAt,
    SeasonId,
    MediaId,
    EpId,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Video {
    Table,
    SourceId,
    SourceType,
    SeasonId,
    MediaId,
    EpId,
} 