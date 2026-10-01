//! Growth domain models.
//!
//! Entities: Source, Publication, Topic, TopicState, TopicTrend,
//! Opportunity.
//!
//! # Identifier contract
//!
//! `SourceId` and `PublicationId` are deterministic. Callers MUST normalize
//! inputs before computing them:
//!
//! - `SourceId::from_feed` expects a canonical feed URL.
//! - `PublicationId::from_parts` expects a canonical `external_id`.
//!
//! The normalization rules are owned by the ingestion layer, not by this
//! module. This module only guarantees that the same normalized input
//! produces the same identifier.
//!
//! # Invariants
//!
//! Range and consistency invariants (for example `score_bp <= 10_000`) are
//! enforced by `validation`, not by the domain types themselves.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;
use url::Url;

// ---------------------------------------------------------------------------
// Identifiers
// ---------------------------------------------------------------------------

/// Deterministic identifier for a `Source`.
///
/// `feed_url` must already be normalized by the caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SourceId([u8; 16]);

impl SourceId {
    pub fn from_feed(platform: Platform, feed_url: &Url) -> Self {
        let mut h = Sha256::new();
        h.update(b"source:");
        h.update(platform.as_str().as_bytes());
        h.update(b":");
        h.update(feed_url.as_str().as_bytes());
        let digest = h.finalize();
        let mut out = [0u8; 16];
        out.copy_from_slice(&digest[..16]);
        Self(out)
    }

    pub fn as_hex(&self) -> String {
        hex::encode(self.0)
    }

    /// Construct a `SourceId` from its canonical 32-character
    /// lowercase hex encoding.
    ///
    /// Returns `None` if the input is not exactly 32 hex characters
    /// or does not decode to 16 bytes.
    pub fn from_hex(s: &str) -> Option<Self> {
        if s.len() != 32 {
            return None;
        }
        let bytes = hex::decode(s).ok()?;
        if bytes.len() != 16 {
            return None;
        }
        let mut out = [0u8; 16];
        out.copy_from_slice(&bytes);
        Some(Self(out))
    }
}

impl fmt::Display for SourceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.as_hex())
    }
}

/// Deterministic identifier for a `Publication`.
///
/// `external_id` must already be normalized by the caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PublicationId([u8; 16]);

impl PublicationId {
    pub fn from_parts(source: SourceId, external_id: &str) -> Self {
        let mut h = Sha256::new();
        h.update(b"pub:");
        h.update(source.0);
        h.update(b":");
        h.update(external_id.as_bytes());
        let digest = h.finalize();
        let mut out = [0u8; 16];
        out.copy_from_slice(&digest[..16]);
        Self(out)
    }

    pub fn as_hex(&self) -> String {
        hex::encode(self.0)
    }

    /// Construct a `PublicationId` from its canonical 32-character
    /// lowercase hex encoding.
    ///
    /// Returns `None` if the input is not exactly 32 hex characters
    /// or does not decode to 16 bytes.
    pub fn from_hex(s: &str) -> Option<Self> {
        if s.len() != 32 {
            return None;
        }
        let bytes = hex::decode(s).ok()?;
        if bytes.len() != 16 {
            return None;
        }
        let mut out = [0u8; 16];
        out.copy_from_slice(&bytes);
        Some(Self(out))
    }
}

impl fmt::Display for PublicationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.as_hex())
    }
}

/// Non-deterministic identifier for an `Opportunity`.
///
/// Each detection is unique even if the same signal appears again later.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OpportunityId(pub uuid::Uuid);

impl OpportunityId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

impl Default for OpportunityId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for OpportunityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Platform {
    Rss,
    Atom,
}

impl Platform {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Rss => "rss",
            Self::Atom => "atom",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "rss" => Some(Self::Rss),
            "atom" => Some(Self::Atom),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceStatus {
    Active,
    Paused,
    Error,
    Disabled,
}

impl SourceStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Paused => "paused",
            Self::Error => "error",
            Self::Disabled => "disabled",
        }
    }
}

impl Default for SourceStatus {
    fn default() -> Self {
        Self::Active
    }
}

/// Fixed set of Phase 1 verticals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Topic {
    PostQuantum,
    DistributedSystems,
    Rust,
    BlockchainArchitecture,
    GpuCompute,
    StorageSystems,
}

