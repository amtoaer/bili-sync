use sea_orm::ActiveValue::NotSet;
use serde_json::json;

use crate::bilibili::VideoInfo;
use crate::utils::id_time_key;

impl VideoInfo {
    /// 将 VideoInfo 转换为 ActiveModel
    pub fn to_model(&self) -> bili_sync_entity::video::ActiveModel {
        let mut v: bili_sync_entity::video::ActiveModel = match self {
            VideoInfo::Simple {
                bvid,
                cover,
                ctime,
                pubtime,
            } => bili_sync_entity::video::Model {
                bvid: bvid.clone(),
                cover: cover.clone(),
                ctime: ctime.naive_utc(),
                pubtime: pubtime.naive_utc(),
                category: 2, // 视频合集里的内容类型肯定是视频
                valid: true,
                ..Default::default()
            }
            .into(),
            VideoInfo::Detail {
                title,
                vtype,
                bvid,
                intro,
                cover,
                upper,
                ctime,
                fav_time,
                pubtime,
                attr,
            } => bili_sync_entity::video::Model {
                bvid: bvid.clone(),
                name: title.clone(),
                category: *vtype,
                intro: intro.clone(),
                cover: cover.clone(),
                ctime: ctime.naive_utc(),
                pubtime: pubtime.naive_utc(),
                favtime: fav_time.naive_utc(),
                download_status: 0,
                valid: *attr == 0,
                tags: None,
                single_page: None,
                upper_id: upper.mid,
                upper_name: upper.name.clone(),
                upper_face: upper.face.clone(),
                ..Default::default()
            }
            .into(),
            VideoInfo::View {
                title,
                bvid,
                intro,
                cover,
                upper,
                ctime,
                pubtime,
                state,
                ..
            } => bili_sync_entity::video::Model {
                bvid: bvid.clone(),
                name: title.clone(),
                category: 2, // 视频合集里的内容类型肯定是视频
                intro: intro.clone(),
                cover: cover.clone(),
                ctime: ctime.naive_utc(),
                pubtime: pubtime.naive_utc(),
                favtime: pubtime.naive_utc(), // 合集不包括 fav_time，使用发布时间代替
                download_status: 0,
                valid: *state == 0,
                tags: None,
                single_page: None,
                upper_id: upper.mid,
                upper_name: upper.name.clone(),
                upper_face: upper.face.clone(),
                ..Default::default()
            }
            .into(),
        };
        // 注意此处为了应用上 Model 的默认值，都是先获取 Model 再 into 成 ActiveModel
        // 但这样会导致 id 被设置为 Unchanged(0)，这里需要手动设置成 Unset 以确保数据库自行处理 id
        v.id = NotSet;
        v
    }

    pub fn to_fmt_args(&self) -> Option<serde_json::Value> {
        match self {
            VideoInfo::Simple { .. } => None, // 不能从简单的视频信息中构造格式化参数
            VideoInfo::Detail { title, bvid, upper, .. } => Some(json!({
                "bvid": &bvid,
                "title": &title,
                "upper_name": &upper.name,
                "upper_mid": &upper.mid,
            })),
            VideoInfo::View { title, bvid, upper, .. } => Some(json!({
                "bvid": &bvid,
                "title": &title,
                "upper_name": &upper.name,
                "upper_mid": &upper.mid,
            })),
        }
    }

    pub fn video_key(&self) -> String {
        match self {
            // 对于合集没有 fav_time，只能用 pubtime 代替
            VideoInfo::Simple { bvid, pubtime, .. } => id_time_key(bvid, pubtime),
            VideoInfo::Detail { bvid, fav_time, .. } => id_time_key(bvid, fav_time),
            // 详情接口返回的数据仅用于填充详情，不会被作为 video_key
            _ => unreachable!(),
        }
    }

    pub fn bvid(&self) -> &str {
        match self {
            VideoInfo::Simple { bvid, .. } => bvid,
            VideoInfo::Detail { bvid, .. } => bvid,
            // 同上
            _ => unreachable!(),
        }
    }
}
