//! Growth storage abstractions.
//!
//! Five specialized traits, one per entity class. Each trait is
//! implemented by the same production struct (`AevumDbGrowthStorage`) and
//! by an in-memory test struct when needed.
//!
//! # Contract
//!
//! - All methods are `async` and return `Result<_, ApiError>`.
//! - "Not found" is expressed as `Ok(None)`, not as an error.
//! - List methods take an explicit `limit: usize`. There is no unbounded
//!   `list_all()` on purpose.
//! - Event recording is best-effort at the call site: growth flows MUST
//!   NOT fail because event recording failed. The storage itself still
//!   returns a `Result` so that callers *can* observe failures when they
//!   want to.

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::error::ApiError;

use super::events::{GrowthEvent, GrowthEventKind};
use super::models::{
    Opportunity, OpportunityId, OpportunityKind, Publication, PublicationId, Source, SourceId,
    SourceStatus, Topic, TopicState,
};

// ---------------------------------------------------------------------------
// Source
// ---------------------------------------------------------------------------

#[async_trait]
pub trait SourceStorage: Send + Sync {
    async fn put_source(&self, source: &Source) -> Result<(), ApiError>;

    async fn get_source(&self, id: SourceId) -> Result<Option<Source>, ApiError>;

    async fn delete_source(&self, id: SourceId) -> Result<(), ApiError>;

    async fn list_sources(&self, limit: usize) -> Result<Vec<Source>, ApiError>;

    async fn list_sources_by_topic(
        &self,
        topic: Topic,
        limit: usize,
    ) -> Result<Vec<Source>, ApiError>;

    async fn list_sources_by_status(
        &self,
        status: SourceStatus,
        limit: usize,
    ) -> Result<Vec<Source>, ApiError>;
}

// ---------------------------------------------------------------------------
// Publication
// ---------------------------------------------------------------------------

#[async_trait]
pub trait PublicationStorage: Send + Sync {
    async fn put_publication(&self, publication: &Publication) -> Result<(), ApiError>;

    async fn get_publication(&self, id: PublicationId) -> Result<Option<Publication>, ApiError>;

    async fn publication_exists(
        &self,
        source_id: SourceId,
        external_id: &str,
    ) -> Result<bool, ApiError>;

    async fn list_publications_by_source(
        &self,
        source_id: SourceId,
        limit: usize,
    ) -> Result<Vec<Publication>, ApiError>;

    async fn list_publications_by_topic(
        &self,
        topic: Topic,
        limit: usize,
    ) -> Result<Vec<Publication>, ApiError>;

    async fn list_recent_publications(&self, limit: usize) -> Result<Vec<Publication>, ApiError>;
}

// ---------------------------------------------------------------------------
// Opportunity
// ---------------------------------------------------------------------------

#[async_trait]
pub trait OpportunityStorage: Send + Sync {
    async fn put_opportunity(&self, opportunity: &Opportunity) -> Result<(), ApiError>;

    async fn get_opportunity(&self, id: OpportunityId) -> Result<Option<Opportunity>, ApiError>;

    async fn list_opportunities_by_topic(
        &self,
        topic: Topic,
        limit: usize,
    ) -> Result<Vec<Opportunity>, ApiError>;

    async fn list_opportunities_by_kind(
        &self,
        kind: OpportunityKind,
        limit: usize,
    ) -> Result<Vec<Opportunity>, ApiError>;

    async fn list_top_opportunities(&self, limit: usize) -> Result<Vec<Opportunity>, ApiError>;
}

// ---------------------------------------------------------------------------
// TopicState
// ---------------------------------------------------------------------------

#[async_trait]
pub trait TopicStateStorage: Send + Sync {
    async fn put_topic_state(&self, state: &TopicState) -> Result<(), ApiError>;

    async fn get_topic_state(&self, topic: Topic) -> Result<Option<TopicState>, ApiError>;

    /// Safe to expose without a `limit` because `Topic::ALL` is a fixed,
    /// small set.
    async fn list_topic_states(&self) -> Result<Vec<TopicState>, ApiError>;
}

// ---------------------------------------------------------------------------
// GrowthEvent
// ---------------------------------------------------------------------------

#[async_trait]
pub trait GrowthEventStorage: Send + Sync {
    async fn record_event(&self, event: GrowthEvent) -> Result<(), ApiError>;

    async fn get_events_by_kind(
        &self,
        kind: GrowthEventKind,
        limit: usize,
    ) -> Result<Vec<GrowthEvent>, ApiError>;

    async fn get_events_by_topic(
        &self,
        topic: Topic,
        limit: usize,
    ) -> Result<Vec<GrowthEvent>, ApiError>;

    async fn get_recent_events(&self, limit: usize) -> Result<Vec<GrowthEvent>, ApiError>;

    async fn get_events_since(
        &self,
        ts: DateTime<Utc>,
        limit: usize,
    ) -> Result<Vec<GrowthEvent>, ApiError>;

    /// Remove events older than `ts`, together with their secondary
    /// indexes. Returns the number of primary records removed.
    async fn prune_events_before(&self, ts: DateTime<Utc>) -> Result<usize, ApiError>;
}

// ---------------------------------------------------------------------------
// GrowthStorage — supertrait for generic composition
// ---------------------------------------------------------------------------

/// Composite trait implemented by any type that provides all five
/// Growth storage capabilities.
///
/// This is a marker trait: it adds no methods of its own. Its
/// purpose is to allow generic code — notably the conformance test
/// suite — to be written once and run against any backend that
/// implements the full Growth storage contract.
///
/// Both `AevumDbGrowthStorage` and `InMemoryGrowthStorage`
/// automatically implement `GrowthStorage` via the blanket impl
/// below.
pub trait GrowthStorage:
    SourceStorage + PublicationStorage + OpportunityStorage + TopicStateStorage + GrowthEventStorage
{
}

impl<T> GrowthStorage for T where
    T: SourceStorage
        + PublicationStorage
        + OpportunityStorage
        + TopicStateStorage
        + GrowthEventStorage
{
}
