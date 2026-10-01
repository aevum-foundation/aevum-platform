//! In-memory Growth storage backend.
//!
//! Alternative implementation of the five Growth storage traits.
//! Used by tests and by development composition.
//!
//! IMPORTANT:
//!
//! This implementation prioritizes behavioural compatibility
//! with `AevumDbGrowthStorage` over performance.
//!
//! Any observable ordering, filtering, idempotency,
//! overwrite semantics, or pruning behaviour MUST remain
//! equivalent to the production backend.
//!
//! # Design
//!
//! No secondary indexes are materialized. Every list operation
//! reads the primary map, filters in memory, sorts with the same
//! comparator semantics as the AevumDB key layout, and truncates
//! to `limit`.
//!
//! Ordering semantics (must match AevumDB):
//!
//! - Source           : `source_id.as_hex()` ASC
//! - Publication      : `effective_ts` DESC, then `id.as_hex()` ASC
//! - Opportunity      : `score_bp` DESC, then `id.0.to_string()` ASC
//! - TopicState       : `topic.as_str()` ASC
//! - GrowthEvent      : `occurred_at` DESC, then `id.to_string()` ASC
//!                      (ASC for `get_events_since`)
//!
//! # Contracts (shared with AevumDbGrowthStorage)
//!
//! - `Publication` is immutable after write. Immutability is a
//!   domain-level contract enforced by callers, NOT by storage.
//!   Both backends perform plain overwrite on repeated writes.
//! - `GrowthEvent` is append-only; removal is only via
//!   `prune_events_before`.
//! - `delete_source` is a soft delete (`status = Disabled`),
//!   idempotent.
//! - `put_publication` and `put_source` are idempotent upserts
//!   keyed by deterministic IDs.

use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::error::ApiError;
use crate::growth::events::{GrowthEvent, GrowthEventKind};
use crate::growth::models::{
    Opportunity, OpportunityId, OpportunityKind, Publication, PublicationId, Source, SourceId,
    SourceStatus, Topic, TopicState,
};
use crate::growth::storage::{
    GrowthEventStorage, OpportunityStorage, PublicationStorage, SourceStorage, TopicStateStorage,
};

// ---------------------------------------------------------------------------
// Storage struct
// ---------------------------------------------------------------------------

/// In-memory storage backend for the Growth domain.
///
/// Cheap to construct, not durable. Suitable for unit tests and
/// development composition.
pub struct InMemoryGrowthStorage {
    sources: Mutex<HashMap<SourceId, Source>>,
    publications: Mutex<HashMap<PublicationId, Publication>>,
    opportunities: Mutex<HashMap<OpportunityId, Opportunity>>,
    topic_states: Mutex<HashMap<Topic, TopicState>>,
    events: Mutex<Vec<GrowthEvent>>,
}

impl Default for InMemoryGrowthStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for InMemoryGrowthStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InMemoryGrowthStorage").finish_non_exhaustive()
    }
}

impl InMemoryGrowthStorage {
    pub fn new() -> Self {
        Self {
            sources: Mutex::new(HashMap::new()),
            publications: Mutex::new(HashMap::new()),
            opportunities: Mutex::new(HashMap::new()),
            topic_states: Mutex::new(HashMap::new()),
            events: Mutex::new(Vec::new()),
        }
    }

    fn lock<'a, T>(m: &'a Mutex<T>) -> Result<std::sync::MutexGuard<'a, T>, ApiError> {
        m.lock().map_err(|error| {
            log::error!("InMemoryGrowthStorage: lock poisoned: {}", error);
            ApiError::Internal
        })
    }
}

// ---------------------------------------------------------------------------
// Comparators (canonical ordering, shared by all list methods)
// ---------------------------------------------------------------------------

fn cmp_ts_desc_then_uuid(
    a_ts: DateTime<Utc>,
    a_id: Uuid,
    b_ts: DateTime<Utc>,
    b_id: Uuid,
) -> std::cmp::Ordering {
    b_ts.cmp(&a_ts)
        .then_with(|| a_id.to_string().cmp(&b_id.to_string()))
}

fn cmp_ts_asc_then_uuid(
    a_ts: DateTime<Utc>,
    a_id: Uuid,
    b_ts: DateTime<Utc>,
    b_id: Uuid,
) -> std::cmp::Ordering {
    a_ts.cmp(&b_ts)
        .then_with(|| a_id.to_string().cmp(&b_id.to_string()))
}

