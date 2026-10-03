//! Publication storage adapter.
//!
//! Key layout:
//!
//! ```text
//! growth:publication:id:{publication_id}
//! growth:publication:by_source:{source_id}:{inv_ts}:{publication_id}
//! growth:publication:by_topic:{topic}:{inv_ts}:{publication_id}
//! growth:publication:by_time:{inv_ts}:{publication_id}
//! growth:publication:by_external:{source_id}:{external_id}:{publication_id}
//! ```
//!
//! # Contract
//!
//! **Publication is immutable after write.** Once a Publication has
//! been stored, its `published_at`, `ingested_at`, `topics`,
//! `source_id`, and `external_id` MUST NOT change. `put_publication`
//! is therefore an idempotent upsert keyed by the deterministic
//! `PublicationId`:
//!
//! ```text
//! PublicationId = SHA256("pub:" + source_id + ":" + external_id)[..16]
//! ```
//!
//! Two publications with the same `(source_id, external_id)` MUST
//! therefore produce the same `PublicationId`. Repeating a `put`
//! with the same pair overwrites the same primary key and the same
//! index keys; it does NOT create a second logical entity.
//!
//! The `by_external` index is a **deterministic lookup index**, not a
//! storage-level uniqueness constraint. Uniqueness is guaranteed by
//! the deterministic `PublicationId` and by the single-writer
//! ingestion flow, not by AevumDB.
//!
//! # Ordering
//!
//! `inv_ts` is the inverted effective timestamp
//! (`effective_ts = published_at ?? ingested_at`), so the natural
//! lexicographic order of `by_source`, `by_topic`, and `by_time`
//! equals DESC chronological order. Ties on `effective_ts` are
//! broken by `publication_id` (hex-32), giving fully deterministic
//! ordering.
//!
//! # Performance note
//!
//! List methods currently materialize the full prefix via
//! `prefix_scan` and then truncate to `limit`. This is correctness-
//! neutral. A future AevumDB Tier-1 primitive (`prefix_scan_limited`)
//! will replace `prefix_scan` here without changing the public
//! contract. See `docs/architecture/growth-storage-design-v1.md`
//! section 19.

use async_trait::async_trait;

use crate::error::ApiError;
use crate::growth::models::{Publication, PublicationId, SourceId, Topic};
use crate::growth::storage::PublicationStorage;

use super::{
    deserialize, effective_timestamp, encode_inv_ts_micros, map_db_error, serialize,
    AevumDbGrowthStorage, PUBLICATION_BY_EXTERNAL_PREFIX, PUBLICATION_BY_SOURCE_PREFIX,
    PUBLICATION_BY_TIME_PREFIX, PUBLICATION_BY_TOPIC_PREFIX, PUBLICATION_PREFIX,
};

// ---------------------------------------------------------------------------
// Key builders (canonical, pub(crate) for Key Layout Contract tests)
// ---------------------------------------------------------------------------

pub(crate) fn publication_key(id: PublicationId) -> String {
    format!("{}{}", PUBLICATION_PREFIX, id.as_hex())
}

pub(crate) fn publication_by_source_key(
    source_id: SourceId,
    inv_ts: &str,
    id: PublicationId,
) -> String {
    format!(
        "{}{}:{}:{}",
        PUBLICATION_BY_SOURCE_PREFIX,
        source_id.as_hex(),
        inv_ts,
        id.as_hex()
    )
}

pub(crate) fn publication_by_source_prefix(source_id: SourceId) -> String {
    format!("{}{}:", PUBLICATION_BY_SOURCE_PREFIX, source_id.as_hex())
}

pub(crate) fn publication_by_topic_key(topic: Topic, inv_ts: &str, id: PublicationId) -> String {
    format!(
        "{}{}:{}:{}",
        PUBLICATION_BY_TOPIC_PREFIX,
        topic.as_str(),
        inv_ts,
        id.as_hex()
    )
}

pub(crate) fn publication_by_topic_prefix(topic: Topic) -> String {
    format!("{}{}:", PUBLICATION_BY_TOPIC_PREFIX, topic.as_str())
}

pub(crate) fn publication_by_time_key(inv_ts: &str, id: PublicationId) -> String {
    format!("{}{}:{}", PUBLICATION_BY_TIME_PREFIX, inv_ts, id.as_hex())
}

pub(crate) fn publication_by_external_key(source_id: SourceId, external_id: &str) -> String {
    format!(
        "{}{}:{}",
        PUBLICATION_BY_EXTERNAL_PREFIX,
        source_id.as_hex(),
        external_id
    )
}

