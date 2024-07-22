use std::borrow::Cow;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use arc_swap::ArcSwapOption;
use handlebars::handlebars_helper;
use once_cell::sync::Lazy;
use serde::de::{Deserializer, MapAccess, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Serialize};

use crate::bilibili::{CollectionItem, CollectionType, Credential, DanmakuOption, FilterOption};

pub static TEMPLATE: Lazy<handlebars::Handlebars> = Lazy::new(|| {
    let mut handlebars = handlebars::Handlebars::new();
    handlebars_helper!(truncate: |s: String, len: usize| {
        if s.chars().count() > len {
            s.chars().take(len).collect::<String>()
        } else {
            s.to_string()
        }
    });
    handlebars.register_helper("truncate", Box::new(truncate));
    handlebars
        .register_template_string("video", &CONFIG.video_name)
        .unwrap();
    handlebars.register_template_string("page", &CONFIG.page_name).unwrap();
    handlebars
});

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
        default,
        serialize_with = "serialize_collection_list",
        deserialize_with = "deserialize_collection_list"
    )]
    pub collection_list: HashMap<CollectionItem, PathBuf>,
    #[serde(default)]
    pub watch_later: WatchLaterConfig,
    pub video_name: Cow<'static, str>,
    pub page_name: Cow<'static, str>,
    pub interval: u64,
    pub upper_path: PathBuf,
    #[serde(default)]
    pub nfo_time_type: NFOTimeType,
    #[serde(default)]
    pub delay: DelayConfig,
}

#[derive(Serialize, Deserialize, Default)]
pub struct WatchLaterConfig {
    pub enabled: bool,
    pub path: PathBuf,
}

#[derive(Serialize, Deserialize, Default)]
pub struct DelayConfig {
    pub refresh_video_list: Option<Delay>,
    pub fetch_video_detail: Option<Delay>,
    pub download_video: Option<Delay>,
    pub download_page: Option<Delay>,
}

#[derive(Serialize, Deserialize)]
#[serde(untagged, rename_all = "lowercase")]
pub enum Delay {
    Random { min: u64, max: u64 },
    Fixed(u64),
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
            watch_later: Default::default(),
            video_name: Cow::Borrowed("{{title}}"),
            page_name: Cow::Borrowed("{{bvid}}"),
            interval: 1200,
            upper_path: CONFIG_DIR.join("upper_face"),
            nfo_time_type: NFOTimeType::FavTime,
            delay: Default::default(),
        }
    }
}

impl Config {
    /// 简单的预检查
    pub fn check(&self) {
        let mut ok = true;
        if self.favorite_list.is_empty() && self.collection_list.is_empty() && !self.watch_later.enabled {
            ok = false;
            error!("没有配置任何需要扫描的内容，程序空转没有意义");
        }
        if self.watch_later.enabled && !self.watch_later.path.is_absolute() {
            error!(
                "稍后再看保存的路径应为绝对路径，检测到：{}",
                self.watch_later.path.display()
            );
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
        for delay_config in [
            &self.delay.refresh_video_list,
            &self.delay.fetch_video_detail,
            &self.delay.download_video,
            &self.delay.download_page,
        ]
        .iter()
        .filter_map(|x| x.as_ref())
        {
            if let Delay::Random { min, max } = delay_config {
                if min >= max {
                    ok = false;
                    error!("随机延迟的最小值应小于最大值");
                }
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
        let prefix = match k.collection_type {
            CollectionType::Series => "series",
            CollectionType::Season => "season",
        };
        map.serialize_entry(&[prefix, &k.mid, &k.sid].join(":"), v)?;
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

use clap::Parser;

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(short, long, env = "SCAN_ONLY")]
    pub scan_only: bool,

    #[arg(short, long, default_value = "None,bili_sync=info", env = "RUST_LOG")]
    pub log_level: String,
}
