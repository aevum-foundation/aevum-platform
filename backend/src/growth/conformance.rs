//! Conformance tests: `InMemoryGrowthStorage` vs `AevumDbGrowthStorage`.
//!
//! Every scenario in this file runs the same sequence of operations
//! against both backends and asserts that the observable results
//! are identical.
//!
//! # Purpose
//!
//! Guarantee that the two backends are interchangeable from the
//! point of view of the five Growth storage traits. If they ever
//! diverge, at least one test in this file MUST fail.
//!
//! # Projections
//!
//! Raw records contain generated values (UUIDs, timestamps) that
//! are not comparable across backends. Comparisons use projections:
//! only stable, deterministic fields participate.
//!
//! # Ordering
//!
//! Results are compared in the order returned by the backend.
//! Sorting the projections before comparison would hide ordering
//! divergences, which are exactly what we want to catch.
//!
//! See `docs/architecture/growth-storage-design-v1.md` section 20.

#![cfg(test)]

use chrono::{DateTime, Duration, Utc};
use serde_json::json;
use tempfile::TempDir;
use url::Url;
use uuid::Uuid;

use crate::growth::aevumdb::AevumDbGrowthStorage;
use crate::growth::events::{GrowthEvent, GrowthEventKind};
use crate::growth::in_memory::InMemoryGrowthStorage;
use crate::growth::models::{
    Opportunity, OpportunityId, OpportunityKind, Platform, Publication, PublicationId, Source,
    SourceId, SourceStatus, Topic, TopicState,
};
use crate::growth::storage::{
    GrowthEventStorage, GrowthStorage, OpportunityStorage, PublicationStorage, SourceStorage,
    TopicStateStorage,
};

// ---------------------------------------------------------------------------
// Fixed identifiers
// ---------------------------------------------------------------------------

const EV_ID_A: &str = "11111111-1111-4111-8111-111111111111";
const EV_ID_B: &str = "22222222-2222-4222-8222-222222222222";
const EV_ID_C: &str = "33333333-3333-4333-8333-333333333333";

const OP_ID_A: &str = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa";
const OP_ID_B: &str = "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb";
const OP_ID_C: &str = "cccccccc-cccc-4ccc-8ccc-cccccccccccc";

// ---------------------------------------------------------------------------
// Setup
// ---------------------------------------------------------------------------

fn in_memory() -> InMemoryGrowthStorage {
    InMemoryGrowthStorage::new()
}

fn aevumdb() -> (AevumDbGrowthStorage, TempDir) {
    let temp = TempDir::new().unwrap();
    let config = aevum_db::DbConfig::plaintext(temp.path().to_path_buf());
    let runtime = aevum_db::DbRuntime::plaintext();
    let storage = AevumDbGrowthStorage::open(config, runtime).unwrap();
    (storage, temp)
}

// ---------------------------------------------------------------------------
// Fixed entities
// ---------------------------------------------------------------------------

fn fixed_source(url: &str, topics: Vec<Topic>, status: SourceStatus) -> Source {
    let mut s = Source::new(
        Platform::Rss,
        Url::parse(url).unwrap(),
        None,
        topics,
        Utc::now(),
    );
    s.status = status;
    s
}

fn fixed_publication(
    source_id: SourceId,
    external_id: &str,
    topics: Vec<Topic>,
    published_at: Option<DateTime<Utc>>,
) -> Publication {
    Publication {
        id: PublicationId::from_parts(source_id, external_id),
        source_id,
        external_id: external_id.to_owned(),
        url: None,
        title: format!("Title {}", external_id),
        summary: None,
        author: None,
        language: None,
        topics,
        published_at,
        ingested_at: Utc::now(),
    }
}

fn fixed_opportunity(
    id: &str,
    topic: Topic,
    kind: OpportunityKind,
    score_bp: u32,
) -> Opportunity {
    Opportunity {
        id: OpportunityId(Uuid::parse_str(id).unwrap()),
        kind,
        topic,
        score_bp,
        evidence: vec![],
        detected_at: Utc::now(),
        payload: json!({}),
    }
}

