use chrono::{DateTime, NaiveDateTime, Utc};
use sea_orm::ActiveValue::{NotSet, Set};
use sea_orm::IntoActiveModel;

use crate::bilibili::{PageInfo, VideoInfo};

impl VideoInfo {
    /// 在检测视频更新时，通过该方法将 VideoInfo 转换为简单的 ActiveModel，此处仅填充一些简单信息，后续会使用详情覆盖
    pub fn into_simple_model(self) -> bili_sync_entity::video::ActiveModel {
        let default = bili_sync_entity::video::ActiveModel {
            id: NotSet,
            created_at: NotSet,
            // 此处不使用 ActiveModel::default() 是为了让其它字段有默认值
            ..bili_sync_entity::video::Model::default().into_active_model()
        };
        match self {
            VideoInfo::Collection {
                bvid,
                cover,
                ctime,
                pubtime,
            } => bili_sync_entity::video::ActiveModel {
                bvid: Set(bvid),
                cover: Set(cover),
                ctime: Set(ctime.naive_utc()),
                pubtime: Set(pubtime.naive_utc()),
                category: Set(2), // 视频合集里的内容类型肯定是视频
                valid: Set(true),
                ..default
            },
            VideoInfo::Favorite {
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
                bvid: Set(bvid),
                name: Set(title),
                category: Set(vtype),
                intro: Set(intro),
                cover: Set(cover),
                ctime: Set(ctime.naive_utc()),
                pubtime: Set(pubtime.naive_utc()),
                favtime: Set(fav_time.naive_utc()),
                download_status: Set(0),
                valid: Set(attr == 0),
                upper_id: Set(upper.mid),
                upper_name: Set(upper.name),
                upper_face: Set(upper.face),
                ..default
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
                bvid: Set(bvid),
                name: Set(title),
                category: Set(2), // 稍后再看里的内容类型肯定是视频
                intro: Set(intro),
                cover: Set(cover),
                ctime: Set(ctime.naive_utc()),
                pubtime: Set(pubtime.naive_utc()),
                favtime: Set(fav_time.naive_utc()),
                download_status: Set(0),
                valid: Set(state == 0),
                upper_id: Set(upper.mid),
                upper_name: Set(upper.name),
                upper_face: Set(upper.face),
                ..default
            },
            VideoInfo::Submission {
                title,
                bvid,
                intro,
                cover,
                ctime,
            } => bili_sync_entity::video::ActiveModel {
                bvid: Set(bvid),
                name: Set(title),
                intro: Set(intro),
                cover: Set(cover),
                ctime: Set(ctime.naive_utc()),
                category: Set(2), // 投稿视频的内容类型肯定是视频
                valid: Set(true),
                ..default
            },
            VideoInfo::Dynamic {
                title,
                bvid,
                desc,
                cover,
                pubtime,
            } => bili_sync_entity::video::ActiveModel {
                bvid: Set(bvid),
                name: Set(title),
                intro: Set(desc),
                cover: Set(cover),
                pubtime: Set(pubtime.naive_utc()),
                category: Set(2), // 动态里的视频内容类型肯定是视频
                valid: Set(true),
                ..default
            },
            VideoInfo::Detail { .. } => unreachable!(),
        }
    }

    /// 填充视频详情时调用，该方法会将视频详情附加到原有的 Model 上
    /// 特殊地，如果在检测视频更新时记录了 favtime，那么 favtime 会维持原样，否则会使用 pubtime 填充
    pub fn into_detail_model(self, base_model: bili_sync_entity::video::Model) -> bili_sync_entity::video::ActiveModel {
        match self {
            VideoInfo::Detail {
                title,
                bvid,
                intro,
                cover,
                upper,
                ctime,
                pubtime,
                state,
                is_upower_exclusive,
                is_upower_play,
                redirect_url,
                ..
            } => bili_sync_entity::video::ActiveModel {
                bvid: Set(bvid),
                name: Set(title),
                intro: Set(intro),
                cover: Set(cover),
                ctime: Set(ctime.naive_utc()),
                pubtime: Set(pubtime.naive_utc()),
                favtime: if base_model.favtime != NaiveDateTime::default() {
                    Set(base_model.favtime) // 之前设置了 favtime，使用之前的值（等价于 unset，但设置上以支持后续的规则匹配）
                } else {
                    Set(pubtime.naive_utc()) // 未设置过 favtime，使用 pubtime 填充
                },
                download_status: Set(0),
                // state == 0 表示开放浏览
                // is_upower_exclusive 和 is_upower_play 相等有两种情况：
                //  1. 都为 true，表示视频是充电专享但是已经充过电，有权观看
                //  2. 都为 false，表示视频是非充电视频
                // redirect_url 仅在视频为番剧、影视、纪录片等特殊视频时才会有值，如果为空说明是普通视频
                // 仅在三种条件都满足时，才认为视频是可下载的
                valid: Set(state == 0 && (is_upower_exclusive == is_upower_play) && redirect_url.is_none()),
                upper_id: Set(upper.mid),
                upper_name: Set(upper.name),
                upper_face: Set(upper.face),
                ..base_model.into_active_model()
            },
            _ => unreachable!(),
        }
    }

    /// 获取视频的发布时间，用于对时间做筛选检查新视频
    pub fn release_datetime(&self) -> &DateTime<Utc> {
        match self {
            VideoInfo::Collection { pubtime: time, .. }
            | VideoInfo::Favorite { fav_time: time, .. }
            | VideoInfo::WatchLater { fav_time: time, .. }
            | VideoInfo::Submission { ctime: time, .. }
            | VideoInfo::Dynamic { pubtime: time, .. } => time,
            VideoInfo::Detail { .. } => unreachable!(),
        }
    }
}

impl PageInfo {
    pub fn into_active_model(self, video_model_id: i32) -> bili_sync_entity::page::ActiveModel {
        let (width, height) = match &self.dimension {
            Some(d) => {
                if d.rotate == 0 {
                    (Some(d.width), Some(d.height))
                } else {
                    (Some(d.height), Some(d.width))
                }
            }
            None => (None, None),
        };
        bili_sync_entity::page::ActiveModel {
            video_id: Set(video_model_id),
            cid: Set(self.cid),
            pid: Set(self.page),
            name: Set(self.name),
            width: Set(width),
            height: Set(height),
            duration: Set(self.duration),
            image: Set(self.first_frame),
            download_status: Set(0),
            ..Default::default()
        }
    }
}