fn cmp_score_desc_then_uuid(
    a_score: u32,
    a_id: Uuid,
    b_score: u32,
    b_id: Uuid,
) -> std::cmp::Ordering {
    b_score
        .cmp(&a_score)
        .then_with(|| a_id.to_string().cmp(&b_id.to_string()))
}

fn publication_effective_ts(p: &Publication) -> DateTime<Utc> {
    p.published_at.unwrap_or(p.ingested_at)
}

// ---------------------------------------------------------------------------
// SourceStorage
// ---------------------------------------------------------------------------

#[async_trait]
impl SourceStorage for InMemoryGrowthStorage {
    /// Idempotent upsert. Source identity is `SourceId`, which is
    /// deterministic over `(platform, feed_url)`.
    async fn put_source(&self, source: &Source) -> Result<(), ApiError> {
        let mut map = Self::lock(&self.sources)?;
        map.insert(source.id, source.clone());
        Ok(())
    }

    async fn get_source(&self, id: SourceId) -> Result<Option<Source>, ApiError> {
        let map = Self::lock(&self.sources)?;
        Ok(map.get(&id).cloned())
    }

    /// Soft delete: sets `status = Disabled`. Idempotent.
    async fn delete_source(&self, id: SourceId) -> Result<(), ApiError> {
        let mut map = Self::lock(&self.sources)?;
        let Some(source) = map.get_mut(&id) else {
            return Ok(());
        };
        if source.status == SourceStatus::Disabled {
            return Ok(());
        }
        source.status = SourceStatus::Disabled;
        source.updated_at = Utc::now();
        Ok(())
    }

    async fn list_sources(&self, limit: usize) -> Result<Vec<Source>, ApiError> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        let map = Self::lock(&self.sources)?;
        let mut items: Vec<Source> = map.values().cloned().collect();
        items.sort_by(|a, b| a.id.as_hex().cmp(&b.id.as_hex()));
        items.truncate(limit);
        Ok(items)
    }

    async fn list_sources_by_topic(
        &self,
        topic: Topic,
        limit: usize,
    ) -> Result<Vec<Source>, ApiError> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        let map = Self::lock(&self.sources)?;
        let mut items: Vec<Source> = map
            .values()
            .filter(|s| s.topics.contains(&topic))
            .cloned()
            .collect();
        items.sort_by(|a, b| a.id.as_hex().cmp(&b.id.as_hex()));
        items.truncate(limit);
        Ok(items)
    }

    async fn list_sources_by_status(
        &self,
        status: SourceStatus,
        limit: usize,
    ) -> Result<Vec<Source>, ApiError> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        let map = Self::lock(&self.sources)?;
        let mut items: Vec<Source> = map
            .values()
            .filter(|s| s.status == status)
            .cloned()
            .collect();
        items.sort_by(|a, b| a.id.as_hex().cmp(&b.id.as_hex()));
        items.truncate(limit);
        Ok(items)
    }
}

// ---------------------------------------------------------------------------
// PublicationStorage
// ---------------------------------------------------------------------------

#[async_trait]
impl PublicationStorage for InMemoryGrowthStorage {
    // NOTE:
    //
    // Publication immutability is a DOMAIN-LEVEL contract,
    // enforced by callers, NOT by storage.
    //
    // Storage MUST perform plain overwrite semantics, identical
    // to `AevumDbGrowthStorage`. Do NOT change `insert` to
    // `entry().or_insert()` here: that would silently break
    // conformance with the production backend.
    async fn put_publication(&self, publication: &Publication) -> Result<(), ApiError> {
        let mut map = Self::lock(&self.publications)?;
        map.insert(publication.id, publication.clone());
        Ok(())
    }

    async fn get_publication(
        &self,
        id: PublicationId,
    ) -> Result<Option<Publication>, ApiError> {
        let map = Self::lock(&self.publications)?;
        Ok(map.get(&id).cloned())
    }

    async fn publication_exists(
        &self,
        source_id: SourceId,
        external_id: &str,
    ) -> Result<bool, ApiError> {
        let map = Self::lock(&self.publications)?;
        Ok(map
            .values()
            .any(|p| p.source_id == source_id && p.external_id == external_id))
    }

