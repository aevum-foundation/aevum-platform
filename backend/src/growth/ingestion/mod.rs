//! Growth feed ingestion.
//!
//! This module transforms raw feed bytes into normalized
//! `ParsedPublication` values. It does NOT perform HTTP, storage,
//! classification, or scheduling.
//!
//! Contract: see `growth-feed-audit-v1.md`.
//!
//! Layers:
//!
//! ```text
//! ingestion/fetcher.rs  →  bytes (over HTTP)
//! ingestion/rss.rs      →  Vec<ParsedPublication>
//! ingestion/mod.rs      →  shared types (this file)
//! ```

pub mod rss;

use chrono::{DateTime, Utc};

use crate::error::ApiError;

// ---------------------------------------------------------------------------
// Output contract
// ---------------------------------------------------------------------------

/// Normalized feed entry, produced by `parse()`.
///
/// The parser does not fill `source_id`, `id`, `topics`, or
/// `ingested_at`. Those are composed by the ingestion service.
pub struct ParsedPublication {
    pub external_id: String,
    pub url: Option<String>,
    pub title: String,
    pub summary: Option<String>,
    pub author: Option<String>,
    pub published_at: Option<DateTime<Utc>>,
}

// ---------------------------------------------------------------------------
// Feed type
// ---------------------------------------------------------------------------

/// Type of the source feed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedType {
    Rss2,
    Atom,
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Parse an RSS 2.0 or Atom XML document.
///
/// Returns the list of successfully parsed entries.
///
/// # Errors
///
/// Feed-level errors (malformed XML, wrong root element) return
/// `Err(ApiError)`. Entry-level errors (missing title, missing
/// identity) cause the affected entry to be skipped and logged at
/// `warn` level; the parser continues with the next entry.
pub fn parse(xml: &str) -> Result<Vec<ParsedPublication>, ApiError> {
    rss::parse(xml)
}
