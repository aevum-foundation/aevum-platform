//! Concrete `GrowthApi` implementation over `Arc<GrowthService>`.
//!
//! The impl is a thin adapter: it composes existing service methods
//! into the response DTOs declared in `growth/contracts.rs`.
//!
//! No business logic lives here.

use std::sync::Arc;

use async_trait::async_trait;

use crate::error::ApiError;
use crate::growth::aevumdb::AevumDbGrowthStorage;
use crate::growth::api::{GrowthApi, AppGrowthService};
use crate::growth::contracts::{
    GrowthHealthResponse, OpportunityResponse, SourceResponse, TopicReportResponse,
    TopicSummaryResponse,
};
use crate::growth::models::Topic;
use crate::growth::service::GrowthService;

/// Adapter: `GrowthService<AevumDbGrowthStorage>` → `GrowthApi`.
pub struct GrowthApiImpl {
    service: Arc<GrowthService<AevumDbGrowthStorage>>,
}

impl GrowthApiImpl {
    pub fn new(service: Arc<GrowthService<AevumDbGrowthStorage>>) -> Self {
        Self { service }
    }
}

/// Wrap the adapter into the trait-object alias used by handlers.
pub fn into_app(service: Arc<GrowthService<AevumDbGrowthStorage>>) -> AppGrowthService {
    Arc::new(GrowthApiImpl::new(service))
}

#[async_trait]
impl GrowthApi for GrowthApiImpl {
    async fn health(&self) -> Result<GrowthHealthResponse, ApiError> {
        let sources = self.service.list_sources(10_000).await?;
        let snapshot = self.service.snapshot().await?;
        Ok(GrowthHealthResponse {
            sources: sources.len(),
            topics: snapshot.trends.len(),
            opportunities: snapshot.opportunities.len(),
            generated_at: snapshot.generated_at,
        })
    }

    async fn list_topics(&self) -> Result<Vec<TopicSummaryResponse>, ApiError> {
        let trends = self.service.compute_trends().await?;
        Ok(trends.iter().map(TopicSummaryResponse::from).collect())
    }

    async fn topic_report(&self, topic: Topic) -> Result<TopicReportResponse, ApiError> {
        let report = self.service.topic_report(topic).await?;
        Ok(TopicReportResponse::from(report))
    }

    async fn list_opportunities(
        &self,
        limit: usize,
    ) -> Result<Vec<OpportunityResponse>, ApiError> {
        let result = self.service.analyze_opportunities().await?;
        Ok(result
            .opportunities
            .iter()
            .take(limit)
            .map(OpportunityResponse::from)
            .collect())
    }

    async fn list_sources(&self, limit: usize) -> Result<Vec<SourceResponse>, ApiError> {
        let sources = self.service.list_sources(limit).await?;
        Ok(sources.iter().map(SourceResponse::from).collect())
    }
}
