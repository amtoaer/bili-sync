use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::DatabaseBackend;

use crate::utc_now_default;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        manager
            .create_table(
                Table::create()
                    .table(Submission::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Submission::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Submission::UpperId)
                            .unique_key()
                            .big_unsigned()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Submission::UpperName).string().not_null())
                    .col(ColumnDef::new(Submission::Path).string().not_null())
                    .col(
                        ColumnDef::new(Submission::CreatedAt)
                            .timestamp()
                            .default(utc_now_default(manager))
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(Index::drop().table(Video::Table).name("idx_video_unique").to_owned())
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Video::Table)
                    .add_column(ColumnDef::new(Video::SubmissionId).unsigned().null())
                    .to_owned(),
            )
            .await?;
        match manager.get_database_backend() {
            DatabaseBackend::Sqlite => {
                db.execute_unprepared("CREATE UNIQUE INDEX `idx_video_unique` ON `video` (ifnull(`collection_id`, -1), ifnull(`favorite_id`, -1), ifnull(`watch_later_id`, -1), ifnull(`submission_id`, -1), `bvid`)")
                    .await?;
            }
            DatabaseBackend::Postgres => {
                db.execute_unprepared("CREATE UNIQUE INDEX idx_video_unique ON video (COALESCE(collection_id, -1), COALESCE(favorite_id, -1), COALESCE(watch_later_id, -1), COALESCE(submission_id, -1), bvid)")
                    .await?;
            }
            _ => unreachable!(),
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        manager
            .drop_index(Index::drop().table(Video::Table).name("idx_video_unique").to_owned())
            .await?;
        db.execute_unprepared("DELETE FROM video WHERE submission_id IS NOT NULL")
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Video::Table)
                    .drop_column(Video::SubmissionId)
                    .to_owned(),
            )
            .await?;
        match manager.get_database_backend() {
            DatabaseBackend::Sqlite => {
                db.execute_unprepared("CREATE UNIQUE INDEX `idx_video_unique` ON `video` (ifnull(`collection_id`, -1), ifnull(`favorite_id`, -1), ifnull(`watch_later_id`, -1), `bvid`)")
                    .await?;
            }
            DatabaseBackend::Postgres => {
                db.execute_unprepared("CREATE UNIQUE INDEX idx_video_unique ON video (COALESCE(collection_id, -1), COALESCE(favorite_id, -1), COALESCE(watch_later_id, -1), bvid)")
                    .await?;
            }
            _ => unreachable!(),
        }
        manager
            .drop_table(Table::drop().table(Submission::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Submission {
    Table,
    Id,
    UpperId,
    UpperName,
    Path,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Video {
    Table,
    SubmissionId,
}
