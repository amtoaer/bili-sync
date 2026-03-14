use bili_sync_entity::video;

use crate::utils::status::{STATUS_OK, VideoStatus};

pub enum DownloadNotifyInfo {
    List {
        source: String,
        img_url: Option<String>,
        titles: Vec<String>,
    },
    Summary {
        source: String,
        img_url: Option<String>,
        count: usize,
    },
}

impl DownloadNotifyInfo {
    pub fn new(source: String) -> Self {
        Self::List {
            source,
            img_url: None,
            titles: Vec::with_capacity(10),
        }
    }

    pub fn record(&mut self, models: &[video::ActiveModel]) {
        let success_models = models
            .iter()
            .filter(|m| {
                let sub_task_status: [u32; 5] = VideoStatus::from(*m.download_status.as_ref()).into();
                sub_task_status.into_iter().all(|s| s == STATUS_OK)
            })
            .collect::<Vec<_>>();
        match self {
            Self::List {
                source,
                img_url,
                titles,
            } => {
                let count = success_models.len() + titles.len();
                if count > 10 {
                    *self = Self::Summary {
                        source: std::mem::take(source),
                        img_url: std::mem::take(img_url),
                        count,
                    };
                } else {
                    if img_url.is_none() {
                        *img_url = success_models.first().map(|m| m.cover.as_ref().clone());
                    }
                    titles.extend(success_models.into_iter().map(|m| m.name.as_ref().clone()));
                }
            }
            Self::Summary { count, .. } => *count += success_models.len(),
        }
    }

    pub fn should_notify(&self) -> bool {
        if let Self::List { titles, .. } = self
            && titles.is_empty()
        {
            return false;
        }
        true
    }
}
