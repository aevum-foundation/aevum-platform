//! Application state shared across request handlers.

use chrono::{DateTime, Utc};
use std::sync::Arc;

use crate::config::Config;
use crate::storage::Storage;

/// Application state.
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub started_at: DateTime<Utc>,
    pub storage: Arc<dyn Storage>,
}

impl AppState {
    pub fn new(config: Config, storage: Arc<dyn Storage>) -> Self {
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
