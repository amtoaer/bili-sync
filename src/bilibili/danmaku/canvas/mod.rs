//! 决定绘画策略
mod lane;

use anyhow::Result;
use float_ord::FloatOrd;
use lane::Lane;

use super::{Danmu, Drawable};
use crate::bilibili::danmaku::canvas::lane::Collision;
use crate::bilibili::danmaku::danmu::DanmuType;
use crate::bilibili::danmaku::DrawEffect;

#[derive(Debug, Default, Clone, serde::Deserialize, serde::Serialize)]
pub struct SubtitleOption {
    pub duration: f64,
    pub width: u32,
    pub height: u32,
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
    #[serde(skip_deserializing)]
    pub bottom_percentage: f64,
    /// 透明度
    #[serde(rename = "alpha", deserialize_with = "deserialize_alpha_to_opacity")]
    pub opacity: u8,
    /// 是否加粗，1代表是，0代表否
    pub bold: bool,
    /// 描边
    pub outline: f64,
    /// 时间轴偏移
    pub time_offset: f64,
}
fn deserialize_alpha_to_opacity<'de, D>(deserializer: D) -> Result<u8, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Deserialize;
    let alpha = f64::deserialize(deserializer)?;
    if !(0.0..=1.0).contains(&alpha) {
        return Err(serde::de::Error::custom("alpha must be between 0 and 1"));
    }
    Ok(255 - (alpha * 255.) as u8)
}

impl SubtitleOption {
    pub fn canvas(self) -> Canvas {
        let float_lanes_cnt = (self.float_percentage * self.height as f64 / self.lane_size as f64) as usize;

        Canvas {
            config: self,
            float_lanes: vec![None; float_lanes_cnt],
        }
    }
}

pub struct Canvas {
    pub config: SubtitleOption,
    pub float_lanes: Vec<Option<Lane>>,
}

impl Canvas {
    pub fn draw(&mut self, mut danmu: Danmu) -> Result<Option<Drawable>> {
        danmu.timeline_s += self.config.time_offset;
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
        let y = lane_idx as i32 * self.config.lane_size as i32;
        let l = danmu.length(&self.config);
        Drawable::new(
            danmu,
            self.config.duration,
            "Float",
            DrawEffect::Move {
                start: (self.config.width as i32, y),
                end: (-(l as i32), y),
            },
        )
    }
}
