use std::sync::Arc;

use anyhow::{Context, Result, bail};
use chrono::NaiveDate;
use sea_orm::DatabaseConnection;
use tokio::time;

use crate::adapter::VideoSource;
use crate::bilibili::{self, BiliClient, BiliError};
use crate::config::{Config, TEMPLATE, VersionedConfig};
use crate::notifier::NotifierAllExt;
use crate::utils::model::get_enabled_video_sources;
use crate::utils::task_notifier::TASK_STATUS_NOTIFIER;
use crate::workflow::process_video_source;

/// 启动周期下载视频的任务
pub async fn video_downloader(connection: DatabaseConnection, bili_client: Arc<BiliClient>) {
    let mut anchor = chrono::Local::now().date_naive();
    loop {
        let _lock = TASK_STATUS_NOTIFIER.start_running().await;
        let mut config = VersionedConfig::get().snapshot();
        info!("开始执行本轮视频下载任务..");
        if let Err(e) = download_all_video_sources(&connection, &bili_client, &mut config, &mut anchor).await {
            let error_msg = format!("本轮视频下载任务执行遇到错误：{:#}", e);
            error!("{error_msg}");
            let _ = config
                .notifiers
                .notify_all(bili_client.inner_client(), &error_msg)
                .await;
        } else {
            info!("本轮视频下载任务执行完毕");
        }
        TASK_STATUS_NOTIFIER.finish_running(_lock, config.interval as i64);
        time::sleep(time::Duration::from_secs(config.interval)).await;
    }
}

async fn download_all_video_sources(
    connection: &DatabaseConnection,
    bili_client: &BiliClient,
    config: &mut Arc<Config>,
    anchor: &mut NaiveDate,
) -> Result<()> {
    config.check().context("配置检查失败")?;
    let mixin_key = bili_client
        .wbi_img(&config.credential)
        .await
        .context("获取 wbi_img 失败")?
        .into_mixin_key()
        .context("解析 mixin key 失败")?;
    bilibili::set_global_mixin_key(mixin_key);
    if *anchor != chrono::Local::now().date_naive() {
        if let Some(new_credential) = bili_client
            .check_refresh(&config.credential)
            .await
            .context("检查刷新 Credential 失败")?
        {
            *config = VersionedConfig::get()
                .update_credential(new_credential, connection)
                .await
                .context("更新 Credential 失败")?;
        }
        *anchor = chrono::Local::now().date_naive();
    }
    let template = TEMPLATE.snapshot();
    let bili_client = bili_client.snapshot()?;
    let video_sources = get_enabled_video_sources(connection)
        .await
        .context("获取视频源列表失败")?;
    if video_sources.is_empty() {
        bail!("没有可用的视频源");
    }
    for video_source in video_sources {
        let display_name = video_source.display_name();
        if let Err(e) = process_video_source(video_source, &bili_client, connection, &template, config).await {
            let error_msg = format!("处理 {} 时遇到错误：{:#}，跳过该视频源", display_name, e);
            error!("{error_msg}");
            let _ = config
                .notifiers
                .notify_all(bili_client.inner_client(), &error_msg)
                .await;
            if let Ok(e) = e.downcast::<BiliError>()
                && e.is_risk_control_related()
            {
                warn!("检测到风控，终止此轮视频下载任务..");
                break;
            }
        }
    }
    Ok(())
}
