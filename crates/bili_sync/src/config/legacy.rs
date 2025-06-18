use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Result;
use sea_orm::DatabaseConnection;
use serde::de::{Deserializer, MapAccess, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Serialize};

use crate::bilibili::{CollectionItem, CollectionType, Credential, DanmakuOption, FilterOption};
use crate::config::Config;
use crate::config::default::{default_auth_token, default_bind_address, default_time_format};
use crate::config::item::{ConcurrentLimit, NFOTimeType, WatchLaterConfig};
use crate::utils::model::migrate_legacy_config;

#[derive(Serialize, Deserialize)]
pub struct LegacyConfig {
    #[serde(default = "default_auth_token")]
    pub auth_token: String,
    #[serde(default = "default_bind_address")]
    pub bind_address: String,
    pub credential: Credential,
    pub filter_option: FilterOption,
    #[serde(default)]
    pub danmaku_option: DanmakuOption,
    pub favorite_list: HashMap<String, PathBuf>,
    #[serde(
        default,
        serialize_with = "serialize_collection_list",
        deserialize_with = "deserialize_collection_list"
    )]
    pub collection_list: HashMap<CollectionItem, PathBuf>,
    #[serde(default)]
    pub submission_list: HashMap<String, PathBuf>,
    #[serde(default)]
    pub watch_later: WatchLaterConfig,
    pub video_name: String,
    pub page_name: String,
    pub interval: u64,
    pub upper_path: PathBuf,
    #[serde(default)]
    pub nfo_time_type: NFOTimeType,
    #[serde(default)]
    pub concurrent_limit: ConcurrentLimit,
    #[serde(default = "default_time_format")]
    pub time_format: String,
    #[serde(default)]
    pub cdn_sorting: bool,
}

impl LegacyConfig {
    async fn load_from_file(path: &Path) -> Result<Self> {
        let legacy_config_str = tokio::fs::read_to_string(path).await?;
        Ok(toml::from_str(&legacy_config_str)?)
    }

    pub async fn migrate_from_file(path: &Path, connection: &DatabaseConnection) -> Result<Config> {
        let legacy_config = Self::load_from_file(path).await?;
        migrate_legacy_config(&legacy_config, connection).await?;
        Ok(legacy_config.into())
    }
}

/*
后面是用于自定义 Collection 的序列化、反序列化的样板代码
*/
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
                                ));
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
                        ));
                    }
                };
                collection_list.insert(collection_item, value);
            }
            Ok(collection_list)
        }
    }

    deserializer.deserialize_map(CollectionListVisitor)
}
