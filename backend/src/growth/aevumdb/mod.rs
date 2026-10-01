//! AevumDB-backed Growth storage.
//!
//! Single production struct (`AevumDbGrowthStorage`) implementing all
//! five Growth storage traits:
//!
//! - `SourceStorage`
//! - `PublicationStorage`
//! - `OpportunityStorage`
//! - `TopicStateStorage`
//! - `GrowthEventStorage`
//!
//! # Contract
//!
//! - All methods are `async` and return `Result<_, ApiError>`.
//! - "Not found" is expressed as `Ok(None)`, not as an error.
//! - Multi-key writes use a single `db.batch()`.
//! - Events are append-only; `prune_events_before` is the only
//!   removal path.
//!
//! # Encoding invariant
//!
//! Entity modules know the **shape** of their keys but NOT the
//! encoding of timestamps or scores. All such encoding lives in
//! this module:
//!
//! - `encode_ts_micros` — ascending timestamp component
//! - `encode_inv_ts_micros` — descending timestamp component
//! - `encode_inv_score` — descending score component
//! - `effective_timestamp` — published_at ?? ingested_at
//!
//! Identifier serialization uses the canonical domain ID
//! representation (hex for deterministic IDs, UUID for the rest).
//! No Growth-specific encoding is applied to identifiers.
//!
//! See `docs/architecture/growth-storage-design-v1.md` for the full
//! key layout contract.

use std::sync::Arc;

use aevum_db::{AevumDb, DbConfig, DbError, DbRuntime};
use chrono::{DateTime, Utc};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::error::ApiError;

// Entity modules are declared as they are implemented.
pub mod topic_state;

// ---------------------------------------------------------------------------
// Storage struct
// ---------------------------------------------------------------------------

/// Production storage adapter for the Growth domain, backed by AevumDB.
///
/// A single instance implements all five Growth storage traits.
/// Cloning is cheap (`Arc<AevumDb>` inside).
#[derive(Clone)]
pub struct AevumDbGrowthStorage {
    db: Arc<AevumDb>,
}

impl std::fmt::Debug for AevumDbGrowthStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AevumDbGrowthStorage")
            .field("db", &"<redacted>")
            .finish()
    }
}

impl AevumDbGrowthStorage {
    /// Open a new AevumDB instance and wrap it as Growth storage.
    ///
    /// Used by production composition. The caller owns the config
    /// and runtime.
    pub fn open(config: DbConfig, runtime: DbRuntime) -> Result<Self, ApiError> {
        let db = AevumDb::open(config, runtime).map_err(|error| {
            log::error!("Growth: AevumDB open failed: {}", error);
            ApiError::Internal
        })?;
        Ok(Self { db: Arc::new(db) })
    }

    /// Wrap an existing `Arc<AevumDb>` as Growth storage.
    ///
    /// Used by tests and by callers that already own the DB handle
    /// (e.g. shared with other subsystems in the same process).
    pub fn from_arc(db: Arc<AevumDb>) -> Self {
        Self { db }
    }

    /// Access the underlying AevumDB handle.
    ///
    /// Intended for integration and testing. Business logic MUST NOT
    /// use this to bypass the storage traits.
    pub fn db(&self) -> &AevumDb {
        &self.db
    }
}

// ---------------------------------------------------------------------------
// Encoding helpers (single source of truth for key encoding)
// ---------------------------------------------------------------------------

/// Encode a timestamp for ASC ordering.
///
/// Unit: microseconds since Unix epoch, signed (`i64`).
///
/// The signed value is mapped to a monotone `u64` by flipping the
/// sign bit: `(micros as u64) ^ (1 << 63)`. For `ts_a < ts_b` the
/// mapped values satisfy `mapped_a < mapped_b`, so lexicographic
/// order of the encoded strings matches chronological order across
/// the entire `i64` range (including pre-epoch timestamps).
///
/// Format: 20-digit zero-padded decimal.
pub(crate) fn encode_ts_micros(ts: DateTime<Utc>) -> String {
    let micros = ts.timestamp_micros();
    let ordered = (micros as u64) ^ (1u64 << 63);
    format!("{:020}", ordered)
}

