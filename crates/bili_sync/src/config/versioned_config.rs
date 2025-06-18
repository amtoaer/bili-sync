use std::sync::Arc;

use anyhow::{Result, anyhow, bail};
use arc_swap::{ArcSwap, Guard};
use sea_orm::DatabaseConnection;
use tokio::sync::OnceCell;

use crate::bilibili::Credential;
use crate::config::{CONFIG_DIR, Config, LegacyConfig};

pub static VERSIONED_CONFIG: OnceCell<VersionedConfig> = OnceCell::const_new();

pub struct VersionedConfig {
    inner: ArcSwap<Config>,
    update_lock: tokio::sync::Mutex<()>,
}

impl VersionedConfig {
    /// 初始化全局的 `VersionedConfig`，初始化失败或者已初始化过则返回错误
    pub async fn init(connection: &DatabaseConnection) -> Result<()> {
        let mut config = match Config::load_from_database(connection).await? {
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
        // version 本身不具有实际意义，仅用于并发更新时的版本控制，在初始化时可以直接清空
        config.version = 0;
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
            update_lock: tokio::sync::Mutex::new(()),
        }
    }

    pub fn load(&self) -> Guard<Arc<Config>> {
        self.inner.load()
    }

    pub fn load_full(&self) -> Arc<Config> {
        self.inner.load_full()
    }

    pub async fn update_credential(&self, new_credential: Credential, connection: &DatabaseConnection) -> Result<()> {
        // 确保更新内容与写入数据库的操作是原子性的
        let _lock = self.update_lock.lock().await;
        loop {
            let old_config = self.inner.load();
            let mut new_config = old_config.as_ref().clone();
            new_config.credential = new_credential.clone();
            new_config.version += 1;
            if Arc::ptr_eq(
                &old_config,
                &self.inner.compare_and_swap(&old_config, Arc::new(new_config)),
            ) {
                break;
            }
        }
        self.inner.load().save_to_database(connection).await
    }

    /// 外部 API 会调用这个方法，如果更新失败直接返回错误
    pub async fn update(&self, mut new_config: Config, connection: &DatabaseConnection) -> Result<Arc<Config>> {
        let _lock = self.update_lock.lock().await;
        let old_config = self.inner.load();
        if old_config.version != new_config.version {
            bail!("配置版本不匹配，请刷新页面修改后重新提交");
        }
        new_config.version += 1;
        let new_config = Arc::new(new_config);
        if !Arc::ptr_eq(
            &old_config,
            &self.inner.compare_and_swap(&old_config, new_config.clone()),
        ) {
            bail!("配置版本不匹配，请刷新页面修改后重新提交");
        }
        new_config.save_to_database(connection).await?;
        Ok(new_config)
    }
}
