//! Growth service — orchestration layer.
//!
//! Ties together the ingestion pipeline:
//!
//! ```text
//! Source → fetch → parse → classify → store Publication
//! ```
//!
//! The service is a thin coordinator. It does NOT:
//!
//! - implement HTTP (that is `ingestion::fetcher`);
//! - parse XML (that is `ingestion::rss`);
//! - classify topics (that is `analysis::classifier`);
//! - persist data directly (that is the storage traits);
//! - schedule runs (that is the caller / future scheduler);
//! - check `SourceStatus` (that is the caller / scheduler).
//!
//! # Source eligibility
//!
//! `ingest_source` assumes the caller has already decided the
//! source is eligible for ingestion. Lifecycle filtering
//! (`Active` vs `Paused` vs `Disabled`) belongs to the scheduler
//! or selection layer. This keeps the service a pure
//! orchestration layer with no policy.
//!
//! # Idempotency
//!
//! Publication identity is deterministic:
//! `PublicationId = SHA256("pub:" + source_id + ":" + external_id)`.
//! Re-ingesting the same feed does NOT create duplicates.
//!
//! # Immutability of existing publications
//!
//! Existing publications are treated as immutable ingestion
//! records. If a publication with the same identity already
//! exists, the service:
//!
//! - does NOT re-classify it;
//! - does NOT update `summary`, `url`, `author`, or `topics`;
//! - does NOT touch `ingested_at`.
//!
//! This is a deliberate contract. Feed updates are not re-applied
//! after the identity exists. When re-application becomes
//! necessary (e.g. for feed corrections), a separate "re-index"
//! flow with explicit semantics will be added.
//!
//! # Duplicate check is telemetry, not uniqueness
//!
//! The `publication_exists` check is a fast path that avoids
//! re-doing classification work and gives accurate
//! `skipped_duplicate` counters in the single-writer case.
//!
//! It is NOT a uniqueness guarantee. The storage layer is
//! idempotent by construction (deterministic `PublicationId`,
//! overwrite semantics). Under concurrent ingestion the counters
//! may over-report `inserted`; the stored state remains correct.
//!
//! # Determinism
//!
//! The classifier, parser, and storage layers are deterministic.
//! The fetch layer is not (HTTP is external). `ingested_at` is set
//! once per run via `Utc::now()` and is therefore not reproducible
//! across runs — but re-ingestion does not overwrite an existing
//! `ingested_at`.
//!
//! # Errors
//!
//! Fetch errors, parse errors, and storage errors propagate as
//! `ApiError`. A storage error mid-feed stops that feed's
//! ingestion; the caller may retry safely because re-ingestion is
//! idempotent.

use chrono::Utc;

use std::collections::HashMap;

use crate::error::ApiError;
use crate::growth::analysis::classifier;
use crate::growth::analysis::opportunities::{self, OpportunityEvidence};
use crate::growth::analysis::trends::{self, TrendInput};
use crate::growth::ingestion::fetcher::Fetcher;
use crate::growth::ingestion::rss;
use crate::growth::models::{
    Opportunity, Publication, PublicationId, Source, Topic, TopicTrend,
};
use crate::growth::storage::{OpportunityStorage, PublicationStorage, SourceStorage};

// ---------------------------------------------------------------------------
// Stats
// ---------------------------------------------------------------------------

/// Result of an analysis run.
///
/// Carries both the trend snapshot and the opportunities detected
/// from it, so that callers (CLI, HTTP API, scheduler) share the
/// same view of "what was computed in this pass".
#[derive(Debug, Clone)]
pub struct AnalysisResult {
    pub trends: Vec<TopicTrend>,
    pub opportunities: Vec<Opportunity>,
}

/// Maximum number of publications loaded per analysis run.
///
/// See the module docs for the Phase 1 limitation.
const ANALYSIS_MAX_PUBLICATIONS: usize = 10_000;

/// Summary of a single source ingestion run.
///
/// `skipped_invalid` is intentionally absent: entries skipped by
/// the parser (missing title, missing identity) are the parser's
/// responsibility and are not visible to the service.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngestStats {
    /// Number of entries the parser produced (already valid).
    pub parsed: usize,
    /// Number of entries that were not already in storage and were
    /// written by this run.
    pub inserted: usize,
    /// Number of entries that already existed in storage and were
    /// skipped without modification.
    pub skipped_duplicate: usize,
}