// ---------------------------------------------------------------------------
// Trait implementation
// ---------------------------------------------------------------------------

#[async_trait]
impl PublicationStorage for AevumDbGrowthStorage {
    /// Idempotent upsert of an immutable Publication.
    ///
    /// Writes primary + 4 secondary indexes in one atomic batch.
    async fn put_publication(&self, publication: &Publication) -> Result<(), ApiError> {
        let id = publication.id;
        let effective_ts = effective_timestamp(publication.published_at, publication.ingested_at);
        let inv_ts = encode_inv_ts_micros(effective_ts);

        let primary_key = publication_key(id);
        let primary_bytes = serialize(publication)?;

        let mut batch = self.db().batch();
        batch.put(primary_key.as_bytes(), &primary_bytes);

        let by_source = publication_by_source_key(publication.source_id, &inv_ts, id);
        batch.put(by_source.as_bytes(), id.as_hex().as_bytes());

        for topic in &publication.topics {
            let by_topic = publication_by_topic_key(*topic, &inv_ts, id);
            batch.put(by_topic.as_bytes(), id.as_hex().as_bytes());
        }

        let by_time = publication_by_time_key(&inv_ts, id);
        batch.put(by_time.as_bytes(), id.as_hex().as_bytes());

        let by_external =
            publication_by_external_key(publication.source_id, &publication.external_id);
        batch.put(by_external.as_bytes(), id.as_hex().as_bytes());

        batch.commit().map_err(map_db_error)
    }

    async fn get_publication(&self, id: PublicationId) -> Result<Option<Publication>, ApiError> {
        let key = publication_key(id);
        match self.db().get(key.as_bytes()).map_err(map_db_error)? {
            None => Ok(None),
            Some(bytes) => Ok(Some(deserialize::<Publication>(&bytes)?)),
        }
    }

    async fn publication_exists(
        &self,
        source_id: SourceId,
        external_id: &str,
    ) -> Result<bool, ApiError> {
        let key = publication_by_external_key(source_id, external_id);
        Ok(self
            .db()
            .get(key.as_bytes())
            .map_err(map_db_error)?
            .is_some())
    }

    async fn list_publications_by_source(
        &self,
        source_id: SourceId,
        limit: usize,
    ) -> Result<Vec<Publication>, ApiError> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        let prefix = publication_by_source_prefix(source_id);
        self.list_publications_via_index(&prefix, limit).await
    }

    async fn list_publications_by_topic(
        &self,
        topic: Topic,
        limit: usize,
    ) -> Result<Vec<Publication>, ApiError> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        let prefix = publication_by_topic_prefix(topic);
        self.list_publications_via_index(&prefix, limit).await
    }

    async fn list_recent_publications(&self, limit: usize) -> Result<Vec<Publication>, ApiError> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        self.list_publications_via_index(PUBLICATION_BY_TIME_PREFIX, limit)
            .await
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

impl AevumDbGrowthStorage {
    /// Generic index-driven publication listing.
    ///
    /// Scans `prefix` in natural (lexicographic) order, which for
    /// `by_source`, `by_topic`, and `by_time` corresponds to DESC
    /// effective_ts order, with `publication_id` as deterministic
    /// tie-breaker.
    ///
    /// Returns at most `limit` records.
    ///
    /// TODO(AevumDB Tier-1): switch to `prefix_scan_limited(prefix, limit)`
    /// when the primitive becomes available. Current implementation
    /// materializes the full prefix, then truncates.
    async fn list_publications_via_index(
        &self,
        prefix: &str,
        limit: usize,
    ) -> Result<Vec<Publication>, ApiError> {
        let raw = self
            .db()
            .prefix_scan(prefix.as_bytes())
            .map_err(map_db_error)?;

        let mut publications = Vec::with_capacity(raw.len().min(limit));
        for (_key, value) in raw {
            if publications.len() >= limit {
                break;
            }
            let id = parse_publication_id_value(&value)?;
            if let Some(publication) = self.get_publication(id).await? {
                publications.push(publication);
            }
        }

        Ok(publications)
    }
}

