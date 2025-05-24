use std::sync::Arc;

use anyhow::{Result, anyhow};
use axum::extract::{Extension, Path, Query};
use bili_sync_entity::*;
use bili_sync_migration::{Expr, OnConflict};
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, Set, TransactionTrait, Unchanged,
};
use utoipa::OpenApi;

use crate::api::auth::OpenAPIAuth;
use crate::api::error::InnerApiError;
use crate::api::request::{VideosRequest, AddVideoSourceRequest};
use crate::api::response::{
    PageInfo, ResetVideoResponse, VideoInfo, VideoResponse, VideoSource, VideoSourcesResponse, VideosResponse,
    AddVideoSourceResponse, DeleteVideoSourceResponse,
};
use crate::api::wrapper::{ApiError, ApiResponse};
use crate::utils::status::{PageStatus, VideoStatus};

use std::fs;

#[derive(OpenApi)]
#[openapi(
    paths(get_video_sources, get_videos, get_video, reset_video, add_video_source, delete_video_source, reload_config),
    modifiers(&OpenAPIAuth),
    security(
        ("Token" = []),
    )
)]
pub struct ApiDoc;

/// 列出所有视频来源
#[utoipa::path(
    get,
    path = "/api/video-sources",
    responses(
        (status = 200, body = ApiResponse<VideoSourcesResponse>),
    )
)]
pub async fn get_video_sources(
    Extension(db): Extension<Arc<DatabaseConnection>>,
) -> Result<ApiResponse<VideoSourcesResponse>, ApiError> {
    // 获取各类视频源
    let collection_sources = collection::Entity::find()
            .select_only()
            .columns([collection::Column::Id, collection::Column::Name])
            .into_model::<VideoSource>()
            .all(db.as_ref())
        .await?;
        
    let favorite_sources = favorite::Entity::find()
            .select_only()
            .columns([favorite::Column::Id, favorite::Column::Name])
            .into_model::<VideoSource>()
            .all(db.as_ref())
        .await?;
        
    let submission_sources = submission::Entity::find()
            .select_only()
            .column(submission::Column::Id)
            .column_as(submission::Column::UpperName, "name")
            .into_model::<VideoSource>()
            .all(db.as_ref())
        .await?;
        
    let watch_later_sources = watch_later::Entity::find()
            .select_only()
            .column(watch_later::Column::Id)
            .column_as(Expr::value("稍后再看"), "name")
            .into_model::<VideoSource>()
            .all(db.as_ref())
        .await?;
        
    // 确保bangumi_sources是一个数组，即使为空
    let bangumi_sources = video_source::Entity::find()
            .filter(video_source::Column::Type.eq(1))
            .select_only()
            .columns([video_source::Column::Id, video_source::Column::Name])
            .into_model::<VideoSource>()
            .all(db.as_ref())
        .await?;
    
    // 返回响应，确保每个分类都是一个数组
    Ok(ApiResponse::ok(VideoSourcesResponse {
        collection: collection_sources,
        favorite: favorite_sources,
        submission: submission_sources,
        watch_later: watch_later_sources,
        bangumi: bangumi_sources,
    }))
}

