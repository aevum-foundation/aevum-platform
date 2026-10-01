//! Source storage adapter.
//!
//! Key layout:
//!
//! ```text
//! growth:source:id:{source_id}
//! growth:source:by_topic:{topic}:{source_id}
//! growth:source:by_status:{status}:{source_id}
//! ```
//!
//! `delete_source` is a soft delete: the primary record is
//! preserved with `status = Disabled`, and the status index is
//! updated. The `by_topic` index is NOT touched — topics do not
//! change when a source is disabled.

use async_trait::async_trait;

use crate::error::ApiError;
use crate::growth::models::{Source, SourceId, SourceStatus, Topic};
use crate::growth::storage::SourceStorage;

use super::{
    deserialize, map_db_error, serialize, AevumDbGrowthStorage, SOURCE_BY_STATUS_PREFIX,
    SOURCE_BY_TOPIC_PREFIX, SOURCE_PREFIX,
};

// ---------------------------------------------------------------------------
// Key builders (canonical, pub(crate) for Key Layout Contract tests)
// ---------------------------------------------------------------------------

pub(crate) fn source_key(id: SourceId) -> String {
    format!("{}{}", SOURCE_PREFIX, id.as_hex())
}

pub(crate) fn source_by_topic_key(topic: Topic, id: SourceId) -> String {
    format!("{}{}:{}", SOURCE_BY_TOPIC_PREFIX, topic.as_str(), id.as_hex())
}

pub(crate) fn source_by_topic_prefix(topic: Topic) -> String {
    format!("{}{}:", SOURCE_BY_TOPIC_PREFIX, topic.as_str())
}

pub(crate) fn source_by_status_key(status: SourceStatus, id: SourceId) -> String {
    format!(
        "{}{}:{}",
        SOURCE_BY_STATUS_PREFIX,
        status.as_str(),
        id.as_hex()
    )
}

pub(crate) fn source_by_status_prefix(status: SourceStatus) -> String {
    format!("{}{}:", SOURCE_BY_STATUS_PREFIX, status.as_str())
}

// ---------------------------------------------------------------------------
// Trait implementation
// ---------------------------------------------------------------------------

#[async_trait]
impl SourceStorage for AevumDbGrowthStorage {
    async fn put_source(&self, source: &Source) -> Result<(), ApiError> {
        let id = source.id;
        let primary_key = source_key(id);
        let primary_bytes = serialize(source)?;

        let mut batch = self.db().batch();
        batch.put(primary_key.as_bytes(), &primary_bytes);

        for topic in &source.topics {
            let key = source_by_topic_key(*topic, id);
            batch.put(key.as_bytes(), id.as_hex().as_bytes());
        }

        let status_key = source_by_status_key(source.status, id);
        batch.put(status_key.as_bytes(), id.as_hex().as_bytes());

        batch.commit().map_err(map_db_error)
    }

    async fn get_source(&self, id: SourceId) -> Result<Option<Source>, ApiError> {
        let key = source_key(id);
        match self.db().get(key.as_bytes()).map_err(map_db_error)? {
            None => Ok(None),
            Some(bytes) => Ok(Some(deserialize::<Source>(&bytes)?)),
        }
    }

    /// Soft delete: preserves the primary record, sets
    /// `status = Disabled`, and updates the status index.
    ///
    /// Idempotent: a second call on an already-disabled source is
    /// a no-op. The `by_topic` index is left untouched.
    async fn delete_source(&self, id: SourceId) -> Result<(), ApiError> {
        let key = source_key(id);
        let Some(bytes) = self.db().get(key.as_bytes()).map_err(map_db_error)? else {
            return Ok(());
        };

        let mut source: Source = deserialize(&bytes)?;
        if source.status == SourceStatus::Disabled {
            return Ok(());
        }

        let old_status = source.status;
        source.status = SourceStatus::Disabled;
        source.updated_at = chrono::Utc::now();

        let new_bytes = serialize(&source)?;
        let old_status_key = source_by_status_key(old_status, id);
        let new_status_key = source_by_status_key(SourceStatus::Disabled, id);

        let mut batch = self.db().batch();
        batch.put(key.as_bytes(), &new_bytes);
        batch.delete(old_status_key.as_bytes());
        batch.put(new_status_key.as_bytes(), id.as_hex().as_bytes());
        batch.commit().map_err(map_db_error)
    }

