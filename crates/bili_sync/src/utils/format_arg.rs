use serde_json::json;

pub fn video_format_args(video_model: &bili_sync_entity::video::Model, time_format: &str) -> serde_json::Value {
    json!({
        "bvid": &video_model.bvid,
        "title": &video_model.name,
        "upper_name": &video_model.upper_name,
        "upper_mid": &video_model.upper_id,
        "pubtime": &video_model.pubtime.and_utc().format(time_format).to_string(),
        "fav_time": &video_model.favtime.and_utc().format(time_format).to_string(),
    })
}

pub fn page_format_args(
    video_model: &bili_sync_entity::video::Model,
    page_model: &bili_sync_entity::page::Model,
    time_format: &str,
) -> serde_json::Value {
    json!({
        "bvid": &video_model.bvid,
        "title": &video_model.name,
        "upper_name": &video_model.upper_name,
        "upper_mid": &video_model.upper_id,
        "ptitle": &page_model.name,
        "pid": page_model.pid,
        "pubtime": video_model.pubtime.and_utc().format(time_format).to_string(),
        "fav_time": video_model.favtime.and_utc().format(time_format).to_string(),
    })
}
