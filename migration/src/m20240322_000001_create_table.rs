use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::{EnumIter, Iterable};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Favorite::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Favorite::Id)
                            .unsigned()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Favorite::FId)
                            .unique_key()
                            .unsigned()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Favorite::Name).string().not_null())
                    .col(ColumnDef::new(Favorite::Path).string().not_null())
                    .col(ColumnDef::new(Favorite::Enabled).boolean().not_null())
                    .col(
                        ColumnDef::new(Favorite::CreatedAt)
                            .timestamp()
                            .default(Expr::current_timestamp())
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Favorite::UpdatedAt)
                            .timestamp()
                            .default(Expr::current_timestamp())
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(Video::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Video::Id)
                            .unsigned()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Video::FavoriteId).unsigned().not_null())
                    .col(ColumnDef::new(Video::UpperId).unsigned().not_null())
                    .col(ColumnDef::new(Video::Name).string().not_null())
                    .col(ColumnDef::new(Video::Path).string().not_null())
                    .col(
                        ColumnDef::new(Video::Category)
                            .enumeration(Alias::new("category"), Category::iter())
                            .not_null(),
                    )
                    .col(ColumnDef::new(Video::Bvid).string().not_null())
                    .col(ColumnDef::new(Video::Intro).string().not_null())
                    .col(ColumnDef::new(Video::Cover).string().not_null())
                    .col(ColumnDef::new(Video::Ctime).timestamp().not_null())
                    .col(ColumnDef::new(Video::Pubtime).timestamp().not_null())
                    .col(ColumnDef::new(Video::Favtime).timestamp().not_null())
                    .col(ColumnDef::new(Video::Downloaded).boolean().not_null())
                    .col(ColumnDef::new(Video::Valid).boolean().not_null())
                    .col(ColumnDef::new(Video::Tags).json_binary().not_null())
                    .col(ColumnDef::new(Video::SinglePage).boolean().not_null())
                    .col(
                        ColumnDef::new(Video::CreatedAt)
                            .timestamp()
                            .default(Expr::current_timestamp())
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Video::UpdatedAt)
                            .timestamp()
                            .default(Expr::current_timestamp())
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(Page::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Page::Id)
                            .unsigned()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Page::VideoId).unsigned().not_null())
                    .col(ColumnDef::new(Page::Cid).unsigned().not_null())
                    .col(ColumnDef::new(Page::Pid).unsigned().not_null())
                    .col(ColumnDef::new(Page::Name).string().not_null())
                    .col(ColumnDef::new(Page::Path).string().not_null())
                    .col(ColumnDef::new(Page::Image).string().not_null())
                    .col(ColumnDef::new(Page::Valid).boolean().not_null())
                    .col(ColumnDef::new(Page::DownloadStatus).unsigned().not_null())
                    .col(ColumnDef::new(Page::Downloaded).boolean().not_null())
                    .col(
                        ColumnDef::new(Page::CreatedAt)
                            .timestamp()
                            .default(Expr::current_timestamp())
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Page::UpdatedAt)
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
                    .table(Video::Table)
                    .name("idx_video_favorite_id_bvid")
                    .col(Video::FavoriteId)
                    .col(Video::Bvid)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .table(Page::Table)
                    .name("idx_page_video_id_pid")
                    .col(Page::VideoId)
                    .col(Page::Pid)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Favorite::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Video::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Page::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum Favorite {
    Table,
    Id,
    FId,
    Name,
    Path,
    Enabled,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Video {
    Table,
    Id,
    FavoriteId,
    UpperId,
    Name,
    Path,
    Category,
    Bvid,
    Intro,
    Cover,
    Ctime,
    Pubtime,
    Favtime,
    Downloaded,
    Valid,
    Tags,
    SinglePage,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Page {
    Table,
    Id,
    VideoId,
    Cid,
    Pid,
    Name,
    Path,
    Image,
    Valid,
    DownloadStatus,
    Downloaded,
    CreatedAt,
    UpdatedAt,
}

// 参考: https://socialsisteryi.github.io/bilibili-API-collect/docs/fav/list.html#%E8%8E%B7%E5%8F%96%E6%94%B6%E8%97%8F%E5%A4%B9%E5%86%85%E5%AE%B9%E6%98%8E%E7%BB%86%E5%88%97%E8%A1%A8
#[derive(Iden, EnumIter)]
pub enum Category {
    #[iden = "2"]
    Video,
    #[iden = "12"]
    Audio,
    #[iden = "21"]
    VideoCollection,
}
