//! Growth domain validation.
//!
//! Validation is intentionally separate from domain models.
//! It verifies semantic invariants without mutating domain objects.

use std::fmt;

use crate::growth::models::{
    Opportunity, Publication, Source, Topic, TopicTrend,
};

// ---------------------------------------------------------------------------
// Limits
// ---------------------------------------------------------------------------

/// Maximum representable opportunity score in basis points.
///
/// 10_000 bp = 1.0.
pub const SCORE_BP_MAX: u32 = 10_000;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    SourceFeedUrlInvalidScheme,
    SourceTopicsEmpty,
    SourceTopicsDuplicate { topic: Topic },

    PublicationExternalIdEmpty,
    PublicationTitleEmpty,

    OpportunityScoreOutOfRange { max: u32, actual: u32 },
    OpportunityEvidenceEmpty,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SourceFeedUrlInvalidScheme => {
                formatter.write_str("source feed URL must use http or https")
            }

            Self::SourceTopicsEmpty => {
                formatter.write_str("source must contain at least one topic")
            }

            Self::SourceTopicsDuplicate { topic } => {
                write!(formatter, "source contains duplicate topic {topic:?}")
            }

            Self::PublicationExternalIdEmpty => {
                formatter.write_str("publication external_id must not be empty")
            }

            Self::PublicationTitleEmpty => {
                formatter.write_str("publication title must not be empty")
            }

            Self::OpportunityScoreOutOfRange { max, actual } => {
                write!(
                    formatter,
                    "opportunity score is {actual} bp; maximum is {max} bp"
                )
            }

            Self::OpportunityEvidenceEmpty => {
                formatter.write_str("opportunity must contain at least one evidence publication")
            }
        }
    }
}

impl std::error::Error for ValidationError {}

// ---------------------------------------------------------------------------
// Source
// ---------------------------------------------------------------------------