    async fn list_publications_by_source(
        &self,
        source_id: SourceId,
        limit: usize,
    ) -> Result<Vec<Publication>, ApiError> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        let map = Self::lock(&self.publications)?;
        let mut items: Vec<Publication> = map
            .values()
            .filter(|p| p.source_id == source_id)
            .cloned()
            .collect();
        sort_publications_desc(&mut items);
        items.truncate(limit);
        Ok(items)
    }

    async fn list_publications_by_topic(
        &self,
        topic: Topic,
        limit: usize,
    ) -> Result<Vec<Publication>, ApiError> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        let map = Self::lock(&self.publications)?;
        let mut items: Vec<Publication> = map
            .values()
            .filter(|p| p.topics.contains(&topic))
            .cloned()
            .collect();
        sort_publications_desc(&mut items);
        items.truncate(limit);
        Ok(items)
    }

    async fn list_recent_publications(
        &self,
        limit: usize,
    ) -> Result<Vec<Publication>, ApiError> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        let map = Self::lock(&self.publications)?;
        let mut items: Vec<Publication> = map.values().cloned().collect();
        sort_publications_desc(&mut items);
        items.truncate(limit);
        Ok(items)
    }
}

fn sort_publications_desc(items: &mut [Publication]) {
    items.sort_by(|a, b| {
        let a_ts = publication_effective_ts(a);
        let b_ts = publication_effective_ts(b);
        b_ts.cmp(&a_ts)
            .then_with(|| a.id.as_hex().cmp(&b.id.as_hex()))
    });
}

// ---------------------------------------------------------------------------
// OpportunityStorage
// ---------------------------------------------------------------------------

#[async_trait]
impl OpportunityStorage for InMemoryGrowthStorage {
    async fn put_opportunity(&self, opportunity: &Opportunity) -> Result<(), ApiError> {
        let mut map = Self::lock(&self.opportunities)?;
        map.insert(opportunity.id, opportunity.clone());
        Ok(())
    }

    async fn get_opportunity(
        &self,
        id: OpportunityId,
    ) -> Result<Option<Opportunity>, ApiError> {
        let map = Self::lock(&self.opportunities)?;
        Ok(map.get(&id).cloned())
    }

    async fn list_opportunities_by_topic(
        &self,
        topic: Topic,
        limit: usize,
    ) -> Result<Vec<Opportunity>, ApiError> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        let map = Self::lock(&self.opportunities)?;
        let mut items: Vec<Opportunity> = map
            .values()
            .filter(|o| o.topic == topic)
            .cloned()
            .collect();
        sort_opportunities_desc(&mut items);
        items.truncate(limit);
        Ok(items)
    }

    async fn list_opportunities_by_kind(
        &self,
        kind: OpportunityKind,
        limit: usize,
    ) -> Result<Vec<Opportunity>, ApiError> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        let map = Self::lock(&self.opportunities)?;
        let mut items: Vec<Opportunity> = map
            .values()
            .filter(|o| o.kind == kind)
            .cloned()
            .collect();
        sort_opportunities_desc(&mut items);
        items.truncate(limit);
        Ok(items)
    }

    async fn list_top_opportunities(
        &self,
        limit: usize,
    ) -> Result<Vec<Opportunity>, ApiError> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        let map = Self::lock(&self.opportunities)?;
        let mut items: Vec<Opportunity> = map.values().cloned().collect();
        sort_opportunities_desc(&mut items);
        items.truncate(limit);
        Ok(items)
    }
}

fn sort_opportunities_desc(items: &mut [Opportunity]) {
    items.sort_by(|a, b| {
        cmp_score_desc_then_uuid(a.score_bp, a.id.0, b.score_bp, b.id.0)
    });
}

// ---------------------------------------------------------------------------
// TopicStateStorage
// ---------------------------------------------------------------------------

#[async_trait]
impl TopicStateStorage for InMemoryGrowthStorage {
    async fn put_topic_state(&self, state: &TopicState) -> Result<(), ApiError> {
        let mut map = Self::lock(&self.topic_states)?;
        map.insert(state.topic, state.clone());
        Ok(())
    }

    async fn get_topic_state(&self, topic: Topic) -> Result<Option<TopicState>, ApiError> {
        let map = Self::lock(&self.topic_states)?;
        Ok(map.get(&topic).cloned())
    }

    async fn list_topic_states(&self) -> Result<Vec<TopicState>, ApiError> {
        let map = Self::lock(&self.topic_states)?;
        let mut items: Vec<TopicState> = map.values().cloned().collect();
        items.sort_by(|a, b| a.topic.as_str().cmp(b.topic.as_str()));
        Ok(items)
    }
}

// ---------------------------------------------------------------------------
// GrowthEventStorage
// ---------------------------------------------------------------------------

#[async_trait]
impl GrowthEventStorage for InMemoryGrowthStorage {
    async fn record_event(&self, event: GrowthEvent) -> Result<(), ApiError> {
        let mut events = Self::lock(&self.events)?;
        events.push(event);
        Ok(())
    }

