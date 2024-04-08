//! 出于减少编译引入考虑，直接翻译了一下 pb，不引入 prost-build
//!
//! 可以看旁边的 dm.proto

use prost::Message;

use crate::bilibili::danmaku::danmu::{Danmu, DanmuType};
/// 弹幕 pb 定义
#[derive(Clone, Message)]
pub struct DanmakuElem {
    /// 弹幕 dmid
    #[prost(int64, tag = "1")]
    pub id: i64,

    /// 弹幕出现位置（单位 ms）
    #[prost(int32, tag = "2")]
    pub progress: i32,

    /// 弹幕类型
    #[prost(int32, tag = "3")]
    pub mode: i32,

    /// 弹幕字号
    #[prost(int32, tag = "4")]
    pub fontsize: i32,

    /// 弹幕颜色
    #[prost(uint32, tag = "5")]
    pub color: u32,

    /// 发送者 mid hash
    #[prost(string, tag = "6")]
    pub mid_hash: String,

    /// 弹幕正文
    #[prost(string, tag = "7")]
    pub content: String,

    /// 弹幕发送时间
    #[prost(int64, tag = "8")]
    pub ctime: i64,

    /// 弹幕权重
    #[prost(int32, tag = "9")]
    pub weight: i32,

    /// 动作？
    #[prost(string, tag = "10")]
    pub action: String,

    /// 弹幕池
    #[prost(int32, tag = "11")]
    pub pool: i32,

    /// 弹幕 dmid str
    #[prost(string, tag = "12")]
    pub dmid_str: String,

    /// 弹幕属性
    #[prost(int32, tag = "13")]
    pub attr: i32,
}

#[derive(Clone, Message)]
pub struct DmSegMobileReply {
    #[prost(message, repeated, tag = "1")]
    pub elems: Vec<DanmakuElem>,
}

impl From<DanmakuElem> for Danmu {
    fn from(elem: DanmakuElem) -> Self {
        Self {
            timeline_s: elem.progress as f64 / 1000.0,
            content: elem.content,
            r#type: DanmuType::from_num(elem.mode).unwrap_or_default(),
            fontsize: elem.fontsize as u32,
            rgb: (
                ((elem.color >> 16) & 0xFF) as u8,
                ((elem.color >> 8) & 0xFF) as u8,
                (elem.color & 0xFF) as u8,
            ),
        }
    }
}
