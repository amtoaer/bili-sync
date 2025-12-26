use std::sync::Arc;

use anyhow::{Result, bail};
use arc_swap::{ArcSwap, Guard};
use sea_orm::DatabaseConnection;
use tokio::sync::{OnceCell, watch};

use crate::bilibili::Credential;
use crate::config::Config;

static VERSIONED_CONFIG: OnceCell<VersionedConfig> = OnceCell::const_new();

pub struct VersionedConfig {
    inner: ArcSwap<Config>,
    update_lock: tokio::sync::Mutex<()>,
    tx: watch::Sender<Arc<Config>>,
    rx: watch::Receiver<Arc<Config>>,
}

impl VersionedConfig {
    /// 初始化全局的 `VersionedConfig`，初始化失败或者已初始化过则返回错误
    pub async fn init(connection: &DatabaseConnection) -> Result<&'static VersionedConfig> {
        VERSIONED_CONFIG
            .get_or_try_init(|| async move {
                let mut config = match Config::load_from_database(connection).await? {
                    Some(Ok(config)) => config,
                    Some(Err(e)) => bail!("解析数据库配置失败： {}", e),
                    None => {
                        let config = Config::default();
                        warn!(
                            "生成 auth_token：{}，可使用该 token 登录 web UI，该信息仅在首次运行时打印",
                            config.auth_token
                        );
                        config.save_to_database(connection).await?;
                        config
                    }
                };
                // version 本身不具有实际意义，仅用于并发更新时的版本控制，在初始化时可以直接清空
                config.version = 0;
                Ok(VersionedConfig::new(config))
            })
            .await
    }

    #[cfg(test)]
    /// 仅在测试环境使用，该方法会尝试从测试数据库中加载配置并写入到全局的 VERSIONED_CONFIG
    pub async fn init_for_test(connection: &DatabaseConnection) -> Result<&'static VersionedConfig> {
        VERSIONED_CONFIG
            .get_or_try_init(|| async move {
                let Some(Ok(config)) = Config::load_from_database(&connection).await? else {
                    bail!("no config found in test database");
                };
                Ok(VersionedConfig::new(config))
            })
            .await
    }

    #[cfg(not(test))]
    /// 获取全局的 `VersionedConfig`，如果未初始化则会 panic
    pub fn get() -> &'static VersionedConfig {
        VERSIONED_CONFIG.get().expect("VERSIONED_CONFIG is not initialized")
    }

    #[cfg(test)]
    /// 尝试获取全局的 `VersionedConfig`，如果未初始化则退回默认配置
    pub fn get() -> &'static VersionedConfig {
        use std::sync::LazyLock;
        static FALLBACK_CONFIG: LazyLock<VersionedConfig> = LazyLock::new(|| VersionedConfig::new(Config::default()));
        // 优先从全局变量获取，未初始化则退回默认配置
        return VERSIONED_CONFIG.get().unwrap_or_else(|| &FALLBACK_CONFIG);
    }

    fn new(config: Config) -> Self {
        let inner = ArcSwap::from_pointee(config);
        let (tx, rx) = watch::channel(inner.load_full());
        Self {
            inner,
            update_lock: tokio::sync::Mutex::new(()),
            tx,
            rx,
        }
    }

    pub fn read(&self) -> Guard<Arc<Config>> {
        self.inner.load()
    }

    pub fn snapshot(&self) -> Arc<Config> {
        self.inner.load_full()
    }

    pub fn subscribe(&self) -> watch::Receiver<Arc<Config>> {
        self.rx.clone()
    }

    pub async fn update_credential(
        &self,
        new_credential: Credential,
        connection: &DatabaseConnection,
    ) -> Result<Arc<Config>> {
        let _lock = self.update_lock.lock().await;
        let mut new_config = self.inner.load().as_ref().clone();
        new_config.credential = new_credential;
        new_config.version += 1;
        new_config.save_to_database(connection).await?;
        let new_config = Arc::new(new_config);
        self.inner.store(new_config.clone());
        self.tx.send(new_config.clone())?;
        Ok(new_config)
    }

    /// 外部 API 会调用这个方法，如果更新失败直接返回错误
    pub async fn update(&self, mut new_config: Config, connection: &DatabaseConnection) -> Result<Arc<Config>> {
        let _lock = self.update_lock.lock().await;
        let old_config = self.inner.load();
        if old_config.version != new_config.version {
            bail!("配置版本不匹配，请刷新页面修改后重新提交");
        }
        new_config.version += 1;
        new_config.save_to_database(connection).await?;
        let new_config = Arc::new(new_config);
        self.inner.store(new_config.clone());
        self.tx.send(new_config.clone())?;
        Ok(new_config)
    }
}
