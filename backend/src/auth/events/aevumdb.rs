//! AevumDB-backed security event storage.
//!
//! AUTH-16 — Security Events
//!
//! Persistent audit log using AevumDB with secondary indexes
//! for user, event type, and timeline queries.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use aevum_db::{AevumDb, DbConfig, DbError, DbRuntime};

use crate::auth::events::{SecurityEvent, SecurityEventKind, SecurityEventRecord};
use crate::error::ApiError;

use super::storage::SecurityEventStorage;

const EVENT_PREFIX: &str = "platform:audit:event:";
const USER_INDEX_PREFIX: &str = "platform:audit:user:";
const TYPE_INDEX_PREFIX: &str = "platform:audit:type:";
const TIMELINE_PREFIX: &str = "platform:audit:timeline:";

/// Format timestamp as fixed-width string for lexicographic ordering.
fn ts_component(timestamp: DateTime<Utc>) -> String {
    format!("{:020}", timestamp.timestamp_micros())
}

#[derive(Clone)]
pub struct AevumDbSecurityEventStorage {
    db: Arc<AevumDb>,
}

impl std::fmt::Debug for AevumDbSecurityEventStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AevumDbSecurityEventStorage")
            .field("db", &"<redacted>")
            .finish()
    }
}

impl AevumDbSecurityEventStorage {
    pub fn open(config: DbConfig, runtime: DbRuntime) -> Result<Self, ApiError> {
        let db = AevumDb::open(config, runtime).map_err(|error| {
            log::error!("AevumDB open failed: {}", error);
            ApiError::Internal
        })?;
        Ok(Self { db: Arc::new(db) })
    }

    fn event_key(event_id: &Uuid) -> String {
        format!("{}{}", EVENT_PREFIX, event_id)
    }

    fn user_index_key(user_id: &Uuid, timestamp: DateTime<Utc>, event_id: &Uuid) -> String {
        format!(
            "{}{}:{}:{}",
            USER_INDEX_PREFIX,
            user_id,
            ts_component(timestamp),
            event_id
        )
    }

    fn user_index_prefix(user_id: &Uuid) -> String {
        format!("{}{}:", USER_INDEX_PREFIX, user_id)
    }

    fn type_index_key(
        event_kind: SecurityEventKind,
        timestamp: DateTime<Utc>,
        event_id: &Uuid,
    ) -> String {
        format!(
            "{}{}:{}:{}",
            TYPE_INDEX_PREFIX,
            event_kind.as_str(),
            ts_component(timestamp),
            event_id
        )
    }

    fn type_index_prefix(event_kind: SecurityEventKind) -> String {
        format!("{}{}:", TYPE_INDEX_PREFIX, event_kind.as_str())
    }

    fn timeline_key(timestamp: DateTime<Utc>, event_id: &Uuid) -> String {
        format!(
            "{}{}:{}",
            TIMELINE_PREFIX,
            ts_component(timestamp),
            event_id
        )
    }

    fn timeline_prefix() -> &'static str {
        TIMELINE_PREFIX
    }

    fn serialize<T: Serialize>(value: &T) -> Result<Vec<u8>, ApiError> {
        serde_json::to_vec(value).map_err(|error| {
            log::error!("Serialization failed: {}", error);
            ApiError::Internal
        })
    }

    fn deserialize<T: for<'de> Deserialize<'de>>(data: &[u8]) -> Result<T, ApiError> {
        serde_json::from_slice(data).map_err(|error| {
            log::error!("Deserialization failed: {}", error);
            ApiError::Internal
        })
    }

    fn map_db_error(error: DbError) -> ApiError {
        log::error!("AevumDB error: {}", error);
        ApiError::Internal
    }

    fn parse_timestamp_from_key(key: &[u8], prefix: &str) -> Option<i64> {
        let key_str = String::from_utf8_lossy(key);
        let rest = key_str.strip_prefix(prefix)?;
        let timestamp_str = rest.split(':').next()?;
        timestamp_str.parse::<i64>().ok()
    }
}

