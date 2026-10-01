//! GrowthEvent storage adapter.
//!
//! Key layout:
//!
//! ```text
//! growth:event:event:{event_id}
//! growth:event:timeline:{inv_ts}:{event_id}
//! growth:event:timeline_asc:{ts}:{event_id}
//! growth:event:by_kind:{kind}:{inv_ts}:{event_id}
//! growth:event:by_topic:{topic}:{inv_ts}:{event_id}
//! ```
//!
//! # Identity
//!
//! `GrowthEvent.id` is UUID v4. Every emission is a new record.
//! Events are append-only: there is no `delete_event` primitive.
//! Removal is only possible through `prune_events_before(ts)`.
//!
//! # Ordering
//!
//! Two timeline indexes are maintained:
//!
//! - `timeline` uses inverted timestamp and provides DESC order
//!   for `get_recent_events`.
//! - `timeline_asc` uses ascending timestamp and provides ASC
//!   order for `get_events_since`.
//!
//! Ties on timestamp are broken by `event_id` (UUID hyphenated).
//!
//! The `by_kind` and `by_topic` indexes use inverted timestamp and
//! provide DESC order, matching `get_recent_events` semantics.
//!
//! # Prune
//!
//! `prune_events_before(ts)` removes events with
//! `occurred_at < ts`. It processes events in batches of
//! `PRUNE_BATCH_SIZE`. Each batch is a single atomic AevumDB
//! batch that removes the primary record and all secondary
//! indexes for each event. The function returns the total number
//! of primary records removed.
//!
//! `prune_events_before` currently scans the full DESC timeline
//! and filters by `occurred_at < ts`. It does NOT yet use the
//! ordering as an early-exit optimization. This is a correctness-
//! neutral fallback; a future AevumDB `range_scan` primitive will
//! allow pruning to stop as soon as it passes the cutoff.
//!
//! # Performance note
//!
//! List methods currently materialize the full prefix via
//! `prefix_scan` and then truncate to `limit`. This is correctness-
//! neutral. A future AevumDB Tier-1 primitive (`prefix_scan_limited`)
//! will replace `prefix_scan` here without changing the public
//! contract. `get_events_since` similarly scans the full
//! `timeline_asc` prefix and filters in the adapter; a future
//! `range_scan(start, end, limit)` will replace it.
//!
//! See `docs/architecture/growth-storage-design-v1.md` section 19.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::error::ApiError;
use crate::growth::events::{GrowthEvent, GrowthEventKind};
use crate::growth::models::Topic;
use crate::growth::storage::GrowthEventStorage;

use super::{
    deserialize, encode_inv_ts_micros, encode_ts_micros, map_db_error, serialize,
    AevumDbGrowthStorage, EVENT_BY_KIND_PREFIX, EVENT_BY_TOPIC_PREFIX, EVENT_PREFIX,
    EVENT_TIMELINE_ASC_PREFIX, EVENT_TIMELINE_PREFIX,
};

/// Maximum number of events removed per prune batch.
///
/// This is a private implementation constant. It MAY be tuned
/// without a version bump.
const PRUNE_BATCH_SIZE: usize = 500;

// ---------------------------------------------------------------------------
// Key builders (canonical, pub(crate) for Key Layout Contract tests)
// ---------------------------------------------------------------------------

pub(crate) fn event_key(id: Uuid) -> String {
    format!("{}{}", EVENT_PREFIX, id)
}

pub(crate) fn event_timeline_key(inv_ts: &str, id: Uuid) -> String {
    format!("{}{}:{}", EVENT_TIMELINE_PREFIX, inv_ts, id)
}

pub(crate) fn event_timeline_asc_key(ts: &str, id: Uuid) -> String {
    format!("{}{}:{}", EVENT_TIMELINE_ASC_PREFIX, ts, id)
}

pub(crate) fn event_by_kind_key(kind: GrowthEventKind, inv_ts: &str, id: Uuid) -> String {
    format!(
        "{}{}:{}:{}",
        EVENT_BY_KIND_PREFIX,
        kind.as_str(),
        inv_ts,
        id
    )
}

pub(crate) fn event_by_kind_prefix(kind: GrowthEventKind) -> String {
    format!("{}{}:", EVENT_BY_KIND_PREFIX, kind.as_str())
}

