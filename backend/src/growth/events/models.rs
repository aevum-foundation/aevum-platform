//! Growth event ledger models.
//!
//! Events are append-only records of what happened in the Growth
//! subsystem. They are NOT domain entities: they exist to explain *why*
//! the current state of Sources, Publications, Topics and Opportunities
//! looks the way it does.
//!
//! Identity is UUID v4. Ordering is `occurred_at`.
//!
//! Payload is free-form JSON because each event kind has its own
//! documented shape. Validation of payload structure belongs to the
//! producer of the event, not to this module.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::growth::models::Topic;

/// Lifecycle and analysis events emitted by the Growth subsystem.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GrowthEventKind {
    // ---- Source lifecycle ----
    SourceRegistered,
    SourceFetchStarted,
    SourceFetchSucceeded,
    SourceFetchFailed,
    SourcePaused,
    SourceResumed,

    // ---- Ingestion ----
    PublicationIngested,
    PublicationSkippedDuplicate,

    // ---- Analysis ----
    TopicClassified,
    TopicStateUpdated,
    TrendComputed,
    OpportunityDetected,
}

impl GrowthEventKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SourceRegistered => "source_registered",
            Self::SourceFetchStarted => "source_fetch_started",
            Self::SourceFetchSucceeded => "source_fetch_succeeded",
            Self::SourceFetchFailed => "source_fetch_failed",
            Self::SourcePaused => "source_paused",
            Self::SourceResumed => "source_resumed",
            Self::PublicationIngested => "publication_ingested",
            Self::PublicationSkippedDuplicate => "publication_skipped_duplicate",
            Self::TopicClassified => "topic_classified",
            Self::TopicStateUpdated => "topic_state_updated",
            Self::TrendComputed => "trend_computed",
            Self::OpportunityDetected => "opportunity_detected",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "source_registered" => Some(Self::SourceRegistered),
            "source_fetch_started" => Some(Self::SourceFetchStarted),
            "source_fetch_succeeded" => Some(Self::SourceFetchSucceeded),
            "source_fetch_failed" => Some(Self::SourceFetchFailed),
            "source_paused" => Some(Self::SourcePaused),
            "source_resumed" => Some(Self::SourceResumed),
            "publication_ingested" => Some(Self::PublicationIngested),
            "publication_skipped_duplicate" => Some(Self::PublicationSkippedDuplicate),
            "topic_classified" => Some(Self::TopicClassified),
            "topic_state_updated" => Some(Self::TopicStateUpdated),
            "trend_computed" => Some(Self::TrendComputed),
            "opportunity_detected" => Some(Self::OpportunityDetected),
            _ => None,
        }
    }
}

/// Append-only record of a Growth subsystem event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrowthEvent {
    /// UUID v4. Identity only; do NOT use for ordering.
    pub id: Uuid,
    /// Event time. Ordering key.
    pub occurred_at: DateTime<Utc>,
    pub kind: GrowthEventKind,
    /// Topic this event belongs to, when applicable.
    /// Used for the `growth:event:topic:*` secondary index.
    pub topic: Option<Topic>,
    /// Free-form payload. Shape depends on `kind` and is documented at
    /// the producer side.
    pub payload: serde_json::Value,
}

impl GrowthEvent {
    pub fn new(
        kind: GrowthEventKind,
        topic: Option<Topic>,
        payload: serde_json::Value,
        occurred_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            occurred_at,
            kind,
            topic,
            payload,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn kind_as_str_is_stable() {
        assert_eq!(
            GrowthEventKind::SourceRegistered.as_str(),
            "source_registered"
        );
        assert_eq!(
            GrowthEventKind::PublicationIngested.as_str(),
            "publication_ingested"
        );
        assert_eq!(
            GrowthEventKind::OpportunityDetected.as_str(),
            "opportunity_detected"
        );
    }

    #[test]
    fn kind_round_trips() {
        let kinds = [
            GrowthEventKind::SourceRegistered,
            GrowthEventKind::SourceFetchStarted,
            GrowthEventKind::SourceFetchSucceeded,
            GrowthEventKind::SourceFetchFailed,
            GrowthEventKind::SourcePaused,
            GrowthEventKind::SourceResumed,
            GrowthEventKind::PublicationIngested,
            GrowthEventKind::PublicationSkippedDuplicate,
            GrowthEventKind::TopicClassified,
            GrowthEventKind::TopicStateUpdated,
            GrowthEventKind::TrendComputed,
            GrowthEventKind::OpportunityDetected,
        ];
        for kind in kinds {
            assert_eq!(GrowthEventKind::from_str(kind.as_str()), Some(kind));
        }
    }

    #[test]
    fn kind_from_str_rejects_unknown() {
        assert_eq!(GrowthEventKind::from_str("unknown"), None);
        assert_eq!(GrowthEventKind::from_str(""), None);
        assert_eq!(GrowthEventKind::from_str("SOURCE_REGISTERED"), None);
    }

    #[test]
    fn event_id_is_unique_v4() {
        let a = GrowthEvent::new(
            GrowthEventKind::SourceRegistered,
            None,
            json!({}),
            Utc::now(),
        );
        let b = GrowthEvent::new(
            GrowthEventKind::SourceRegistered,
            None,
            json!({}),
            Utc::now(),
        );
        assert_ne!(a.id, b.id);
        assert_eq!(a.id.get_version_num(), 4);
        assert_eq!(b.id.get_version_num(), 4);
    }

    #[test]
    fn event_with_topic_serializes_topic() {
        let event = GrowthEvent::new(
            GrowthEventKind::TopicClassified,
            Some(Topic::Rust),
            json!({ "publication_id": "abc" }),
            Utc::now(),
        );
        let value = serde_json::to_value(&event).unwrap();
        assert_eq!(value["topic"], "rust");
        assert_eq!(value["kind"], "topic_classified");
    }

    #[test]
    fn event_without_topic_serializes_null() {
        let event = GrowthEvent::new(
            GrowthEventKind::SourceRegistered,
            None,
            json!({}),
            Utc::now(),
        );
        let value = serde_json::to_value(&event).unwrap();
        assert!(value["topic"].is_null());
    }
}
