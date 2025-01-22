use sea_orm_migration::prelude::*;
use sea_orm_migration::schema::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 为四张 video list 表添加 latest_row_at 字段，表示该列表处理到的最新时间
        manager
            .alter_table(
                Table::alter()
                    .table(Favorite::Table)
                    .add_column(timestamp(Favorite::LatestRowAt).default("1970-01-01 00:00:00"))
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Collection::Table)
                    .add_column(timestamp(Collection::LatestRowAt).default("1970-01-01 00:00:00"))
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(WatchLater::Table)
                    .add_column(timestamp(WatchLater::LatestRowAt).default("1970-01-01 00:00:00"))
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Submission::Table)
                    .add_column(timestamp(Submission::LatestRowAt).default("1970-01-01 00:00:00"))
                    .to_owned(),
            )
            .await?;
        // 手动写 SQL 更新这四张表的 latest 字段到当前取值
        let db = manager.get_connection();
        db.execute_unprepared(
            "UPDATE favorite SET latest_row_at = (SELECT IFNULL(MAX(favtime), '1970-01-01 00:00:00') FROM video WHERE favorite_id = favorite.id)",
        )
        .await?;
        db.execute_unprepared(
            "UPDATE collection SET latest_row_at = (SELECT IFNULL(MAX(pubtime), '1970-01-01 00:00:00') FROM video WHERE collection_id = collection.id)",
        )
        .await?;
        db.execute_unprepared(
            "UPDATE watch_later SET latest_row_at = (SELECT IFNULL(MAX(favtime), '1970-01-01 00:00:00') FROM video WHERE watch_later_id = watch_later.id)",
        )
        .await?;
        db.execute_unprepared(
            "UPDATE submission SET latest_row_at = (SELECT IFNULL(MAX(ctime), '1970-01-01 00:00:00') FROM video WHERE submission_id = submission.id)",
        )
        .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Favorite::Table)
                    .drop_column(Favorite::LatestRowAt)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Collection::Table)
                    .drop_column(Collection::LatestRowAt)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(WatchLater::Table)
                    .drop_column(WatchLater::LatestRowAt)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Submission::Table)
                    .drop_column(Submission::LatestRowAt)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum Favorite {
    Table,
    LatestRowAt,
}

#[derive(DeriveIden)]
enum Collection {
    Table,
    LatestRowAt,
}

#[derive(DeriveIden)]
enum WatchLater {
    Table,
    LatestRowAt,
}

#[derive(DeriveIden)]
enum Submission {
    Table,
    LatestRowAt,
}
