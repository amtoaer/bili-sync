use std::borrow::Cow;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use arc_swap::ArcSwapOption;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

mod clap;
mod global;
mod item;

use crate::adapter::Args;
use crate::bilibili::{CollectionItem, Credential, DanmakuOption, FilterOption};
pub use crate::config::clap::version;
pub use crate::config::global::{ARGS, CONFIG, CONFIG_DIR, TEMPLATE, reload_config};
use crate::config::item::{ConcurrentLimit, deserialize_collection_list, serialize_collection_list};
pub use crate::config::item::{NFOTimeType, PathSafeTemplate, RateLimit, WatchLaterConfig};

// 定义番剧配置结构体
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct BangumiConfig {
    pub season_id: Option<String>,
    pub media_id: Option<String>,
    pub ep_id: Option<String>,
    pub path: PathBuf,
    #[serde(default = "default_download_all_seasons")]
    pub download_all_seasons: bool,
}

fn default_time_format() -> String {
    "%Y-%m-%d".to_string()
}

/// 默认的 auth_token 实现，生成随机 16 位字符串
fn default_auth_token() -> Option<String> {
    let byte_choices = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!@#$%^&*()_+-=";
    let mut rng = rand::thread_rng();
    Some(
        (0..16)
            .map(|_| *(byte_choices.choose(&mut rng).expect("choose byte failed")) as char)
            .collect(),
    )
}

fn default_bind_address() -> String {
    "0.0.0.0:12345".to_string()
}

