use std::sync::Arc;

use anyhow::Result;
use arc_swap::{ArcSwap, Guard};
use tokio_util::future::FutureExt;
use tokio_util::sync::CancellationToken;

use crate::config::{Config, VersionedConfig};

pub struct VersionedCache<T> {
    inner: Arc<ArcSwap<T>>,
    cancel_token: CancellationToken,
}

/// 一个跟随全局配置变化自动更新的缓存
impl<T: Send + Sync + 'static> VersionedCache<T> {
    pub fn new(builder: fn(&Config) -> Result<T>) -> Result<Self> {
        let mut rx = VersionedConfig::get().subscribe();
        let initial_value = builder(&rx.borrow_and_update())?;
        let cancel_token = CancellationToken::new();
        let inner = Arc::new(ArcSwap::from_pointee(initial_value));
        let inner_clone = inner.clone();
        tokio::spawn(
            async move {
                while rx.changed().await.is_ok() {
                    match builder(&rx.borrow()) {
                        Ok(new_value) => {
                            inner_clone.store(Arc::new(new_value));
                        }
                        Err(e) => {
                            error!("Failed to update versioned cache: {:?}", e);
                        }
                    }
                }
            }
            .with_cancellation_token_owned(cancel_token.clone()),
        );
        Ok(Self { inner, cancel_token })
    }

    /// 获取一个临时的只读引用
    pub fn read(&self) -> Guard<Arc<T>> {
        self.inner.load()
    }

    /// 获取当前缓存的完整快照
    pub fn snapshot(&self) -> Arc<T> {
        self.inner.load_full()
    }
}

impl<T> Drop for VersionedCache<T> {
    fn drop(&mut self) {
        self.cancel_token.cancel();
    }
}
