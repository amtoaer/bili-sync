use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(YoutubeChannel::Table)
                    .add_column(
                        ColumnDef::new(YoutubeChannel::SourceType)
                            .string()
                            .not_null()
                            .default("channel"),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .table(YoutubeChannel::Table)
                    .name("idx_youtube_channel_channel_id")
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .table(YoutubeVideo::Table)
                    .name("idx_youtube_video_video_id")
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(YoutubeChannel::Table)
                    .name("idx_youtube_channel_source_type_channel_id")
                    .col(YoutubeChannel::SourceType)
                    .col(YoutubeChannel::ChannelId)
                    .unique()
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .table(YoutubeVideo::Table)
                    .name("idx_youtube_video_source_video_id")
                    .col(YoutubeVideo::YoutubeChannelId)
                    .col(YoutubeVideo::VideoId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared("DELETE FROM youtube_channel WHERE source_type = 'playlist'")
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .table(YoutubeChannel::Table)
                    .name("idx_youtube_channel_source_type_channel_id")
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .table(YoutubeVideo::Table)
                    .name("idx_youtube_video_source_video_id")
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(YoutubeChannel::Table)
                    .drop_column(YoutubeChannel::SourceType)
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

        Ok(())
    }
}

#[derive(DeriveIden)]
enum YoutubeChannel {
    Table,
    SourceType,
    ChannelId,
}

#[derive(DeriveIden)]
enum YoutubeVideo {
    Table,
    YoutubeChannelId,
    VideoId,
}