pub fn validate_source(source: &Source) -> Result<(), ValidationError> {
    match source.feed_url.scheme() {
        "http" | "https" => {}
        _ => return Err(ValidationError::SourceFeedUrlInvalidScheme),
    }

    if source.topics.is_empty() {
        return Err(ValidationError::SourceTopicsEmpty);
    }

    for (index, topic) in source.topics.iter().enumerate() {
        if source.topics[..index].contains(topic) {
            return Err(ValidationError::SourceTopicsDuplicate { topic: *topic });
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Publication
// ---------------------------------------------------------------------------

pub fn validate_publication(publication: &Publication) -> Result<(), ValidationError> {
    if publication.external_id.trim().is_empty() {
        return Err(ValidationError::PublicationExternalIdEmpty);
    }

    if publication.title.trim().is_empty() {
        return Err(ValidationError::PublicationTitleEmpty);
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Topic trend
// ---------------------------------------------------------------------------

pub fn validate_topic_trend(_trend: &TopicTrend) -> Result<(), ValidationError> {
    // All persisted trend counters and the ratio are represented by u32.
    // Arithmetic overflow is an invariant of the calculation layer, not of
    // an already-materialized domain value.
    Ok(())
}

// ---------------------------------------------------------------------------
// Opportunity
// ---------------------------------------------------------------------------

pub fn validate_opportunity(
    opportunity: &Opportunity,
) -> Result<(), ValidationError> {
    if opportunity.score_bp > SCORE_BP_MAX {
        return Err(ValidationError::OpportunityScoreOutOfRange {
            max: SCORE_BP_MAX,
            actual: opportunity.score_bp,
        });
    }

    if opportunity.evidence.is_empty() {
        return Err(ValidationError::OpportunityEvidenceEmpty);
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    use chrono::Utc;
    use url::Url;
    use uuid::Uuid;

    use crate::growth::models::{
        OpportunityId, OpportunityKind, Platform, PublicationId, SourceId,
        SourceStatus,
    };

    fn test_source() -> Source {
        Source::new(
            Platform::Rss,
            Url::parse("https://example.com/feed.xml").unwrap(),
            None,
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
            summary: None,
            author: None,
            language: None,
            topics: vec![Topic::Rust],
            published_at: None,
            ingested_at: Utc::now(),
        }
    }

    fn test_opportunity() -> Opportunity {
        let source = test_source();

        Opportunity {
            id: OpportunityId::new(),
            kind: OpportunityKind::TopicAccelerating,
            topic: Topic::Rust,
            score_bp: 5_000,
            evidence: vec![PublicationId::from_parts(source.id, "guid-1")],
            detected_at: Utc::now(),
            payload: serde_json::json!({}),
        }
    }

    #[test]
    fn source_accepts_https_feed() {
        assert!(validate_source(&test_source()).is_ok());
    }

    #[test]
    fn source_rejects_non_http_scheme() {
        let mut source = test_source();
        source.feed_url = Url::parse("ftp://example.com/feed.xml").unwrap();

        assert_eq!(
            validate_source(&source),
            Err(ValidationError::SourceFeedUrlInvalidScheme)
        );
    }

    #[test]
    fn source_rejects_empty_topics() {
        let mut source = test_source();
        source.topics.clear();

        assert_eq!(
            validate_source(&source),
            Err(ValidationError::SourceTopicsEmpty)
        );
    }

    #[test]
    fn source_rejects_duplicate_topics() {
        let mut source = test_source();
        source.topics = vec![Topic::Rust, Topic::Rust];

        assert_eq!(
            validate_source(&source),
            Err(ValidationError::SourceTopicsDuplicate {
                topic: Topic::Rust
            })
        );
    }

    #[test]
    fn publication_accepts_valid_value() {
        assert!(validate_publication(&test_publication()).is_ok());
    }

    #[test]
    fn publication_rejects_empty_external_id() {
        let mut publication = test_publication();
        publication.external_id = "   ".to_owned();

        assert_eq!(
            validate_publication(&publication),
            Err(ValidationError::PublicationExternalIdEmpty)
        );
    }

    #[test]
    fn publication_rejects_empty_title() {
        let mut publication = test_publication();
        publication.title = "   ".to_owned();

        assert_eq!(
            validate_publication(&publication),
            Err(ValidationError::PublicationTitleEmpty)
        );
    }

    #[test]
    fn trend_accepts_materialized_u32_values() {
        let trend = TopicTrend {
            topic: Topic::Rust,
            count_24h: u32::MAX,
            count_7d: u32::MAX,
            count_30d: u32::MAX,
            ratio_7d_vs_30d_bp: u32::MAX,
            computed_at: Utc::now(),
        };

        assert!(validate_topic_trend(&trend).is_ok());
    }

    #[test]
    fn opportunity_accepts_score_at_maximum() {
        let mut opportunity = test_opportunity();
        opportunity.score_bp = SCORE_BP_MAX;

        assert!(validate_opportunity(&opportunity).is_ok());
    }

    #[test]
    fn opportunity_rejects_score_above_maximum() {
        let mut opportunity = test_opportunity();
        opportunity.score_bp = SCORE_BP_MAX + 1;

        assert_eq!(
            validate_opportunity(&opportunity),
            Err(ValidationError::OpportunityScoreOutOfRange {
                max: SCORE_BP_MAX,
                actual: SCORE_BP_MAX + 1
            })
        );
    }

    #[test]
    fn opportunity_rejects_empty_evidence() {
        let mut opportunity = test_opportunity();
        opportunity.evidence.clear();

        assert_eq!(
            validate_opportunity(&opportunity),
            Err(ValidationError::OpportunityEvidenceEmpty)
        );
    }

    #[test]
    fn validation_error_implements_error() {
        let error: Box<dyn std::error::Error> =
            Box::new(ValidationError::SourceTopicsEmpty);

        assert!(error.to_string().contains("at least one topic"));
    }

    #[test]
    fn source_status_does_not_affect_validation() {
        let mut source = test_source();
        source.status = SourceStatus::Paused;

        assert!(validate_source(&source).is_ok());
    }

    #[test]
    fn publication_id_is_not_part_of_publication_validation() {
        let mut publication = test_publication();

        let other_source = SourceId::from_feed(
            Platform::Rss,
            &Url::parse("https://other.example.com/feed.xml").unwrap(),
        );

        publication.id = PublicationId::from_parts(
            other_source,
            &Uuid::new_v4().to_string(),
        );

        assert!(validate_publication(&publication).is_ok());
    }
}