pub(crate) fn event_by_topic_key(topic: Topic, inv_ts: &str, id: Uuid) -> String {
    format!(
        "{}{}:{}:{}",
        EVENT_BY_TOPIC_PREFIX,
        topic.as_str(),
        inv_ts,
        id
    )
}

pub(crate) fn event_by_topic_prefix(topic: Topic) -> String {
    format!("{}{}:", EVENT_BY_TOPIC_PREFIX, topic.as_str())
}

// ---------------------------------------------------------------------------
// Trait implementation
// ---------------------------------------------------------------------------

#[async_trait]
impl GrowthEventStorage for AevumDbGrowthStorage {
    /// Record a new event and write all applicable indexes.
    ///
    /// A single atomic batch covers:
    ///
    /// - primary key
    /// - `timeline` (DESC)
    /// - `timeline_asc` (ASC)
    /// - `by_kind`
    /// - `by_topic` (only if `event.topic.is_some()`)
    async fn record_event(&self, event: GrowthEvent) -> Result<(), ApiError> {
        let id = event.id;
        let inv_ts = encode_inv_ts_micros(event.occurred_at);
        let ts = encode_ts_micros(event.occurred_at);

        let primary_key = event_key(id);
        let primary_bytes = serialize(&event)?;

        let mut batch = self.db().batch();
        batch.put(primary_key.as_bytes(), &primary_bytes);

        let timeline = event_timeline_key(&inv_ts, id);
        batch.put(timeline.as_bytes(), id.to_string().as_bytes());

        let timeline_asc = event_timeline_asc_key(&ts, id);
        batch.put(timeline_asc.as_bytes(), id.to_string().as_bytes());

        let by_kind = event_by_kind_key(event.kind, &inv_ts, id);
        batch.put(by_kind.as_bytes(), id.to_string().as_bytes());

        if let Some(topic) = event.topic {
            let by_topic = event_by_topic_key(topic, &inv_ts, id);
            batch.put(by_topic.as_bytes(), id.to_string().as_bytes());
        }

        batch.commit().map_err(map_db_error)
    }

