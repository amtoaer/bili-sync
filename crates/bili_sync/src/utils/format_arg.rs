use serde_json::json;

use crate::config::CONFIG;

pub fn video_format_args(video_model: &bili_sync_entity::video::Model) -> serde_json::Value {
    json!({
        "bvid": &video_model.bvid,
        "title": &video_model.name,
        "upper_name": &video_model.upper_name,
        "upper_mid": &video_model.upper_id,
        "pubtime": &video_model.pubtime.and_utc().format(&CONFIG.time_format).to_string(),
        "fav_time": &video_model.favtime.and_utc().format(&CONFIG.time_format).to_string(),
        "show_title": &video_model.name,
    })
}

pub fn page_format_args(
    video_model: &bili_sync_entity::video::Model,
    page_model: &bili_sync_entity::page::Model,
) -> serde_json::Value {
    // 检查是否为番剧类型
    let is_bangumi = match video_model.source_type {
        Some(1) => true,  // source_type = 1 表示为番剧
        _ => false,
    };
    
    // 对于番剧，格式化一个更友好的文件名
    if is_bangumi {
        json!({
            "bvid": &video_model.bvid,
            "title": &video_model.name,
            "upper_name": &video_model.upper_name,
            "upper_mid": &video_model.upper_id,
            "ptitle": &page_model.name,
            "pid": page_model.pid,
            "pid_pad": format!("{:02}", page_model.pid),
            "pubtime": video_model.pubtime.and_utc().format(&CONFIG.time_format).to_string(),
            "fav_time": video_model.favtime.and_utc().format(&CONFIG.time_format).to_string(),
            "long_title": &page_model.name,
            "show_title": format!("E{:02} - {}", page_model.pid, page_model.name),
        })
    } else {
    json!({
        "bvid": &video_model.bvid,
        "title": &video_model.name,
        "upper_name": &video_model.upper_name,
        "upper_mid": &video_model.upper_id,
        "ptitle": &page_model.name,
        "pid": page_model.pid,
        "pid_pad": format!("{:02}", page_model.pid),
        "pubtime": video_model.pubtime.and_utc().format(&CONFIG.time_format).to_string(),
        "fav_time": video_model.favtime.and_utc().format(&CONFIG.time_format).to_string(),
        "long_title": &page_model.name,
        "show_title": &page_model.name,
    })
    }
}
