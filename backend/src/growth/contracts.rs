//! Growth HTTP/API contracts and stable validation error vocabulary.
//!
//! Domain models are intentionally not exposed directly through the API.
//! Response DTOs define the public contract and prevent accidental leakage
//! of internal storage or ingestion details.

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::error::ApiError;

use super::models::{
    Opportunity, Publication, Source, TopicTrend,
};
use super::service::TopicReport;
use super::validation::ValidationError;

// ---------------------------------------------------------------------------
// Stable validation error vocabulary
// ---------------------------------------------------------------------------

pub const SOURCE_FEED_URL_INVALID_SCHEME_CODE: &str =
    "GROWTH_SOURCE_FEED_URL_INVALID_SCHEME";
pub const SOURCE_FEED_URL_INVALID_SCHEME_MESSAGE: &str =
    "Source feed URL must use http or https";

pub const SOURCE_TOPICS_REQUIRED_CODE: &str =
    "GROWTH_SOURCE_TOPICS_REQUIRED";
pub const SOURCE_TOPICS_REQUIRED_MESSAGE: &str =
    "Source must contain at least one topic";

pub const SOURCE_TOPICS_DUPLICATE_CODE: &str =
    "GROWTH_SOURCE_TOPICS_DUPLICATE";
pub const SOURCE_TOPICS_DUPLICATE_MESSAGE: &str =
    "Source contains duplicate topics";

pub const PUBLICATION_EXTERNAL_ID_REQUIRED_CODE: &str =
    "GROWTH_PUBLICATION_EXTERNAL_ID_REQUIRED";
pub const PUBLICATION_EXTERNAL_ID_REQUIRED_MESSAGE: &str =
    "Publication external_id is required";

pub const PUBLICATION_TITLE_REQUIRED_CODE: &str =
    "GROWTH_PUBLICATION_TITLE_REQUIRED";
pub const PUBLICATION_TITLE_REQUIRED_MESSAGE: &str =
    "Publication title is required";

pub const OPPORTUNITY_SCORE_OUT_OF_RANGE_CODE: &str =
    "GROWTH_OPPORTUNITY_SCORE_OUT_OF_RANGE";
pub const OPPORTUNITY_SCORE_OUT_OF_RANGE_MESSAGE: &str =
    "Opportunity score must be between 0 and 10000 basis points";

pub const OPPORTUNITY_EVIDENCE_REQUIRED_CODE: &str =
    "GROWTH_OPPORTUNITY_EVIDENCE_REQUIRED";
pub const OPPORTUNITY_EVIDENCE_REQUIRED_MESSAGE: &str =
    "Opportunity must contain at least one evidence publication";

