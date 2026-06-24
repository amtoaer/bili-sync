//! 弹幕增量更新的调度决策函数（纯函数，易测试）。
//!
//! 依据发布时间（pubtime）和上次同步时间（last_synced），给出当前时刻是否应该
//! 触发弹幕刷新的判决。策略参数来自 [`DanmakuUpdatePolicy`]，采用三段式：
//! 新鲜期 -> 成熟期 -> 老化期 -> 冷冻。

use chrono::{DateTime, Duration, Utc};

use crate::config::item::DanmakuUpdatePolicy;

/// 弹幕同步阶段（与数据库 `page.danmaku_sync_generation` 字段一一对应）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// 从未同步（0）。
    Initial = 0,
    /// 新鲜期（1），发布后 `fresh_days` 内。
    Fresh = 1,
    /// 成熟期（2）。
    Mature = 2,
    /// 老化期（3）。
    Cold = 3,
    /// 冷冻（4），不再自动同步。
    Frozen = 4,
}

impl Stage {
    pub fn from_generation(g: u32) -> Self {
        match g {
            0 => Stage::Initial,
            1 => Stage::Fresh,
            2 => Stage::Mature,
            3 => Stage::Cold,
            _ => Stage::Frozen,
        }
    }

    pub fn as_generation(self) -> u32 {
        self as u32
    }
}

/// 决策结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    /// 无需同步。
    Skip,
    /// 应同步，并将下一次写入的阶段推进到 `next_stage`。
    Sync { next_stage: Stage },
}

/// 仅根据视频年龄计算 page 应处于的阶段。
///
/// `allow_freeze` 控制是否允许返回 [`Stage::Frozen`]：
/// - 调度路径：传 `true`，超过 cold_days 的视频会被冻结（最后触发一次后不再自动同步）。
/// - 手动触发路径：传 `false`，cap 在 [`Stage::Cold`]，避免用户手动刷新后视频反而被冻结。
pub fn stage_for_age(
    policy: &DanmakuUpdatePolicy,
    pubtime: DateTime<Utc>,
    now: DateTime<Utc>,
    allow_freeze: bool,
) -> Stage {
    let age = now.signed_duration_since(pubtime).max(Duration::zero());
    let fresh_end = Duration::days(policy.fresh_days as i64);
    let mature_end = Duration::days(policy.mature_days as i64);
    let cold_end = Duration::days(policy.cold_days as i64);
    if allow_freeze && age >= cold_end {
        Stage::Frozen
    } else if age < fresh_end {
        Stage::Fresh
    } else if age < mature_end {
        Stage::Mature
    } else {
        Stage::Cold
    }
}

