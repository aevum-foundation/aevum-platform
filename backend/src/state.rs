use chrono::{DateTime, Utc};
use std::sync::Arc;

use crate::config::Config;
use crate::storage::{Storage, SharedStorage};

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub started_at: DateTime<Utc>,
    pub storage: SharedStorage,
}

impl AppState {
    pub fn new(config: Config, storage: SharedStorage) -> Self {
        Self {
            config: Arc::new(config),
            started_at: Utc::now(),
            storage,
        }
    }

    pub fn uptime(&self) -> chrono::Duration {
        Utc::now() - self.started_at
    }
}