// ---------------------------------------------------------------------------
// Public response DTOs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct SourceResponse {
    pub id: String,
    pub platform: String,
    pub feed_url: String,
    pub homepage: Option<String>,
    pub title: Option<String>,
    pub topics: Vec<String>,
    pub status: String,
    pub consecutive_failures: u32,
    pub last_checked: Option<DateTime<Utc>>,
    pub last_success: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PublicationResponse {
    pub id: String,
    pub source_id: String,
    pub external_id: String,
    pub url: Option<String>,
    pub title: String,
    pub summary: Option<String>,
    pub author: Option<String>,
    pub language: Option<String>,
    pub topics: Vec<String>,
    pub published_at: Option<DateTime<Utc>>,
    pub ingested_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TopicTrendResponse {
    pub topic: String,
    pub count_24h: u32,
    pub count_7d: u32,
    pub count_30d: u32,
    pub ratio_7d_vs_30d_bp: u32,
    pub computed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OpportunityResponse {
    pub id: String,
    pub kind: String,
    pub topic: String,
    pub score_bp: u32,
    pub evidence: Vec<String>,
    pub detected_at: DateTime<Utc>,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct TopicsResponse {
    pub topics: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GrowthMetricsResponse {
    pub source_count: u64,
    pub publication_count: u64,
    pub opportunity_count: u64,
    pub computed_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Domain → API conversions
// ---------------------------------------------------------------------------

/// Health/status snapshot for the Growth subsystem.
///
/// Served by `GET /growth/health`.
#[derive(Debug, Clone, Serialize)]
pub struct GrowthHealthResponse {
    pub sources: usize,
    pub topics: usize,
    pub opportunities: usize,
    pub generated_at: DateTime<Utc>,
}

/// Summary of a single topic for `GET /growth/topics`.
#[derive(Debug, Clone, Serialize)]
pub struct TopicSummaryResponse {
    pub topic: String,
    pub count_24h: u32,
    pub count_7d: u32,
    pub count_30d: u32,
    pub ratio_7d_vs_30d_bp: u32,
    /// True when `ratio_7d_vs_30d_bp` exceeds the acceleration
    /// threshold (2.0×). Presentation layers use this without
    /// re-deriving the threshold.
    pub accelerating: bool,
}

/// Full report for `GET /growth/topics/{topic}`.
#[derive(Debug, Clone, Serialize)]
pub struct TopicReportResponse {
    pub topic: String,
    pub generated_at: DateTime<Utc>,
    pub trend: TopicTrendResponse,
    pub recent_publications: Vec<PublicationResponse>,
    pub opportunities: Vec<OpportunityResponse>,
}

impl From<&Source> for SourceResponse {
    fn from(source: &Source) -> Self {
        Self {
            id: source.id.to_string(),
            platform: source.platform.as_str().to_owned(),
            feed_url: source.feed_url.to_string(),
            homepage: source.homepage.as_ref().map(ToString::to_string),
            title: source.title.clone(),
            topics: source
                .topics
                .iter()
                .map(|topic| topic.as_str().to_owned())
                .collect(),
            status: source.status.as_str().to_owned(),
            consecutive_failures: source.consecutive_failures,
            last_checked: source.last_checked,
            last_success: source.last_success,
            created_at: source.created_at,
            updated_at: source.updated_at,
        }
    }
}

impl From<&Publication> for PublicationResponse {
    fn from(publication: &Publication) -> Self {
        Self {
            id: publication.id.to_string(),
            source_id: publication.source_id.to_string(),
            external_id: publication.external_id.clone(),
            url: publication.url.as_ref().map(ToString::to_string),
            title: publication.title.clone(),
            summary: publication.summary.clone(),
            author: publication.author.clone(),
            language: publication.language.clone(),
            topics: publication
                .topics
                .iter()
                .map(|topic| topic.as_str().to_owned())
                .collect(),
            published_at: publication.published_at,
            ingested_at: publication.ingested_at,
        }
    }
}

impl From<&TopicTrend> for TopicTrendResponse {
    fn from(trend: &TopicTrend) -> Self {
        Self {
            topic: trend.topic.as_str().to_owned(),
            count_24h: trend.count_24h,
            count_7d: trend.count_7d,
            count_30d: trend.count_30d,
            ratio_7d_vs_30d_bp: trend.ratio_7d_vs_30d_bp,
            computed_at: trend.computed_at,
        }
    }
}

impl From<&Opportunity> for OpportunityResponse {
    fn from(opportunity: &Opportunity) -> Self {
        Self {
            id: opportunity.id.to_string(),
            kind: opportunity.kind.as_str().to_owned(),
            topic: opportunity.topic.as_str().to_owned(),
            score_bp: opportunity.score_bp,
            evidence: opportunity
                .evidence
                .iter()
                .map(ToString::to_string)
                .collect(),
            detected_at: opportunity.detected_at,
            payload: opportunity.payload.clone(),
        }
    }
}

// ---------------------------------------------------------------------------
// ValidationError → ApiError
// ---------------------------------------------------------------------------

impl From<&TopicTrend> for TopicSummaryResponse {
    fn from(trend: &TopicTrend) -> Self {
        use crate::growth::analysis::trends::ACCELERATION_THRESHOLD_BP;
        Self {
            topic: trend.topic.as_str().to_owned(),
            count_24h: trend.count_24h,
            count_7d: trend.count_7d,
            count_30d: trend.count_30d,
            ratio_7d_vs_30d_bp: trend.ratio_7d_vs_30d_bp,
            accelerating: trend.ratio_7d_vs_30d_bp > ACCELERATION_THRESHOLD_BP,
        }
    }
}

impl From<TopicReport> for TopicReportResponse {
    fn from(report: TopicReport) -> Self {
        Self {
            topic: report.trend.topic.as_str().to_owned(),
            generated_at: report.generated_at,
            trend: TopicTrendResponse::from(&report.trend),
            recent_publications: report
                .recent_publications
                .iter()
                .map(PublicationResponse::from)
                .collect(),
            opportunities: report
                .opportunities
                .iter()
                .map(OpportunityResponse::from)
                .collect(),
        }
    }
}

impl From<ValidationError> for ApiError {
    fn from(error: ValidationError) -> Self {
        let (code, message) = match error {
            ValidationError::SourceFeedUrlInvalidScheme => (
                SOURCE_FEED_URL_INVALID_SCHEME_CODE,
                SOURCE_FEED_URL_INVALID_SCHEME_MESSAGE,
            ),

            ValidationError::SourceTopicsEmpty => (
                SOURCE_TOPICS_REQUIRED_CODE,
                SOURCE_TOPICS_REQUIRED_MESSAGE,
            ),

            ValidationError::SourceTopicsDuplicate { .. } => (
                SOURCE_TOPICS_DUPLICATE_CODE,
                SOURCE_TOPICS_DUPLICATE_MESSAGE,
            ),

            ValidationError::PublicationExternalIdEmpty => (
                PUBLICATION_EXTERNAL_ID_REQUIRED_CODE,
                PUBLICATION_EXTERNAL_ID_REQUIRED_MESSAGE,
            ),

            ValidationError::PublicationTitleEmpty => (
                PUBLICATION_TITLE_REQUIRED_CODE,
                PUBLICATION_TITLE_REQUIRED_MESSAGE,
            ),

            ValidationError::OpportunityScoreOutOfRange { .. } => (
                OPPORTUNITY_SCORE_OUT_OF_RANGE_CODE,
                OPPORTUNITY_SCORE_OUT_OF_RANGE_MESSAGE,
            ),

            ValidationError::OpportunityEvidenceEmpty => (
                OPPORTUNITY_EVIDENCE_REQUIRED_CODE,
                OPPORTUNITY_EVIDENCE_REQUIRED_MESSAGE,
            ),
        };

        ApiError::ValidationFailed { code, message }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    use chrono::Utc;
    use url::Url;

    use crate::growth::models::{
        OpportunityId, OpportunityKind, Platform, PublicationId, Source,
        Topic,
    };

    fn test_source() -> Source {
        Source::new(
            Platform::Rss,
            Url::parse("https://example.com/feed.xml").unwrap(),
            Some(Url::parse("https://example.com").unwrap()),
            vec![Topic::Rust],
            Utc::now(),
        )
    }

    fn test_publication() -> Publication {
        let source = test_source();

        Publication {
            id: PublicationId::from_parts(source.id, "guid-1"),
            source_id: source.id,
            external_id: "guid-1".to_owned(),
            url: Some(Url::parse("https://example.com/post").unwrap()),
            title: "Rust publication".to_owned(),
            summary: Some("Summary".to_owned()),
            author: Some("Author".to_owned()),
            language: Some("en".to_owned()),
            topics: vec![Topic::Rust],
            published_at: None,
            ingested_at: Utc::now(),
        }
    }

    fn test_trend() -> TopicTrend {
        TopicTrend {
            topic: Topic::Rust,
            count_24h: 1,
            count_7d: 7,
            count_30d: 30,
            ratio_7d_vs_30d_bp: 10_000,
            computed_at: Utc::now(),
        }
    }

    fn test_opportunity() -> Opportunity {
        let publication = test_publication();

        Opportunity {
            id: OpportunityId::new(),
            kind: OpportunityKind::TopicAccelerating,
            topic: Topic::Rust,
            score_bp: 5_000,
            evidence: vec![publication.id],
            detected_at: Utc::now(),
            payload: serde_json::json!({
                "signal": "test"
            }),
        }
    }

    #[test]
    fn frozen_validation_messages_are_stable() {
        assert_eq!(
            SOURCE_FEED_URL_INVALID_SCHEME_MESSAGE,
            "Source feed URL must use http or https"
        );

        assert_eq!(
            SOURCE_TOPICS_REQUIRED_MESSAGE,
            "Source must contain at least one topic"
        );

        assert_eq!(
            SOURCE_TOPICS_DUPLICATE_MESSAGE,
            "Source contains duplicate topics"
        );

        assert_eq!(
            PUBLICATION_EXTERNAL_ID_REQUIRED_MESSAGE,
            "Publication external_id is required"
        );

        assert_eq!(
            PUBLICATION_TITLE_REQUIRED_MESSAGE,
            "Publication title is required"
        );

        assert_eq!(
            OPPORTUNITY_SCORE_OUT_OF_RANGE_MESSAGE,
            "Opportunity score must be between 0 and 10000 basis points"
        );

        assert_eq!(
            OPPORTUNITY_EVIDENCE_REQUIRED_MESSAGE,
            "Opportunity must contain at least one evidence publication"
        );
    }

    #[test]
    fn validation_error_maps_to_stable_api_error() {
        let error = ApiError::from(
            ValidationError::SourceTopicsDuplicate {
                topic: Topic::Rust,
            },
        );

        assert_eq!(error.code(), SOURCE_TOPICS_DUPLICATE_CODE);
        assert_eq!(error.message(), SOURCE_TOPICS_DUPLICATE_MESSAGE);
    }

    #[test]
    fn validation_error_does_not_leak_topic_detail() {
        let error = ApiError::from(
            ValidationError::SourceTopicsDuplicate {
                topic: Topic::Rust,
            },
        );

        assert_eq!(error.code(), SOURCE_TOPICS_DUPLICATE_CODE);
        assert_eq!(error.message(), SOURCE_TOPICS_DUPLICATE_MESSAGE);
        assert!(!error.message().contains("rust"));
    }

    #[test]
    fn source_response_contains_public_source_fields() {
        let response = SourceResponse::from(&test_source());
        let value = serde_json::to_value(response).unwrap();

        assert_eq!(value["platform"], "rss");
        assert_eq!(value["feed_url"], "https://example.com/feed.xml");
        assert!(value.get("last_error").is_none());
    }

    #[test]
    fn publication_response_contains_expected_fields() {
        let response = PublicationResponse::from(&test_publication());
        let value = serde_json::to_value(response).unwrap();

        assert_eq!(value["external_id"], "guid-1");
        assert_eq!(value["title"], "Rust publication");
    }

    #[test]
    fn trend_response_serializes_integer_ratio() {
        let response = TopicTrendResponse::from(&test_trend());
        let value = serde_json::to_value(response).unwrap();

        assert_eq!(value["ratio_7d_vs_30d_bp"], 10_000);
        assert!(value["ratio_7d_vs_30d_bp"].is_number());
    }

    #[test]
    fn opportunity_response_contains_evidence_ids() {
        let opportunity = test_opportunity();
        let response = OpportunityResponse::from(&opportunity);

        assert_eq!(response.evidence.len(), 1);
        assert_eq!(response.score_bp, 5_000);
    }

    #[test]
    fn public_responses_do_not_expose_internal_error() {
        let source = test_source();
        let response = SourceResponse::from(&source);
        let value = serde_json::to_value(response).unwrap();

        assert!(value.get("last_error").is_none());
    }

    #[test]
    fn public_response_does_not_expose_domain_debug_fields() {
        let publication = test_publication();
        let response = PublicationResponse::from(&publication);
        let value = serde_json::to_value(response).unwrap();

        assert!(value.get("debug").is_none());
        assert!(value.get("storage_key").is_none());
    }
}
