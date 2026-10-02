//! Opportunity detection.
//!
//! Consumes `TopicTrend` snapshots and produces `Opportunity`
//! records — analytical signals about topics that are worth
//! looking at.
//!
//! # Determinism
//!
//! The detection logic is deterministic: same trends → same set of
//! `(kind, topic, score, evidence, payload, detected_at)`. Only
//! `OpportunityId` varies between calls, because each detection is
//! a new event.
//!
//! # Scope
//!
//! Phase 1 covers two opportunity kinds:
//!
//! - `TopicAccelerating` — recent rate is significantly above the
//!   30-day baseline.
//! - `TopicEmerging` — small absolute volume but concentrated in
//!   the last 24 hours.
//!
//! `TopicPeak` and `SourceSurge` are planned but not yet
//! implemented: they need additional inputs (`TrendInput` does not
//! carry source_id, and the peak detector needs a per-day series
//! that we do not maintain in Phase 1).
//!
//! # Scoring
//!
//! Acceleration score is a linear function of the excess of
//! `ratio_7d_vs_30d_bp` over `ACCELERATION_THRESHOLD_BP`,
//! normalized by the threshold and saturated at `SCORE_BP_MAX`:
//!
//! ```text
//! score = (ratio - threshold) / threshold * SCORE_BP_MAX
//!       = (ratio - threshold) * SCORE_BP_MAX / threshold
//! ```
//!
//! Concrete values:
//!
//! - `ratio == threshold`        → score 0 (no opportunity anyway)
//! - `ratio == 2 * threshold`    → score `SCORE_BP_MAX / 2`
//! - `ratio >= 2 * threshold`    → score `SCORE_BP_MAX` (saturated)
//!
//! Emerging score is a fixed `EMERGING_SCORE_BP`.
//!
//! # Evidence contract
//!
//! `evidence_lookup(topic)` MUST return exactly the publications
//! that the caller considers evidence for a signal on `topic`.
//! The detector does NOT re-filter by time, source, or any other
//! criterion. Time-window filtering is the caller's
//! responsibility.
//!
//! If a signal is not fired for `topic`, `evidence_lookup` is NOT
//! called for that topic.
//!
//! If a topic fires two kinds (e.g. both `TopicAccelerating` and
//! `TopicEmerging`), `evidence_lookup` is called at most once and
//! the same evidence list is reused for both records.
//!
//! # Input invariant
//!
//! `detect_all` preserves the input order of `trends`. It does NOT
//! sort.
//!
//! `trends::compute_all` guarantees one `TopicTrend` per
//! `Topic::ALL` entry. A caller that bypasses `compute_all` MUST
//! supply at most one trend per topic. Duplicated trends produce
//! duplicated opportunities, which is intentional: the detector
//! does not silently mask a caller bug.
//!
//! # Technical debt
//!
//! Emerging v1 (current): `count_24h >= MIN` and `count_30d <= MAX`.
//! Emerging v2 (future): compare `count_24h` against
//! `previous_7d` and `previous_30d` windows. Those counters are not
//! maintained in Phase 1 and would require extending `TopicTrend`.

use chrono::{DateTime, Utc};

use crate::growth::analysis::trends::{ACCELERATION_THRESHOLD_BP, BP_ONE};
use crate::growth::models::{
    Opportunity, OpportunityId, OpportunityKind, PublicationId, Topic, TopicTrend,
};

// ---------------------------------------------------------------------------
// Thresholds
// ---------------------------------------------------------------------------

/// Minimum `count_24h` for a topic to be considered "emerging".
pub const EMERGING_MIN_COUNT_24H: u32 = 3;

/// Maximum `count_30d` for a topic to be considered "emerging".
/// Above this value the topic is established and its 24h spike is
/// less interesting.
pub const EMERGING_MAX_COUNT_30D: u32 = 10;

/// Maximum value of `Opportunity::score_bp`.
pub const SCORE_BP_MAX: u32 = 10_000;

/// Score awarded to an emerging topic.
pub const EMERGING_SCORE_BP: u32 = 6_000;

