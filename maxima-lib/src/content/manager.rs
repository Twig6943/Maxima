use std::sync::Arc;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::{fs, sync::Mutex};

use crate::{core::auth::storage::LockedAuthStorage, util::native::maxima_dir};

use super::ContentService;

const QUEUE_FILE: &str = "download_queue.json";

#[derive(Default, Serialize, Deserialize)]
struct QueuedGame {
    slug: String,
    build_id: String,
}

#[derive(Default, Serialize, Deserialize)]
struct DownloadQueue {
    current: Option<QueuedGame>,
    queued: Vec<QueuedGame>,
    completed: Vec<QueuedGame>,
}

impl DownloadQueue {
    pub(crate) async fn load() -> Result<DownloadQueue> {
        let file = maxima_dir()?.join(QUEUE_FILE);
        if !file.exists() {
            return Ok(Self::default());
        }

        let data = fs::read_to_string(file).await?;
        let result = serde_json::from_str(&data);
        if result.is_err() {
            return Ok(Self::default());
        }

        Ok(result.unwrap())
    }

    pub(crate) async fn save(&self) -> Result<()> {
        let file = maxima_dir()?.join(QUEUE_FILE);
        fs::write(file, serde_json::to_string(&self)?).await?;
        Ok(())
    }
}

pub struct ContentManager {
    queue: DownloadQueue,
    service: ContentService,
}

impl ContentManager {
    pub async fn new(auth: LockedAuthStorage) -> Result<Self> {
        Ok(Self {
            queue: DownloadQueue::load().await?,
            service: ContentService::new(auth),
        })
    }

    pub async fn install() {
        
    }
}
