#![allow(unused)]

use anyhow::Context;
use bili_sync_entity::*;
use bili_sync_migration::ExprTrait;
use sea_orm::prelude::*;
use sea_orm::sea_query::{IntoColumnRef, SimpleExpr};
use sea_orm::{QuerySelect, SelectColumns, UpdateResult};

use crate::utils::status::{STATUS_MAX_RETRY, STATUS_OK};

fn failed_sub_status(status_col: impl IntoColumnRef, offset: usize) -> SimpleExpr {
    let shift = offset * 3;
    Expr::col(status_col)
        .right_shift(shift as u32)
        .bit_and(0b111)
        .gte(STATUS_MAX_RETRY)
        .ne(STATUS_OK)
}

fn reset_sub_status(status_col: impl IntoColumnRef, offset: usize) -> SimpleExpr {
    let mask = !(0b111 << (offset * 3) | 1 << 31);
    Expr::col(status_col).bit_and(mask)
}

async fn reset_failed_video_status(
    offset: usize,
    database_connection: &DatabaseConnection,
) -> Result<UpdateResult, DbErr> {
    video::Entity::update_many()
        .filter(failed_sub_status(video::Column::DownloadStatus, offset))
        .col_expr(
            video::Column::DownloadStatus,
            reset_sub_status(video::Column::DownloadStatus, offset),
        )
        .exec(database_connection)
        .await
}

async fn reset_all_failed_video_status(database_connection: &DatabaseConnection) -> Result<(), DbErr> {
    // 第四位是用于标记分页状态的，不需要在 video 层级重置
    for offset in 0..4 {
        reset_failed_video_status(offset, database_connection).await?;
    }
    Ok(())
}

async fn filter_all_failed_page_status(database_connection: &DatabaseConnection) -> Result<Vec<i32>, DbErr> {
    let filter_condition = (0..=4)
        .fold(Option::<SimpleExpr>::None, |acc, offset| {
            let expr = failed_sub_status(page::Column::DownloadStatus, offset);
            if let Some(acc) = acc {
                Some(acc.or(expr))
            } else {
                Some(expr)
            }
        })
        .ok_or(DbErr::Custom("Failed to build filter condition".to_string()))?;
    page::Entity::find()
        .filter(filter_condition)
        .select_only()
        .column(page::Column::VideoId)
        .into_tuple()
        .all(database_connection)
        .await
}
