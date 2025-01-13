use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::Result;
use serde::de::{Deserializer, MapAccess, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Serialize};

use crate::bilibili::{CollectionItem, CollectionType};
use crate::utils::filenamify::filenamify;

/// 稍后再看的配置
#[derive(Serialize, Deserialize, Default)]
pub struct WatchLaterConfig {
    pub enabled: bool,
    pub path: PathBuf,
}

/// NFO 文件使用的时间类型
#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum NFOTimeType {
    #[default]
    FavTime,
    PubTime,
}

/// 并发下载相关的配置
#[derive(Serialize, Deserialize)]
pub struct ConcurrentLimit {
    pub video: usize,
    pub page: usize,
    pub rate_limit: Option<RateLimit>,
}

#[derive(Serialize, Deserialize)]
pub struct RateLimit {
    pub limit: usize,
    pub duration: u64,
}

impl Default for ConcurrentLimit {
    fn default() -> Self {
        Self {
            video: 3,
            page: 2,
            // 默认的限速配置，每 250ms 允许请求 4 次
            rate_limit: Some(RateLimit {
                limit: 4,
                duration: 250,
            }),
        }
    }
}

pub trait PathSafeTemplate {
    fn path_safe_register(&mut self, name: &'static str, template: &'static str) -> Result<()>;
    fn path_safe_render(&self, name: &'static str, data: &serde_json::Value) -> Result<String>;
}

/// 通过将模板字符串中的分隔符替换为自定义的字符串，使得模板字符串中的分隔符得以保留
impl PathSafeTemplate for handlebars::Handlebars<'_> {
    fn path_safe_register(&mut self, name: &'static str, template: &'static str) -> Result<()> {
        Ok(self.register_template_string(name, template.replace(std::path::MAIN_SEPARATOR_STR, "__SEP__"))?)
    }

    fn path_safe_render(&self, name: &'static str, data: &serde_json::Value) -> Result<String> {
        Ok(filenamify(&self.render(name, data)?).replace("__SEP__", std::path::MAIN_SEPARATOR_STR))
    }
}
/* 后面是用于自定义 Collection 的序列化、反序列化的样板代码 */
pub(super) fn serialize_collection_list<S>(
    collection_list: &HashMap<CollectionItem, PathBuf>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let mut map = serializer.serialize_map(Some(collection_list.len()))?;
    for (k, v) in collection_list {
        let prefix = match k.collection_type {
            CollectionType::Series => "series",
            CollectionType::Season => "season",
        };
        map.serialize_entry(&[prefix, &k.mid, &k.sid].join(":"), v)?;
    }
    map.end()
}

pub(super) fn deserialize_collection_list<'de, D>(deserializer: D) -> Result<HashMap<CollectionItem, PathBuf>, D::Error>
where
    D: Deserializer<'de>,
{
    struct CollectionListVisitor;

    impl<'de> Visitor<'de> for CollectionListVisitor {
        type Value = HashMap<CollectionItem, PathBuf>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a map of collection list")
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: MapAccess<'de>,
        {
            let mut collection_list = HashMap::new();
            while let Some((key, value)) = map.next_entry::<String, PathBuf>()? {
                let collection_item = match key.split(':').collect::<Vec<&str>>().as_slice() {
                    [prefix, mid, sid] => {
                        let collection_type = match *prefix {
                            "series" => CollectionType::Series,
                            "season" => CollectionType::Season,
                            _ => {
                                return Err(serde::de::Error::custom(
                                    "invalid collection type, should be series or season",
                                ))
                            }
                        };
                        CollectionItem {
                            mid: mid.to_string(),
                            sid: sid.to_string(),
                            collection_type,
                        }
                    }
                    _ => {
                        return Err(serde::de::Error::custom(
                            "invalid collection key, should be series:mid:sid or season:mid:sid",
                        ))
                    }
                };
                collection_list.insert(collection_item, value);
            }
            Ok(collection_list)
        }
    }

    deserializer.deserialize_map(CollectionListVisitor)
}
