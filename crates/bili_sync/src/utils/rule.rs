use bili_sync_entity::rule::{AndGroup, Condition, Rule, RuleTarget};
use bili_sync_entity::{page, video};
use chrono::NaiveDateTime;

pub(crate) trait FieldEvaluatable<T> {
    fn evaluate(&self, value: T) -> bool;
}

pub(crate) trait Evaluatable {
    fn evaluate(&self, video: &video::ActiveModel, pages: &[page::ActiveModel]) -> bool;
}

impl FieldEvaluatable<&str> for Condition<String> {
    fn evaluate(&self, value: &str) -> bool {
        match self {
            Condition::Equals(expected) => expected == value,
            Condition::Contains(substring) => value.contains(substring),
            Condition::Prefix(prefix) => value.starts_with(prefix),
            Condition::Suffix(suffix) => value.ends_with(suffix),
            Condition::MatchesRegex(_, regex) => regex.as_ref().is_ok_and(|re| re.is_match(value)),
            _ => false,
        }
    }
}

impl FieldEvaluatable<usize> for Condition<usize> {
    fn evaluate(&self, value: usize) -> bool {
        match self {
            Condition::Equals(expected) => *expected == value,
            Condition::GreaterThan(threshold) => value > *threshold,
            Condition::LessThan(threshold) => value < *threshold,
            Condition::Between(start, end) => value > *start && value < *end,
            _ => false,
        }
    }
}

impl FieldEvaluatable<&NaiveDateTime> for Condition<NaiveDateTime> {
    fn evaluate(&self, value: &NaiveDateTime) -> bool {
        match self {
            Condition::Equals(expected) => expected == value,
            Condition::GreaterThan(threshold) => value > threshold,
            Condition::LessThan(threshold) => value < threshold,
            Condition::Between(start, end) => value > start && value < end,
            _ => false,
        }
    }
}

impl Evaluatable for RuleTarget {
    fn evaluate(&self, video: &video::ActiveModel, pages: &[page::ActiveModel]) -> bool {
        match self {
            RuleTarget::Title(cond) => video.name.try_as_ref().is_some_and(|title| cond.evaluate(&title)),
            // 目前的所有条件都是分别针对全体标签进行 any 评估的，例如 Prefix("a") && Suffix("b") 意味着 any(tag.Prefix("a")) && any(tag.Suffix("b")) 而非 any(tag.Prefix("a") && tag.Suffix("b"))
            // 这可能不满足用户预期，但应该问题不大，如果真有很多人用复杂标签筛选再单独改
            RuleTarget::Tags(cond) => video
                .tags
                .try_as_ref()
                .and_then(|t| t.as_ref())
                .is_some_and(|tags| tags.0.iter().any(|tag| cond.evaluate(&tag))),
            RuleTarget::FavTime(cond) => video
                .favtime
                .try_as_ref()
                .is_some_and(|fav_time| cond.evaluate(&fav_time)),
            RuleTarget::PubTime(cond) => video
                .pubtime
                .try_as_ref()
                .is_some_and(|pub_time| cond.evaluate(&pub_time)),
            RuleTarget::PageCount(cond) => cond.evaluate(pages.len()),
            RuleTarget::Not(inner) => !inner.evaluate(video, pages),
        }
    }
}

impl Evaluatable for AndGroup {
    fn evaluate(&self, video: &video::ActiveModel, pages: &[page::ActiveModel]) -> bool {
        self.iter().all(|target| target.evaluate(video, pages))
    }
}

impl Evaluatable for Rule {
    fn evaluate(&self, video: &video::ActiveModel, pages: &[page::ActiveModel]) -> bool {
        self.0.iter().any(|group| group.evaluate(video, pages))
    }
}

#[cfg(test)]
mod tests {
    use bili_sync_entity::page;
    use chrono::NaiveDate;
    use sea_orm::ActiveValue::Set;

    use super::*;