    async fn get_events_by_kind(
        &self,
        kind: GrowthEventKind,
        limit: usize,
    ) -> Result<Vec<GrowthEvent>, ApiError> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        let events = Self::lock(&self.events)?;
        let mut items: Vec<GrowthEvent> = events
            .iter()
            .filter(|e| e.kind == kind)
            .cloned()
            .collect();
        sort_events_desc(&mut items);
        items.truncate(limit);
        Ok(items)
    }

    async fn get_events_by_topic(
        &self,
        topic: Topic,
        limit: usize,
    ) -> Result<Vec<GrowthEvent>, ApiError> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        let events = Self::lock(&self.events)?;
        let mut items: Vec<GrowthEvent> = events
            .iter()
            .filter(|e| e.topic == Some(topic))
            .cloned()
            .collect();
        sort_events_desc(&mut items);
        items.truncate(limit);
        Ok(items)
    }

    async fn get_recent_events(&self, limit: usize) -> Result<Vec<GrowthEvent>, ApiError> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        let events = Self::lock(&self.events)?;
        let mut items: Vec<GrowthEvent> = events.clone();
        sort_events_desc(&mut items);
        items.truncate(limit);
        Ok(items)
    }

    async fn get_events_since(
        &self,
        ts: DateTime<Utc>,
        limit: usize,
    ) -> Result<Vec<GrowthEvent>, ApiError> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        let events = Self::lock(&self.events)?;
        let mut items: Vec<GrowthEvent> = events
            .iter()
            .filter(|e| e.occurred_at >= ts)
            .cloned()
            .collect();
        sort_events_asc(&mut items);
        items.truncate(limit);
        Ok(items)
    }

    async fn prune_events_before(&self, ts: DateTime<Utc>) -> Result<usize, ApiError> {
        let mut events = Self::lock(&self.events)?;
        let before = events.len();
        events.retain(|e| e.occurred_at >= ts);
        Ok(before - events.len())
    }
}

fn sort_events_desc(items: &mut [GrowthEvent]) {
    items.sort_by(|a, b| cmp_ts_desc_then_uuid(a.occurred_at, a.id, b.occurred_at, b.id));
}

fn sort_events_asc(items: &mut [GrowthEvent]) {
    items.sort_by(|a, b| cmp_ts_asc_then_uuid(a.occurred_at, a.id, b.occurred_at, b.id));
}

