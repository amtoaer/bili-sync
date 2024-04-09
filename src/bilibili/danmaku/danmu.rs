//! 一个弹幕实例，但是没有位置信息
use anyhow::Result;

use super::canvas::CanvasConfig;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DanmuType {
    #[default]
    Float,
    Top,
    Bottom,
    Reverse,
}

impl DanmuType {
    pub fn from_num(num: i32) -> Result<Self> {
        Ok(match num {
            1 => DanmuType::Float,
            4 => DanmuType::Bottom,
            5 => DanmuType::Top,
            6 => DanmuType::Reverse,
            _ => unreachable!(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Danmu {
    pub timeline_s: f64,
    pub content: String,
    pub r#type: DanmuType,
    /// 虽然这里有 fontsize，但是我们实际上使用 canvas config 的 font size，
    /// 否在在调节分辨率的时候字体会发生变化。
    pub fontsize: u32,
    pub rgb: (u8, u8, u8),
}

impl Danmu {
    /// 计算弹幕的“像素长度”，会乘上一个缩放因子
    ///
    /// 汉字算一个全宽，英文算2/3宽
    pub fn length(&self, config: &CanvasConfig) -> f64 {
        let pts = config.danmaku_option.font_size
            * self
                .content
                .chars()
                .map(|ch| if ch.is_ascii() { 2 } else { 3 })
                .sum::<u32>()
            / 3;

        pts as f64 * config.danmaku_option.width_ratio
    }
}