    async fn list_sources(&self, limit: usize) -> Result<Vec<Source>, ApiError> {
        if limit == 0 {
            return Ok(Vec::new());
        }

        let raw = self
            .db()
            .prefix_scan(SOURCE_PREFIX.as_bytes())
            .map_err(map_db_error)?;

        let mut sources = Vec::with_capacity(raw.len().min(limit));
        for (_key, value) in raw {
            if sources.len() >= limit {
                break;
            }
            sources.push(deserialize::<Source>(&value)?);
        }

        // Deterministic order: source_id ASC (hex-32).
        sources.sort_by_key(|s| s.id.as_hex());
        Ok(sources)
    }

    async fn list_sources_by_topic(
        &self,
        topic: Topic,
        limit: usize,
    ) -> Result<Vec<Source>, ApiError> {
        if limit == 0 {
            return Ok(Vec::new());
        }

        let prefix = source_by_topic_prefix(topic);
        let raw = self
            .db()
            .prefix_scan(prefix.as_bytes())
            .map_err(map_db_error)?;

        let mut sources = Vec::with_capacity(raw.len().min(limit));
        for (_key, value) in raw {
            if sources.len() >= limit {
                break;
            }
            let id = parse_source_id_value(&value)?;
            if let Some(source) = self.get_source(id).await? {
                sources.push(source);
            }
        }

        sources.sort_by_key(|s| s.id.as_hex());
        Ok(sources)
    }