// ---------------------------------------------------------------------------
// Tests (smoke)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    use chrono::Duration;
    use serde_json::json;
    use url::Url;

    use crate::growth::models::{Platform, PublicationId, SourceId};

    fn now() -> DateTime<Utc> {
        Utc::now()
    }

    fn make_source(url: &str) -> Source {
        Source::new(
            Platform::Rss,
            Url::parse(url).unwrap(),
            None,
            vec![Topic::Rust],
            now(),
        )
    }

    fn make_publication(
        source_id: SourceId,
        ext: &str,
        published_at: Option<DateTime<Utc>>,
    ) -> Publication {
        Publication {
            id: PublicationId::from_parts(source_id, ext),
            source_id,
            external_id: ext.to_owned(),
            url: None,
            title: format!("Post {}", ext),
            summary: None,
            author: None,
            language: None,
            topics: vec![Topic::Rust],
            published_at,
            ingested_at: now(),
        }
    }

    fn make_event(
        kind: GrowthEventKind,
        topic: Option<Topic>,
        at: DateTime<Utc>,
    ) -> GrowthEvent {
        GrowthEvent::new(kind, topic, json!({}), at)
    }

    #[tokio::test]
    async fn source_put_get_roundtrip() {
        let s = InMemoryGrowthStorage::new();
        let src = make_source("https://example.com/feed.xml");
        s.put_source(&src).await.unwrap();
        assert_eq!(s.get_source(src.id).await.unwrap().unwrap().id, src.id);
    }

    #[tokio::test]
    async fn source_soft_delete_idempotent() {
        let s = InMemoryGrowthStorage::new();
        let src = make_source("https://example.com/feed.xml");
        s.put_source(&src).await.unwrap();
        s.delete_source(src.id).await.unwrap();
        s.delete_source(src.id).await.unwrap();
        assert_eq!(
            s.get_source(src.id).await.unwrap().unwrap().status,
            SourceStatus::Disabled
        );
    }

    #[tokio::test]
    async fn source_list_respects_limit() {
        let s = InMemoryGrowthStorage::new();
        for i in 0..5 {
            s.put_source(&make_source(&format!("https://example.com/{}.xml", i)))
                .await
                .unwrap();
        }
        assert_eq!(s.list_sources(2).await.unwrap().len(), 2);
        assert!(s.list_sources(0).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn publication_list_desc_and_limit() {
        let s = InMemoryGrowthStorage::new();
        let src = make_source("https://example.com/feed.xml");
        let now = now();
        s.put_publication(&make_publication(
            src.id,
            "a",
            Some(now - Duration::hours(3)),
        ))
        .await
        .unwrap();
        s.put_publication(&make_publication(src.id, "b", Some(now)))
            .await
            .unwrap();
        s.put_publication(&make_publication(
            src.id,
            "c",
            Some(now - Duration::hours(1)),
        ))
        .await
        .unwrap();
        let list = s.list_recent_publications(2).await.unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].external_id, "b");
        assert_eq!(list[1].external_id, "c");
    }

    #[tokio::test]
    async fn publication_exists_is_scoped_by_source() {
        let s = InMemoryGrowthStorage::new();
        let a = make_source("https://example.com/a.xml");
        let b = make_source("https://example.com/b.xml");
        s.put_publication(&make_publication(a.id, "x", None))
            .await
            .unwrap();
        assert!(s.publication_exists(a.id, "x").await.unwrap());
        assert!(!s.publication_exists(b.id, "x").await.unwrap());
    }

    #[tokio::test]
    async fn opportunity_top_is_desc_by_score() {
        let s = InMemoryGrowthStorage::new();
        for score in [2000u32, 9000, 5000] {
            s.put_opportunity(&Opportunity {
                id: OpportunityId::new(),
                kind: OpportunityKind::TopicAccelerating,
                topic: Topic::Rust,
                score_bp: score,
                evidence: vec![],
                detected_at: now(),
                payload: json!({}),
            })
            .await
            .unwrap();
        }
        let list = s.list_top_opportunities(10).await.unwrap();
        assert_eq!(list.len(), 3);
        assert_eq!(list[0].score_bp, 9000);
        assert_eq!(list[2].score_bp, 2000);
    }

    #[tokio::test]
    async fn topic_states_sorted_asc() {
        let s = InMemoryGrowthStorage::new();
        for topic in [Topic::Rust, Topic::GpuCompute, Topic::PostQuantum] {
            s.put_topic_state(&TopicState::empty(topic, now()))
                .await
                .unwrap();
        }
        let list = s.list_topic_states().await.unwrap();
        assert_eq!(list.len(), 3);
        assert_eq!(list[0].topic, Topic::GpuCompute);
        assert_eq!(list[1].topic, Topic::PostQuantum);
        assert_eq!(list[2].topic, Topic::Rust);
    }

    #[tokio::test]
    async fn events_desc_and_asc_ordering() {
        let s = InMemoryGrowthStorage::new();
        let now = now();
        for h in [5, 1, 3] {
            s.record_event(make_event(
                GrowthEventKind::SourceRegistered,
                None,
                now - Duration::hours(h),
            ))
            .await
            .unwrap();
        }
        let recent = s.get_recent_events(10).await.unwrap();
        assert_eq!(recent.len(), 3);
        assert!(recent[0].occurred_at > recent[1].occurred_at);
        assert!(recent[1].occurred_at > recent[2].occurred_at);

        let since = s
            .get_events_since(now - Duration::hours(4), 10)
            .await
            .unwrap();
        assert_eq!(since.len(), 2);
        assert!(since[0].occurred_at < since[1].occurred_at);
    }

    #[tokio::test]
    async fn same_timestamp_is_deterministic() {
        let s = InMemoryGrowthStorage::new();
        let ts = now();
        let a = make_event(GrowthEventKind::SourceRegistered, None, ts);
        let b = make_event(GrowthEventKind::SourceRegistered, None, ts);
        s.record_event(a).await.unwrap();
        s.record_event(b).await.unwrap();
        let first = s.get_recent_events(10).await.unwrap();
        let second = s.get_recent_events(10).await.unwrap();
        assert_eq!(first[0].id, second[0].id);
        assert_eq!(first[1].id, second[1].id);
    }

    #[tokio::test]
    async fn prune_is_idempotent() {
        let s = InMemoryGrowthStorage::new();
        let now = now();
        let cutoff = now - Duration::hours(2);
        s.record_event(make_event(
            GrowthEventKind::SourceRegistered,
            None,
            now - Duration::hours(5),
        ))
        .await
        .unwrap();
        assert_eq!(s.prune_events_before(cutoff).await.unwrap(), 1);
        assert_eq!(s.prune_events_before(cutoff).await.unwrap(), 0);
    }
}