impl IngestStats {
    fn empty() -> Self {
        Self {
            parsed: 0,
            inserted: 0,
            skipped_duplicate: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Service
// ---------------------------------------------------------------------------

/// Growth ingestion service.
///
/// Generic over the storage backend. Production composes it with
/// `AevumDbGrowthStorage`; tests compose it with
/// `InMemoryGrowthStorage`.
pub struct GrowthService<S, F = Fetcher> {
    storage: S,
    fetcher: F,
}

impl<S> GrowthService<S, Fetcher>
where
    S: PublicationStorage + SourceStorage + Send + Sync,
{
    /// Build a service with the default fetcher.
    pub fn new(storage: S) -> Result<Self, ApiError> {
        let fetcher = Fetcher::new()?;
        Ok(Self { storage, fetcher })
    }
}

impl<S, F> GrowthService<S, F>
where
    S: PublicationStorage + SourceStorage + Send + Sync,
    F: Fetch,
{
    /// Build a service with an explicit fetcher.
    ///
    /// Used by tests that want to stub the network.
    pub fn with_fetcher(storage: S, fetcher: F) -> Self {
        Self { storage, fetcher }
    }

    /// Ingest a single source.
    ///
    /// Pipeline:
    ///
    /// 1. fetch the feed URL as a string;
    /// 2. parse it into `ParsedPublication` values;
    /// 3. for each parsed entry:
    ///    a. compute the deterministic `PublicationId`;
    ///    b. skip if `publication_exists` (existing records are
    ///       immutable — see module docs);
    ///    c. classify topics (base from source + detected from text);
    ///    d. compose `Publication` and store it.
    ///
    /// The source is not modified by this call. Its `last_checked`
    /// / `last_success` fields are owned by a future scheduler.
    pub async fn ingest_source(&self, source: &Source) -> Result<IngestStats, ApiError> {
        let feed_url = source.feed_url.as_str();
        let xml = self.fetcher.fetch(feed_url).await?;
        let parsed = rss::parse(&xml)?;

        let mut stats = IngestStats::empty();
        stats.parsed = parsed.len();
        let now = Utc::now();

        for entry in parsed {
            let id = PublicationId::from_parts(source.id, &entry.external_id);

            if self
                .storage
                .publication_exists(source.id, &entry.external_id)
                .await?
            {
                stats.skipped_duplicate += 1;
                continue;
            }

            let topics = classifier::classify(
                &entry.title,
                entry.summary.as_deref(),
                &source.topics,
            )
            .topics;

            if topics.is_empty() {
                log::warn!(
                    "Growth service: publication {} has no topics",
                    entry.external_id
                );
            }

            let url = entry.url.and_then(|raw| match url::Url::parse(&raw) {
                Ok(u) => Some(u),
                Err(err) => {
                    log::warn!(
                        "Growth service: invalid publication URL {:?}: {}",
                        raw,
                        err
                    );
                    None
                }
            });

            let publication = Publication {
                id,
                source_id: source.id,
                external_id: entry.external_id,
                url,
                title: entry.title,
                summary: entry.summary,
                author: entry.author,
                language: None,
                topics,
                published_at: entry.published_at,
                ingested_at: now,
            };

            self.storage.put_publication(&publication).await?;
            stats.inserted += 1;
        }

        Ok(stats)
    }

    /// Access the underlying storage.
    pub fn storage(&self) -> &S {
        &self.storage
    }
}

impl<S, F> GrowthService<S, F>
where
    S: PublicationStorage + SourceStorage + OpportunityStorage + Send + Sync,
    F: Fetch,
{
    /// Load trends and detect opportunities with evidence.
    ///
    /// This is the single analysis entrypoint used by the CLI, the
    /// HTTP API, and (future) the scheduler.
    ///
    /// # Evidence
    ///
    /// Each detected opportunity carries the IDs of the publications
    /// that contributed to the signal. Evidence is looked up per
    /// topic from the most recent publications. Topics that do not
    /// fire any signal are never looked up.
    ///
    /// # Phase 1 limitation
    ///
    /// Loads at most `ANALYSIS_MAX_PUBLICATIONS` recent publications.
    /// Correctness-neutral for small datasets, incomplete at scale.
    /// The correct approach is a time-bounded scan `[now - 30d, now]`
    /// (AevumDB Tier-1 primitive, tracked in module docs).
    pub async fn analyze_opportunities(
        &self,
    ) -> Result<AnalysisResult, ApiError> {
        // 1. Load recent publications once.
        let publications = self
            .storage
            .list_recent_publications(ANALYSIS_MAX_PUBLICATIONS)
            .await?;

        // 2. Build trend inputs.
        let inputs: Vec<TrendInput> = publications
            .iter()
            .map(|p| TrendInput {
                topics: p.topics.clone(),
                effective_ts: p.published_at.unwrap_or(p.ingested_at),
            })
            .collect();

        // 3. Compute trends.
        let now = Utc::now();
        let trends = trends::compute_all(&inputs, now);

        // 4. Build evidence map: topic -> publications within the
        //    signal window.
        //
        // The acceleration and emerging detectors both consider
        // only publications within the last 30 days. We therefore
        // restrict evidence to the same window. This keeps
        // `Opportunity::evidence` consistent with the trend counts
        // that produced the signal.
        //
        // Phase 1 uses 30 days as the widest signal window. If a
        // future detector uses a different window, this filter must
        // be extended (e.g. per-kind evidence maps).
        let now = Utc::now();
        let evidence_cutoff = now - chrono::Duration::days(30);

        let mut evidence_map: HashMap<Topic, Vec<OpportunityEvidence>> =
            HashMap::new();
        for p in &publications {
            let effective_ts = p.published_at.unwrap_or(p.ingested_at);
            if effective_ts < evidence_cutoff {
                continue;
            }
            for topic in &p.topics {
                evidence_map
                    .entry(*topic)
                    .or_default()
                    .push(OpportunityEvidence {
                        publication_id: Some(p.id),
                        effective_ts,
                    });
            }
        }

        // 5. Detect opportunities, passing real evidence.
        let opportunities = opportunities::detect_all(&trends, now, |topic| {
            evidence_map.get(&topic).cloned().unwrap_or_default()
        });

        Ok(AnalysisResult {
            trends,
            opportunities,
        })
    }
}

// ---------------------------------------------------------------------------
// Fetch trait (for test stubbing)
// ---------------------------------------------------------------------------

/// Abstraction over "give me the bytes at this URL".
///
/// Implemented by `ingestion::fetcher::Fetcher` for production, and
/// by test doubles in unit tests.
pub trait Fetch: Send + Sync {
    fn fetch(
        &self,
        url: &str,
    ) -> impl std::future::Future<Output = Result<String, ApiError>> + Send;
}

impl Fetch for Fetcher {
    async fn fetch(&self, url: &str) -> Result<String, ApiError> {
        Fetcher::fetch(self, url).await
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    use chrono::Utc;
    use std::sync::Mutex;
    use url::Url;

    use crate::growth::in_memory::InMemoryGrowthStorage;
    use crate::growth::models::{Platform, Topic};
    use crate::growth::storage::PublicationStorage;

    struct StubFetcher {
        body: Mutex<Option<String>>,
        calls: Mutex<u32>,
    }

    impl StubFetcher {
        fn ok(body: &str) -> Self {
            Self {
                body: Mutex::new(Some(body.to_owned())),
                calls: Mutex::new(0),
            }
        }

        fn err() -> Self {
            Self {
                body: Mutex::new(None),
                calls: Mutex::new(0),
            }
        }

        fn calls(&self) -> u32 {
            *self.calls.lock().unwrap()
        }
    }

    impl Fetch for StubFetcher {
        async fn fetch(&self, _url: &str) -> Result<String, ApiError> {
            *self.calls.lock().unwrap() += 1;
            match self.body.lock().unwrap().clone() {
                Some(b) => Ok(b),
                None => Err(ApiError::ValidationFailed {
                    code: "GROWTH_FETCH_TRANSPORT",
                    message: "Feed fetch failed (transport)",
                }),
            }
        }
    }

    fn make_source(topics: Vec<Topic>) -> Source {
        Source::new(
            Platform::Rss,
            Url::parse("https://example.com/feed.xml").unwrap(),
            None,
            topics,
            Utc::now(),
        )
    }

    const RSS_ONE_ITEM: &str = r#"<?xml version="1.0"?>
<rss version="2.0"><channel>
  <item>
    <title>Rust 1.99 with LSM compaction</title>
    <link>https://example.com/1</link>
    <guid>https://example.com/1</guid>
    <pubDate>Wed, 01 Oct 2026 00:00:00 +0000</pubDate>
  </item>
</channel></rss>"#;

    #[tokio::test]
    async fn ingest_stores_new_publication() {
        let storage = InMemoryGrowthStorage::new();
        let fetcher = StubFetcher::ok(RSS_ONE_ITEM);
        let service = GrowthService::with_fetcher(storage, fetcher);

        let source = make_source(vec![Topic::Rust]);
        let stats = service.ingest_source(&source).await.unwrap();

        assert_eq!(stats.parsed, 1);
        assert_eq!(stats.inserted, 1);
        assert_eq!(stats.skipped_duplicate, 0);
    }

    #[tokio::test]
    async fn ingest_is_idempotent() {
        let storage = InMemoryGrowthStorage::new();
        let fetcher = StubFetcher::ok(RSS_ONE_ITEM);
        let service = GrowthService::with_fetcher(storage, fetcher);

        let source = make_source(vec![Topic::Rust]);
        let first = service.ingest_source(&source).await.unwrap();
        let second = service.ingest_source(&source).await.unwrap();

        assert_eq!(first.inserted, 1);
        assert_eq!(second.inserted, 0);
        assert_eq!(second.skipped_duplicate, 1);
    }

    #[tokio::test]
    async fn existing_publication_is_not_reclassified() {
        // Re-ingesting the SAME source must skip the existing
        // publication without touching its fields.
        let storage = InMemoryGrowthStorage::new();
        let source = make_source(vec![Topic::PostQuantum]);

        let fetcher = StubFetcher::ok(RSS_ONE_ITEM);
        let service = GrowthService::with_fetcher(storage, fetcher);

        service.ingest_source(&source).await.unwrap();
        let before = service
            .storage()
            .list_recent_publications(10)
            .await
            .unwrap();
        assert_eq!(before.len(), 1);
        let first_topics = before[0].topics.clone();
        let first_ingested_at = before[0].ingested_at;

        let stats = service.ingest_source(&source).await.unwrap();
        assert_eq!(stats.skipped_duplicate, 1);
        assert_eq!(stats.inserted, 0);

        let after = service
            .storage()
            .list_recent_publications(10)
            .await
            .unwrap();
        assert_eq!(after.len(), 1);
        assert_eq!(after[0].topics, first_topics);
        assert_eq!(after[0].ingested_at, first_ingested_at);
    }

    #[tokio::test]
    async fn classify_uses_base_topics() {
        // RSS_ONE_ITEM text: "Rust 1.99 with LSM compaction"
        //   base from source:  [PostQuantum]
        //   detected by text:  [StorageSystems]  (lsm + compaction)
        //   Rust is NOT detected: only "rust" matches (1 < MIN_MATCHES=2)
        let storage = InMemoryGrowthStorage::new();
        let fetcher = StubFetcher::ok(RSS_ONE_ITEM);
        let service = GrowthService::with_fetcher(storage, fetcher);

        let source = make_source(vec![Topic::PostQuantum]);
        service.ingest_source(&source).await.unwrap();

        let pubs = service
            .storage()
            .list_recent_publications(10)
            .await
            .unwrap();
        assert_eq!(pubs.len(), 1);
        assert!(pubs[0].topics.contains(&Topic::PostQuantum));
        assert!(pubs[0].topics.contains(&Topic::StorageSystems));
    }

    const RSS_RUST_ITEM: &str = r#"<?xml version="1.0"?>
<rss version="2.0"><channel>
  <item>
    <title>Cargo team ships Rust 1.99</title>
    <link>https://example.com/rust</link>
    <guid>https://example.com/rust</guid>
  </item>
</channel></rss>"#;

    #[tokio::test]
    async fn classify_detects_rust_from_text() {
        // base = GpuCompute (irrelevant), text should detect Rust.
        // "cargo" + "rust" = 2 matches → Rust detected.
        let storage = InMemoryGrowthStorage::new();
        let fetcher = StubFetcher::ok(RSS_RUST_ITEM);
        let service = GrowthService::with_fetcher(storage, fetcher);

        let source = make_source(vec![Topic::GpuCompute]);
        service.ingest_source(&source).await.unwrap();

        let pubs = service
            .storage()
            .list_recent_publications(10)
            .await
            .unwrap();
        assert_eq!(pubs.len(), 1);
        assert!(pubs[0].topics.contains(&Topic::GpuCompute));
        assert!(pubs[0].topics.contains(&Topic::Rust));
    }

    #[tokio::test]
    async fn fetch_error_propagates() {
        let storage = InMemoryGrowthStorage::new();
        let fetcher = StubFetcher::err();
        let service = GrowthService::with_fetcher(storage, fetcher);

        let source = make_source(vec![Topic::Rust]);
        assert!(service.ingest_source(&source).await.is_err());
    }

    #[tokio::test]
    async fn fetch_called_once_per_ingest() {
        let storage = InMemoryGrowthStorage::new();
        let fetcher = StubFetcher::ok(RSS_ONE_ITEM);
        let service = GrowthService::with_fetcher(storage, fetcher);

        let source = make_source(vec![Topic::Rust]);
        let _ = service.ingest_source(&source).await.unwrap();
        let _ = service.ingest_source(&source).await.unwrap();

        assert_eq!(service.fetcher.calls(), 2);
    }

    #[tokio::test]
    async fn empty_feed_produces_zero_stats() {
        let storage = InMemoryGrowthStorage::new();
        let xml = r#"<?xml version="1.0"?>
<rss version="2.0"><channel><title>x</title></channel></rss>"#;
        let fetcher = StubFetcher::ok(xml);
        let service = GrowthService::with_fetcher(storage, fetcher);

        let source = make_source(vec![Topic::Rust]);
        let stats = service.ingest_source(&source).await.unwrap();
        assert_eq!(stats.parsed, 0);
        assert_eq!(stats.inserted, 0);
        assert_eq!(stats.skipped_duplicate, 0);
    }

    #[tokio::test]
    async fn malformed_feed_returns_error() {
        let storage = InMemoryGrowthStorage::new();
        let fetcher = StubFetcher::ok("<rss><unclosed>");
        let service = GrowthService::with_fetcher(storage, fetcher);

        let source = make_source(vec![Topic::Rust]);
        assert!(service.ingest_source(&source).await.is_err());
    }

    #[tokio::test]
    async fn entry_with_invalid_url_is_stored_with_none() {
        let storage = InMemoryGrowthStorage::new();
        let xml = r#"<?xml version="1.0"?>
<rss version="2.0"><channel>
  <item>
    <title>Broken URL</title>
    <link>not a url</link>
    <guid>broken-1</guid>
  </item>
</channel></rss>"#;
        let fetcher = StubFetcher::ok(xml);
        let service = GrowthService::with_fetcher(storage, fetcher);

        let source = make_source(vec![Topic::Rust]);
        let stats = service.ingest_source(&source).await.unwrap();
        assert_eq!(stats.inserted, 1);

        let pubs = service
            .storage()
            .list_recent_publications(10)
            .await
            .unwrap();
        assert_eq!(pubs.len(), 1);
        assert!(pubs[0].url.is_none());
    }

    #[tokio::test]
    async fn analyze_opportunities_returns_evidence() {
        // Ingest several publications for a topic, then run
        // analyze_opportunities and verify that each detected
        // opportunity carries non-empty evidence.
        let storage = InMemoryGrowthStorage::new();
        let fetcher = StubFetcher::ok(RSS_ONE_ITEM);
        let service = GrowthService::with_fetcher(storage, fetcher);

        // Base topic = StorageSystems so RSS_ONE_ITEM
        // ("Rust 1.99 with LSM compaction") classifies cleanly.
        let source = make_source(vec![Topic::StorageSystems]);
        service.ingest_source(&source).await.unwrap();

        let result = service.analyze_opportunities().await.unwrap();

        // Any detected opportunity MUST carry evidence.
        for opp in &result.opportunities {
            assert!(
                !opp.evidence.is_empty(),
                "opportunity for topic {:?} has empty evidence",
                opp.topic,
            );
        }

        // We also expect at least one trend to be present.
        assert!(!result.trends.is_empty());
    }
}
