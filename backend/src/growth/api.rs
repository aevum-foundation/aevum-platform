//! Growth HTTP API contract.
//!
//! The HTTP layer depends only on this trait, and therefore does
//! not know which concrete `GrowthService<S, F>` is behind it.
//!
//! This mirrors the pattern used by `AuthApi`, `CommunityApi`, and
//! `NotificationApi`.
//!
//! Response DTOs live in `growth/contracts.rs`. They are the public
//! contract; domain models are never returned directly.

use std::sync::Arc;

use async_trait::async_trait;

use crate::error::ApiError;
use crate::growth::contracts::{
    GrowthHealthResponse, OpportunityResponse, SourceResponse, TopicReportResponse,
    TopicSummaryResponse,
};
use crate::growth::models::Topic;

/// Application-facing Growth service exposed to HTTP handlers.
///
/// Handlers use `web::Data<AppGrowthService>`, never a concrete
/// `GrowthService` type.
pub type AppGrowthService = Arc<dyn GrowthApi>;

/// Public contract of the Growth subsystem.
#[async_trait]
pub trait GrowthApi: Send + Sync {
    /// Health / status snapshot.
    ///
    /// Returns counts and a timestamp. Cheap to call; used by
    /// monitoring, systemd healthchecks, and `/growth/health`.
    async fn health(&self) -> Result<GrowthHealthResponse, ApiError>;

    /// List summaries for every topic.
    ///
    /// Ordered by `Topic::ALL`, one entry per topic.
    async fn list_topics(&self) -> Result<Vec<TopicSummaryResponse>, ApiError>;

    /// Full report for a single topic.
    ///
    /// Includes the topic trend, recent publications, and
    /// opportunities for that topic.
    async fn topic_report(&self, topic: Topic) -> Result<TopicReportResponse, ApiError>;

    /// List current opportunities across all topics.
    ///
    /// Bounded by `limit`. There is no unbounded variant; the
    /// caller decides the page size.
    async fn list_opportunities(&self, limit: usize) -> Result<Vec<OpportunityResponse>, ApiError>;

    /// List sources currently in the registry.
    async fn list_sources(&self, limit: usize) -> Result<Vec<SourceResponse>, ApiError>;
}
