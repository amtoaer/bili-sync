use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{Result, anyhow, bail};
use arc_swap::{ArcSwap, Guard};
use sea_orm::DatabaseConnection;
use tokio::sync::OnceCell;

use crate::config::{CONFIG_DIR, Config, LegacyConfig};

pub static VERSIONED_CONFIG: OnceCell<VersionedConfig> = OnceCell::const_new();

pub struct VersionedConfig {
    inner: ArcSwap<Config>,
    version: AtomicU64,
}

impl VersionedConfig {
    /// 初始化全局的 `VersionedConfig`，初始化失败或者已初始化过则返回错误
    pub async fn init(connection: &DatabaseConnection) -> Result<()> {
        let config = match Config::load_from_database(connection).await? {
            Some(Ok(config)) => config,
            Some(Err(e)) => bail!("解析数据库配置失败： {}", e),
            None => {
                let config = match LegacyConfig::migrate_from_file(&CONFIG_DIR.join("config.toml"), connection).await {
                    Ok(config) => config,
                    Err(e) => {
                        if e.downcast_ref::<std::io::Error>()
                            .is_none_or(|e| e.kind() != std::io::ErrorKind::NotFound)
                        {
                            bail!("未成功读取并迁移旧版本配置：{:#}", e);
                        } else {
                            let config = Config::default();
                            warn!(
                                "生成 auth_token：{}，可使用该 token 登录 web UI，该信息仅在首次运行时打印",
                                config.auth_token
                            );
                            config
                        }
                    }
                };
                config.save_to_database(connection).await?;
                config
            }
        };
        let versioned_config = VersionedConfig::new(config);
        VERSIONED_CONFIG
            .set(versioned_config)
            .map_err(|e| anyhow!("VERSIONED_CONFIG has already been initialized: {}", e))?;
        Ok(())
    }

    #[cfg(test)]
    /// 单元测试直接使用测试专用的配置即可
    pub fn get() -> &'static VersionedConfig {
        use std::sync::LazyLock;
        static TEST_CONFIG: LazyLock<VersionedConfig> = LazyLock::new(|| VersionedConfig::new(Config::test_default()));
        return &TEST_CONFIG;
    }

    #[cfg(not(test))]
    /// 获取全局的 `VersionedConfig`，如果未初始化则会 panic
    pub fn get() -> &'static VersionedConfig {
        VERSIONED_CONFIG.get().expect("VERSIONED_CONFIG is not initialized")
    }

    pub fn new(config: Config) -> Self {
        Self {
            inner: ArcSwap::from_pointee(config),
            version: AtomicU64::new(1),
        }
    }

    pub fn load(&self) -> Guard<Arc<Config>> {
        self.inner.load()
    }

    pub fn load_full(&self) -> Arc<Config> {
        self.inner.load_full()
    }

    pub fn version(&self) -> u64 {
        self.version.load(Ordering::Acquire)
    }

    #[allow(dead_code)]
    pub fn update(&self, new_config: Config) {
        self.inner.store(Arc::new(new_config));
        self.version.fetch_add(1, Ordering::AcqRel);
    }
}