    async fn get_events_by_kind(
        &self,
        kind: GrowthEventKind,
        limit: usize,
    ) -> Result<Vec<GrowthEvent>, ApiError> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        let prefix = event_by_kind_prefix(kind);
        self.list_events_desc_via_index(&prefix, limit).await
    }

    async fn get_events_by_topic(
        &self,
        topic: Topic,
        limit: usize,
    ) -> Result<Vec<GrowthEvent>, ApiError> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        let prefix = event_by_topic_prefix(topic);
        self.list_events_desc_via_index(&prefix, limit).await
    }

    async fn get_recent_events(
        &self,
        limit: usize,
    ) -> Result<Vec<GrowthEvent>, ApiError> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        self.list_events_desc_via_index(EVENT_TIMELINE_PREFIX, limit)
            .await
    }

    /// # Performance note
    ///
    /// This method currently performs a full scan of `timeline_asc`
    /// and filters in the adapter. It is correctness-neutral but not
    /// optimal. A future AevumDB Tier-1 primitive
    /// `range_scan(start, end, limit)` will replace the full scan
    /// without changing the public contract.
    ///
    /// The natural ASC order of `timeline_asc` matches the required
    /// output order for `since`.
    async fn get_events_since(
        &self,
        ts: DateTime<Utc>,
        limit: usize,
    ) -> Result<Vec<GrowthEvent>, ApiError> {
        if limit == 0 {
            return Ok(Vec::new());
        }

        let raw = self
            .db()
            .prefix_scan(EVENT_TIMELINE_ASC_PREFIX.as_bytes())
            .map_err(map_db_error)?;

        let mut events = Vec::with_capacity(raw.len().min(limit));
        for (_key, value) in raw {
            if events.len() >= limit {
                break;
            }
            let id = parse_event_id_value(&value)?;
            let Some(event) = self.get_event(id).await? else {
                continue;
            };
            if event.occurred_at >= ts {
                events.push(event);
            }
        }

        Ok(events)
    }

    /// Remove events with `occurred_at < ts` in batches.
    ///
    /// Returns the total number of primary records removed.
    /// Idempotent: a second call with the same `ts` returns 0 after
    /// the first call has already removed matching events.
    async fn prune_events_before(&self, ts: DateTime<Utc>) -> Result<usize, ApiError> {
        let cutoff_micros = ts.timestamp_micros();
        let mut total_removed = 0usize;

        loop {
            // Full DESC timeline scan. Matching events may be
            // interleaved with non-matching ones, so we MUST NOT
            // stop early. We collect up to PRUNE_BATCH_SIZE
            // matching ids per iteration.
            let raw = self
                .db()
                .prefix_scan(EVENT_TIMELINE_PREFIX.as_bytes())
                .map_err(map_db_error)?;

            let mut batch_ids: Vec<Uuid> = Vec::with_capacity(PRUNE_BATCH_SIZE);

            for (_key, value) in raw {
                if batch_ids.len() >= PRUNE_BATCH_SIZE {
                    break;
                }
                let id = parse_event_id_value(&value)?;
                let Some(event) = self.get_event(id).await? else {
                    continue;
                };
                if event.occurred_at.timestamp_micros() < cutoff_micros {
                    batch_ids.push(id);
                }
            }

            if batch_ids.is_empty() {
                break;
            }

            let removed_in_batch = batch_ids.len();
            self.delete_events_batch(&batch_ids).await?;
            total_removed += removed_in_batch;

            // If fewer than PRUNE_BATCH_SIZE were removed, no more
            // matching events remain.
            if removed_in_batch < PRUNE_BATCH_SIZE {
                break;
            }
        }

        Ok(total_removed)
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

impl AevumDbGrowthStorage {
    /// Fetch a single event by id.
    async fn get_event(&self, id: Uuid) -> Result<Option<GrowthEvent>, ApiError> {
        let key = event_key(id);
        match self.db().get(key.as_bytes()).map_err(map_db_error)? {
            None => Ok(None),
            Some(bytes) => Ok(Some(deserialize::<GrowthEvent>(&bytes)?)),
        }
    }

    /// DESC listing via an index whose keys use inverted timestamps.
    async fn list_events_desc_via_index(
        &self,
        prefix: &str,
        limit: usize,
    ) -> Result<Vec<GrowthEvent>, ApiError> {
        let raw = self
            .db()
            .prefix_scan(prefix.as_bytes())
            .map_err(map_db_error)?;

        let mut events = Vec::with_capacity(raw.len().min(limit));
        for (_key, value) in raw {
            if events.len() >= limit {
                break;
            }
            let id = parse_event_id_value(&value)?;
            if let Some(event) = self.get_event(id).await? {
                events.push(event);
            }
        }

        Ok(events)
    }

    /// Delete a set of events atomically.
    ///
    /// Removes primary + timeline + timeline_asc + by_kind + by_topic
    /// (when applicable) for each event, all in one AevumDB batch.
    ///
    /// MISSING EVENTS ARE SKIPPED: if a primary record cannot be
    /// loaded, the event is ignored. This makes the batch resilient
    /// to concurrent prune attempts.
    async fn delete_events_batch(&self, ids: &[Uuid]) -> Result<(), ApiError> {
        if ids.is_empty() {
            return Ok(());
        }

        let mut batch = self.db().batch();

        for &id in ids {
            let Some(event) = self.get_event(id).await? else {
                continue;
            };

            let inv_ts = encode_inv_ts_micros(event.occurred_at);
            let ts = encode_ts_micros(event.occurred_at);

            batch.delete(event_key(id).as_bytes());
            batch.delete(event_timeline_key(&inv_ts, id).as_bytes());
            batch.delete(event_timeline_asc_key(&ts, id).as_bytes());
            batch.delete(event_by_kind_key(event.kind, &inv_ts, id).as_bytes());

            if let Some(topic) = event.topic {
                batch.delete(event_by_topic_key(topic, &inv_ts, id).as_bytes());
            }
        }

        batch.commit().map_err(map_db_error)
    }
}

/// Decode a secondary-index value (UUID hyphenated string) into a
/// `Uuid`.
fn parse_event_id_value(bytes: &[u8]) -> Result<Uuid, ApiError> {
    let uuid_str = std::str::from_utf8(bytes).map_err(|error| {
        log::error!("Growth: event id utf8 decode failed: {}", error);
        ApiError::Internal
    })?;

    Uuid::parse_str(uuid_str).map_err(|error| {
        log::error!("Growth: event id uuid parse failed: {}", error);
        ApiError::Internal
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    use chrono::Duration;
    use serde_json::json;
    use tempfile::TempDir;

    fn test_storage() -> (AevumDbGrowthStorage, TempDir) {
        let temp = TempDir::new().unwrap();
        let config = aevum_db::DbConfig::plaintext(temp.path().to_path_buf());
        let runtime = aevum_db::DbRuntime::plaintext();
        let storage = AevumDbGrowthStorage::open(config, runtime).unwrap();
        (storage, temp)
    }

    fn make_event(
        kind: GrowthEventKind,
        topic: Option<Topic>,
        occurred_at: DateTime<Utc>,
    ) -> GrowthEvent {
        GrowthEvent::new(kind, topic, json!({}), occurred_at)
    }

    // ─── Key Layout Contract ────────────────────────────

    #[test]
    fn event_key_is_stable() {
        let id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        assert_eq!(
            event_key(id),
            "growth:event:event:550e8400-e29b-41d4-a716-446655440000"
        );
    }

    #[test]
    fn event_timeline_key_is_stable() {
        let id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let key = event_timeline_key("00000000000000000000", id);
        assert!(key.starts_with("growth:event:timeline:"));
        assert!(key.ends_with(":550e8400-e29b-41d4-a716-446655440000"));
    }

    #[test]
    fn event_timeline_asc_key_is_stable() {
        let id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let key = event_timeline_asc_key("00000000000000000000", id);
        assert!(key.starts_with("growth:event:timeline_asc:"));
        assert!(key.ends_with(":550e8400-e29b-41d4-a716-446655440000"));
    }

    #[test]
    fn event_by_kind_key_is_stable() {
        let id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let key = event_by_kind_key(
            GrowthEventKind::SourceRegistered,
            "00000000000000000000",
            id,
        );
        assert!(key.starts_with("growth:event:by_kind:source_registered:"));
        assert!(key.ends_with(":550e8400-e29b-41d4-a716-446655440000"));
    }

    #[test]
    fn event_by_topic_key_is_stable() {
        let id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let key = event_by_topic_key(Topic::Rust, "00000000000000000000", id);
        assert!(key.starts_with("growth:event:by_topic:rust:"));
        assert!(key.ends_with(":550e8400-e29b-41d4-a716-446655440000"));
    }

    #[test]
    fn primary_prefix_does_not_match_secondary_keys() {
        let id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let primary = event_key(id);
        let timeline = event_timeline_key("00000000000000000000", id);
        let timeline_asc = event_timeline_asc_key("00000000000000000000", id);
        let by_kind =
            event_by_kind_key(GrowthEventKind::SourceRegistered, "00000000000000000000", id);
        assert!(!timeline.starts_with(&primary));
        assert!(!timeline_asc.starts_with(&primary));
        assert!(!by_kind.starts_with(&primary));
    }

    // ─── Behaviour ──────────────────────────────────────

    #[tokio::test]
    async fn record_event_writes_all_indexes() {
        let (storage, _temp) = test_storage();
        let event = make_event(
            GrowthEventKind::TopicClassified,
            Some(Topic::Rust),
            Utc::now(),
        );

        storage.record_event(event.clone()).await.unwrap();

        let by_kind = storage
            .get_events_by_kind(GrowthEventKind::TopicClassified, 10)
            .await
            .unwrap();
        assert_eq!(by_kind.len(), 1);
        assert_eq!(by_kind[0].id, event.id);

        let by_topic = storage
            .get_events_by_topic(Topic::Rust, 10)
            .await
            .unwrap();
        assert_eq!(by_topic.len(), 1);
        assert_eq!(by_topic[0].id, event.id);

        let recent = storage.get_recent_events(10).await.unwrap();
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].id, event.id);
    }

    #[tokio::test]
    async fn record_event_without_topic_skips_topic_index() {
        let (storage, _temp) = test_storage();
        let event = make_event(GrowthEventKind::SourceRegistered, None, Utc::now());

        storage.record_event(event.clone()).await.unwrap();

        let by_kind = storage
            .get_events_by_kind(GrowthEventKind::SourceRegistered, 10)
            .await
            .unwrap();
        assert_eq!(by_kind.len(), 1);

        let by_topic = storage
            .get_events_by_topic(Topic::Rust, 10)
            .await
            .unwrap();
        assert!(by_topic.is_empty());
    }

    #[tokio::test]
    async fn get_recent_events_is_desc() {
        let (storage, _temp) = test_storage();
        let now = Utc::now();

        let old = make_event(
            GrowthEventKind::SourceRegistered,
            None,
            now - Duration::hours(3),
        );
        let mid = make_event(
            GrowthEventKind::SourceRegistered,
            None,
            now - Duration::hours(1),
        );
        let new = make_event(GrowthEventKind::SourceRegistered, None, now);

        storage.record_event(old.clone()).await.unwrap();
        storage.record_event(mid.clone()).await.unwrap();
        storage.record_event(new.clone()).await.unwrap();

        let recent = storage.get_recent_events(10).await.unwrap();
        assert_eq!(recent.len(), 3);
        assert_eq!(recent[0].id, new.id);
        assert_eq!(recent[1].id, mid.id);
        assert_eq!(recent[2].id, old.id);
    }

    #[tokio::test]
    async fn get_events_since_is_asc_and_filters_by_ts() {
        let (storage, _temp) = test_storage();
        let now = Utc::now();

        let t0 = now - Duration::hours(5);
        let t1 = now - Duration::hours(3);
        let t2 = now - Duration::hours(1);

        let a = make_event(GrowthEventKind::SourceRegistered, None, t0);
        let b = make_event(GrowthEventKind::SourceRegistered, None, t1);
        let c = make_event(GrowthEventKind::SourceRegistered, None, t2);

        storage.record_event(a).await.unwrap();
        storage.record_event(b.clone()).await.unwrap();
        storage.record_event(c.clone()).await.unwrap();

        // cutoff = now - 4h lies between t0 (now - 5h) and
        // t1 (now - 3h), so only b and c are included.
        let since = storage
            .get_events_since(now - Duration::hours(4), 10)
            .await
            .unwrap();
        assert_eq!(since.len(), 2);
        assert_eq!(since[0].id, b.id, "ASC: earlier first");
        assert_eq!(since[1].id, c.id);
    }

    #[tokio::test]
    async fn get_events_since_excludes_older() {
        let (storage, _temp) = test_storage();
        let now = Utc::now();
        let cutoff = now - Duration::hours(2);

        let old = make_event(GrowthEventKind::SourceRegistered, None, now - Duration::hours(3));
        let young = make_event(GrowthEventKind::SourceRegistered, None, now - Duration::hours(1));

        storage.record_event(old).await.unwrap();
        storage.record_event(young.clone()).await.unwrap();

        let since = storage.get_events_since(cutoff, 10).await.unwrap();
        assert_eq!(since.len(), 1);
        assert_eq!(since[0].id, young.id);
    }

    #[tokio::test]
    async fn get_events_by_kind_filters_correctly() {
        let (storage, _temp) = test_storage();
        let now = Utc::now();

        let a = make_event(GrowthEventKind::SourceRegistered, None, now);
        let b = make_event(GrowthEventKind::SourceFetchSucceeded, None, now);
        let c = make_event(GrowthEventKind::SourceRegistered, None, now);

        storage.record_event(a.clone()).await.unwrap();
        storage.record_event(b).await.unwrap();
        storage.record_event(c.clone()).await.unwrap();

        let registered = storage
            .get_events_by_kind(GrowthEventKind::SourceRegistered, 10)
            .await
            .unwrap();
        assert_eq!(registered.len(), 2);
    }

    #[tokio::test]
    async fn get_events_by_topic_filters_correctly() {
        let (storage, _temp) = test_storage();
        let now = Utc::now();

        let a = make_event(GrowthEventKind::TopicClassified, Some(Topic::Rust), now);
        let b = make_event(
            GrowthEventKind::TopicClassified,
            Some(Topic::PostQuantum),
            now,
        );
        let c = make_event(GrowthEventKind::TopicClassified, None, now);

        storage.record_event(a.clone()).await.unwrap();
        storage.record_event(b.clone()).await.unwrap();
        storage.record_event(c).await.unwrap();

        let rust = storage
            .get_events_by_topic(Topic::Rust, 10)
            .await
            .unwrap();
        assert_eq!(rust.len(), 1);
        assert_eq!(rust[0].id, a.id);

        let pq = storage
            .get_events_by_topic(Topic::PostQuantum, 10)
            .await
            .unwrap();
        assert_eq!(pq.len(), 1);
        assert_eq!(pq[0].id, b.id);
    }

    #[tokio::test]
    async fn prune_removes_only_old_events() {
        let (storage, _temp) = test_storage();
        let now = Utc::now();
        let cutoff = now - Duration::hours(2);

        let old1 = make_event(
            GrowthEventKind::SourceRegistered,
            None,
            now - Duration::hours(5),
        );
        let old2 = make_event(
            GrowthEventKind::SourceRegistered,
            None,
            now - Duration::hours(3),
        );
        let young = make_event(
            GrowthEventKind::SourceRegistered,
            None,
            now - Duration::hours(1),
        );

        storage.record_event(old1).await.unwrap();
        storage.record_event(old2).await.unwrap();
        storage.record_event(young.clone()).await.unwrap();

        let removed = storage.prune_events_before(cutoff).await.unwrap();
        assert_eq!(removed, 2);

        let recent = storage.get_recent_events(10).await.unwrap();
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].id, young.id);
    }

    #[tokio::test]
    async fn prune_is_idempotent() {
        let (storage, _temp) = test_storage();
        let now = Utc::now();
        let cutoff = now - Duration::hours(2);

        let old = make_event(
            GrowthEventKind::SourceRegistered,
            None,
            now - Duration::hours(5),
        );
        storage.record_event(old).await.unwrap();

        let first = storage.prune_events_before(cutoff).await.unwrap();
        assert_eq!(first, 1);

        let second = storage.prune_events_before(cutoff).await.unwrap();
        assert_eq!(second, 0, "second prune removes nothing");
    }

    #[tokio::test]
    async fn prune_removes_all_indexes() {
        let (storage, _temp) = test_storage();
        let now = Utc::now();
        let cutoff = now - Duration::hours(2);

        let event = make_event(
            GrowthEventKind::TopicClassified,
            Some(Topic::Rust),
            now - Duration::hours(5),
        );
        storage.record_event(event).await.unwrap();

        storage.prune_events_before(cutoff).await.unwrap();

        assert!(storage.get_recent_events(10).await.unwrap().is_empty());
        assert!(storage
            .get_events_by_kind(GrowthEventKind::TopicClassified, 10)
            .await
            .unwrap()
            .is_empty());
        assert!(storage
            .get_events_by_topic(Topic::Rust, 10)
            .await
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn limit_zero_returns_empty() {
        let (storage, _temp) = test_storage();
        let event = make_event(GrowthEventKind::SourceRegistered, None, Utc::now());
        storage.record_event(event).await.unwrap();

        assert!(storage
            .get_events_by_kind(GrowthEventKind::SourceRegistered, 0)
            .await
            .unwrap()
            .is_empty());
        assert!(storage.get_recent_events(0).await.unwrap().is_empty());
        assert!(storage
            .get_events_since(Utc::now() - Duration::days(1), 0)
            .await
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn limit_is_respected() {
        let (storage, _temp) = test_storage();
        for _ in 0..5 {
            let event = make_event(GrowthEventKind::SourceRegistered, None, Utc::now());
            storage.record_event(event).await.unwrap();
        }
        let list = storage.get_recent_events(2).await.unwrap();
        assert_eq!(list.len(), 2);
    }

    #[tokio::test]
    async fn same_timestamp_is_deterministic() {
        let (storage, _temp) = test_storage();
        let ts = Utc::now();

        let a = make_event(GrowthEventKind::SourceRegistered, None, ts);
        let b = make_event(GrowthEventKind::SourceRegistered, None, ts);

        storage.record_event(a).await.unwrap();
        storage.record_event(b).await.unwrap();

        let first = storage.get_recent_events(10).await.unwrap();
        let second = storage.get_recent_events(10).await.unwrap();
        assert_eq!(first.len(), 2);
        assert_eq!(first[0].id, second[0].id);
        assert_eq!(first[1].id, second[1].id);
    }
}
