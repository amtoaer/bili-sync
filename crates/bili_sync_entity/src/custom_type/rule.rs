use std::fmt::Display;

use derivative::Derivative;
use sea_orm::FromJsonQueryResult;
use sea_orm::prelude::DateTime;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Clone, Debug, Serialize, Deserialize, Derivative)]
#[derivative(PartialEq, Eq)]
#[serde(rename_all = "camelCase", tag = "operator", content = "value")]
pub enum Condition<T: Serialize + Display> {
    Equals(T),
    Contains(T),
    #[serde(rename = "icontains")]
    IContains(T),
    #[serde(deserialize_with = "deserialize_regex", serialize_with = "serialize_regex")]
    MatchesRegex(String, #[derivative(PartialEq = "ignore")] regex::Regex),
    Prefix(T),
    Suffix(T),
    GreaterThan(T),
    LessThan(T),
    Between(T, T),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, FromJsonQueryResult)]
#[serde(rename_all = "camelCase", tag = "field", content = "rule")]
pub enum RuleTarget {
    Title(Condition<String>),
    Tags(Condition<String>),
    FavTime(Condition<DateTime>),
    PubTime(Condition<DateTime>),
    PageCount(Condition<usize>),
    Not(Box<RuleTarget>),
}

pub type AndGroup = Vec<RuleTarget>;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct Rule(pub Vec<AndGroup>);

impl<T: Serialize + Display> Display for Condition<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Condition::Equals(v) => write!(f, "等于“{}”", v),
            Condition::Contains(v) => write!(f, "包含“{}”", v),
            Condition::IContains(v) => write!(f, "包含（不区分大小写）“{}”", v),
            Condition::MatchesRegex(pat, _) => write!(f, "匹配“{}”", pat),
            Condition::Prefix(v) => write!(f, "以“{}”开头", v),
            Condition::Suffix(v) => write!(f, "以“{}”结尾", v),
            Condition::GreaterThan(v) => write!(f, "大于“{}”", v),
            Condition::LessThan(v) => write!(f, "小于“{}”", v),
            Condition::Between(start, end) => write!(f, "在“{}”和“{}”之间", start, end),
        }
    }
}

impl Display for RuleTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fn get_field_name(rt: &RuleTarget, depth: usize) -> &'static str {
            match rt {
                RuleTarget::Title(_) => "标题",
                RuleTarget::Tags(_) => "标签",
                RuleTarget::FavTime(_) => "收藏时间",
                RuleTarget::PubTime(_) => "发布时间",
                RuleTarget::PageCount(_) => "视频分页数量",
                RuleTarget::Not(inner) => {
                    if depth == 0 {
                        get_field_name(inner, depth + 1)
                    } else {
                        "格式化失败"
                    }
                }
            }
        }
        let field_name = get_field_name(self, 0);
        match self {
            RuleTarget::Not(inner) => match inner.as_ref() {
                RuleTarget::Title(cond) | RuleTarget::Tags(cond) => write!(f, "{}不{}", field_name, cond),
                RuleTarget::FavTime(cond) | RuleTarget::PubTime(cond) => {
                    write!(f, "{}不{}", field_name, cond)
                }
                RuleTarget::PageCount(cond) => write!(f, "{}不{}", field_name, cond),
                RuleTarget::Not(_) => write!(f, "格式化失败"),
            },
            RuleTarget::Title(cond) | RuleTarget::Tags(cond) => write!(f, "{}{}", field_name, cond),
            RuleTarget::FavTime(cond) | RuleTarget::PubTime(cond) => {
                write!(f, "{}{}", field_name, cond)
            }
            RuleTarget::PageCount(cond) => write!(f, "{}{}", field_name, cond),
        }
    }
}

impl Display for Rule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let groups: Vec<String> = self
            .0
            .iter()
            .map(|group| {
                let conditions: Vec<String> = group.iter().map(|target| format!("（{}）", target)).collect();
                format!("「{}」", conditions.join("且"))
            })
            .collect();
        write!(f, "{}", groups.join("或"))
    }
}

fn deserialize_regex<'de, D>(deserializer: D) -> Result<(String, regex::Regex), D::Error>
where
    D: Deserializer<'de>,
{
    let pattern = String::deserialize(deserializer)?;
    // 反序列化时预编译 regex，优化性能
    let regex = regex::Regex::new(&pattern).map_err(serde::de::Error::custom)?;
    Ok((pattern, regex))
}

fn serialize_regex<S>(pattern: &str, _regex: &regex::Regex, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(pattern)
}