#[async_trait]
impl SecurityEventStorage for AevumDbSecurityEventStorage {
    async fn record_event(&self, event: SecurityEvent) -> Result<(), ApiError> {
        let record = SecurityEventRecord::new(event);

        let event_key = Self::event_key(&record.id);
        let event_data = Self::serialize(&record)?;

        let event_id = record.id;
        let timestamp = record.timestamp;
        let mut batch = self.db.batch();

        batch.put(event_key.as_bytes(), &event_data);

        if let Some(user_id) = record.event.user_id() {
            let user_index_key = Self::user_index_key(&user_id, timestamp, &event_id);
            batch.put(user_index_key.as_bytes(), event_id.as_bytes());
        }

        let type_index_key = Self::type_index_key(record.event.kind(), timestamp, &event_id);
        batch.put(type_index_key.as_bytes(), event_id.as_bytes());

        let timeline_key = Self::timeline_key(timestamp, &event_id);
        batch.put(timeline_key.as_bytes(), event_id.as_bytes());

        batch.commit().map_err(Self::map_db_error)?;
        Ok(())
    }

    async fn get_events_for_user(
        &self,
        user_id: Uuid,
        limit: usize,
    ) -> Result<Vec<SecurityEvent>, ApiError> {
        let prefix = Self::user_index_prefix(&user_id);
        let entries = self
            .db
            .prefix_scan(prefix.as_bytes())
            .map_err(Self::map_db_error)?;

        let mut events = Vec::with_capacity(limit.min(entries.len()));
        for (_, event_id_bytes) in entries.iter().rev().take(limit) {
            let Ok(event_id) = Uuid::from_slice(event_id_bytes) else {
                continue;
            };

            let event_key = Self::event_key(&event_id);
            if let Some(data) = self
                .db
                .get(event_key.as_bytes())
                .map_err(Self::map_db_error)?
            {
                let record: SecurityEventRecord = Self::deserialize(&data)?;
                events.push(record.event);
            }
        }

        Ok(events)
    }

    async fn get_events_by_type(
        &self,
        event_kind: SecurityEventKind,
        limit: usize,
    ) -> Result<Vec<SecurityEvent>, ApiError> {
        let prefix = Self::type_index_prefix(event_kind);
        let entries = self
            .db
            .prefix_scan(prefix.as_bytes())
            .map_err(Self::map_db_error)?;

        let mut events = Vec::with_capacity(limit.min(entries.len()));
        for (_, event_id_bytes) in entries.iter().rev().take(limit) {
            let Ok(event_id) = Uuid::from_slice(event_id_bytes) else {
                continue;
            };

            let event_key = Self::event_key(&event_id);
            if let Some(data) = self
                .db
                .get(event_key.as_bytes())
                .map_err(Self::map_db_error)?
            {
                let record: SecurityEventRecord = Self::deserialize(&data)?;
                events.push(record.event);
            }
        }

        Ok(events)
    }

    async fn get_recent_events(&self, limit: usize) -> Result<Vec<SecurityEvent>, ApiError> {
        let entries = self
            .db
            .prefix_scan(Self::timeline_prefix().as_bytes())
            .map_err(Self::map_db_error)?;

        let mut events = Vec::with_capacity(limit.min(entries.len()));
        for (_, event_id_bytes) in entries.iter().rev().take(limit) {
            let Ok(event_id) = Uuid::from_slice(event_id_bytes) else {
                continue;
            };

            let event_key = Self::event_key(&event_id);
            if let Some(data) = self
                .db
                .get(event_key.as_bytes())
                .map_err(Self::map_db_error)?
            {
                let record: SecurityEventRecord = Self::deserialize(&data)?;
                events.push(record.event);
            }
        }

        Ok(events)
    }

    async fn get_events_since(
        &self,
        timestamp: DateTime<Utc>,
        limit: usize,
    ) -> Result<Vec<SecurityEvent>, ApiError> {
        let entries = self
            .db
            .prefix_scan(Self::timeline_prefix().as_bytes())
            .map_err(Self::map_db_error)?;

        let threshold = timestamp.timestamp_micros();
        let mut events = Vec::with_capacity(limit);

        for (key, event_id_bytes) in entries.iter().rev() {
            if events.len() >= limit {
                break;
            }

            let Some(timestamp_micros) =
                Self::parse_timestamp_from_key(key, Self::timeline_prefix())
            else {
                continue;
            };

            if timestamp_micros < threshold {
                break;
            }

            let Ok(event_id) = Uuid::from_slice(event_id_bytes) else {
                continue;
            };

            let event_key = Self::event_key(&event_id);
            if let Some(data) = self
                .db
                .get(event_key.as_bytes())
                .map_err(Self::map_db_error)?
            {
                let record: SecurityEventRecord = Self::deserialize(&data)?;
                events.push(record.event);
            }
        }

        Ok(events)
    }

