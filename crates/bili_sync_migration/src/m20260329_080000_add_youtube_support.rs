use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(YoutubeChannel::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(YoutubeChannel::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(YoutubeChannel::ChannelId).string().not_null())
                    .col(ColumnDef::new(YoutubeChannel::Name).string().not_null())
                    .col(ColumnDef::new(YoutubeChannel::Url).string().not_null())
                    .col(ColumnDef::new(YoutubeChannel::Thumbnail).string())
                    .col(ColumnDef::new(YoutubeChannel::Path).string().not_null())
                    .col(ColumnDef::new(YoutubeChannel::LatestPublishedAt).timestamp())
                    .col(
                        ColumnDef::new(YoutubeChannel::Enabled)
                            .boolean()
                            .default(true)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(YoutubeChannel::CreatedAt)
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
                    .table(YoutubeVideo::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(YoutubeVideo::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(YoutubeVideo::YoutubeChannelId).integer().not_null())
                    .col(ColumnDef::new(YoutubeVideo::VideoId).string().not_null())
                    .col(ColumnDef::new(YoutubeVideo::Title).string().not_null())
                    .col(ColumnDef::new(YoutubeVideo::Url).string().not_null())
                    .col(ColumnDef::new(YoutubeVideo::Description).string().not_null())
                    .col(ColumnDef::new(YoutubeVideo::Uploader).string().not_null())
                    .col(ColumnDef::new(YoutubeVideo::Thumbnail).string())
                    .col(ColumnDef::new(YoutubeVideo::PublishedAt).timestamp().not_null())
                    .col(ColumnDef::new(YoutubeVideo::DownloadStatus).unsigned().not_null())
                    .col(ColumnDef::new(YoutubeVideo::Valid).boolean().default(true).not_null())
                    .col(
                        ColumnDef::new(YoutubeVideo::ShouldDownload)
                            .boolean()
                            .default(true)
                            .not_null(),
                    )
                    .col(ColumnDef::new(YoutubeVideo::Path).string())
                    .col(
                        ColumnDef::new(YoutubeVideo::CreatedAt)
                            .timestamp()
                            .default(Expr::current_timestamp())
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_youtube_video_channel")
                            .from(YoutubeVideo::Table, YoutubeVideo::YoutubeChannelId)
                            .to(YoutubeChannel::Table, YoutubeChannel::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(YoutubeChannel::Table)
                    .name("idx_youtube_channel_channel_id")
                    .col(YoutubeChannel::ChannelId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(YoutubeVideo::Table)
                    .name("idx_youtube_video_video_id")
                    .col(YoutubeVideo::VideoId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(YoutubeVideo::Table)
                    .name("idx_youtube_video_channel_published_at")
                    .col(YoutubeVideo::YoutubeChannelId)
                    .col(YoutubeVideo::PublishedAt)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(YoutubeVideo::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(YoutubeChannel::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum YoutubeChannel {
    Table,
    Id,
    ChannelId,
    Name,
    Url,
    Thumbnail,
    Path,
    LatestPublishedAt,
    Enabled,
    CreatedAt,
}

#[derive(DeriveIden)]
enum YoutubeVideo {
    Table,
    Id,
    YoutubeChannelId,
    VideoId,
    Title,
    Url,
    Description,
    Uploader,
    Thumbnail,
    PublishedAt,
    DownloadStatus,
    Valid,
    ShouldDownload,
    Path,
    CreatedAt,
}