    #[test]
    fn test_display() {
        let test_cases = vec![
            (
                Rule(vec![vec![RuleTarget::Title(Condition::Contains("唐氏".to_string()))]]),
                "「(标题包含“唐氏”)」",
            ),
            (
                Rule(vec![vec![
                    RuleTarget::Title(Condition::Prefix("街霸".to_string())),
                    RuleTarget::Tags(Condition::Contains("套路".to_string())),
                ]]),
                "「(标题以“街霸”开头)且(标签包含“套路”)」",
            ),
            (
                Rule(vec![
                    vec![
                        RuleTarget::Title(Condition::Contains("Rust".to_string())),
                        RuleTarget::PageCount(Condition::GreaterThan(5)),
                    ],
                    vec![
                        RuleTarget::Tags(Condition::Suffix("入门".to_string())),
                        RuleTarget::PubTime(Condition::GreaterThan(
                            NaiveDate::from_ymd_opt(2023, 1, 1)
                                .unwrap()
                                .and_hms_opt(0, 0, 0)
                                .unwrap(),
                        )),
                    ],
                ]),
                "「(标题包含“Rust”)且(视频分页数量大于“5”)」或「(标签以“入门”结尾)且(发布时间大于“2023-01-01 00:00:00”)」",
            ),
            (
                Rule(vec![vec![
                    RuleTarget::Not(Box::new(RuleTarget::Title(Condition::Contains("广告".to_string())))),
                    RuleTarget::PageCount(Condition::LessThan(10)),
                ]]),
                "「(标题不包含“广告”)且(视频分页数量小于“10”)」",
            ),
            (
                Rule(vec![vec![
                    RuleTarget::FavTime(Condition::Between(
                        NaiveDate::from_ymd_opt(2023, 6, 1)
                            .unwrap()
                            .and_hms_opt(0, 0, 0)
                            .unwrap(),
                        NaiveDate::from_ymd_opt(2023, 12, 31)
                            .unwrap()
                            .and_hms_opt(23, 59, 59)
                            .unwrap(),
                    )),
                    RuleTarget::Tags(Condition::MatchesRegex(
                        "技术|教程".to_string(),
                        Ok(regex::Regex::new("技术|教程").unwrap()),
                    )),
                ]]),
                "「(收藏时间在“2023-06-01 00:00:00”和“2023-12-31 23:59:59”之间)且(标签匹配“技术|教程”)」",
            ),
        ];

        for (rule, expected) in test_cases {
            assert_eq!(rule.to_string(), expected);
        }
    }

    #[test]
    fn test_evaluate() {
        let test_cases = vec![
            (
                (
                    video::ActiveModel {
                        name: Set("骂谁唐氏呢！！！".to_string()),
                        ..Default::default()
                    },
                    vec![],
                ),
                Rule(vec![vec![RuleTarget::Title(Condition::Contains("唐氏".to_string()))]]),
                true,
            ),
            (
                (
                    video::ActiveModel::default(),
                    vec![page::ActiveModel::default(); 2],
                ),
                Rule(vec![vec![RuleTarget::PageCount(Condition::Equals(1))]]),
                false,
            ),
            (
                (
                    video::ActiveModel{
                        tags: Set(Some(vec!["原神".to_owned(),"永雏塔菲".to_owned(),"虚拟主播".to_owned()].into())),
                        ..Default::default()
                    },
                    vec![],
                ),
                Rule (vec![vec![RuleTarget::Not(Box::new(RuleTarget::Tags(Condition::Equals(
                        "原神".to_string(),
                    ))))]],
                ),
                false,
            ),
            (
                (
                    video::ActiveModel {
                        name: Set(
                            "万字怒扒网易《归唐》底裤！中国首款大厂买断制单机，靠谱吗？——全网最全！官方非独家幕后！关于《归唐》PV的所有秘密~都在这里了~".to_owned(),
                        ),
                        ..Default::default()
                    },
                    vec![],
                ),
                Rule(vec![vec![RuleTarget::Not(Box::new(RuleTarget::Title(Condition::MatchesRegex(
                        r"^\S+字(解析|怒扒|拆解)".to_owned(),
                        Ok(regex::Regex::new(r"^\S+字(解析|怒扒)").unwrap()),
                    ))))]],
                ),
                false,
            ),
        ];

        for ((video, pages), rule, expected) in test_cases {
            assert_eq!(rule.evaluate(&video, &pages), expected);
        }
    }
}