    async fn flush(&self) -> Result<(), ApiError> {
        self.db.flush().map_err(Self::map_db_error)
    }

    async fn prune_before(&self, timestamp: DateTime<Utc>) -> Result<usize, ApiError> {
        let entries = self
            .db
            .prefix_scan(Self::timeline_prefix().as_bytes())
            .map_err(Self::map_db_error)?;

        let threshold = timestamp.timestamp_micros();
        let mut batch = self.db.batch();
        let mut pruned = 0;

        for (key, event_id_bytes) in entries.iter() {
            let Some(timestamp_micros) =
                Self::parse_timestamp_from_key(key, Self::timeline_prefix())
            else {
                continue;
            };

            if timestamp_micros >= threshold {
                break;
            }

            let Ok(event_id) = Uuid::from_slice(event_id_bytes) else {
                continue;
            };

            // Сначала читаем запись, потом удаляем все индексы
            let event_key = Self::event_key(&event_id);
            if let Some(data) = self
                .db
                .get(event_key.as_bytes())
                .map_err(Self::map_db_error)?
            {
                let record: SecurityEventRecord = Self::deserialize(&data)?;

                // Удаляем user index если есть
                if let Some(user_id) = record.event.user_id() {
                    let user_index_key =
                        Self::user_index_key(&user_id, record.timestamp, &event_id);
                    batch.delete(user_index_key.as_bytes());
                }

                // Удаляем type index
                let type_index_key =
                    Self::type_index_key(record.event.kind(), record.timestamp, &event_id);
                batch.delete(type_index_key.as_bytes());
            }

            // Удаляем timeline index и event
            batch.delete(key.as_slice());
            batch.delete(event_key.as_bytes());

            pruned += 1;
        }

        batch.commit().map_err(Self::map_db_error)?;
        Ok(pruned)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::events::SecurityMetadata;
    use aevum_db::config::SyncMode;
    use tempfile::TempDir;

    fn test_storage() -> (AevumDbSecurityEventStorage, TempDir) {
        let temp = TempDir::new().unwrap();
        let config = DbConfig {
            path: temp.path().to_path_buf(),
            memtable_max_bytes: 64 * 1024 * 1024,
            wal_segment_bytes: 64 * 1024 * 1024,
            sync_mode: SyncMode::Always,
            max_open_files: 1000,
            block_size: 4096,
            storage_mode: aevum_db::config::StorageMode::default(),
        };
        let runtime = DbRuntime::plaintext();
        let storage = AevumDbSecurityEventStorage::open(config, runtime).unwrap();
        (storage, temp)
    }

    #[tokio::test]
    async fn event_recording_and_retrieval() {
        let (storage, _temp) = test_storage();
        let user_id = Uuid::new_v4();

        let event = SecurityEvent::LoginSuccess {
            metadata: SecurityMetadata::new(None, None),
            user_id,
            email: "test@example.com".to_string(),
        };

        storage.record_event(event.clone()).await.unwrap();

        let user_events = storage.get_events_for_user(user_id, 10).await.unwrap();
        assert_eq!(user_events.len(), 1);

        let recent = storage.get_recent_events(10).await.unwrap();
        assert_eq!(recent.len(), 1);

        let by_type = storage
            .get_events_by_type(SecurityEventKind::LoginSuccess, 10)
            .await
            .unwrap();
        assert_eq!(by_type.len(), 1);
    }

    #[test]
    fn timestamp_component_is_fixed_width() {
        let ts = DateTime::<Utc>::from_timestamp_micros(12345).unwrap();
        assert_eq!(ts_component(ts), "00000000000000012345");
    }
}