/// 判断某个 page 是否应该在当前时刻触发弹幕更新。
///
/// 语义说明：
/// - `generation` 表示**当前阶段**（`last_synced` 所处的阶段），如果 `generation` 已经是 `Frozen`，
///   永远返回 `Skip`。
/// - 根据当前时间相对 `pubtime` 的年龄，判断"本次应该处于哪个阶段"（target_stage）；
/// - 结合 `last_synced` 和对应阶段的间隔，判断是否应该刷新；
/// - 若 target_stage 已经超过 cold_days，返回一次 `Sync { next_stage: Frozen }`（最后触发一次即冻结）。
///
/// 首次同步（`last_synced=None`）：只要策略开启且未冻结，立即触发。
pub fn should_sync_danmaku(
    policy: &DanmakuUpdatePolicy,
    pubtime: DateTime<Utc>,
    last_synced: Option<DateTime<Utc>>,
    generation: u32,
    now: DateTime<Utc>,
) -> Decision {
    if !policy.enabled {
        return Decision::Skip;
    }

    let current_stage = Stage::from_generation(generation);
    if current_stage == Stage::Frozen {
        return Decision::Skip;
    }

    let target_stage = stage_for_age(policy, pubtime, now, true);
    if target_stage == Stage::Frozen {
        // 超过冷冻期：最后触发一次（无论之前是否同步过），之后置为 Frozen
        return Decision::Sync { next_stage: Stage::Frozen };
    }

    let interval = match target_stage {
        Stage::Fresh => Duration::hours(policy.fresh_interval_hours as i64),
        Stage::Mature => Duration::days(policy.mature_interval_days as i64),
        Stage::Cold => Duration::days(policy.cold_interval_days as i64),
        // 上面已 early-return，理论不可达
        Stage::Initial | Stage::Frozen => Duration::zero(),
    };

    match last_synced {
        // 从未同步过，立即触发
        None => Decision::Sync { next_stage: target_stage },
        Some(ts) => {
            let since_last = now.signed_duration_since(ts);
            // 阶段刚刚迁移（比如从新鲜期迈入成熟期），立即触发一次，把 generation 同步到新阶段
            if target_stage.as_generation() > current_stage.as_generation() {
                return Decision::Sync { next_stage: target_stage };
            }
            if since_last >= interval {
                Decision::Sync { next_stage: target_stage }
            } else {
                Decision::Skip
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> DanmakuUpdatePolicy {
        DanmakuUpdatePolicy {
            enabled: true,
            fresh_days: 3,
            fresh_interval_hours: 6,
            mature_days: 30,
            mature_interval_days: 3,
            cold_days: 180,
            cold_interval_days: 30,
        }
    }

    fn t(days: i64, hours: i64) -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(0, 0).unwrap() + Duration::days(days) + Duration::hours(hours)
    }

    #[test]
    fn disabled_always_skip() {
        let mut p = policy();
        p.enabled = false;
        let now = t(10, 0);
        let pub_t = t(0, 0);
        assert_eq!(
            should_sync_danmaku(&p, pub_t, None, 0, now),
            Decision::Skip
        );
    }

    #[test]
    fn first_time_fresh_triggers_immediately() {
        let p = policy();
        let pub_t = t(0, 0);
        let now = t(0, 1); // 发布 1 小时后，首次
        assert_eq!(
            should_sync_danmaku(&p, pub_t, None, 0, now),
            Decision::Sync { next_stage: Stage::Fresh }
        );
    }

    #[test]
    fn fresh_interval_not_elapsed_skips() {
        let p = policy();
        let pub_t = t(0, 0);
        let last = t(0, 2);
        let now = t(0, 5); // 距上次 3 小时，不足 6 小时
        assert_eq!(
            should_sync_danmaku(&p, pub_t, Some(last), Stage::Fresh.as_generation(), now),
            Decision::Skip
        );
    }

    #[test]
    fn fresh_interval_elapsed_syncs() {
        let p = policy();
        let pub_t = t(0, 0);
        let last = t(0, 2);
        let now = t(0, 9); // 距上次 7 小时，超过 6 小时
        assert_eq!(
            should_sync_danmaku(&p, pub_t, Some(last), Stage::Fresh.as_generation(), now),
            Decision::Sync { next_stage: Stage::Fresh }
        );
    }

    #[test]
    fn stage_transition_fresh_to_mature_triggers_once() {
        // 上次在新鲜期同步（2h），现在已进入成熟期（第 5 天），即使成熟期的间隔（3 天）未到，
        // 也应立即触发一次，推进 generation。
        let p = policy();
        let pub_t = t(0, 0);
        let last = t(0, 2);
        let now = t(5, 0);
        assert_eq!(
            should_sync_danmaku(&p, pub_t, Some(last), Stage::Fresh.as_generation(), now),
            Decision::Sync { next_stage: Stage::Mature }
        );
    }

    #[test]
    fn mature_interval_not_elapsed_skips() {
        let p = policy();
        let pub_t = t(0, 0);
        let last = t(5, 0); // 成熟期开始就同步了一次
        let now = t(7, 0); // 过了 2 天，不足 3 天
        assert_eq!(
            should_sync_danmaku(&p, pub_t, Some(last), Stage::Mature.as_generation(), now),
            Decision::Skip
        );
    }

    #[test]
    fn mature_interval_elapsed_syncs() {
        let p = policy();
        let pub_t = t(0, 0);
        let last = t(5, 0);
        let now = t(9, 0); // 过了 4 天
        assert_eq!(
            should_sync_danmaku(&p, pub_t, Some(last), Stage::Mature.as_generation(), now),
            Decision::Sync { next_stage: Stage::Mature }
        );
    }

    #[test]
    fn mature_to_cold_stage_transition() {
        let p = policy();
        let pub_t = t(0, 0);
        let last = t(15, 0); // 成熟期中段
        let now = t(35, 0); // 已进入老化期（>30 天）
        assert_eq!(
            should_sync_danmaku(&p, pub_t, Some(last), Stage::Mature.as_generation(), now),
            Decision::Sync { next_stage: Stage::Cold }
        );
    }

    #[test]
    fn cold_interval_elapsed_syncs() {
        let p = policy();
        let pub_t = t(0, 0);
        let last = t(40, 0);
        let now = t(80, 0); // 过了 40 天
        assert_eq!(
            should_sync_danmaku(&p, pub_t, Some(last), Stage::Cold.as_generation(), now),
            Decision::Sync { next_stage: Stage::Cold }
        );
    }

    #[test]
    fn exceeding_cold_days_final_sync_then_freeze() {
        let p = policy();
        let pub_t = t(0, 0);
        let last = t(100, 0);
        let now = t(181, 0); // 超过 cold_days=180
        assert_eq!(
            should_sync_danmaku(&p, pub_t, Some(last), Stage::Cold.as_generation(), now),
            Decision::Sync { next_stage: Stage::Frozen }
        );
    }

    #[test]
    fn stage_for_age_classifies_correctly() {
        let p = policy();
        let pub_t = t(0, 0);
        // age=0 → Fresh
        assert_eq!(stage_for_age(&p, pub_t, t(0, 1), true), Stage::Fresh);
        // age=4 days → Mature
        assert_eq!(stage_for_age(&p, pub_t, t(4, 0), true), Stage::Mature);
        // age=40 days → Cold
        assert_eq!(stage_for_age(&p, pub_t, t(40, 0), true), Stage::Cold);
        // age=200 days, allow_freeze=true → Frozen
        assert_eq!(stage_for_age(&p, pub_t, t(200, 0), true), Stage::Frozen);
        // age=200 days, allow_freeze=false → Cold（手动模式不冻结）
        assert_eq!(stage_for_age(&p, pub_t, t(200, 0), false), Stage::Cold);
    }

    #[test]
    fn frozen_stays_frozen() {
        let p = policy();
        let pub_t = t(0, 0);
        let last = t(181, 0);
        let now = t(500, 0);
        assert_eq!(
            should_sync_danmaku(&p, pub_t, Some(last), Stage::Frozen.as_generation(), now),
            Decision::Skip
        );
    }

    #[test]
    fn never_synced_old_video_final_sync() {
        // 一个老视频第一次被纳入同步范围（age 已超 cold_days），应触发一次并直接冻结。
        let p = policy();
        let pub_t = t(0, 0);
        let now = t(200, 0);
        assert_eq!(
            should_sync_danmaku(&p, pub_t, None, 0, now),
            Decision::Sync { next_stage: Stage::Frozen }
        );
    }

    #[test]
    fn pubtime_in_future_clamps_to_zero_age() {
        // 时钟偏差导致发布时间晚于当前时间：按 age=0 处理（新鲜期首次）。
        let p = policy();
        let pub_t = t(10, 0);
        let now = t(5, 0);
        assert_eq!(
            should_sync_danmaku(&p, pub_t, None, 0, now),
            Decision::Sync { next_stage: Stage::Fresh }
        );
    }

    #[test]
    fn policy_as_once_after_days_equivalent() {
        // 方案 A：只触发一次然后冻结。通过把 fresh/mature 设为 0、cold_days=N 实现。
        let p = DanmakuUpdatePolicy {
            enabled: true,
            fresh_days: 0,
            fresh_interval_hours: 1,
            mature_days: 0,
            mature_interval_days: 1,
            cold_days: 7,
            cold_interval_days: 999_999, // 实际不会触发
        };
        let pub_t = t(0, 0);
        // 7 天内，cold_interval_days 极大，last_synced=None 首次必触发
        let now = t(3, 0);
        assert_eq!(
            should_sync_danmaku(&p, pub_t, None, 0, now),
            Decision::Sync { next_stage: Stage::Cold }
        );
        // 已同步过，间隔极大，跳过
        let last = t(3, 0);
        let now2 = t(6, 0);
        assert_eq!(
            should_sync_danmaku(&p, pub_t, Some(last), Stage::Cold.as_generation(), now2),
            Decision::Skip
        );
        // 超过 7 天，最后一次 + 冻结
        let now3 = t(8, 0);
        assert_eq!(
            should_sync_danmaku(&p, pub_t, Some(last), Stage::Cold.as_generation(), now3),
            Decision::Sync { next_stage: Stage::Frozen }
        );
    }
}