fn fixed_event(id: &str, kind: GrowthEventKind, topic: Option<Topic>, at: DateTime<Utc>) -> GrowthEvent {
    GrowthEvent {
        id: Uuid::parse_str(id).unwrap(),
        occurred_at: at,
        kind,
        topic,
        payload: json!({}),
    }
}

// ---------------------------------------------------------------------------
// Projections (stable, deterministic fields only)
// ---------------------------------------------------------------------------

fn proj_sources(v: &[Source]) -> Vec<(String, String, Vec<String>)> {
    v.iter()
        .map(|s| {
            let mut topics: Vec<String> = s.topics.iter().map(|t| t.as_str().to_owned()).collect();
            topics.sort();
            (s.id.as_hex(), s.status.as_str().to_owned(), topics)
        })
        .collect()
}

fn proj_publications(v: &[Publication]) -> Vec<(String, String, Vec<String>)> {
    v.iter()
        .map(|p| {
            let mut topics: Vec<String> = p.topics.iter().map(|t| t.as_str().to_owned()).collect();
            topics.sort();
            (p.id.as_hex(), p.external_id.clone(), topics)
        })
        .collect()
}

fn proj_opportunities(v: &[Opportunity]) -> Vec<(String, String, String, u32)> {
    v.iter()
        .map(|o| {
            (
                o.id.0.to_string(),
                o.topic.as_str().to_owned(),
                o.kind.as_str().to_owned(),
                o.score_bp,
            )
        })
        .collect()
}

fn proj_topic_states(v: &[TopicState]) -> Vec<(String, u64, u64)> {
    v.iter()
        .map(|s| {
            (
                s.topic.as_str().to_owned(),
                s.publication_count,
                s.source_count,
            )
        })
        .collect()
}

