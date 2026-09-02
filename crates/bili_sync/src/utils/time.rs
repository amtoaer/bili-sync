use chrono::{Days, Local, LocalResult, NaiveDateTime, TimeZone};

/// 将服务器本地墙钟时间换算为 UTC，与 SQLite 侧 `datetime(?, 'localtime')` 的语义对齐。
/// `upper` 表示换算结果将用作比较上界（lt/lte）：夏令时回拨产生的歧义时刻取较晚的
/// UTC 瞬间，使上界覆盖两种解释，与 SQLite 逐行转换的比较结果一致；用作下界（gte）
/// 时取较早瞬间，同理保证覆盖。不存在的本地时刻（春季拨快）按原值降级，视为 UTC。
///
/// 与 SQLite 的差异：SQLite 按查询时刻的当前偏移换算所有行，本函数按目标日期的历史
/// 偏移换算，DST 过渡窗口（每年约各一天）内两后端的比较边界可能相差至多 1 小时，
/// 本实现按日历日语义更精确。
pub fn local_to_utc(local: NaiveDateTime, upper: bool) -> NaiveDateTime {
    match Local.from_local_datetime(&local) {
        LocalResult::Single(dt) => dt.naive_utc(),
        LocalResult::Ambiguous(earlier, later) => {
            if upper {
                later.naive_utc()
            } else {
                earlier.naive_utc()
            }
        }
        LocalResult::None => local,
    }
}

/// 计算最近 7 个本地自然日的 UTC 起止边界，与 SQLite 侧
/// `DATE('now', '-' || n || ' days', 'localtime')` 的语义对齐。
/// 返回 (日期字符串, 起始 UTC, 结束 UTC)，按日期升序（最旧在前）。
pub fn last_seven_local_days() -> Vec<(String, NaiveDateTime, NaiveDateTime)> {
    let today = Local::now().date_naive();
    (0..7)
        .rev()
        .map(|n| {
            let day = today - Days::new(n);
            let next = day + Days::new(1);
            (
                day.format("%Y-%m-%d").to_string(),
                // 起始为下界取较早解释，结束为上界取较晚解释，均保证覆盖
                local_to_utc(day.and_hms_opt(0, 0, 0).expect("本地日期必然有效"), false),
                local_to_utc(next.and_hms_opt(0, 0, 0).expect("本地日期必然有效"), true),
            )
        })
        .collect()
}