/// 列出视频的基本信息，支持根据视频来源筛选、名称查找和分页
#[utoipa::path(
    get,
    path = "/api/videos",
    params(
        VideosRequest,
    ),
    responses(
        (status = 200, body = ApiResponse<VideosResponse>),
    )
)]
pub async fn get_videos(
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Query(params): Query<VideosRequest>,
) -> Result<ApiResponse<VideosResponse>, ApiError> {
    let mut query = video::Entity::find();
    
    // 直接检查是否存在bangumi参数，单独处理
    if let Some(id) = params.bangumi {
        query = query.filter(
            video::Column::SourceId.eq(id).and(video::Column::SourceType.eq(1))
        );
    } else {
        // 处理其他常规类型
    for (field, column) in [
        (params.collection, video::Column::CollectionId),
        (params.favorite, video::Column::FavoriteId),
        (params.submission, video::Column::SubmissionId),
        (params.watch_later, video::Column::WatchLaterId),
    ] {
        if let Some(id) = field {
            query = query.filter(column.eq(id));
            }
        }
    }
    if let Some(query_word) = params.query {
        query = query.filter(video::Column::Name.contains(query_word));
    }
    let total_count = query.clone().count(db.as_ref()).await?;
    let (page, page_size) = if let (Some(page), Some(page_size)) = (params.page, params.page_size) {
        (page, page_size)
    } else {
        (1, 10)
    };
    Ok(ApiResponse::ok(VideosResponse {
        videos: query
            .order_by_desc(video::Column::Id)
            .select_only()
            .columns([
                video::Column::Id,
                video::Column::Name,
                video::Column::UpperName,
                video::Column::DownloadStatus,
            ])
            .into_tuple::<(i32, String, String, u32)>()
            .paginate(db.as_ref(), page_size)
            .fetch_page(page)
            .await?
            .into_iter()
            .map(VideoInfo::from)
            .collect(),
        total_count,
    }))
}

/// 获取视频详细信息，包括关联的所有 page
#[utoipa::path(
    get,
    path = "/api/videos/{id}",
    responses(
        (status = 200, body = ApiResponse<VideoResponse>),
    )
)]
pub async fn get_video(
    Path(id): Path<i32>,
    Extension(db): Extension<Arc<DatabaseConnection>>,
) -> Result<ApiResponse<VideoResponse>, ApiError> {
    let video_info = video::Entity::find_by_id(id)
        .select_only()
        .columns([
            video::Column::Id,
            video::Column::Name,
            video::Column::UpperName,
            video::Column::DownloadStatus,
        ])
        .into_tuple::<(i32, String, String, u32)>()
        .one(db.as_ref())
        .await?
        .map(VideoInfo::from);
    let Some(video_info) = video_info else {
        return Err(InnerApiError::NotFound(id).into());
    };
    let pages = page::Entity::find()
        .filter(page::Column::VideoId.eq(id))
        .order_by_asc(page::Column::Pid)
        .select_only()
        .columns([
            page::Column::Id,
            page::Column::Pid,
            page::Column::Name,
            page::Column::DownloadStatus,
        ])
        .into_tuple::<(i32, i32, String, u32)>()
        .all(db.as_ref())
        .await?
        .into_iter()
        .map(PageInfo::from)
        .collect();
    Ok(ApiResponse::ok(VideoResponse {
        video: video_info,
        pages,
    }))
}

/// 将某个视频与其所有分页的失败状态清空为未下载状态，这样在下次下载任务中会触发重试
#[utoipa::path(
    post,
    path = "/api/videos/{id}/reset",
    responses(
        (status = 200, body = ApiResponse<ResetVideoResponse> ),
    )
)]
pub async fn reset_video(
    Path(id): Path<i32>,
    Extension(db): Extension<Arc<DatabaseConnection>>,
) -> Result<ApiResponse<ResetVideoResponse>, ApiError> {
    let txn = db.begin().await?;
    let video_status: Option<u32> = video::Entity::find_by_id(id)
        .select_only()
        .column(video::Column::DownloadStatus)
        .into_tuple()
        .one(&txn)
        .await?;
    let Some(video_status) = video_status else {
        return Err(anyhow!(InnerApiError::NotFound(id)).into());
    };
    let resetted_pages_model: Vec<_> = page::Entity::find()
        .filter(page::Column::VideoId.eq(id))
        .all(&txn)
        .await?
        .into_iter()
        .filter_map(|mut model| {
            let mut page_status = PageStatus::from(model.download_status);
            if page_status.reset_failed() {
                model.download_status = page_status.into();
                Some(model)
            } else {
                None
            }
        })
        .collect();
    let mut video_status = VideoStatus::from(video_status);
    let mut should_update_video = video_status.reset_failed();
    if !resetted_pages_model.is_empty() {
        // 视频状态标志的第 5 位表示是否有分 P 下载失败，如果有需要重置的分页，需要同时重置视频的该状态
        video_status.set(4, 0);
        should_update_video = true;
    }
    if should_update_video {
        video::Entity::update(video::ActiveModel {
            id: Unchanged(id),
            download_status: Set(video_status.into()),
            ..Default::default()
        })
        .exec(&txn)
        .await?;
    }
    let resetted_pages_id: Vec<_> = resetted_pages_model.iter().map(|model| model.id).collect();
    let resetted_pages_model: Vec<page::ActiveModel> = resetted_pages_model
        .into_iter()
        .map(|model| model.into_active_model())
        .collect();
    for page_trunk in resetted_pages_model.chunks(50) {
        page::Entity::insert_many(page_trunk.to_vec())
            .on_conflict(
                OnConflict::column(page::Column::Id)
                    .update_column(page::Column::DownloadStatus)
                    .to_owned(),
            )
            .exec(&txn)
            .await?;
    }
    txn.commit().await?;
    Ok(ApiResponse::ok(ResetVideoResponse {
        resetted: should_update_video,
        video: id,
        pages: resetted_pages_id,
    }))
}