impl Topic {
    pub const ALL: [Topic; 6] = [
        Topic::PostQuantum,
        Topic::DistributedSystems,
        Topic::Rust,
        Topic::BlockchainArchitecture,
        Topic::GpuCompute,
        Topic::StorageSystems,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PostQuantum => "post_quantum",
            Self::DistributedSystems => "distributed_systems",
            Self::Rust => "rust",
            Self::BlockchainArchitecture => "blockchain_architecture",
            Self::GpuCompute => "gpu_compute",
            Self::StorageSystems => "storage_systems",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "post_quantum" => Some(Self::PostQuantum),
            "distributed_systems" => Some(Self::DistributedSystems),
            "rust" => Some(Self::Rust),
            "blockchain_architecture" => Some(Self::BlockchainArchitecture),
            "gpu_compute" => Some(Self::GpuCompute),
            "storage_systems" => Some(Self::StorageSystems),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Entities
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    pub id: SourceId,
    pub platform: Platform,
    pub feed_url: Url,
    pub homepage: Option<Url>,
    pub title: Option<String>,
    pub topics: Vec<Topic>,
    pub status: SourceStatus,
    pub consecutive_failures: u32,
    pub last_checked: Option<DateTime<Utc>>,
    pub last_success: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Source {
    pub fn new(
        platform: Platform,
        feed_url: Url,
        homepage: Option<Url>,
        topics: Vec<Topic>,
        now: DateTime<Utc>,
    ) -> Self {
        let id = SourceId::from_feed(platform, &feed_url);
        Self {
            id,
            platform,
            feed_url,
            homepage,
            title: None,
            topics,
            status: SourceStatus::Active,
            consecutive_failures: 0,
            last_checked: None,
            last_success: None,
            last_error: None,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Publication {
    pub id: PublicationId,
    pub source_id: SourceId,
    pub external_id: String,
    pub url: Option<Url>,
    pub title: String,
    pub summary: Option<String>,
    pub author: Option<String>,
    pub language: Option<String>,
    pub topics: Vec<Topic>,
    pub published_at: Option<DateTime<Utc>>,
    pub ingested_at: DateTime<Utc>,
}

/// Aggregate snapshot of a single topic.
///
/// Persisted at `growth:topic:state:{topic}`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicState {
    pub topic: Topic,
    pub publication_count: u64,
    pub source_count: u64,
    pub first_seen_at: Option<DateTime<Utc>>,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

impl TopicState {
    pub fn empty(topic: Topic, now: DateTime<Utc>) -> Self {
        Self {
            topic,
            publication_count: 0,
            source_count: 0,
            first_seen_at: None,
            last_seen_at: None,
            updated_at: now,
        }
    }
}

/// Integer-based trend snapshot for a topic.
///
/// Ratio is stored in basis points (1 bp = 0.0001).
/// Example: ratio 2.0 -> `ratio_7d_vs_30d_bp = 20_000`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicTrend {
    pub topic: Topic,
    pub count_24h: u32,
    pub count_7d: u32,
    pub count_30d: u32,
    pub ratio_7d_vs_30d_bp: u32,
    pub computed_at: DateTime<Utc>,
}

/// Analytical signal detected by the Trend Engine.
///
/// An `Opportunity` is NOT an automatic trigger for any action.
/// It is a marker for later human/content analysis.
///
/// `score_bp` is expected to be in `0..=10_000`, enforced by `validation`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Opportunity {
    pub id: OpportunityId,
    pub kind: OpportunityKind,
    pub topic: Topic,
    pub score_bp: u32,
    pub evidence: Vec<PublicationId>,
    pub detected_at: DateTime<Utc>,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpportunityKind {
    /// `ratio_7d_vs_30d_bp` above the configured threshold.
    TopicAccelerating,
    /// `count_24h > 0` and `count_30d` below the emerging threshold.
    TopicEmerging,
    /// `count_24h` far above the trailing average.
    TopicPeak,
    /// A single source produced an unusually high number of items.
    SourceSurge,
}

impl OpportunityKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TopicAccelerating => "topic_accelerating",
            Self::TopicEmerging => "topic_emerging",
            Self::TopicPeak => "topic_peak",
            Self::SourceSurge => "source_surge",
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_id_is_deterministic() {
        let url = Url::parse("https://example.com/feed.xml").unwrap();
        let a = SourceId::from_feed(Platform::Rss, &url);
        let b = SourceId::from_feed(Platform::Rss, &url);
        assert_eq!(a, b);
    }

    #[test]
    fn source_id_differs_by_platform() {
        let url = Url::parse("https://example.com/feed.xml").unwrap();
        let a = SourceId::from_feed(Platform::Rss, &url);
        let b = SourceId::from_feed(Platform::Atom, &url);
        assert_ne!(a, b);
    }

    #[test]
    fn publication_id_is_deterministic() {
        let url = Url::parse("https://example.com/feed.xml").unwrap();
        let src = SourceId::from_feed(Platform::Rss, &url);
        let a = PublicationId::from_parts(src, "guid-1");
        let b = PublicationId::from_parts(src, "guid-1");
        assert_eq!(a, b);
    }

    #[test]
    fn topic_round_trips() {
        for topic in Topic::ALL {
            assert_eq!(Topic::from_str(topic.as_str()), Some(topic));
        }
    }

    #[test]
    fn topic_from_str_rejects_unknown() {
        assert_eq!(Topic::from_str("unknown"), None);
        assert_eq!(Topic::from_str(""), None);
        assert_eq!(Topic::from_str("POST_QUANTUM"), None);
    }

    #[test]
    fn platform_round_trips() {
        for p in [Platform::Rss, Platform::Atom] {
            assert_eq!(Platform::from_str(p.as_str()), Some(p));
        }
    }

    #[test]
    fn source_status_default_is_active() {
        assert_eq!(SourceStatus::default(), SourceStatus::Active);
    }

    #[test]
    fn topic_state_empty_has_zero_counts() {
        let s = TopicState::empty(Topic::Rust, Utc::now());
        assert_eq!(s.publication_count, 0);
        assert_eq!(s.source_count, 0);
        assert!(s.first_seen_at.is_none());
        assert!(s.last_seen_at.is_none());
    }
}
