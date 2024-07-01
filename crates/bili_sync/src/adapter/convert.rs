use sea_orm::ActiveValue::Set;
use serde_json::json;

use crate::bilibili::VideoInfo;
use crate::core::utils::id_time_key;

impl VideoInfo {
    pub fn to_model(&self) -> bili_sync_entity::video::ActiveModel {
        match self {
            VideoInfo::Simple {
                bvid,
                cover,
                ctime,
                pubtime,
            } => bili_sync_entity::video::ActiveModel {
                bvid: Set(bvid.clone()),
                cover: Set(cover.clone()),
                ctime: Set(ctime.naive_utc()),
                pubtime: Set(pubtime.naive_utc()),
                ..Default::default()
            },
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
            } => bili_sync_entity::video::ActiveModel {
                bvid: Set(bvid.clone()),
                name: Set(title.clone()),
                category: Set(*vtype),
                intro: Set(intro.clone()),
                cover: Set(cover.clone()),
                ctime: Set(ctime.naive_utc()),
                pubtime: Set(pubtime.naive_utc()),
                favtime: Set(fav_time.naive_utc()),
                download_status: Set(0),
                valid: Set(*attr == 0),
                tags: Set(None),
                single_page: Set(None),
                upper_id: Set(upper.mid),
                upper_name: Set(upper.name.clone()),
                upper_face: Set(upper.face.clone()),
                ..Default::default()
            },
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
            } => bili_sync_entity::video::ActiveModel {
                bvid: Set(bvid.clone()),
                name: Set(title.clone()),
                category: Set(2), // 视频合集里的内容类型肯定是视频
                intro: Set(intro.clone()),
                cover: Set(cover.clone()),
                ctime: Set(ctime.naive_utc()),
                pubtime: Set(pubtime.naive_utc()),
                favtime: Set(pubtime.naive_utc()), // 合集不包括 fav_time，使用发布时间代替
                download_status: Set(0),
                valid: Set(*state == 0),
                tags: Set(None),
                single_page: Set(None),
                upper_id: Set(upper.mid),
                upper_name: Set(upper.name.clone()),
                upper_face: Set(upper.face.clone()),
                ..Default::default()
            },
        }
    }

    pub fn to_fmt_args(&self) -> serde_json::Value {
        match self {
            VideoInfo::Simple { .. } => unreachable!(), // 不能从简单的视频信息中构造格式化参数
            VideoInfo::Detail { title, bvid, upper, .. } => json!({
                "bvid": &bvid,
                "title": &title,
                "upper_name": &upper.name,
                "upper_mid": &upper.mid,
            }),
            VideoInfo::View { title, bvid, upper, .. } => json!({
                "bvid": &bvid,
                "title": &title,
                "upper_name": &upper.name,
                "upper_mid": &upper.mid,
            }),
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