fn default_download_all_seasons() -> bool {
    false
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_auth_token")]
    pub auth_token: Option<String>,
    #[serde(default = "default_bind_address")]
    pub bind_address: String,
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
    pub submission_list: HashMap<String, PathBuf>,
    #[serde(default)]
    pub watch_later: WatchLaterConfig,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub bangumi: Vec<BangumiConfig>,
    pub video_name: Cow<'static, str>,
    pub page_name: Cow<'static, str>,
    pub folder_structure: Cow<'static, str>,
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

impl Default for Config {
    fn default() -> Self {
        Self {
            auth_token: default_auth_token(),
            bind_address: default_bind_address(),
            credential: ArcSwapOption::from(Some(Arc::new(Credential::default()))),
            filter_option: FilterOption::default(),
            danmaku_option: DanmakuOption::default(),
            favorite_list: HashMap::new(),
            collection_list: HashMap::new(),
            submission_list: HashMap::new(),
            watch_later: Default::default(),
            bangumi: Vec::new(),
            video_name: Cow::Borrowed("{{title}}"),
            page_name: Cow::Borrowed("{{title}} - S01E{{pid_pad}}"),
            folder_structure: Cow::Borrowed("{{show_title}}"),
            interval: 60,
            upper_path: CONFIG_DIR.join("upper_face"),
            nfo_time_type: NFOTimeType::FavTime,
            concurrent_limit: ConcurrentLimit::default(),
            time_format: default_time_format(),
            cdn_sorting: false,
        }
    }
}

impl Config {
    pub fn save(&self) -> Result<()> {
        let config_path = CONFIG_DIR.join("config.toml");
        std::fs::create_dir_all(&*CONFIG_DIR)?;
        
        // 先将配置序列化为TOML
        let mut config_content = toml::to_string_pretty(self)?;
        
        // 添加各种配置的注释
        let favorite_list_comment = "\n# 收藏夹配置\n# 格式: 收藏夹ID = \"保存路径\"\n# 收藏夹ID可以从收藏夹URL中获取\n";
        let collection_list_comment = "\n# 合集配置\n# 格式: 合集ID = \"保存路径\"\n";
        let submission_list_comment = "\n# UP主投稿配置\n# 格式: UP主ID = \"保存路径\"\n# UP主ID可以从UP主空间URL中获取\n";
        let bangumi_comment = "\n# 番剧配置，可以添加多个[[bangumi]]块\n# season_id: 番剧的season_id，可以从B站番剧页面URL中获取\n# path: 保存番剧的本地路径，必须是绝对路径\n# 注意: season_id和path不能为空，否则程序会报错\n";
        let parallel_download_comment = "\n# 多线程下载配置\n# enabled: 是否启用多线程下载，默认为true\n# threads: 每个文件的下载线程数，默认为4\n# min_size: 最小文件大小(字节)，小于此大小的文件不使用多线程下载，默认为10MB\n";
        
        // 查找各部分位置并插入注释
        if let Some(pos) = config_content.find("[favorite_list]") {
            let (before, after) = config_content.split_at(pos);
            config_content = format!("{}{}{}", before, favorite_list_comment, after);
        }
        
        if let Some(pos) = config_content.find("[collection_list]") {
            let (before, after) = config_content.split_at(pos);
            config_content = format!("{}{}{}", before, collection_list_comment, after);
        }
        
        if let Some(pos) = config_content.find("[submission_list]") {
            let (before, after) = config_content.split_at(pos);
            config_content = format!("{}{}{}", before, submission_list_comment, after);
        }
        
        if let Some(pos) = config_content.find("[[bangumi]]") {
            let (before, after) = config_content.split_at(pos);
            config_content = format!("{}{}{}", before, bangumi_comment, after);
        }
        
        if let Some(pos) = config_content.find("[concurrent_limit.parallel_download]") {
            let (before, after) = config_content.split_at(pos);
            config_content = format!("{}{}{}", before, parallel_download_comment, after);
        }
        
        std::fs::write(config_path, config_content)?;
        Ok(())
    }

    #[cfg(not(test))]
    fn load() -> Result<Self> {
        let config_path = CONFIG_DIR.join("config.toml");
        let config_content = std::fs::read_to_string(config_path)?;
        Ok(toml::from_str(&config_content)?)
    }

    pub fn as_video_sources(&self) -> Vec<(Args<'_>, &PathBuf)> {
        let mut params = Vec::new();
        self.favorite_list
            .iter()
            .for_each(|(fid, path)| params.push((Args::Favorite { fid }, path)));
        self.collection_list
            .iter()
            .for_each(|(collection_item, path)| params.push((Args::Collection { collection_item }, path)));
        if self.watch_later.enabled {
            params.push((Args::WatchLater, &self.watch_later.path));
        }
        self.submission_list
            .iter()
            .for_each(|(upper_id, path)| params.push((Args::Submission { upper_id }, path)));
        // 处理番剧配置
        self.bangumi
            .iter()
            .for_each(|bangumi| params.push((Args::Bangumi {
                season_id: &bangumi.season_id,
                media_id: &bangumi.media_id,
                ep_id: &bangumi.ep_id,
            }, &bangumi.path)));
        params
    }

    #[cfg(not(test))]
    pub fn check(&self) -> bool {
        let mut ok = true;
        let mut critical_error = false;
        
        let video_sources = self.as_video_sources();
        if video_sources.is_empty() && self.bangumi.is_empty() {
            ok = false;
            // 移除错误日志
            // error!("没有配置任何需要扫描的内容，程序空转没有意义");
        }
        for (args, path) in video_sources {
            if !path.is_absolute() {
                ok = false;
                error!("{:?} 保存的路径应为绝对路径，检测到: {}", args, path.display());
            }
        }
        // 检查番剧配置的路径
        for bangumi in &self.bangumi {
            if !bangumi.path.is_absolute() {
                ok = false;
                let season_id_display = match &bangumi.season_id {
                    Some(id) => id.clone(),
                    None => "未知".to_string(),
                };
                error!("番剧 {} 保存的路径应为绝对路径，检测到: {}", season_id_display, bangumi.path.display());
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
        if self.folder_structure.is_empty() {
            ok = false;
            error!("未设置 folder_structure 模板");
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
                    critical_error = true;
                    error!("Credential 信息不完整，请确保填写完整");
                }
            }
            None => {
                ok = false;
                critical_error = true;
                error!("未设置 Credential 信息");
            }
        }
        if !(self.concurrent_limit.video > 0 && self.concurrent_limit.page > 0) {
            ok = false;
            error!("video 和 page 允许的并发数必须大于 0");
        }
        
        if critical_error {
            panic!(
                "位于 {} 的配置文件存在严重错误，请参考提示信息修复后继续运行",
                CONFIG_DIR.join("config.toml").display()
            );
        }
        
        ok
    }
}
