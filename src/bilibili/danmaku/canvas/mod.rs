//! 决定绘画策略
mod lane;

use anyhow::Result;
use float_ord::FloatOrd;
use lane::Lane;

use crate::bilibili::danmaku::canvas::lane::Collision;
use crate::bilibili::danmaku::danmu::DanmuType;
use crate::bilibili::danmaku::{Danmu, DrawEffect, Drawable};
use crate::bilibili::PageInfo;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct DanmakuOption {
    pub duration: f64,
    pub font: String,
    pub font_size: u32,
    pub width_ratio: f64,
    /// 两条弹幕之间最小的水平距离
    pub horizontal_gap: f64,
    /// lane 大小
    pub lane_size: u32,
    /// 屏幕上滚动弹幕最多高度百分比
    pub float_percentage: f64,
    /// 屏幕上底部弹幕最多高度百分比
    pub bottom_percentage: f64,
    /// 透明度（0-255）
    pub opacity: u8,
    /// 是否加粗，1代表是，0代表否
    pub bold: bool,
    /// 描边
    pub outline: f64,
    /// 时间轴偏移
    pub time_offset: f64,
}

impl Default for DanmakuOption {
    fn default() -> Self {
        Self {
            duration: 15.0,
            font: "黑体".to_string(),
            font_size: 25,
            width_ratio: 1.2,
            horizontal_gap: 20.0,
            lane_size: 32,
            float_percentage: 0.5,
            bottom_percentage: 0.3,
            opacity: (0.3 * 255.0) as u8,
            bold: true,
            outline: 0.8,
            time_offset: 0.0,
        }
    }
}

#[derive(Clone)]
pub struct CanvasConfig {
    pub width: u64,
    pub height: u64,
    pub danmaku_option: &'static DanmakuOption,
}
impl CanvasConfig {
    pub fn new(danmaku_option: &'static DanmakuOption, page: &PageInfo) -> Self {
        let (width, height) = Self::dimension(page);
        Self {
            width,
            height,
            danmaku_option,
        }
    }

    /// 获取画布的宽高
    fn dimension(page: &PageInfo) -> (u64, u64) {
        let (width, height) = match &page.dimension {
            Some(d) => {
                if d.rotate == 0 {
                    (d.width, d.height)
                } else {
                    (d.height, d.width)
                }
            }
            None => (1280, 720),
        };
        // 对于指定的字体大小，画布的大小同样会影响到字体的实际显示大小
        // 怀疑字体的大小会根据 height 缩放，尝试将视频的 height 对齐到 720
        ((720.0 / height as f64 * width as f64) as u64, 720)
    }

    pub fn canvas(self) -> Canvas {
        let float_lanes_cnt =
            (self.danmaku_option.float_percentage * self.height as f64 / self.danmaku_option.lane_size as f64) as usize;

        Canvas {
            config: self,
            float_lanes: vec![None; float_lanes_cnt],
        }
    }
}

pub struct Canvas {
    pub config: CanvasConfig,
    pub float_lanes: Vec<Option<Lane>>,
}

impl Canvas {
    pub fn draw(&mut self, mut danmu: Danmu) -> Result<Option<Drawable>> {
        danmu.timeline_s += self.config.danmaku_option.time_offset;
        if danmu.timeline_s < 0.0 {
            return Ok(None);
        }
        match danmu.r#type {
            DanmuType::Float => Ok(self.draw_float(danmu)),
            DanmuType::Bottom | DanmuType::Top | DanmuType::Reverse => {
                // 不喜欢底部弹幕，直接转成 Bottom
                // 这是 feature 不是 bug
                danmu.r#type = DanmuType::Float;
                Ok(self.draw_float(danmu))
            }
        }
    }

    fn draw_float(&mut self, mut danmu: Danmu) -> Option<Drawable> {
        let mut collisions = Vec::with_capacity(self.float_lanes.len());
        for (idx, lane) in self.float_lanes.iter_mut().enumerate() {
            match lane {
                // 优先画不存在的槽位
                None => {
                    return Some(self.draw_float_in_lane(danmu, idx));
                }
                Some(l) => {
                    let col = l.available_for(&danmu, &self.config);
                    match col {
                        Collision::Separate | Collision::NotEnoughTime => {
                            return Some(self.draw_float_in_lane(danmu, idx));
                        }
                        Collision::Collide { time_needed } => {
                            collisions.push((FloatOrd(time_needed), idx));
                        }
                    }
                }
            }
        }
        // 允许部分弹幕在延迟后填充
        if !collisions.is_empty() {
            collisions.sort_unstable();
            let (FloatOrd(time_need), lane_idx) = collisions[0];
            if time_need < 1.0 {
                debug!("延迟弹幕 {} 秒", time_need);
                // 只允许延迟 1s
                danmu.timeline_s += time_need + 0.01; // 间隔也不要太小了
                return Some(self.draw_float_in_lane(danmu, lane_idx));
            }
        }
        debug!("skipping danmu: {}", danmu.content);
        None
    }

    fn draw_float_in_lane(&mut self, danmu: Danmu, lane_idx: usize) -> Drawable {
        self.float_lanes[lane_idx] = Some(Lane::draw(&danmu, &self.config));
        let y = lane_idx as i32 * self.config.danmaku_option.lane_size as i32;
        let l = danmu.length(&self.config);
        Drawable::new(
            danmu,
            self.config.danmaku_option.duration,
            "Float",
            DrawEffect::Move {
                start: (self.config.width as i32, y),
                end: (-(l as i32), y),
            },
        )
    }
}
