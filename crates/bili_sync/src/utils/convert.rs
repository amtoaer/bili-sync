use sea_orm::ActiveValue::NotSet;
use sea_orm::{IntoActiveModel, Set};
use serde_json::json;

use crate::bilibili::VideoInfo;
use crate::config::CONFIG;
use crate::utils::id_time_key;

impl VideoInfo {
    /// 将 VideoInfo 转换为 ActiveModel
    pub fn to_model(&self, base_model: Option<bili_sync_entity::video::Model>) -> bili_sync_entity::video::ActiveModel {
        let base_model = match base_model {
            Some(base_model) => base_model.into_active_model(),
            None => {
                let mut tmp_model = bili_sync_entity::video::Model::default().into_active_model();
                // 注意此处要把 id 和 created_at 设置为 NotSet，方便在 sql 中忽略这些字段，交由数据库自动生成
                tmp_model.id = NotSet;
                tmp_model.created_at = NotSet;
                tmp_model
            }
        };
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
                category: Set(2), // 视频合集里的内容类型肯定是视频
                valid: Set(true),
                ..base_model
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
                ..base_model
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
                ..base_model
            },
            VideoInfo::WatchLater {
                title,
                bvid,
                intro,
                cover,
                upper,
                ctime,
                fav_time,
                pubtime,
                state,
            } => bili_sync_entity::video::ActiveModel {
                bvid: Set(bvid.clone()),
                name: Set(title.clone()),
                category: Set(2), // 稍后再看里的内容类型肯定是视频
                intro: Set(intro.clone()),
                cover: Set(cover.clone()),
                ctime: Set(ctime.naive_utc()),
                pubtime: Set(pubtime.naive_utc()),
                favtime: Set(fav_time.naive_utc()),
                download_status: Set(0),
                valid: Set(*state == 0),
                tags: Set(None),
                single_page: Set(None),
                upper_id: Set(upper.mid),
                upper_name: Set(upper.name.clone()),
                upper_face: Set(upper.face.clone()),
                ..base_model
            },
        }
    }

    pub fn to_fmt_args(&self) -> Option<serde_json::Value> {
        match self {
            VideoInfo::Simple { .. } => None, // 不能从简单的视频信息中构造格式化参数
            VideoInfo::Detail {
                title,
                bvid,
                upper,
                pubtime,
                fav_time,
                ..
            }
            | VideoInfo::WatchLater {
                title,
                bvid,
                upper,
                pubtime,
                fav_time,
                ..
            } => Some(json!({
                "bvid": &bvid,
                "title": &title,
                "upper_name": &upper.name,
                "upper_mid": &upper.mid,
                "pubtime": pubtime.format(&CONFIG.time_format).to_string(),
                "fav_time": fav_time.format(&CONFIG.time_format).to_string(),
            })),
            VideoInfo::View {
                title,
                bvid,
                upper,
                pubtime,
                ..
            } => {
                let pubtime = pubtime.format(&CONFIG.time_format).to_string();
                Some(json!({
                    "bvid": &bvid,
                    "title": &title,
                    "upper_name": &upper.name,
                    "upper_mid": &upper.mid,
                    "pubtime": &pubtime,
                    "fav_time": &pubtime,
                }))
            }
        }
    }

    pub fn video_key(&self) -> String {
        match self {
            // 对于合集没有 fav_time，只能用 pubtime 代替
            VideoInfo::Simple { bvid, pubtime, .. } => id_time_key(bvid, pubtime),
            VideoInfo::Detail { bvid, fav_time, .. } => id_time_key(bvid, fav_time),
            VideoInfo::WatchLater { bvid, fav_time, .. } => id_time_key(bvid, fav_time),
            // 详情接口返回的数据仅用于填充详情，不会被作为 video_key
            _ => unreachable!(),
        }
    }

    pub fn bvid(&self) -> &str {
        match self {
            VideoInfo::Simple { bvid, .. } => bvid,
            VideoInfo::Detail { bvid, .. } => bvid,
            VideoInfo::WatchLater { bvid, .. } => bvid,
            // 同上
            _ => unreachable!(),
        }
    }
}