/// Decode a secondary-index value (hex-32 id) into a `PublicationId`.
fn parse_publication_id_value(bytes: &[u8]) -> Result<PublicationId, ApiError> {
    let hex_str = std::str::from_utf8(bytes).map_err(|error| {
        log::error!("Growth: publication id utf8 decode failed: {}", error);
        ApiError::Internal
    })?;

    PublicationId::from_hex(hex_str).ok_or_else(|| {
        log::error!("Growth: publication id hex parse failed: {}", hex_str);
        ApiError::Internal
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    use chrono::{Duration, Utc};
    use tempfile::TempDir;
    use url::Url;

    use crate::growth::models::{Platform, Source};

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

    fn make_publication(
        source: &Source,
        external_id: &str,
        topics: Vec<Topic>,
        published_at: Option<chrono::DateTime<Utc>>,
    ) -> Publication {
        let id = PublicationId::from_parts(source.id, external_id);
        Publication {
            id,
            source_id: source.id,
            external_id: external_id.to_owned(),
            url: Some(Url::parse("https://example.com/post").unwrap()),
            title: format!("Post {}", external_id),
            summary: None,
            author: None,
            language: None,
            topics,
            published_at,
            ingested_at: Utc::now(),
        }
    }

    // ─── Key Layout Contract ────────────────────────────

    #[test]
    fn publication_key_is_stable() {
        let source = make_source(Topic::Rust, "https://example.com/feed.xml");
        let pub_ = make_publication(&source, "ext-1", vec![Topic::Rust], None);
        let key = publication_key(pub_.id);
        assert!(key.starts_with("growth:publication:id:"));
        assert_eq!(key.len(), "growth:publication:id:".len() + 32);
    }

    #[test]
    fn publication_by_source_key_is_stable() {
        let source = make_source(Topic::Rust, "https://example.com/feed.xml");
        let pub_ = make_publication(&source, "ext-1", vec![Topic::Rust], None);
        let key = publication_by_source_key(source.id, "00000000000000000000", pub_.id);
        assert!(key.starts_with("growth:publication:by_source:"));
        assert!(key.contains(&source.id.as_hex()));
        assert!(key.ends_with(&pub_.id.as_hex()));
    }

    #[test]
    fn publication_by_topic_key_is_stable() {
        let source = make_source(Topic::Rust, "https://example.com/feed.xml");
        let pub_ = make_publication(&source, "ext-1", vec![Topic::Rust], None);
        let key = publication_by_topic_key(Topic::Rust, "00000000000000000000", pub_.id);
        assert!(key.starts_with("growth:publication:by_topic:rust:"));
        assert!(key.ends_with(&pub_.id.as_hex()));
    }

    #[test]
    fn publication_by_time_key_is_stable() {
        let source = make_source(Topic::Rust, "https://example.com/feed.xml");
        let pub_ = make_publication(&source, "ext-1", vec![Topic::Rust], None);
        let key = publication_by_time_key("00000000000000000000", pub_.id);
        assert!(key.starts_with("growth:publication:by_time:"));
        assert!(key.ends_with(&pub_.id.as_hex()));
    }

    #[test]
    fn publication_by_external_key_is_stable() {
        let source = make_source(Topic::Rust, "https://example.com/feed.xml");
        let key = publication_by_external_key(source.id, "guid-1");
        assert!(key.starts_with("growth:publication:by_external:"));
        assert!(key.ends_with(":guid-1"));
    }

    #[test]
    fn primary_prefix_does_not_match_secondary_keys() {
        let source = make_source(Topic::Rust, "https://example.com/feed.xml");
        let pub_ = make_publication(&source, "ext-1", vec![Topic::Rust], None);
        let primary = publication_key(pub_.id);
        let by_time = publication_by_time_key("00000000000000000000", pub_.id);
        let by_source = publication_by_source_key(source.id, "00000000000000000000", pub_.id);
        assert!(!by_time.starts_with(&primary));
        assert!(!by_source.starts_with(&primary));
    }

    // ─── Deterministic identity / idempotency ──────────

    #[test]
    fn same_external_id_same_source_is_same_id() {
        let source = make_source(Topic::Rust, "https://example.com/feed.xml");
        let a = PublicationId::from_parts(source.id, "guid-1");
        let b = PublicationId::from_parts(source.id, "guid-1");
        assert_eq!(a, b);
    }

    #[test]
    fn same_external_id_different_source_is_different_id() {
        let s1 = make_source(Topic::Rust, "https://example.com/a.xml");
        let s2 = make_source(Topic::Rust, "https://example.com/b.xml");
        let a = PublicationId::from_parts(s1.id, "guid-1");
        let b = PublicationId::from_parts(s2.id, "guid-1");
        assert_ne!(a, b);
    }

    #[tokio::test]
    async fn repeated_put_is_idempotent() {
        let (storage, _temp) = test_storage();
        let source = make_source(Topic::Rust, "https://example.com/feed.xml");
        let pub_ = make_publication(&source, "guid-1", vec![Topic::Rust], None);

        storage.put_publication(&pub_).await.unwrap();
        storage.put_publication(&pub_).await.unwrap();
        storage.put_publication(&pub_).await.unwrap();

        // Only one logical publication exists.
        let all = storage.list_recent_publications(10).await.unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, pub_.id);
    }

    #[tokio::test]
    async fn same_timestamp_has_deterministic_order() {
        let (storage, _temp) = test_storage();
        let source = make_source(Topic::Rust, "https://example.com/feed.xml");
        let ts = Utc::now();

        let a = make_publication(&source, "aaa", vec![Topic::Rust], Some(ts));
        let b = make_publication(&source, "bbb", vec![Topic::Rust], Some(ts));

        storage.put_publication(&a).await.unwrap();
        storage.put_publication(&b).await.unwrap();

        let first_run = storage
            .list_publications_by_source(source.id, 10)
            .await
            .unwrap();
        let second_run = storage
            .list_publications_by_source(source.id, 10)
            .await
            .unwrap();

        assert_eq!(first_run.len(), 2);
        assert_eq!(second_run.len(), 2);
        assert_eq!(first_run[0].id, second_run[0].id, "stable tie-break");
        assert_eq!(first_run[1].id, second_run[1].id);
        // Deterministic tie-break by publication_id (hex).
        let expected_first = if a.id.as_hex() < b.id.as_hex() {
            a.id
        } else {
            b.id
        };
        assert_eq!(first_run[0].id, expected_first);
    }

    #[tokio::test]
    async fn multiple_topics_create_multiple_topic_indexes() {
        let (storage, _temp) = test_storage();
        let source = make_source(Topic::Rust, "https://example.com/feed.xml");
        let both = make_publication(&source, "both", vec![Topic::Rust, Topic::PostQuantum], None);

        storage.put_publication(&both).await.unwrap();

        let rust = storage
            .list_publications_by_topic(Topic::Rust, 10)
            .await
            .unwrap();
        let pq = storage
            .list_publications_by_topic(Topic::PostQuantum, 10)
            .await
            .unwrap();

        assert_eq!(rust.len(), 1);
        assert_eq!(pq.len(), 1);
        assert_eq!(rust[0].id, both.id);
        assert_eq!(pq[0].id, both.id);
    }

    #[tokio::test]
    async fn publication_exists_uses_source_scoped_external_id() {
        let (storage, _temp) = test_storage();
        let s1 = make_source(Topic::Rust, "https://example.com/a.xml");
        let s2 = make_source(Topic::Rust, "https://example.com/b.xml");
        let p1 = make_publication(&s1, "guid-1", vec![Topic::Rust], None);

        storage.put_publication(&p1).await.unwrap();

        assert!(storage.publication_exists(s1.id, "guid-1").await.unwrap());
        // Same external_id under a different source is NOT the same publication.
        assert!(!storage.publication_exists(s2.id, "guid-1").await.unwrap());
    }

    // ─── Behaviour ──────────────────────────────────────

    #[tokio::test]
    async fn put_get_roundtrip() {
        let (storage, _temp) = test_storage();
        let source = make_source(Topic::Rust, "https://example.com/feed.xml");
        let pub_ = make_publication(&source, "guid-1", vec![Topic::Rust], None);

        storage.put_publication(&pub_).await.unwrap();
        let loaded = storage
            .get_publication(pub_.id)
            .await
            .unwrap()
            .expect("must exist");
        assert_eq!(loaded.id, pub_.id);
        assert_eq!(loaded.external_id, "guid-1");
    }

    #[tokio::test]
    async fn get_missing_returns_none() {
        let (storage, _temp) = test_storage();
        let source = make_source(Topic::Rust, "https://example.com/feed.xml");
        let pub_ = make_publication(&source, "guid-1", vec![Topic::Rust], None);
        assert!(storage.get_publication(pub_.id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn list_by_source_returns_newest_first() {
        let (storage, _temp) = test_storage();
        let source = make_source(Topic::Rust, "https://example.com/feed.xml");
        let now = Utc::now();

        let older = make_publication(
            &source,
            "old",
            vec![Topic::Rust],
            Some(now - Duration::hours(2)),
        );
        let newer = make_publication(&source, "new", vec![Topic::Rust], Some(now));

        storage.put_publication(&older).await.unwrap();
        storage.put_publication(&newer).await.unwrap();

        let list = storage
            .list_publications_by_source(source.id, 10)
            .await
            .unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].id, newer.id);
        assert_eq!(list[1].id, older.id);
    }

    #[tokio::test]
    async fn list_recent_publications_is_newest_first() {
        let (storage, _temp) = test_storage();
        let s1 = make_source(Topic::Rust, "https://example.com/a.xml");
        let s2 = make_source(Topic::PostQuantum, "https://example.com/b.xml");
        let now = Utc::now();

        let p1 = make_publication(
            &s1,
            "a-1",
            vec![Topic::Rust],
            Some(now - Duration::hours(3)),
        );
        let p2 = make_publication(&s2, "b-1", vec![Topic::PostQuantum], Some(now));
        let p3 = make_publication(
            &s1,
            "a-2",
            vec![Topic::Rust],
            Some(now - Duration::hours(1)),
        );

        storage.put_publication(&p1).await.unwrap();
        storage.put_publication(&p2).await.unwrap();
        storage.put_publication(&p3).await.unwrap();

        let list = storage.list_recent_publications(10).await.unwrap();
        assert_eq!(list.len(), 3);
        assert_eq!(list[0].id, p2.id);
        assert_eq!(list[1].id, p3.id);
        assert_eq!(list[2].id, p1.id);
    }

    #[tokio::test]
    async fn effective_ts_falls_back_to_ingested_at() {
        let (storage, _temp) = test_storage();
        let source = make_source(Topic::Rust, "https://example.com/feed.xml");

        let p_early = make_publication(&source, "early", vec![Topic::Rust], None);
        let mut p_late = make_publication(&source, "late", vec![Topic::Rust], None);
        p_late.ingested_at = Utc::now() + Duration::seconds(2);

        storage.put_publication(&p_early).await.unwrap();
        storage.put_publication(&p_late).await.unwrap();

        let list = storage
            .list_publications_by_source(source.id, 10)
            .await
            .unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].id, p_late.id);
    }

    #[tokio::test]
    async fn list_by_source_limit_is_respected() {
        let (storage, _temp) = test_storage();
        let source = make_source(Topic::Rust, "https://example.com/feed.xml");
        for i in 0..5 {
            let pub_ = make_publication(&source, &format!("ext-{}", i), vec![Topic::Rust], None);
            storage.put_publication(&pub_).await.unwrap();
        }
        let list = storage
            .list_publications_by_source(source.id, 2)
            .await
            .unwrap();
        assert_eq!(list.len(), 2);
    }

    #[tokio::test]
    async fn list_by_topic_limit_is_respected() {
        let (storage, _temp) = test_storage();
        let source = make_source(Topic::Rust, "https://example.com/feed.xml");
        for i in 0..5 {
            let pub_ = make_publication(&source, &format!("ext-{}", i), vec![Topic::Rust], None);
            storage.put_publication(&pub_).await.unwrap();
        }
        let list = storage
            .list_publications_by_topic(Topic::Rust, 3)
            .await
            .unwrap();
        assert_eq!(list.len(), 3);
    }

    #[tokio::test]
    async fn list_recent_limit_is_respected() {
        let (storage, _temp) = test_storage();
        let source = make_source(Topic::Rust, "https://example.com/feed.xml");
        for i in 0..5 {
            let pub_ = make_publication(&source, &format!("ext-{}", i), vec![Topic::Rust], None);
            storage.put_publication(&pub_).await.unwrap();
        }
        let list = storage.list_recent_publications(2).await.unwrap();
        assert_eq!(list.len(), 2);
    }

    #[tokio::test]
    async fn limit_zero_returns_empty() {
        let (storage, _temp) = test_storage();
        let source = make_source(Topic::Rust, "https://example.com/feed.xml");
        let pub_ = make_publication(&source, "guid-1", vec![Topic::Rust], None);
        storage.put_publication(&pub_).await.unwrap();

        assert!(storage
            .list_publications_by_source(source.id, 0)
            .await
            .unwrap()
            .is_empty());
        assert!(storage
            .list_publications_by_topic(Topic::Rust, 0)
            .await
            .unwrap()
            .is_empty());
        assert!(storage
            .list_recent_publications(0)
            .await
            .unwrap()
            .is_empty());
    }
}