/// 添加新的视频源
#[utoipa::path(
    post,
    path = "/api/video-sources",
    request_body = AddVideoSourceRequest,
    responses(
        (status = 200, body = ApiResponse<AddVideoSourceResponse>),
    )
)]
pub async fn add_video_source(
    Extension(db): Extension<Arc<DatabaseConnection>>,
    axum::Json(params): axum::Json<AddVideoSourceRequest>,
) -> Result<ApiResponse<AddVideoSourceResponse>, ApiError> {
    let txn = db.begin().await?;
    
    let result = match params.source_type.as_str() {
        "collection" => {
            // 添加合集
            let collection_type_value = params.collection_type.as_deref().unwrap_or("season");
            let collection_type = match collection_type_value {
                "season" => 2, // 视频合集
                "series" => 1, // 视频列表
                _ => 2, // 默认使用season类型
            };
            
            let collection = collection::ActiveModel {
                id: sea_orm::ActiveValue::NotSet,
                s_id: sea_orm::Set(params.source_id.parse::<i64>().map_err(|_| anyhow!("无效的合集ID"))?),
                m_id: sea_orm::Set(123456789), // 合集需要UP主ID，但用户可能不知道，暂时使用占位值
                name: sea_orm::Set(params.name),
                r#type: sea_orm::Set(collection_type), // 使用用户选择的类型
                path: sea_orm::Set(params.path.clone()),
                created_at: sea_orm::Set(chrono::Utc::now().to_string()),
                latest_row_at: sea_orm::Set(chrono::Utc::now().naive_utc()),
                ..Default::default()
            };
            
            let insert_result = collection::Entity::insert(collection)
                .exec(&txn)
                .await?;
                
            // 更新配置文件 - 直接修改文件
            let config_path = dirs::config_dir().unwrap().join("bili-sync").join("config.toml");
            let config_content = std::fs::read_to_string(&config_path)?;
            let mut config: toml::Value = toml::from_str(&config_content)?;
            
            // 确保collection_list存在
            if config.get("collection_list").is_none() {
                config["collection_list"] = toml::Value::Table(toml::value::Table::new());
            }
            
            // 添加新项 - 使用正确的格式 type:mid:sid
            if let Some(table) = config["collection_list"].as_table_mut() {
                // 使用用户选择的类型，mid使用占位值123456789
                let key = format!("{}:123456789:{}", collection_type_value, params.source_id);
                table.insert(key, toml::Value::String(params.path.clone()));
            }
            
            // 写回文件
            let config_str = toml::to_string_pretty(&config)?;
            std::fs::write(&config_path, config_str)?;
            
            // 重新加载配置，使修改立即生效
            reload_config_file()?;
            
            AddVideoSourceResponse {
                success: true,
                source_id: insert_result.last_insert_id,
                source_type: "collection".to_string(),
                message: "合集添加成功".to_string(),
            }
        },
        "favorite" => {
            // 添加收藏夹
            let favorite = favorite::ActiveModel {
                id: sea_orm::ActiveValue::NotSet,
                f_id: sea_orm::Set(params.source_id.parse::<i64>().map_err(|_| anyhow!("无效的收藏夹ID"))?),
                name: sea_orm::Set(params.name),
                path: sea_orm::Set(params.path.clone()),
                created_at: sea_orm::Set(chrono::Utc::now().to_string()),
                latest_row_at: sea_orm::Set(chrono::Utc::now().naive_utc()),
                ..Default::default()
            };
            
            let insert_result = favorite::Entity::insert(favorite)
                .exec(&txn)
                .await?;
                
            // 更新配置文件 - 直接修改文件
            let config_path = dirs::config_dir().unwrap().join("bili-sync").join("config.toml");
            let config_content = std::fs::read_to_string(&config_path)?;
            let mut config: toml::Value = toml::from_str(&config_content)?;
            
            // 确保favorite_list存在
            if config.get("favorite_list").is_none() {
                config["favorite_list"] = toml::Value::Table(toml::value::Table::new());
            }
            
            // 添加新项
            if let Some(table) = config["favorite_list"].as_table_mut() {
                table.insert(params.source_id.clone(), toml::Value::String(params.path.clone()));
            }
            
            // 写回文件
            let config_str = toml::to_string_pretty(&config)?;
            std::fs::write(&config_path, config_str)?;
            
            // 重新加载配置，使修改立即生效
            reload_config_file()?;
            
            AddVideoSourceResponse {
                success: true,
                source_id: insert_result.last_insert_id,
                source_type: "favorite".to_string(),
                message: "收藏夹添加成功".to_string(),
            }
        },
        "submission" => {
            // 添加UP主投稿
            let submission = submission::ActiveModel {
                id: sea_orm::ActiveValue::NotSet,
                upper_id: sea_orm::Set(params.source_id.parse::<i64>().map_err(|_| anyhow!("无效的UP主ID"))?),
                upper_name: sea_orm::Set(params.name),
                path: sea_orm::Set(params.path.clone()),
                created_at: sea_orm::Set(chrono::Utc::now().to_string()),
                latest_row_at: sea_orm::Set(chrono::Utc::now().naive_utc()),
                ..Default::default()
            };
            
            let insert_result = submission::Entity::insert(submission)
                .exec(&txn)
                .await?;
                
            // 更新配置文件 - 直接修改文件
            let config_path = dirs::config_dir().unwrap().join("bili-sync").join("config.toml");
            let config_content = std::fs::read_to_string(&config_path)?;
            let mut config: toml::Value = toml::from_str(&config_content)?;
            
            // 确保submission_list存在
            if config.get("submission_list").is_none() {
                config["submission_list"] = toml::Value::Table(toml::value::Table::new());
            }
            
            // 添加新项
            if let Some(table) = config["submission_list"].as_table_mut() {
                table.insert(params.source_id.clone(), toml::Value::String(params.path.clone()));
            }
            
            // 写回文件
            let config_str = toml::to_string_pretty(&config)?;
            std::fs::write(&config_path, config_str)?;
            
            // 重新加载配置，使修改立即生效
            reload_config_file()?;
            
            AddVideoSourceResponse {
                success: true,
                source_id: insert_result.last_insert_id,
                source_type: "submission".to_string(),
                message: "UP主投稿添加成功".to_string(),
            }
        },
        "bangumi" => {
            // 添加番剧
            let media_id_clone = params.media_id.clone();
            let ep_id_clone = params.ep_id.clone();
            let download_all_seasons = params.download_all_seasons.unwrap_or(false);
            
            // 验证至少有一个ID不为空
            if params.source_id.is_empty() && params.media_id.is_none() && params.ep_id.is_none() {
                return Err(anyhow!("番剧标识不能全部为空，请至少提供 season_id、media_id 或 ep_id 中的一个").into());
            }
            
            let bangumi = video_source::ActiveModel {
                id: sea_orm::ActiveValue::NotSet,
                name: sea_orm::Set(params.name),
                path: sea_orm::Set(params.path.clone()),
                r#type: sea_orm::Set(1), // 1表示番剧类型
                latest_row_at: sea_orm::Set(chrono::Utc::now().naive_utc()),
                season_id: sea_orm::Set(Some(params.source_id.clone())),
                media_id: sea_orm::Set(params.media_id),
                ep_id: sea_orm::Set(params.ep_id),
                download_all_seasons: sea_orm::Set(Some(download_all_seasons)),
                ..Default::default()
            };
            
            let insert_result = video_source::Entity::insert(bangumi)
                .exec(&txn)
                .await?;
                
            // 更新配置文件 - 直接修改文件
            let config_path = dirs::config_dir().unwrap().join("bili-sync").join("config.toml");
            let config_content = std::fs::read_to_string(&config_path)?;
            let mut config: toml::Value = toml::from_str(&config_content)?;
            
            // 创建新的bangumi配置
            let mut bangumi_item = toml::value::Table::new();
            if !params.source_id.is_empty() {
            bangumi_item.insert("season_id".to_string(), toml::Value::String(params.source_id.clone()));
            }
            if let Some(media_id) = &media_id_clone {
                bangumi_item.insert("media_id".to_string(), toml::Value::String(media_id.clone()));
            }
            if let Some(ep_id) = &ep_id_clone {
                bangumi_item.insert("ep_id".to_string(), toml::Value::String(ep_id.clone()));
            }
            bangumi_item.insert("path".to_string(), toml::Value::String(params.path.clone()));
            bangumi_item.insert("download_all_seasons".to_string(), toml::Value::Boolean(download_all_seasons));
            
            // 安全地添加到bangumi数组
            match config.get_mut("bangumi") {
                Some(toml::Value::Array(array)) => {
                    // 如果bangumi存在且是数组类型，直接添加
                array.push(toml::Value::Table(bangumi_item));
                },
                _ => {
                    // 如果bangumi不存在或不是数组类型，创建新数组
                    let mut new_array = Vec::new();
                    new_array.push(toml::Value::Table(bangumi_item));
                    
                    // 使用insert方法而不是索引操作，避免可能的panic
                    if let Some(table) = config.as_table_mut() {
                        table.insert("bangumi".to_string(), toml::Value::Array(new_array));
                    } else {
                        // 如果config不是table类型，这是一个严重错误
                        return Err(anyhow!("配置文件格式错误，无法添加番剧配置").into());
                    }
                }
            }
            
            // 写回文件
            let config_str = toml::to_string_pretty(&config)?;
            std::fs::write(&config_path, config_str)?;
            
            // 重新加载配置，使修改立即生效
            reload_config_file()?;
            
            AddVideoSourceResponse {
                success: true,
                source_id: insert_result.last_insert_id,
                source_type: "bangumi".to_string(),
                message: "番剧添加成功".to_string(),
            }
        },
        "watch_later" => {
            // 稍后观看只能有一个
            let existing = watch_later::Entity::find()
                .count(&txn)
                .await?;
                
            if existing > 0 {
                return Err(anyhow!("已存在稍后观看配置，无法添加多个").into());
            }
                
            let watch_later = watch_later::ActiveModel {
                id: sea_orm::ActiveValue::NotSet,
                path: sea_orm::Set(params.path.clone()),
                latest_row_at: sea_orm::Set(chrono::Utc::now().naive_utc()),
                ..Default::default()
            };
            
            let insert_result = watch_later::Entity::insert(watch_later)
                .exec(&txn)
                .await?;
                
            // 更新配置文件 - 直接修改文件
            let config_path = dirs::config_dir().unwrap().join("bili-sync").join("config.toml");
            let config_content = std::fs::read_to_string(&config_path)?;
            let mut config: toml::Value = toml::from_str(&config_content)?;
            
            // 确保watch_later存在
            if config.get("watch_later").is_none() {
                config["watch_later"] = toml::Value::Table(toml::value::Table::new());
            }
            
            // 设置稍后观看配置
            config["watch_later"]["enabled"] = toml::Value::Boolean(true);
            config["watch_later"]["path"] = toml::Value::String(params.path.clone());
            
            // 写回文件
            let config_str = toml::to_string_pretty(&config)?;
            std::fs::write(&config_path, config_str)?;
            
            // 重新加载配置，使修改立即生效
            reload_config_file()?;
            
            AddVideoSourceResponse {
                success: true,
                source_id: insert_result.last_insert_id,
                source_type: "watch_later".to_string(),
                message: "稍后观看添加成功".to_string(),
            }
        },
        _ => return Err(anyhow!("不支持的视频源类型: {}", params.source_type).into()),
    };
    
    // 确保目标路径存在
    fs::create_dir_all(&params.path).map_err(|e| anyhow!("创建目录失败: {}", e))?;
    
    txn.commit().await?;
    Ok(ApiResponse::ok(result))
}