fn proj_events(v: &[GrowthEvent]) -> Vec<(String, String, Option<String>, DateTime<Utc>)> {
    v.iter()
        .map(|e| {
            (
                e.id.to_string(),
                e.kind.as_str().to_owned(),
                e.topic.map(|t| t.as_str().to_owned()),
                e.occurred_at,
            )
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Empty backend
// ---------------------------------------------------------------------------

#[tokio::test]
async fn empty_backend_source_lists_agree() {
    let mem = in_memory();
    let (db, _t) = aevumdb();
    assert_eq!(
        proj_sources(&mem.list_sources(10).await.unwrap()),
        proj_sources(&db.list_sources(10).await.unwrap())
    );
    assert_eq!(
        proj_sources(&mem.list_sources_by_topic(Topic::Rust, 10).await.unwrap()),
        proj_sources(&db.list_sources_by_topic(Topic::Rust, 10).await.unwrap())
    );
    assert_eq!(
        proj_sources(&mem.list_sources_by_status(SourceStatus::Active, 10).await.unwrap()),
        proj_sources(&db.list_sources_by_status(SourceStatus::Active, 10).await.unwrap())
    );
}

#[tokio::test]
async fn empty_backend_publication_lists_agree() {
    let mem = in_memory();
    let (db, _t) = aevumdb();
    assert_eq!(
        proj_publications(&mem.list_recent_publications(10).await.unwrap()),
        proj_publications(&db.list_recent_publications(10).await.unwrap())
    );
}

#[tokio::test]
async fn empty_backend_opportunity_and_event_lists_agree() {
    let mem = in_memory();
    let (db, _t) = aevumdb();
    assert_eq!(
        proj_opportunities(&mem.list_top_opportunities(10).await.unwrap()),
        proj_opportunities(&db.list_top_opportunities(10).await.unwrap())
    );
    assert_eq!(
        proj_events(&mem.get_recent_events(10).await.unwrap()),
        proj_events(&db.get_recent_events(10).await.unwrap())
    );
    assert_eq!(
        proj_topic_states(&mem.list_topic_states().await.unwrap()),
        proj_topic_states(&db.list_topic_states().await.unwrap())
    );
}

// ---------------------------------------------------------------------------
// Sources
// ---------------------------------------------------------------------------

#[tokio::test]
async fn source_ordering_matches() {
    let mem = in_memory();
    let (db, _t) = aevumdb();

    // Three different URLs → three different deterministic ids.
    for url in [
        "https://example.com/a.xml",
        "https://example.com/b.xml",
        "https://example.com/c.xml",
    ] {
        let s = fixed_source(url, vec![Topic::Rust], SourceStatus::Active);
        mem.put_source(&s).await.unwrap();
        db.put_source(&s).await.unwrap();
    }

    assert_eq!(
        proj_sources(&mem.list_sources(10).await.unwrap()),
        proj_sources(&db.list_sources(10).await.unwrap())
    );
}

#[tokio::test]
async fn source_overwrite_semantics_match() {
    let mem = in_memory();
    let (db, _t) = aevumdb();

    let url = "https://example.com/a.xml";
    let v1 = fixed_source(url, vec![Topic::Rust], SourceStatus::Active);
    let mut v2 = v1.clone();
    v2.status = SourceStatus::Paused;

    mem.put_source(&v1).await.unwrap();
    mem.put_source(&v2).await.unwrap();
    db.put_source(&v1).await.unwrap();
    db.put_source(&v2).await.unwrap();

    assert_eq!(
        proj_sources(&mem.list_sources(10).await.unwrap()),
        proj_sources(&db.list_sources(10).await.unwrap())
    );
}

#[tokio::test]
async fn source_soft_delete_is_idempotent_in_both() {
    let mem = in_memory();
    let (db, _t) = aevumdb();

    let src = fixed_source("https://example.com/a.xml", vec![Topic::Rust], SourceStatus::Active);
    mem.put_source(&src).await.unwrap();
    db.put_source(&src).await.unwrap();

    mem.delete_source(src.id).await.unwrap();
    db.delete_source(src.id).await.unwrap();
    mem.delete_source(src.id).await.unwrap();
    db.delete_source(src.id).await.unwrap();

    assert_eq!(
        proj_sources(&mem.list_sources_by_status(SourceStatus::Disabled, 10).await.unwrap()),
        proj_sources(&db.list_sources_by_status(SourceStatus::Disabled, 10).await.unwrap())
    );
}

#[tokio::test]
async fn source_by_topic_filter_matches() {
    let mem = in_memory();
    let (db, _t) = aevumdb();

    let a = fixed_source(
        "https://example.com/a.xml",
        vec![Topic::Rust, Topic::PostQuantum],
        SourceStatus::Active,
    );
    let b = fixed_source(
        "https://example.com/b.xml",
        vec![Topic::Rust],
        SourceStatus::Active,
    );
    let c = fixed_source(
        "https://example.com/c.xml",
        vec![Topic::GpuCompute],
        SourceStatus::Active,
    );
    for s in [&a, &b, &c] {
        mem.put_source(s).await.unwrap();
        db.put_source(s).await.unwrap();
    }

    for topic in [Topic::Rust, Topic::PostQuantum, Topic::GpuCompute] {
        assert_eq!(
            proj_sources(&mem.list_sources_by_topic(topic, 10).await.unwrap()),
            proj_sources(&db.list_sources_by_topic(topic, 10).await.unwrap()),
            "topic filter mismatch for {:?}",
            topic
        );
    }
}

// ---------------------------------------------------------------------------
// Publications
// ---------------------------------------------------------------------------

#[tokio::test]
async fn publication_ordering_by_effective_ts_matches() {
    let mem = in_memory();
    let (db, _t) = aevumdb();

    let src = fixed_source("https://example.com/a.xml", vec![Topic::Rust], SourceStatus::Active);
    mem.put_source(&src).await.unwrap();
    db.put_source(&src).await.unwrap();

    let now = Utc::now();
    let p1 = fixed_publication(src.id, "1", vec![Topic::Rust], Some(now - Duration::hours(3)));
    let p2 = fixed_publication(src.id, "2", vec![Topic::Rust], Some(now));
    let p3 = fixed_publication(src.id, "3", vec![Topic::Rust], Some(now - Duration::hours(1)));

    for p in [&p1, &p2, &p3] {
        mem.put_publication(p).await.unwrap();
        db.put_publication(p).await.unwrap();
    }

    assert_eq!(
        proj_publications(&mem.list_recent_publications(10).await.unwrap()),
        proj_publications(&db.list_recent_publications(10).await.unwrap())
    );
    assert_eq!(
        proj_publications(&mem.list_publications_by_source(src.id, 10).await.unwrap()),
        proj_publications(&db.list_publications_by_source(src.id, 10).await.unwrap())
    );
}

#[tokio::test]
async fn publication_effective_ts_fallback_matches() {
    let mem = in_memory();
    let (db, _t) = aevumdb();

    let src = fixed_source("https://example.com/a.xml", vec![Topic::Rust], SourceStatus::Active);
    mem.put_source(&src).await.unwrap();
    db.put_source(&src).await.unwrap();

    // published_at = None → effective_ts = ingested_at
    let mut p = fixed_publication(src.id, "1", vec![Topic::Rust], None);
    p.ingested_at = Utc::now();
    mem.put_publication(&p).await.unwrap();
    db.put_publication(&p).await.unwrap();

    assert_eq!(
        proj_publications(&mem.list_recent_publications(10).await.unwrap()),
        proj_publications(&db.list_recent_publications(10).await.unwrap())
    );
}

#[tokio::test]
async fn publication_overwrite_semantics_match() {
    let mem = in_memory();
    let (db, _t) = aevumdb();

    let src = fixed_source("https://example.com/a.xml", vec![Topic::Rust], SourceStatus::Active);
    mem.put_source(&src).await.unwrap();
    db.put_source(&src).await.unwrap();

    let v1 = fixed_publication(src.id, "1", vec![Topic::Rust], None);
    let mut v2 = v1.clone();
    v2.title = "Overwritten".to_owned();

    mem.put_publication(&v1).await.unwrap();
    mem.put_publication(&v2).await.unwrap();
    db.put_publication(&v1).await.unwrap();
    db.put_publication(&v2).await.unwrap();

    let m = mem.get_publication(v1.id).await.unwrap().unwrap();
    let d = db.get_publication(v1.id).await.unwrap().unwrap();
    assert_eq!(m.title, d.title);
}

#[tokio::test]
async fn publication_exists_matches() {
    let mem = in_memory();
    let (db, _t) = aevumdb();

    let a = fixed_source("https://example.com/a.xml", vec![Topic::Rust], SourceStatus::Active);
    let b = fixed_source("https://example.com/b.xml", vec![Topic::Rust], SourceStatus::Active);
    mem.put_source(&a).await.unwrap();
    mem.put_source(&b).await.unwrap();
    db.put_source(&a).await.unwrap();
    db.put_source(&b).await.unwrap();

    let p = fixed_publication(a.id, "x", vec![Topic::Rust], None);
    mem.put_publication(&p).await.unwrap();
    db.put_publication(&p).await.unwrap();

    for (src, ext) in [(a.id, "x"), (a.id, "y"), (b.id, "x")] {
        assert_eq!(
            mem.publication_exists(src, ext).await.unwrap(),
            db.publication_exists(src, ext).await.unwrap(),
            "publication_exists mismatch for ({}, {})",
            src.as_hex(),
            ext
        );
    }
}

// ---------------------------------------------------------------------------
// Opportunities
// ---------------------------------------------------------------------------

#[tokio::test]
async fn opportunity_ordering_by_score_matches() {
    let mem = in_memory();
    let (db, _t) = aevumdb();

    let a = fixed_opportunity(OP_ID_A, Topic::Rust, OpportunityKind::TopicAccelerating, 2000);
    let b = fixed_opportunity(OP_ID_B, Topic::Rust, OpportunityKind::TopicAccelerating, 9000);
    let c = fixed_opportunity(OP_ID_C, Topic::Rust, OpportunityKind::TopicAccelerating, 5000);
    for o in [&a, &b, &c] {
        mem.put_opportunity(o).await.unwrap();
        db.put_opportunity(o).await.unwrap();
    }

    assert_eq!(
        proj_opportunities(&mem.list_top_opportunities(10).await.unwrap()),
        proj_opportunities(&db.list_top_opportunities(10).await.unwrap())
    );
    assert_eq!(
        proj_opportunities(&mem.list_opportunities_by_topic(Topic::Rust, 10).await.unwrap()),
        proj_opportunities(&db.list_opportunities_by_topic(Topic::Rust, 10).await.unwrap())
    );
    assert_eq!(
        proj_opportunities(
            &mem.list_opportunities_by_kind(OpportunityKind::TopicAccelerating, 10)
                .await
                .unwrap()
        ),
        proj_opportunities(
            &db.list_opportunities_by_kind(OpportunityKind::TopicAccelerating, 10)
                .await
                .unwrap()
        )
    );
}

#[tokio::test]
async fn opportunity_score_ties_break_by_uuid_in_both() {
    let mem = in_memory();
    let (db, _t) = aevumdb();

    // Two opportunities with identical score → tie-break by id.
    let a = fixed_opportunity(OP_ID_A, Topic::Rust, OpportunityKind::TopicAccelerating, 5000);
    let b = fixed_opportunity(OP_ID_B, Topic::Rust, OpportunityKind::TopicAccelerating, 5000);
    for o in [&b, &a] {
        mem.put_opportunity(o).await.unwrap();
        db.put_opportunity(o).await.unwrap();
    }

    assert_eq!(
        proj_opportunities(&mem.list_top_opportunities(10).await.unwrap()),
        proj_opportunities(&db.list_top_opportunities(10).await.unwrap())
    );
}

// ---------------------------------------------------------------------------
// TopicState
// ---------------------------------------------------------------------------

#[tokio::test]
async fn topic_states_ordering_matches() {
    let mem = in_memory();
    let (db, _t) = aevumdb();

    for topic in [Topic::Rust, Topic::GpuCompute, Topic::PostQuantum] {
        let mut s = TopicState::empty(topic, Utc::now());
        s.publication_count = 3;
        s.source_count = 1;
        mem.put_topic_state(&s).await.unwrap();
        db.put_topic_state(&s).await.unwrap();
    }

    assert_eq!(
        proj_topic_states(&mem.list_topic_states().await.unwrap()),
        proj_topic_states(&db.list_topic_states().await.unwrap())
    );
}

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

#[tokio::test]
async fn events_desc_ordering_matches() {
    let mem = in_memory();
    let (db, _t) = aevumdb();

    let now = Utc::now();
    let a = fixed_event(EV_ID_A, GrowthEventKind::SourceRegistered, None, now - Duration::hours(3));
    let b = fixed_event(EV_ID_B, GrowthEventKind::SourceRegistered, None, now);
    let c = fixed_event(EV_ID_C, GrowthEventKind::SourceRegistered, None, now - Duration::hours(1));
    for e in [&a, &b, &c] {
        mem.record_event(e.clone()).await.unwrap();
        db.record_event(e.clone()).await.unwrap();
    }

    assert_eq!(
        proj_events(&mem.get_recent_events(10).await.unwrap()),
        proj_events(&db.get_recent_events(10).await.unwrap())
    );
}

#[tokio::test]
async fn events_since_asc_ordering_matches() {
    let mem = in_memory();
    let (db, _t) = aevumdb();

    let now = Utc::now();
    let a = fixed_event(EV_ID_A, GrowthEventKind::SourceRegistered, None, now - Duration::hours(5));
    let b = fixed_event(EV_ID_B, GrowthEventKind::SourceRegistered, None, now - Duration::hours(3));
    let c = fixed_event(EV_ID_C, GrowthEventKind::SourceRegistered, None, now - Duration::hours(1));
    for e in [&a, &b, &c] {
        mem.record_event(e.clone()).await.unwrap();
        db.record_event(e.clone()).await.unwrap();
    }

    let cutoff = now - Duration::hours(4);
    assert_eq!(
        proj_events(&mem.get_events_since(cutoff, 10).await.unwrap()),
        proj_events(&db.get_events_since(cutoff, 10).await.unwrap())
    );
}

#[tokio::test]
async fn events_ties_break_by_uuid_in_both() {
    let mem = in_memory();
    let (db, _t) = aevumdb();

    // Same timestamp, different ids → deterministic order.
    let ts = Utc::now();
    let a = fixed_event(EV_ID_A, GrowthEventKind::SourceRegistered, None, ts);
    let b = fixed_event(EV_ID_B, GrowthEventKind::SourceRegistered, None, ts);
    let c = fixed_event(EV_ID_C, GrowthEventKind::SourceRegistered, None, ts);
    for e in [&c, &a, &b] {
        mem.record_event(e.clone()).await.unwrap();
        db.record_event(e.clone()).await.unwrap();
    }

    assert_eq!(
        proj_events(&mem.get_recent_events(10).await.unwrap()),
        proj_events(&db.get_recent_events(10).await.unwrap())
    );
}

#[tokio::test]
async fn events_kind_and_topic_filters_match() {
    let mem = in_memory();
    let (db, _t) = aevumdb();

    let now = Utc::now();
    let a = fixed_event(EV_ID_A, GrowthEventKind::SourceRegistered, None, now);
    let b = fixed_event(EV_ID_B, GrowthEventKind::TopicClassified, Some(Topic::Rust), now);
    let c = fixed_event(EV_ID_C, GrowthEventKind::TopicClassified, Some(Topic::PostQuantum), now);
    for e in [&a, &b, &c] {
        mem.record_event(e.clone()).await.unwrap();
        db.record_event(e.clone()).await.unwrap();
    }

    assert_eq!(
        proj_events(&mem.get_events_by_kind(GrowthEventKind::SourceRegistered, 10).await.unwrap()),
        proj_events(&db.get_events_by_kind(GrowthEventKind::SourceRegistered, 10).await.unwrap())
    );
    assert_eq!(
        proj_events(&mem.get_events_by_kind(GrowthEventKind::TopicClassified, 10).await.unwrap()),
        proj_events(&db.get_events_by_kind(GrowthEventKind::TopicClassified, 10).await.unwrap())
    );
    assert_eq!(
        proj_events(&mem.get_events_by_topic(Topic::Rust, 10).await.unwrap()),
        proj_events(&db.get_events_by_topic(Topic::Rust, 10).await.unwrap())
    );
    assert_eq!(
        proj_events(&mem.get_events_by_topic(Topic::PostQuantum, 10).await.unwrap()),
        proj_events(&db.get_events_by_topic(Topic::PostQuantum, 10).await.unwrap())
    );
}

// ---------------------------------------------------------------------------
// Prune
// ---------------------------------------------------------------------------

#[tokio::test]
async fn prune_removes_same_events_in_both() {
    let mem = in_memory();
    let (db, _t) = aevumdb();

    let now = Utc::now();
    let old1 = fixed_event(EV_ID_A, GrowthEventKind::SourceRegistered, None, now - Duration::hours(5));
    let old2 = fixed_event(EV_ID_B, GrowthEventKind::SourceRegistered, None, now - Duration::hours(3));
    let young = fixed_event(EV_ID_C, GrowthEventKind::SourceRegistered, None, now - Duration::hours(1));
    for e in [&old1, &old2, &young] {
        mem.record_event(e.clone()).await.unwrap();
        db.record_event(e.clone()).await.unwrap();
    }

    let cutoff = now - Duration::hours(2);
    let mem_removed = mem.prune_events_before(cutoff).await.unwrap();
    let db_removed = db.prune_events_before(cutoff).await.unwrap();
    assert_eq!(mem_removed, db_removed);

    assert_eq!(
        proj_events(&mem.get_recent_events(10).await.unwrap()),
        proj_events(&db.get_recent_events(10).await.unwrap())
    );
}

#[tokio::test]
async fn prune_is_idempotent_in_both() {
    let mem = in_memory();
    let (db, _t) = aevumdb();

    let now = Utc::now();
    let old = fixed_event(EV_ID_A, GrowthEventKind::SourceRegistered, None, now - Duration::hours(5));
    mem.record_event(old.clone()).await.unwrap();
    db.record_event(old.clone()).await.unwrap();

    let cutoff = now - Duration::hours(2);
    assert_eq!(mem.prune_events_before(cutoff).await.unwrap(), 1);
    assert_eq!(db.prune_events_before(cutoff).await.unwrap(), 1);
    assert_eq!(mem.prune_events_before(cutoff).await.unwrap(), 0);
    assert_eq!(db.prune_events_before(cutoff).await.unwrap(), 0);
}

// ---------------------------------------------------------------------------
// Limit behaviour
// ---------------------------------------------------------------------------

#[tokio::test]
async fn limit_zero_returns_empty_in_both() {
    let mem = in_memory();
    let (db, _t) = aevumdb();

    let src = fixed_source("https://example.com/a.xml", vec![Topic::Rust], SourceStatus::Active);
    mem.put_source(&src).await.unwrap();
    db.put_source(&src).await.unwrap();

    assert!(mem.list_sources(0).await.unwrap().is_empty());
    assert!(db.list_sources(0).await.unwrap().is_empty());
    assert!(mem.get_recent_events(0).await.unwrap().is_empty());
    assert!(db.get_recent_events(0).await.unwrap().is_empty());
    assert!(mem.list_top_opportunities(0).await.unwrap().is_empty());
    assert!(db.list_top_opportunities(0).await.unwrap().is_empty());
}

#[tokio::test]
async fn limit_one_respected_in_both() {
    let mem = in_memory();
    let (db, _t) = aevumdb();

    for url in [
        "https://example.com/a.xml",
        "https://example.com/b.xml",
        "https://example.com/c.xml",
    ] {
        let s = fixed_source(url, vec![Topic::Rust], SourceStatus::Active);
        mem.put_source(&s).await.unwrap();
        db.put_source(&s).await.unwrap();
    }

    assert_eq!(mem.list_sources(1).await.unwrap().len(), 1);
    assert_eq!(db.list_sources(1).await.unwrap().len(), 1);
    assert_eq!(
        proj_sources(&mem.list_sources(1).await.unwrap()),
        proj_sources(&db.list_sources(1).await.unwrap())
    );
}

#[tokio::test]
async fn limit_over_count_returns_all_in_both() {
    let mem = in_memory();
    let (db, _t) = aevumdb();

    for url in [
        "https://example.com/a.xml",
        "https://example.com/b.xml",
    ] {
        let s = fixed_source(url, vec![Topic::Rust], SourceStatus::Active);
        mem.put_source(&s).await.unwrap();
        db.put_source(&s).await.unwrap();
    }

    assert_eq!(mem.list_sources(100).await.unwrap().len(), 2);
    assert_eq!(db.list_sources(100).await.unwrap().len(), 2);
}