    async fn list_sources_by_status(
        &self,
        status: SourceStatus,
        limit: usize,
    ) -> Result<Vec<Source>, ApiError> {
        if limit == 0 {
            return Ok(Vec::new());
        }

        let prefix = source_by_status_prefix(status);
        let raw = self
            .db()
            .prefix_scan(prefix.as_bytes())
            .map_err(map_db_error)?;

        let mut sources = Vec::with_capacity(raw.len().min(limit));
        for (_key, value) in raw {
            if sources.len() >= limit {
                break;
            }
            let id = parse_source_id_value(&value)?;
            if let Some(source) = self.get_source(id).await? {
                sources.push(source);
            }
        }

        sources.sort_by_key(|s| s.id.as_hex());
        Ok(sources)
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Decode a secondary-index value (hex-32 id) into a `SourceId`.
fn parse_source_id_value(bytes: &[u8]) -> Result<SourceId, ApiError> {
    let hex_str = std::str::from_utf8(bytes).map_err(|error| {
        log::error!("Growth: source id utf8 decode failed: {}", error);
        ApiError::Internal
    })?;

    SourceId::from_hex(hex_str).ok_or_else(|| {
        log::error!("Growth: source id hex parse failed: {}", hex_str);
        ApiError::Internal
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    use chrono::Utc;
    use tempfile::TempDir;
    use url::Url;

    use crate::growth::models::Platform;

    fn test_storage() -> (AevumDbGrowthStorage, TempDir) {
        let temp = TempDir::new().unwrap();
        let config = aevum_db::DbConfig::plaintext(temp.path().to_path_buf());
        let runtime = aevum_db::DbRuntime::plaintext();
        let storage = AevumDbGrowthStorage::open(config, runtime).unwrap();
        (storage, temp)
    }

    fn make_source(topic: Topic, url: &str) -> Source {
        Source::new(
            Platform::Rss,
            Url::parse(url).unwrap(),
            None,
            vec![topic],
            Utc::now(),
        )
    }

    // ─── Key Layout Contract ────────────────────────────

    #[test]
    fn source_key_is_stable() {
        let source = make_source(Topic::Rust, "https://example.com/feed.xml");
        let key = source_key(source.id);
        assert!(key.starts_with("growth:source:id:"));
        assert_eq!(key.len(), "growth:source:id:".len() + 32);
        assert_eq!(key, format!("growth:source:id:{}", source.id.as_hex()));
    }

    #[test]
    fn source_by_topic_key_is_stable() {
        let source = make_source(Topic::PostQuantum, "https://example.com/pq.xml");
        let key = source_by_topic_key(Topic::PostQuantum, source.id);
        assert!(key.starts_with("growth:source:by_topic:post_quantum:"));
        assert!(key.ends_with(&source.id.as_hex()));
    }

    #[test]
    fn source_by_status_key_is_stable() {
        let source = make_source(Topic::Rust, "https://example.com/feed.xml");
        let key = source_by_status_key(SourceStatus::Active, source.id);
        assert!(key.starts_with("growth:source:by_status:active:"));
        assert!(key.ends_with(&source.id.as_hex()));
    }

    #[test]
    fn primary_prefix_does_not_match_secondary_keys() {
        let source = make_source(Topic::Rust, "https://example.com/feed.xml");
        let primary = source_key(source.id);
        let by_topic = source_by_topic_key(Topic::Rust, source.id);
        let by_status = source_by_status_key(SourceStatus::Active, source.id);

        assert!(!by_topic.starts_with(&primary));
        assert!(!by_status.starts_with(&primary));
    }

    // ─── Behaviour ──────────────────────────────────────

    #[tokio::test]
    async fn put_get_roundtrip() {
        let (storage, _temp) = test_storage();
        let source = make_source(Topic::Rust, "https://example.com/feed.xml");

        storage.put_source(&source).await.unwrap();
        let loaded = storage
            .get_source(source.id)
            .await
            .unwrap()
            .expect("record must exist");

        assert_eq!(loaded.id, source.id);
        assert_eq!(loaded.platform, source.platform);
        assert_eq!(loaded.topics, source.topics);
    }

    #[tokio::test]
    async fn get_missing_returns_none() {
        let (storage, _temp) = test_storage();
        let source = make_source(Topic::Rust, "https://example.com/feed.xml");
        assert!(storage.get_source(source.id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn list_returns_only_primary_records() {
        let (storage, _temp) = test_storage();
        for i in 0..3 {
            let url = format!("https://example.com/feed{}.xml", i);
            let source = make_source(Topic::Rust, &url);
            storage.put_source(&source).await.unwrap();
        }

        let sources = storage.list_sources(10).await.unwrap();
        assert_eq!(sources.len(), 3);
    }

    #[tokio::test]
    async fn list_by_topic_filters_correctly() {
        let (storage, _temp) = test_storage();
        let a = make_source(Topic::Rust, "https://example.com/rust.xml");
        let b = make_source(Topic::PostQuantum, "https://example.com/pq.xml");
        storage.put_source(&a).await.unwrap();
        storage.put_source(&b).await.unwrap();

        let rust = storage.list_sources_by_topic(Topic::Rust, 10).await.unwrap();
        assert_eq!(rust.len(), 1);
        assert_eq!(rust[0].id, a.id);

        let pq = storage
            .list_sources_by_topic(Topic::PostQuantum, 10)
            .await
            .unwrap();
        assert_eq!(pq.len(), 1);
        assert_eq!(pq[0].id, b.id);
    }

    #[tokio::test]
    async fn list_by_status_filters_correctly() {
        let (storage, _temp) = test_storage();
        let a = make_source(Topic::Rust, "https://example.com/a.xml");
        let b = make_source(Topic::Rust, "https://example.com/b.xml");
        storage.put_source(&a).await.unwrap();
        storage.put_source(&b).await.unwrap();

        storage.delete_source(b.id).await.unwrap();

        let active = storage
            .list_sources_by_status(SourceStatus::Active, 10)
            .await
            .unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id, a.id);

        let disabled = storage
            .list_sources_by_status(SourceStatus::Disabled, 10)
            .await
            .unwrap();
        assert_eq!(disabled.len(), 1);
        assert_eq!(disabled[0].id, b.id);
    }

    #[tokio::test]
    async fn delete_source_is_soft_and_idempotent() {
        let (storage, _temp) = test_storage();
        let source = make_source(Topic::Rust, "https://example.com/feed.xml");
        storage.put_source(&source).await.unwrap();

        storage.delete_source(source.id).await.unwrap();
        let after = storage
            .get_source(source.id)
            .await
            .unwrap()
            .expect("record must still exist");
        assert_eq!(after.status, SourceStatus::Disabled);

        // Idempotent.
        storage.delete_source(source.id).await.unwrap();
        let after2 = storage.get_source(source.id).await.unwrap().unwrap();
        assert_eq!(after2.status, SourceStatus::Disabled);
    }

    #[tokio::test]
    async fn list_limit_zero_returns_empty() {
        let (storage, _temp) = test_storage();
        let source = make_source(Topic::Rust, "https://example.com/feed.xml");
        storage.put_source(&source).await.unwrap();

        assert!(storage.list_sources(0).await.unwrap().is_empty());
        assert!(storage
            .list_sources_by_topic(Topic::Rust, 0)
            .await
            .unwrap()
            .is_empty());
        assert!(storage
            .list_sources_by_status(SourceStatus::Active, 0)
            .await
            .unwrap()
            .is_empty());
    }
}