// ---------------------------------------------------------------------------
// Detector input
// ---------------------------------------------------------------------------

/// Minimal per-publication evidence available to the detector.
///
/// `PublicationId` is optional: the detector may run on trend
/// snapshots that were computed without publication ids.
///
/// `effective_ts` is informational. The detector does NOT filter by
/// it; see the evidence contract in the module docs.
#[derive(Debug, Clone)]
pub struct OpportunityEvidence {
    pub publication_id: Option<PublicationId>,
    pub effective_ts: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Detect opportunities from a set of trends.
///
/// `now` is passed in for determinism of `detected_at`.
///
/// `evidence_lookup` returns the evidence publications for a given
/// topic. It MUST return only the publications the caller
/// considers evidence for a signal on that topic.
///
/// Output order follows the input order of `trends`.
pub fn detect_all<F>(
    trends: &[TopicTrend],
    now: DateTime<Utc>,
    mut evidence_lookup: F,
) -> Vec<Opportunity>
where
    F: FnMut(Topic) -> Vec<OpportunityEvidence>,
{
    let mut out: Vec<Opportunity> = Vec::new();

    for trend in trends {
        // Determine which signals fire for this trend.
        let accelerating = wants_accelerating(trend);
        let emerging = wants_emerging(trend);

        if !accelerating && !emerging {
            continue;
        }

        // Look up evidence at most once per topic per call.
        let evidence = collect_evidence(&trend.topic, &mut evidence_lookup);

        if accelerating {
            out.push(build_accelerating(trend, &evidence, now));
        }
        if emerging {
            out.push(build_emerging(trend, &evidence, now));
        }
    }

    out
}

// ---------------------------------------------------------------------------
// Detector predicates
// ---------------------------------------------------------------------------

fn wants_accelerating(trend: &TopicTrend) -> bool {
    trend.ratio_7d_vs_30d_bp > ACCELERATION_THRESHOLD_BP
}

fn wants_emerging(trend: &TopicTrend) -> bool {
    trend.count_24h >= EMERGING_MIN_COUNT_24H && trend.count_30d <= EMERGING_MAX_COUNT_30D
}

// ---------------------------------------------------------------------------
// Score
// ---------------------------------------------------------------------------

/// Acceleration score as a linear function of the excess over the
/// threshold, saturated at `SCORE_BP_MAX`.
///
/// `ratio <= threshold` yields 0 (caller should not have fired).
///
/// Uses integer floor division. Small excesses may round down to 0:
/// with `threshold = 20_000` and `SCORE_BP_MAX = 10_000`, the
/// smallest excess producing a non-zero score is `2`.
pub fn acceleration_score_bp(ratio_bp: u32) -> u32 {
    if ratio_bp <= ACCELERATION_THRESHOLD_BP {
        return 0;
    }
    let excess = (ratio_bp - ACCELERATION_THRESHOLD_BP) as u64;
    let scaled = excess * (SCORE_BP_MAX as u64) / (ACCELERATION_THRESHOLD_BP as u64);
    if scaled > SCORE_BP_MAX as u64 {
        SCORE_BP_MAX
    } else {
        scaled as u32
    }
}

// ---------------------------------------------------------------------------
// Builders
// ---------------------------------------------------------------------------

fn build_accelerating(
    trend: &TopicTrend,
    evidence: &[PublicationId],
    now: DateTime<Utc>,
) -> Opportunity {
    let score_bp = acceleration_score_bp(trend.ratio_7d_vs_30d_bp);
    let payload = serde_json::json!({
        "kind": "topic_accelerating",
        "topic": trend.topic.as_str(),
        "count_24h": trend.count_24h,
        "count_7d": trend.count_7d,
        "count_30d": trend.count_30d,
        "ratio_7d_vs_30d_bp": trend.ratio_7d_vs_30d_bp,
        "threshold_bp": ACCELERATION_THRESHOLD_BP,
    });
    Opportunity {
        id: OpportunityId::new(),
        kind: OpportunityKind::TopicAccelerating,
        topic: trend.topic,
        score_bp,
        evidence: evidence.to_vec(),
        detected_at: now,
        payload,
    }
}

fn build_emerging(
    trend: &TopicTrend,
    evidence: &[PublicationId],
    now: DateTime<Utc>,
) -> Opportunity {
    let payload = serde_json::json!({
        "kind": "topic_emerging",
        "topic": trend.topic.as_str(),
        "count_24h": trend.count_24h,
        "count_7d": trend.count_7d,
        "count_30d": trend.count_30d,
        "min_count_24h": EMERGING_MIN_COUNT_24H,
        "max_count_30d": EMERGING_MAX_COUNT_30D,
    });
    Opportunity {
        id: OpportunityId::new(),
        kind: OpportunityKind::TopicEmerging,
        topic: trend.topic,
        score_bp: EMERGING_SCORE_BP,
        evidence: evidence.to_vec(),
        detected_at: now,
        payload,
    }
}

fn collect_evidence<F>(topic: &Topic, evidence_lookup: &mut F) -> Vec<PublicationId>
where
    F: FnMut(Topic) -> Vec<OpportunityEvidence>,
{
    evidence_lookup(*topic)
        .into_iter()
        .filter_map(|e| e.publication_id)
        .collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn fixed_now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    fn trend(topic: Topic, c24: u32, c7: u32, c30: u32, ratio_bp: u32) -> TopicTrend {
        TopicTrend {
            topic,
            count_24h: c24,
            count_7d: c7,
            count_30d: c30,
            ratio_7d_vs_30d_bp: ratio_bp,
            computed_at: fixed_now(),
        }
    }

    fn no_evidence(_topic: Topic) -> Vec<OpportunityEvidence> {
        Vec::new()
    }

    fn pub_id(n: u8) -> PublicationId {
        let hex: String = (0..32)
            .map(|i| std::char::from_digit(((n as u32) + i) % 16, 16).unwrap())
            .collect();
        PublicationId::from_hex(&hex).unwrap()
    }

    // ─── score function ────────────────────────────────

    #[test]
    fn score_zero_at_or_below_threshold() {
        assert_eq!(acceleration_score_bp(0), 0);
        assert_eq!(acceleration_score_bp(ACCELERATION_THRESHOLD_BP - 1), 0);
        assert_eq!(acceleration_score_bp(ACCELERATION_THRESHOLD_BP), 0);
    }

    #[test]
    fn score_grows_linearly_with_excess() {
        // threshold = 20_000. Half-range = +10_000 → score 5_000.
        assert_eq!(
            acceleration_score_bp(ACCELERATION_THRESHOLD_BP + 10_000),
            SCORE_BP_MAX / 2,
        );
        // Full range = +20_000 → score 10_000.
        assert_eq!(
            acceleration_score_bp(ACCELERATION_THRESHOLD_BP + 20_000),
            SCORE_BP_MAX,
        );
    }

    #[test]
    fn score_uses_floor_division() {
        // Integer arithmetic: excess=1, threshold=20_000 →
        // 1 * 10_000 / 20_000 = 0 (floor). This is documented and
        // deterministic.
        assert_eq!(acceleration_score_bp(ACCELERATION_THRESHOLD_BP + 1), 0);
        // The smallest excess that produces a non-zero score is
        // threshold / SCORE_BP_MAX = 20_000 / 10_000 = 2.
        assert_eq!(acceleration_score_bp(ACCELERATION_THRESHOLD_BP + 2), 1);
    }

    #[test]
    fn score_saturates_above_full_range() {
        assert_eq!(
            acceleration_score_bp(ACCELERATION_THRESHOLD_BP * 2),
            SCORE_BP_MAX,
        );
        assert_eq!(
            acceleration_score_bp(ACCELERATION_THRESHOLD_BP * 10),
            SCORE_BP_MAX,
        );
        assert_eq!(acceleration_score_bp(u32::MAX), SCORE_BP_MAX);
    }

    // ─── acceleration detection ────────────────────────

    #[test]
    fn no_acceleration_at_threshold() {
        let t = trend(Topic::Rust, 10, 10, 30, ACCELERATION_THRESHOLD_BP);
        let opps = detect_all(&[t], fixed_now(), no_evidence);
        assert!(opps.is_empty());
    }

    #[test]
    fn acceleration_detected_above_threshold() {
        let t = trend(Topic::Rust, 10, 20, 30, ACCELERATION_THRESHOLD_BP + 1);
        let opps = detect_all(&[t], fixed_now(), no_evidence);
        assert_eq!(opps.len(), 1);
        assert_eq!(opps[0].kind, OpportunityKind::TopicAccelerating);
    }

    #[test]
    fn acceleration_score_matches_function() {
        let r = ACCELERATION_THRESHOLD_BP + 7_500;
        let t = trend(Topic::Rust, 10, 20, 30, r);
        let opps = detect_all(&[t], fixed_now(), no_evidence);
        assert_eq!(opps[0].score_bp, acceleration_score_bp(r));
    }

    // ─── emerging detection ────────────────────────────

    #[test]
    fn emerging_detected_at_minimum_volume() {
        let t = trend(
            Topic::PostQuantum,
            EMERGING_MIN_COUNT_24H,
            EMERGING_MIN_COUNT_24H,
            EMERGING_MIN_COUNT_24H,
            0,
        );
        let opps = detect_all(&[t], fixed_now(), no_evidence);
        assert_eq!(opps.len(), 1);
        assert_eq!(opps[0].kind, OpportunityKind::TopicEmerging);
        assert_eq!(opps[0].score_bp, EMERGING_SCORE_BP);
    }

    #[test]
    fn emerging_not_detected_below_minimum() {
        let t = trend(
            Topic::PostQuantum,
            EMERGING_MIN_COUNT_24H - 1,
            EMERGING_MIN_COUNT_24H,
            EMERGING_MIN_COUNT_24H,
            0,
        );
        let opps = detect_all(&[t], fixed_now(), no_evidence);
        assert!(opps.is_empty());
    }

    #[test]
    fn emerging_not_detected_above_max_30d() {
        let t = trend(Topic::PostQuantum, 5, 10, EMERGING_MAX_COUNT_30D + 1, 0);
        let opps = detect_all(&[t], fixed_now(), no_evidence);
        assert!(opps.is_empty());
    }

    // ─── combined ──────────────────────────────────────

    #[test]
    fn topic_can_be_both_accelerating_and_emerging() {
        let t = trend(
            Topic::StorageSystems,
            5,
            20,
            8,
            ACCELERATION_THRESHOLD_BP + 10_000,
        );
        let opps = detect_all(&[t], fixed_now(), no_evidence);
        assert_eq!(opps.len(), 2);
        let kinds: Vec<_> = opps.iter().map(|o| o.kind).collect();
        assert!(kinds.contains(&OpportunityKind::TopicAccelerating));
        assert!(kinds.contains(&OpportunityKind::TopicEmerging));
    }

    #[test]
    fn evidence_lookup_called_at_most_once_per_topic() {
        let t = trend(
            Topic::StorageSystems,
            5,
            20,
            8,
            ACCELERATION_THRESHOLD_BP + 10_000,
        );
        let mut calls = 0;
        let _ = detect_all(&[t], fixed_now(), |_| {
            calls += 1;
            Vec::new()
        });
        assert_eq!(calls, 1, "must not look up evidence twice for one topic");
    }

    #[test]
    fn evidence_lookup_not_called_for_clean_topics() {
        let t = trend(Topic::Rust, 0, 0, 0, 0);
        let mut calls = 0;
        let opps = detect_all(&[t], fixed_now(), |_| {
            calls += 1;
            Vec::new()
        });
        assert!(opps.is_empty());
        assert_eq!(calls, 0);
    }

    // ─── evidence preservation ─────────────────────────

    #[test]
    fn evidence_ids_are_preserved_exactly() {
        let t = trend(Topic::Rust, 10, 20, 30, ACCELERATION_THRESHOLD_BP + 10_000);
        let p1 = pub_id(1);
        let p2 = pub_id(2);
        let opps = detect_all(&[t], fixed_now(), |_| {
            vec![
                OpportunityEvidence {
                    publication_id: Some(p1),
                    effective_ts: fixed_now(),
                },
                OpportunityEvidence {
                    publication_id: None,
                    effective_ts: fixed_now(),
                },
                OpportunityEvidence {
                    publication_id: Some(p2),
                    effective_ts: fixed_now(),
                },
            ]
        });
        assert_eq!(opps[0].evidence, vec![p1, p2]);
    }

    // ─── ordering ──────────────────────────────────────

    #[test]
    fn output_order_follows_input_order() {
        let inputs = vec![
            trend(Topic::Rust, 10, 20, 30, ACCELERATION_THRESHOLD_BP + 10_000),
            trend(Topic::PostQuantum, 5, 5, 5, 0),
            trend(Topic::GpuCompute, 10, 20, 30, ACCELERATION_THRESHOLD_BP + 5_000),
        ];
        let opps = detect_all(&inputs, fixed_now(), no_evidence);
        let topics: Vec<_> = opps.iter().map(|o| o.topic).collect();
        assert_eq!(
            topics,
            vec![Topic::Rust, Topic::PostQuantum, Topic::GpuCompute]
        );
    }

    // ─── input invariant ───────────────────────────────

    #[test]
    fn duplicate_trend_produces_two_opportunities() {
        // Documented behavior: the detector does not deduplicate.
        let t1 = trend(Topic::Rust, 10, 20, 30, ACCELERATION_THRESHOLD_BP + 10_000);
        let t2 = t1.clone();
        let opps = detect_all(&[t1, t2], fixed_now(), no_evidence);
        assert_eq!(opps.len(), 2);
        assert_eq!(opps[0].kind, opps[1].kind);
        assert_eq!(opps[0].topic, opps[1].topic);
    }

    // ─── determinism ───────────────────────────────────

    #[test]
    fn detection_is_deterministic_except_id() {
        let inputs = vec![
            trend(Topic::Rust, 10, 20, 30, ACCELERATION_THRESHOLD_BP + 10_000),
            trend(Topic::PostQuantum, 5, 5, 5, 0),
        ];
        let now = fixed_now();
        let a = detect_all(&inputs, now, no_evidence);
        let b = detect_all(&inputs, now, no_evidence);
        assert_eq!(a.len(), b.len());
        for (x, y) in a.iter().zip(b.iter()) {
            assert_eq!(x.kind, y.kind);
            assert_eq!(x.topic, y.topic);
            assert_eq!(x.score_bp, y.score_bp);
            assert_eq!(x.evidence, y.evidence);
            assert_eq!(x.payload, y.payload);
            assert_eq!(x.detected_at, y.detected_at);
            assert_ne!(x.id, y.id);
        }
    }

    // ─── payload ───────────────────────────────────────

    #[test]
    fn payload_contains_threshold_and_counts() {
        let t = trend(Topic::Rust, 10, 20, 30, ACCELERATION_THRESHOLD_BP + 10_000);
        let opps = detect_all(&[t], fixed_now(), no_evidence);
        let p = &opps[0].payload;
        assert_eq!(p["kind"], "topic_accelerating");
        assert_eq!(p["topic"], "rust");
        assert_eq!(p["count_24h"], 10);
        assert_eq!(p["count_7d"], 20);
        assert_eq!(p["count_30d"], 30);
        assert_eq!(p["ratio_7d_vs_30d_bp"], ACCELERATION_THRESHOLD_BP + 10_000);
        assert_eq!(p["threshold_bp"], ACCELERATION_THRESHOLD_BP);
    }

    #[test]
    fn detected_at_matches_passed_now() {
        let now = fixed_now();
        let t = trend(Topic::Rust, 10, 20, 30, ACCELERATION_THRESHOLD_BP + 10_000);
        let opps = detect_all(&[t], now, no_evidence);
        assert_eq!(opps[0].detected_at, now);
    }

    #[test]
    fn empty_input_yields_no_opportunities() {
        let opps = detect_all(&[], fixed_now(), no_evidence);
        assert!(opps.is_empty());
    }
}