/// Encode a timestamp for DESC ordering.
///
/// Uses the same monotone mapping as `encode_ts_micros`, then
/// inverts it: `u64::MAX - ordered`. For `ts_a < ts_b`,
/// `encode_inv_ts_micros(ts_a) > encode_inv_ts_micros(ts_b)`.
pub(crate) fn encode_inv_ts_micros(ts: DateTime<Utc>) -> String {
    let micros = ts.timestamp_micros();
    let ordered = (micros as u64) ^ (1u64 << 63);
    let inverted = u64::MAX - ordered;
    format!("{:020}", inverted)
}

/// Encode a score for DESC ordering.
///
/// `score_bp` is in basis points, `0..=10_000`.
/// The inverted value is computed in `u64` space to guarantee a
/// uniform 20-digit width.
pub(crate) fn encode_inv_score(score_bp: u32) -> String {
    let inv = u64::MAX - (score_bp as u64);
    format!("{:020}", inv)
}

/// Effective timestamp for indexing.
///
/// Publications have an informational `published_at` (may be absent)
/// and a mandatory `ingested_at`. The index uses:
///
/// ```text
/// effective_ts = published_at.unwrap_or(ingested_at)
/// ```
pub(crate) fn effective_timestamp(
    published_at: Option<DateTime<Utc>>,
    ingested_at: DateTime<Utc>,
) -> DateTime<Utc> {
    published_at.unwrap_or(ingested_at)
}

// ---------------------------------------------------------------------------
// Serialization helpers
// ---------------------------------------------------------------------------

pub(crate) fn serialize<T: Serialize>(value: &T) -> Result<Vec<u8>, ApiError> {
    serde_json::to_vec(value).map_err(|error| {
        log::error!("Growth: serialize failed: {}", error);
        ApiError::Internal
    })
}

pub(crate) fn deserialize<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, ApiError> {
    serde_json::from_slice(bytes).map_err(|error| {
        log::error!("Growth: deserialize failed: {}", error);
        ApiError::Internal
    })
}

pub(crate) fn map_db_error(error: DbError) -> ApiError {
    log::error!("Growth: AevumDB error: {}", error);
    ApiError::Internal
}

// ---------------------------------------------------------------------------
// Key prefixes
//
// All Growth keys begin with `growth:`. The prefixes below are the
// only canonical forms. Entity modules MUST use these constants.
// ---------------------------------------------------------------------------

// Source
pub(crate) const SOURCE_PREFIX: &str = "growth:source:";
pub(crate) const SOURCE_BY_HANDLE_PREFIX: &str = "growth:source:by_handle:";
pub(crate) const SOURCE_BY_TOPIC_PREFIX: &str = "growth:source:by_topic:";
pub(crate) const SOURCE_BY_STATUS_PREFIX: &str = "growth:source:by_status:";

// Publication
pub(crate) const PUBLICATION_PREFIX: &str = "growth:publication:";
pub(crate) const PUBLICATION_BY_SOURCE_PREFIX: &str = "growth:publication:by_source:";
pub(crate) const PUBLICATION_BY_TOPIC_PREFIX: &str = "growth:publication:by_topic:";
pub(crate) const PUBLICATION_BY_TIME_PREFIX: &str = "growth:publication:by_time:";
pub(crate) const PUBLICATION_BY_EXTERNAL_PREFIX: &str = "growth:publication:by_external:";

// Opportunity
pub(crate) const OPPORTUNITY_PREFIX: &str = "growth:opportunity:";
pub(crate) const OPPORTUNITY_BY_TOPIC_PREFIX: &str = "growth:opportunity:by_topic:";
pub(crate) const OPPORTUNITY_BY_KIND_PREFIX: &str = "growth:opportunity:by_kind:";
pub(crate) const OPPORTUNITY_BY_SCORE_PREFIX: &str = "growth:opportunity:by_score:";

// TopicState
pub(crate) const TOPIC_STATE_PREFIX: &str = "growth:topic:state:";

// GrowthEvent
pub(crate) const EVENT_PREFIX: &str = "growth:event:event:";
pub(crate) const EVENT_TIMELINE_PREFIX: &str = "growth:event:timeline:";
pub(crate) const EVENT_BY_KIND_PREFIX: &str = "growth:event:by_kind:";
pub(crate) const EVENT_BY_TOPIC_PREFIX: &str = "growth:event:by_topic:";
