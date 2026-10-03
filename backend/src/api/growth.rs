//! Growth HTTP API.
//!
//! HTTP layer only:
//! - path parameter parsing;
//! - query parsing;
//! - delegation to `GrowthApi`;
//! - HTTP response serialization.
//!
//! Business logic lives in `GrowthService` (via `GrowthApi`).
//! This module MUST NOT know about `AevumDbGrowthStorage`,
//! `GrowthService<S, F>`, or any storage internals.
//!
//! Endpoints:
//! - GET /growth/health
//! - GET /growth/topics
//! - GET /growth/topics/{topic}
//! - GET /growth/opportunities?limit=N
//! - GET /growth/sources?limit=N

use actix_web::{route, web, HttpResponse};

use crate::error::ApiError;
use crate::growth::api::AppGrowthService;
use crate::growth::models::Topic;

// ---------------------------------------------------------------------------
// Defaults / bounds
// ---------------------------------------------------------------------------

const DEFAULT_OPPORTUNITY_LIMIT: usize = 50;
const MAX_OPPORTUNITY_LIMIT: usize = 500;

const DEFAULT_SOURCE_LIMIT: usize = 1000;
const MAX_SOURCE_LIMIT: usize = 5000;

// ---------------------------------------------------------------------------
// Query
// ---------------------------------------------------------------------------

#[derive(Debug, serde::Deserialize)]
pub struct LimitQuery {
    pub limit: Option<usize>,
}

impl LimitQuery {
    fn resolve(self, default: usize, max: usize) -> usize {
        let raw = self.limit.unwrap_or(default);
        raw.min(max)
    }
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// GET /growth/health
#[route("/api/growth/health", method = "GET", method = "HEAD")]
pub async fn health(service: web::Data<AppGrowthService>) -> Result<HttpResponse, ApiError> {
    let body = service.health().await?;
    Ok(HttpResponse::Ok().json(body))
}

/// GET /growth/topics
#[route("/api/growth/topics", method = "GET", method = "HEAD")]
pub async fn list_topics(service: web::Data<AppGrowthService>) -> Result<HttpResponse, ApiError> {
    let body = service.list_topics().await?;
    Ok(HttpResponse::Ok().json(body))
}

/// GET /growth/topics/{topic}
///
/// `{topic}` is a canonical URL slug ("post_quantum", "rust", ...).
/// Unknown slugs return 400 with a validation error.
#[route("/api/growth/topics/{topic}", method = "GET", method = "HEAD")]
pub async fn topic_report(
    service: web::Data<AppGrowthService>,
    path: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let slug = path.into_inner();
    let topic = parse_topic(&slug)?;
    let body = service.topic_report(topic).await?;
    Ok(HttpResponse::Ok().json(body))
}

/// GET /growth/opportunities?limit=N
#[route("/api/growth/opportunities", method = "GET", method = "HEAD")]
pub async fn list_opportunities(
    service: web::Data<AppGrowthService>,
    query: web::Query<LimitQuery>,
) -> Result<HttpResponse, ApiError> {
    let limit = query
        .into_inner()
        .resolve(DEFAULT_OPPORTUNITY_LIMIT, MAX_OPPORTUNITY_LIMIT);
    let body = service.list_opportunities(limit).await?;
    Ok(HttpResponse::Ok().json(body))
}

/// GET /growth/sources?limit=N
#[route("/api/growth/sources", method = "GET", method = "HEAD")]
pub async fn list_sources(
    service: web::Data<AppGrowthService>,
    query: web::Query<LimitQuery>,
) -> Result<HttpResponse, ApiError> {
    let limit = query
        .into_inner()
        .resolve(DEFAULT_SOURCE_LIMIT, MAX_SOURCE_LIMIT);
    let body = service.list_sources(limit).await?;
    Ok(HttpResponse::Ok().json(body))
}

// ---------------------------------------------------------------------------
// URL slug → Topic
// ---------------------------------------------------------------------------

/// Parse a canonical URL slug into a `Topic`.
///
/// This is the ONLY place where the URL representation of a topic
/// crosses into the domain. The slug set is defined by
/// `Topic::from_str` (which already handles the canonical
/// snake_case form).
///
/// Unknown slugs produce a `ValidationFailed` error with a stable
/// `GROWTH_TOPIC_UNKNOWN` code. HTTP handlers surface this as 400.
fn parse_topic(slug: &str) -> Result<Topic, ApiError> {
    Topic::from_str(slug).ok_or(ApiError::ValidationFailed {
        code: "GROWTH_TOPIC_UNKNOWN",
        message: "Unknown topic slug",
    })
}

// ---------------------------------------------------------------------------
// Configure
// ---------------------------------------------------------------------------

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(health)
        .service(list_topics)
        .service(topic_report)
        .service(list_opportunities)
        .service(list_sources);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_topic_accepts_canonical_slugs() {
        assert_eq!(parse_topic("post_quantum").unwrap(), Topic::PostQuantum);
        assert_eq!(parse_topic("rust").unwrap(), Topic::Rust);
        assert_eq!(parse_topic("gpu_compute").unwrap(), Topic::GpuCompute);
        assert_eq!(
            parse_topic("storage_systems").unwrap(),
            Topic::StorageSystems,
        );
    }

    #[test]
    fn parse_topic_rejects_unknown_slugs() {
        match parse_topic("does_not_exist") {
            Err(ApiError::ValidationFailed { code, .. }) => {
                assert_eq!(code, "GROWTH_TOPIC_UNKNOWN");
            }
            other => panic!("expected ValidationFailed, got {:?}", other),
        }
    }

    #[test]
    fn parse_topic_rejects_uppercase() {
        assert!(parse_topic("Rust").is_err());
    }

    #[test]
    fn limit_query_uses_default() {
        let q = LimitQuery { limit: None };
        assert_eq!(q.resolve(50, 500), 50);
    }

    #[test]
    fn limit_query_is_capped() {
        let q = LimitQuery {
            limit: Some(10_000),
        };
        assert_eq!(q.resolve(50, 500), 500);
    }

    #[test]
    fn limit_query_respects_user_value() {
        let q = LimitQuery { limit: Some(25) };
        assert_eq!(q.resolve(50, 500), 25);
    }
}
