use sea_orm_migration::prelude::*;
use sea_orm_migration::schema::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for table in VideoSource::tables() {
            manager
                .alter_table(
                    Table::alter()
                        .table(table)
                        .add_column(json_null(VideoSource::FilterOption))
                        .to_owned(),
                )
                .await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for table in VideoSource::tables() {
            manager
                .alter_table(
                    Table::alter()
                        .table(table)
                        .drop_column(VideoSource::FilterOption)
                        .to_owned(),
                )
                .await?;
        }
        Ok(())
    }
}

#[derive(DeriveIden)]
enum VideoSource {
    WatchLater,
    Submission,
    Favorite,
    Collection,
    FilterOption,
}

impl VideoSource {
    fn tables() -> [Self; 4] {
        [Self::WatchLater, Self::Submission, Self::Favorite, Self::Collection]
    }
}
