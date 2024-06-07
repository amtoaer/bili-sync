//! 可以绘制的实体

use crate::bilibili::danmaku::Danmu;

/// 弹幕开始绘制的时间就是 danmu 的时间
pub struct Drawable {
    pub danmu: Danmu,
    /// 弹幕一共绘制的时间
    pub duration: f64,
    /// 弹幕的绘制 style
    pub style_name: &'static str,
    /// 绘制的“特效”
    pub effect: DrawEffect,
}
impl Drawable {
    pub fn new(danmu: Danmu, duration: f64, style_name: &'static str, effect: DrawEffect) -> Self {
        Drawable {
            danmu,
            duration,
            style_name,
            effect,
        }
    }
}

pub enum DrawEffect {
    Move { start: (i32, i32), end: (i32, i32) },
}