/// 重新加载配置
#[utoipa::path(
    post,
    path = "/api/reload-config",
    responses(
        (status = 200, body = ApiResponse<bool>),
    )
)]
pub async fn reload_config() -> Result<ApiResponse<bool>, ApiError> {
    // 调用config中的reload_config函数获取新配置
    let _new_config = crate::config::reload_config();
    
    // 将配置应用到数据库或其他状态管理中
    // 这里我们可以执行额外的初始化操作，如果需要的话
    info!("配置已重新加载");
    
    // 返回成功响应
    Ok(ApiResponse::ok(true))
}

/// 删除视频源
#[utoipa::path(
    delete,
    path = "/api/video-sources/{source_type}/{id}",
    params(
        ("source_type" = String, Path, description = "视频源类型"),
        ("id" = i32, Path, description = "视频源ID"),
        ("delete_local_files" = bool, Query, description = "是否删除本地文件")
    ),
    responses(
        (status = 200, body = ApiResponse<DeleteVideoSourceResponse>),
    )
)]
pub async fn delete_video_source(
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Path((source_type, id)): Path<(String, i32)>,
    Query(params): Query<crate::api::request::DeleteVideoSourceRequest>,
) -> Result<ApiResponse<crate::api::response::DeleteVideoSourceResponse>, ApiError> {
    let txn = db.begin().await?;
    
    let delete_local_files = params.delete_local_files;
    
    // 根据不同类型的视频源执行不同的删除操作
    let result = match source_type.as_str() {
        "collection" => {
            // 查找要删除的合集
            let collection = collection::Entity::find_by_id(id)
                .one(&txn)
                .await?
                .ok_or_else(|| anyhow!("未找到指定的合集"))?;
                
            // 如果需要删除本地文件
            if delete_local_files {
                // 尝试删除本地文件夹
                let path = &collection.path;
                if let Err(e) = std::fs::remove_dir_all(path) {
                    warn!("删除合集文件夹失败: {}", e);
                }
            }
            
            // 删除数据库中的记录
            collection::Entity::delete_by_id(id)
                .exec(&txn)
                .await?;
                
            // 更新配置文件
            let config_path = dirs::config_dir().unwrap().join("bili-sync").join("config.toml");
            if let Ok(config_content) = std::fs::read_to_string(&config_path) {
                if let Ok(mut config) = toml::from_str::<toml::Value>(&config_content) {
                    // 更新collection_list
                    if let Some(collection_list) = config.get_mut("collection_list") {
                        if let Some(list) = collection_list.as_array_mut() {
                            // 找到并删除对应的合集ID
                            if let Some(index) = list.iter().position(|v| {
                                v.get("id").and_then(|id_val| id_val.as_integer()).map_or(false, |v| v == id as i64)
                            }) {
                                list.remove(index);
                                
                                // 保存更新后的配置
                                if let Ok(config_str) = toml::to_string_pretty(&config) {
                                    let _ = std::fs::write(&config_path, config_str);
                                    // 重新加载配置
                                    let _ = reload_config_file();
                                }
                            }
                        }
                    }
                }
            }
            
            crate::api::response::DeleteVideoSourceResponse {
                success: true,
                source_id: id,
                source_type: "collection".to_string(),
                message: format!("合集 {} 已成功删除", collection.name),
            }
        },
        "favorite" => {
            // 查找要删除的收藏夹
            let favorite = favorite::Entity::find_by_id(id)
                .one(&txn)
                .await?
                .ok_or_else(|| anyhow!("未找到指定的收藏夹"))?;
                
            // 如果需要删除本地文件
            if delete_local_files {
                // 尝试删除本地文件夹
                let path = &favorite.path;
                if let Err(e) = std::fs::remove_dir_all(path) {
                    warn!("删除收藏夹文件夹失败: {}", e);
                }
            }
            
            // 删除数据库中的记录
            favorite::Entity::delete_by_id(id)
                .exec(&txn)
                .await?;
                
            // 更新配置文件
            let config_path = dirs::config_dir().unwrap().join("bili-sync").join("config.toml");
            if let Ok(config_content) = std::fs::read_to_string(&config_path) {
                if let Ok(mut config) = toml::from_str::<toml::Value>(&config_content) {
                    // 更新favorite_list
                    if let Some(favorite_list) = config.get_mut("favorite_list") {
                        if let Some(list) = favorite_list.as_array_mut() {
                            // 找到并删除对应的收藏夹ID
                            if let Some(index) = list.iter().position(|v| {
                                v.get("id").and_then(|id_val| id_val.as_integer()).map_or(false, |v| v == id as i64)
                            }) {
                                list.remove(index);
                                
                                // 保存更新后的配置
                                if let Ok(config_str) = toml::to_string_pretty(&config) {
                                    let _ = std::fs::write(&config_path, config_str);
                                    // 重新加载配置
                                    let _ = reload_config_file();
                                }
                            }
                        }
                    }
                }
            }
            
            crate::api::response::DeleteVideoSourceResponse {
                success: true,
                source_id: id,
                source_type: "favorite".to_string(),
                message: format!("收藏夹 {} 已成功删除", favorite.name),
            }
        },
        "submission" => {
            // 查找要删除的UP主投稿
            let submission = submission::Entity::find_by_id(id)
                .one(&txn)
                .await?
                .ok_or_else(|| anyhow!("未找到指定的UP主投稿"))?;
                
            // 如果需要删除本地文件
            if delete_local_files {
                // 尝试删除本地文件夹
                let path = &submission.path;
                if let Err(e) = std::fs::remove_dir_all(path) {
                    warn!("删除UP主投稿文件夹失败: {}", e);
                }
            }
            
            // 删除数据库中的记录
            submission::Entity::delete_by_id(id)
                .exec(&txn)
                .await?;
                
            // 更新配置文件
            let config_path = dirs::config_dir().unwrap().join("bili-sync").join("config.toml");
            if let Ok(config_content) = std::fs::read_to_string(&config_path) {
                if let Ok(mut config) = toml::from_str::<toml::Value>(&config_content) {
                    // 更新submission_list
                    if let Some(submission_list) = config.get_mut("submission_list") {
                        if let Some(list) = submission_list.as_array_mut() {
                            // 找到并删除对应的UP主ID
                            if let Some(index) = list.iter().position(|v| {
                                v.get("id").and_then(|id_val| id_val.as_integer()).map_or(false, |v| v == id as i64)
                            }) {
                                list.remove(index);
                                
                                // 保存更新后的配置
                                if let Ok(config_str) = toml::to_string_pretty(&config) {
                                    let _ = std::fs::write(&config_path, config_str);
                                    // 重新加载配置
                                    let _ = reload_config_file();
                                }
                            }
                        }
                    }
                }
            }
            
            crate::api::response::DeleteVideoSourceResponse {
                success: true,
                source_id: id,
                source_type: "submission".to_string(),
                message: format!("UP主 {} 的投稿已成功删除", submission.upper_name),
            }
        },
        "watch_later" => {
            // 查找要删除的稍后再看
            let watch_later = watch_later::Entity::find_by_id(id)
                .one(&txn)
                .await?
                .ok_or_else(|| anyhow!("未找到指定的稍后再看"))?;
                
            // 如果需要删除本地文件
            if delete_local_files {
                // 尝试删除本地文件夹
                let path = &watch_later.path;
                if let Err(e) = std::fs::remove_dir_all(path) {
                    warn!("删除稍后再看文件夹失败: {}", e);
                }
            }
            
            // 删除数据库中的记录
            watch_later::Entity::delete_by_id(id)
                .exec(&txn)
                .await?;
                
            // 更新配置文件
            let config_path = dirs::config_dir().unwrap().join("bili-sync").join("config.toml");
            if let Ok(config_content) = std::fs::read_to_string(&config_path) {
                if let Ok(mut config) = toml::from_str::<toml::Value>(&config_content) {
                    // 更新watch_later_list
                    if let Some(watch_later_list) = config.get_mut("watch_later_list") {
                        if let Some(list) = watch_later_list.as_array_mut() {
                            // 找到并删除对应的稍后再看ID
                            if let Some(index) = list.iter().position(|v| {
                                v.get("id").and_then(|id_val| id_val.as_integer()).map_or(false, |v| v == id as i64)
                            }) {
                                list.remove(index);
                                
                                // 保存更新后的配置
                                if let Ok(config_str) = toml::to_string_pretty(&config) {
                                    let _ = std::fs::write(&config_path, config_str);
                                    // 重新加载配置
                                    let _ = reload_config_file();
                                }
                            }
                        }
                    }
                }
            }
            
            crate::api::response::DeleteVideoSourceResponse {
                success: true,
                source_id: id,
                source_type: "watch_later".to_string(),
                message: "稍后再看已成功删除".to_string(),
            }
        },
        "bangumi" => {
            // 查找要删除的番剧
            let bangumi = video_source::Entity::find_by_id(id)
                .one(&txn)
                .await?
                .ok_or_else(|| anyhow!("未找到指定的番剧"))?;
                
            // 如果需要删除本地文件
            if delete_local_files {
                // 尝试删除本地文件夹
                let path = &bangumi.path;
                if let Err(e) = std::fs::remove_dir_all(path) {
                    warn!("删除番剧文件夹失败: {}", e);
                }
            }
            
            // 删除数据库中的记录
            video_source::Entity::delete_by_id(id)
                .exec(&txn)
                .await?;
                
            // 更新配置文件
            let config_path = dirs::config_dir().unwrap().join("bili-sync").join("config.toml");
            if let Ok(config_content) = std::fs::read_to_string(&config_path) {
                if let Ok(mut config) = toml::from_str::<toml::Value>(&config_content) {
                    // 更新bangumi_list
                    if let Some(bangumi_list) = config.get_mut("bangumi_list") {
                        if let Some(list) = bangumi_list.as_array_mut() {
                            // 找到并删除对应的番剧ID
                            if let Some(index) = list.iter().position(|v| {
                                v.get("id").and_then(|id_val| id_val.as_integer()).map_or(false, |v| v == id as i64)
                            }) {
                                list.remove(index);
                                
                                // 保存更新后的配置
                                if let Ok(config_str) = toml::to_string_pretty(&config) {
                                    let _ = std::fs::write(&config_path, config_str);
                                    // 重新加载配置
                                    let _ = reload_config_file();
                                }
                            }
                        }
                    }
                }
            }
            
            crate::api::response::DeleteVideoSourceResponse {
                success: true,
                source_id: id,
                source_type: "bangumi".to_string(),
                message: format!("番剧 {} 已成功删除", bangumi.name),
            }
        },
        _ => return Err(anyhow!("不支持的视频源类型: {}", source_type).into()),
    };
    
    txn.commit().await?;
    Ok(ApiResponse::ok(result))
}

// 在添加视频源成功后调用此函数获取新配置
fn reload_config_file() -> Result<()> {
    // 加载新配置
    let _new_config = crate::config::reload_config();
    
    // 更新应用的状态或执行必要的初始化
    info!("配置已重新加载，新添加的视频源将在下一轮下载任务中生效");
    
    Ok(())
}
