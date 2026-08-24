use chrono::{DateTime, Duration, Utc};

use crate::config::DanmakuUpdateMilestone;

pub fn should_sync_danmaku(
    milestones: &[DanmakuUpdateMilestone],
    pubtime: DateTime<Utc>,
    last_synced_at: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> bool {
    let last_synced_at = last_synced_at.unwrap_or(pubtime);
    let mut stage_start = pubtime;
    for milestone in milestones {
        match milestone {
            DanmakuUpdateMilestone::Once { at_days } => {
                let sync_at = pubtime + Duration::days((*at_days).into());
                if last_synced_at < sync_at && now >= sync_at {
                    return true;
                }
                stage_start = sync_at;
            }
            DanmakuUpdateMilestone::Periodic {
                until_days,
                interval_hours,
            } => {
                let stage_end = pubtime + Duration::days((*until_days).into());
                if now >= stage_end {
                    if last_synced_at < stage_end {
                        return true;
                    }
                } else if now > stage_start {
                    let interval = Duration::hours((*interval_hours).into());
                    return last_synced_at < stage_start || now.signed_duration_since(last_synced_at) >= interval;
                } else {
                    return false;
                }
                stage_start = stage_end;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    fn periodic_policy() -> Vec<DanmakuUpdateMilestone> {
        vec![periodic(3, 6), periodic(30, 3 * 24), periodic(180, 30 * 24)]
    }

    fn once(at_days: u32) -> DanmakuUpdateMilestone {
        DanmakuUpdateMilestone::Once { at_days }
    }

    fn periodic(until_days: u32, interval_hours: u32) -> DanmakuUpdateMilestone {
        DanmakuUpdateMilestone::Periodic {
            until_days,
            interval_hours,
        }
    }

    fn t(days: i64, hours: i64) -> DateTime<Utc> {
        Utc.timestamp_opt(0, 0).unwrap() + Duration::days(days) + Duration::hours(hours)
    }

    #[test]
    fn empty_milestones_never_syncs() {
        let policy = vec![];
        assert!(!should_sync_danmaku(&policy, t(0, 0), None, t(1, 0)));
    }

    #[test]
    fn once_items_sync_only_after_their_milestones() {
        let policy = vec![once(3), once(10), once(30)];
        assert!(!should_sync_danmaku(&policy, t(0, 0), Some(t(0, 0)), t(2, 23)));
        assert!(should_sync_danmaku(&policy, t(0, 0), Some(t(0, 0)), t(3, 0)));
        assert!(!should_sync_danmaku(&policy, t(0, 0), Some(t(5, 0)), t(9, 23)));
        assert!(should_sync_danmaku(&policy, t(0, 0), Some(t(5, 0)), t(10, 0)));
    }

    #[test]
    fn missing_last_synced_at_uses_pubtime_as_the_baseline() {
        let once_policy = vec![once(3)];
        assert!(!should_sync_danmaku(&once_policy, t(0, 0), None, t(2, 23)));
        assert!(should_sync_danmaku(&once_policy, t(0, 0), None, t(3, 0)));

        let periodic_policy = vec![periodic(3, 6)];
        assert!(!should_sync_danmaku(&periodic_policy, t(0, 0), None, t(0, 5)));
        assert!(should_sync_danmaku(&periodic_policy, t(0, 0), None, t(0, 6)));
    }

    #[test]
    fn periodic_items_use_the_last_successful_sync() {
        let policy = periodic_policy();
        assert!(!should_sync_danmaku(&policy, t(0, 0), Some(t(1, 0)), t(1, 5)));
        assert!(should_sync_danmaku(&policy, t(0, 0), Some(t(1, 0)), t(1, 6)));
    }

    #[test]
    fn entering_a_new_periodic_stage_syncs_immediately() {
        assert!(should_sync_danmaku(
            &periodic_policy(),
            t(0, 0),
            Some(t(2, 23)),
            t(3, 0)
        ));
    }

    #[test]
    fn mixed_items_share_one_timeline() {
        let policy = vec![periodic(3, 6), once(10), periodic(30, 3 * 24)];
        assert!(!should_sync_danmaku(&policy, t(0, 0), Some(t(4, 0)), t(9, 23)));
        assert!(should_sync_danmaku(&policy, t(0, 0), Some(t(4, 0)), t(10, 0)));
        assert!(!should_sync_danmaku(&policy, t(0, 0), Some(t(10, 0)), t(12, 23)));
        assert!(should_sync_danmaku(&policy, t(0, 0), Some(t(10, 0)), t(13, 0)));
    }

    #[test]
    fn passing_multiple_items_requires_only_one_sync() {
        let policy = vec![once(3), once(10), once(30)];
        assert!(should_sync_danmaku(&policy, t(0, 0), Some(t(1, 0)), t(31, 0)));
        assert!(!should_sync_danmaku(&policy, t(0, 0), Some(t(31, 0)), t(31, 0)));
    }

    #[test]
    fn final_periodic_boundary_syncs_once_then_freezes() {
        assert!(should_sync_danmaku(
            &periodic_policy(),
            t(0, 0),
            Some(t(170, 0)),
            t(200, 0)
        ));
        assert!(!should_sync_danmaku(
            &periodic_policy(),
            t(0, 0),
            Some(t(180, 0)),
            t(200, 0)
        ));
    }

    #[test]
    fn interval_changes_take_effect_immediately() {
        let mut policy = periodic_policy();
        policy[1] = periodic(30, 10 * 24);
        assert!(!should_sync_danmaku(&policy, t(0, 0), Some(t(10, 0)), t(15, 0)));
        policy[1] = periodic(30, 3 * 24);
        assert!(should_sync_danmaku(&policy, t(0, 0), Some(t(10, 0)), t(15, 0)));
    }

    #[test]
    fn inserted_once_item_takes_effect_immediately() {
        let policy = vec![once(3), once(7), once(10)];
        assert!(should_sync_danmaku(&policy, t(0, 0), Some(t(5, 0)), t(8, 0)));
    }
}
