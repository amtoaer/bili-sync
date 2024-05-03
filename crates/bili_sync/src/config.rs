use std::borrow::Cow;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use arc_swap::ArcSwapOption;
use once_cell::sync::Lazy;
use serde::de::{Deserializer, MapAccess, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Serialize};

use crate::bilibili::{CollectionItem, Credential, DanmakuOption, FilterOption};

pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    let config = Config::load().unwrap_or_else(|err| {
        if err
            .downcast_ref::<std::io::Error>()
            .map_or(true, |e| e.kind() != std::io::ErrorKind::NotFound)
        {
            panic!("加载配置文件失败，错误为： {err}");
        }
        warn!("配置文件不存在，使用默认配置...");
        Config::default()
    });
    // 放到外面，确保新的配置项被保存
    info!("配置加载完毕，覆盖刷新原有配置");
    config.save().unwrap();
    // 检查配置文件内容
    info!("校验配置文件内容...");
    config.check();
    config
});

pub static ARGS: Lazy<Args> = Lazy::new(Args::parse);

pub static CONFIG_DIR: Lazy<PathBuf> =
    Lazy::new(|| dirs::config_dir().expect("No config path found").join("bili-sync"));

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub credential: ArcSwapOption<Credential>,
    pub filter_option: FilterOption,
    #[serde(default)]
    pub danmaku_option: DanmakuOption,
    pub favorite_list: HashMap<String, PathBuf>,
    #[serde(
        serialize_with = "serialize_collection_list",
        deserialize_with = "deserialize_collection_list"
    )]
    pub collection_list: HashMap<CollectionItem, PathBuf>,
    pub video_name: Cow<'static, str>,
    pub page_name: Cow<'static, str>,
    pub interval: u64,
    pub upper_path: PathBuf,
    #[serde(default)]
    pub nfo_time_type: NFOTimeType,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum NFOTimeType {
    #[default]
    FavTime,
    PubTime,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            credential: ArcSwapOption::from(Some(Arc::new(Credential::default()))),
            filter_option: FilterOption::default(),
            danmaku_option: DanmakuOption::default(),
            favorite_list: HashMap::new(),
            collection_list: HashMap::new(),
            video_name: Cow::Borrowed("{{title}}"),
            page_name: Cow::Borrowed("{{bvid}}"),
            interval: 1200,
            upper_path: CONFIG_DIR.join("upper_face"),
            nfo_time_type: NFOTimeType::FavTime,
        }
    }
}

impl Config {
    /// 简单的预检查
    pub fn check(&self) {
        let mut ok = true;
        if self.favorite_list.is_empty() && self.collection_list.is_empty() {
            ok = false;
            error!("未设置需监听的收藏夹或视频合集，程序空转没有意义");
        }
        for path in self.favorite_list.values() {
            if !path.is_absolute() {
                ok = false;
                error!("收藏夹保存的路径应为绝对路径，检测到: {}", path.display());
            }
        }
        if !self.upper_path.is_absolute() {
            ok = false;
            error!("up 主头像保存的路径应为绝对路径");
        }
        if self.video_name.is_empty() {
            ok = false;
            error!("未设置 video_name 模板");
        }
        if self.page_name.is_empty() {
            ok = false;
            error!("未设置 page_name 模板");
        }
        let credential = self.credential.load();
        match credential.as_deref() {
            Some(credential) => {
                if credential.sessdata.is_empty()
                    || credential.bili_jct.is_empty()
                    || credential.buvid3.is_empty()
                    || credential.dedeuserid.is_empty()
                    || credential.ac_time_value.is_empty()
                {
                    ok = false;
                    error!("Credential 信息不完整，请确保填写完整");
                }
            }
            None => {
                ok = false;
                error!("未设置 Credential 信息");
            }
        }

        if !ok {
            panic!(
                "位于 {} 的配置文件不合法，请参考提示信息修复后继续运行",
                CONFIG_DIR.join("config.toml").display()
            );
        }
    }

    fn load() -> Result<Self> {
        let config_path = CONFIG_DIR.join("config.toml");
        let config_content = std::fs::read_to_string(config_path)?;
        Ok(toml::from_str(&config_content)?)
    }

    pub fn save(&self) -> Result<()> {
        let config_path = CONFIG_DIR.join("config.toml");
        std::fs::create_dir_all(&*CONFIG_DIR)?;
        std::fs::write(config_path, toml::to_string_pretty(self)?)?;
        Ok(())
    }
}

fn serialize_collection_list<S>(
    collection_list: &HashMap<CollectionItem, PathBuf>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let mut map = serializer.serialize_map(Some(collection_list.len()))?;
    for (k, v) in collection_list {
        let key_str = match k {
            CollectionItem::Series(s) => format!("series:{}", s),
            CollectionItem::Season(s) => format!("season:{}", s),
        };
        map.serialize_entry(&key_str, &v)?;
    }
    map.end()
}

fn deserialize_collection_list<'de, D>(deserializer: D) -> Result<HashMap<CollectionItem, PathBuf>, D::Error>
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
                let (collection_type, collection_id) = match key.split_once(':') {
                    Some(("series", id)) => (CollectionItem::Series(id.to_string()), value),
                    Some(("season", id)) => (CollectionItem::Season(id.to_string()), value),
                    _ => {
                        return Err(serde::de::Error::custom(
                            "invalid collection type, should be series or season",
                        ))
                    }
                };
                collection_list.insert(collection_type, collection_id);
            }
            Ok(collection_list)
        }
    }

    deserializer.deserialize_map(CollectionListVisitor)
}

use clap::Parser;

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(short, long, env = "SCAN_ONLY")]
    pub scan_only: bool,

    #[arg(short, long, default_value = "None,bili_sync=info", env = "RUST_LOG")]
    pub log_level: String,
}
