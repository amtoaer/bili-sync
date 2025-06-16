use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::Result;
use arc_swap::{ArcSwap, Guard};

use crate::config::{Config, VersionedConfig};

pub struct VersionedCache<T> {
    inner: ArcSwap<T>,
    version: AtomicU64,
    builder: fn(&Config) -> Result<T>,
}

impl<T> VersionedCache<T> {
    pub fn new(builder: fn(&Config) -> Result<T>) -> Result<Self> {
        let current_config = VersionedConfig::get().load();
        let initial_value = builder(&current_config)?;
        Ok(Self {
            inner: ArcSwap::from_pointee(initial_value),
            version: AtomicU64::new(0),
            builder,
        })
    }

    pub fn load(&self) -> Guard<Arc<T>> {
        self.reload_if_needed();
        self.inner.load()
    }

    #[allow(dead_code)]
    pub fn load_full(&self) -> Arc<T> {
        self.reload_if_needed();
        self.inner.load_full()
    }

    fn reload_if_needed(&self) {
        let current_version = VersionedConfig::get().version();
        let cached_version = self.version.load(Ordering::Acquire);

        if current_version != cached_version {
            let current_config = VersionedConfig::get().load();
            if let Ok(new_value) = (self.builder)(&current_config) {
                self.inner.store(Arc::new(new_value));
                self.version.store(current_version, Ordering::Release);
            }
        }
    }
}
