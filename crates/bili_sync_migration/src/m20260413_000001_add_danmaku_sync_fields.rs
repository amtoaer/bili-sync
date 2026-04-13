use sea_orm_migration::prelude::*;
use sea_orm_migration::schema::*;

/// 为 page 表新增"弹幕增量更新"所需字段。
///
/// - `danmaku_last_synced_at`: 上次弹幕成功同步的时间戳（含首次下载成功），为空表示从未同步过。
/// - `danmaku_sync_generation`: 弹幕同步阶段标记。0=未开始，1=新鲜期，2=成熟期，3=老化期，4=已冻结。
/// - `danmaku_cid_snapshot`: 上次成功同步时使用的 cid，用于 UP 主换源检测。
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // SQLite 不支持单条 ALTER TABLE 同时修改多列，必须拆分为独立语句。
        manager
            .alter_table(
                Table::alter()
                    .table(Page::Table)
                    .add_column(timestamp_null(Page::DanmakuLastSyncedAt))
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Page::Table)
                    .add_column(
                        ColumnDef::new(Page::DanmakuSyncGeneration)
                            .unsigned()
                            .not_null()
                            .default(0u32),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Page::Table)
                    .add_column(big_integer_null(Page::DanmakuCidSnapshot))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Page::Table)
                    .drop_column(Page::DanmakuLastSyncedAt)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Page::Table)
                    .drop_column(Page::DanmakuSyncGeneration)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Page::Table)
                    .drop_column(Page::DanmakuCidSnapshot)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum Page {
    Table,
    DanmakuLastSyncedAt,
    DanmakuSyncGeneration,
    DanmakuCidSnapshot,
}
