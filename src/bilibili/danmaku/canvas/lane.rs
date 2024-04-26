use crate::bilibili::danmaku::canvas::CanvasConfig;
use crate::bilibili::danmaku::Danmu;

pub enum Collision {
    // 会越来越远
    Separate,
    // 时间够可以追上，但是时间不够
    NotEnoughTime,
    // 需要额外的时间才可以避免碰撞
    Collide { time_needed: f64 },
}

/// 表示一个弹幕槽位
#[derive(Debug, Clone)]
pub struct Lane {
    last_shoot_time: f64,
    last_length: f64,
}

impl Lane {
    pub fn draw(danmu: &Danmu, config: &CanvasConfig) -> Self {
        Lane {
            last_shoot_time: danmu.timeline_s,
            last_length: danmu.length(config),
        }
    }

    /// 这个槽位是否可以发射另外一条弹幕，返回可能的情形
    pub fn available_for(&self, other: &Danmu, config: &CanvasConfig) -> Collision {
        #[allow(non_snake_case)]
        let T = config.danmaku_option.duration;
        #[allow(non_snake_case)]
        let W = config.width as f64;
        let gap = config.danmaku_option.horizontal_gap;

        // 先计算我的速度
        let t1 = self.last_shoot_time;
        let t2 = other.timeline_s;
        let l1 = self.last_length;
        let l2 = other.length(config);

        let v1 = (W + l1) / T;
        let v2 = (W + l2) / T;

        let delta_t = t2 - t1;
        // 第一条弹幕右边到屏幕右边的距离
        let delta_x = v1 * delta_t - l1;
        // 没有足够的空间，必定碰撞
        if delta_x < gap {
            if l2 <= l1 {
                // l2 比 l1 短，因此比它慢
                // 只需要把 l2 安排在 l1 之后就可以避免碰撞
                Collision::Collide {
                    time_needed: (gap - delta_x) / v1,
                }
            } else {
                // 需要延长额外的时间，使得当第一条消失的时候，第二条也有足够的距离
                // 第一条消失的时间点是 (t1 + T)
                // 这个时候第二条的左侧应该在距离出发点 W - gap 处，
                // 第二条已经出发 (W - gap) / v2 时间，因此在 t1 + T - (W - gap) / v2 出发
                // 所需要的额外时间就 - t2
                // let time_needed = (t1 + T - (W - gap) / v2) - t2;
                let time_needed = (T - (W - gap) / v2) - delta_t;
                Collision::Collide { time_needed }
            }
        } else {
            // 第一条已经发射
            if l2 <= l1 {
                // 如果 l2 < l1，则它永远追不上前者，可以发射
                Collision::Separate
            } else {
                // 需要算追击问题了，
                // l1 已经消失，但是 l2 可能追上，我们计算 l1 刚好消失的时候：
                // 此刻是 t1 + T
                // l2 的头部应该在距离起点 v2 * (t1 + T - t2) 处
                let pos = v2 * (T - delta_t);
                if pos < (W - gap) {
                    Collision::NotEnoughTime
                } else {
                    // 需要额外的时间
                    Collision::Collide {
                        time_needed: (pos - (W - gap)) / v2,
                    }
                }
            }
        }
    }
}
