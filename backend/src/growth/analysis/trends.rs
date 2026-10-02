//! Topic trend computation.
//!
//! Pure, deterministic counters over a list of publications.
//!
//! For each `Topic::ALL` entry, computes:
//!
//! - `count_24h` — publications in the last 24 hours
//! - `count_7d`  — publications in the last 7 days
//! - `count_30d` — publications in the last 30 days
//! - `ratio_7d_vs_30d_bp` — how the recent rate compares to the
//!   30-day average rate, in basis points (1 bp = 0.0001).
//!
//! The ratio is defined as:
//!
//! ```text
//! ratio = (count_7d / 7) / (count_30d / 30)
//!       = (count_7d * 30) / (count_30d * 7)
//! ```
//!
//! A ratio of 2.0 (i.e. `20_000` bp) means the recent 7-day rate is
//! twice the 30-day baseline. This is the acceleration signal that
//! `analysis/opportunities.rs` will consume.
//!
//! # Properties
//!
//! - Deterministic: `same input → same output`.
//! - Pure: no I/O, no time, no randomness (`now` is passed in).
//! - Integer arithmetic only: no floating point in the ratio.
//! - No division by zero: an empty window yields `ratio = 0`.
//!
//! # Complexity
//!
//! O(topics × publications). Acceptable for Phase 1 volumes
//! (tens of topics, thousands of publications). If this ever
//! becomes a bottleneck, replace with a single pass that
//! accumulates per-topic buckets.
//!
//! # TODO for `opportunities.rs`
//!
//! When the opportunity detector is implemented, it will likely
//! want to compare the *last* 7-day window against the *previous*
//! 7-day window, not just the 30-day average. At that point
//! `TopicTrend` may gain:
//!
//! - `count_prev_7d`
//! - `count_prev_30d`
//!
//! Those are intentionally not added yet: no consumer exists, and
//! the storage layout for `TopicTrend` would have to change.

use chrono::{DateTime, Duration, Utc};

use crate::growth::models::{Topic, TopicTrend};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// One basis point in terms of a raw ratio: `1.0 == 10_000 bp`.
pub const BP_ONE: u32 = 10_000;

/// Ratio threshold (in bp) above which a topic is considered
/// "accelerating". `20_000 bp == 2.0×`.
pub const ACCELERATION_THRESHOLD_BP: u32 = 20_000;

// ---------------------------------------------------------------------------
// Input
// ---------------------------------------------------------------------------

/// Minimal view of a publication needed for trend computation.
///
/// The full `Publication` type carries many fields that are not
/// needed here. This struct keeps the trends module decoupled from
/// storage layout.
#[derive(Debug, Clone)]
pub struct TrendInput {
    pub topics: Vec<Topic>,
    pub effective_ts: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Compute a `TopicTrend` for every topic in `Topic::ALL`.
///
/// `now` is passed in so that the function stays pure and
/// deterministic.
pub fn compute_all(inputs: &[TrendInput], now: DateTime<Utc>) -> Vec<TopicTrend> {
    Topic::ALL
        .iter()
        .map(|&topic| compute_one(topic, inputs, now))
        .collect()
}

/// Compute a `TopicTrend` for a single topic.
///
/// A publication contributes to a window iff
/// `effective_ts >= now - window`. In particular, an event exactly
/// `7 days` old is counted in `count_7d` (inclusive lower bound).
pub fn compute_one(topic: Topic, inputs: &[TrendInput], now: DateTime<Utc>) -> TopicTrend {
    let cutoff_24h = now - Duration::hours(24);
    let cutoff_7d = now - Duration::days(7);
    let cutoff_30d = now - Duration::days(30);

    let mut count_24h: u32 = 0;
    let mut count_7d: u32 = 0;
    let mut count_30d: u32 = 0;

    for input in inputs {
        if !input.topics.contains(&topic) {
            continue;
        }
        let ts = input.effective_ts;
        if ts > now {
            // Ignore publications dated in the future: they distort
            // the recent window and are almost always feed noise.
            continue;
        }
        if ts >= cutoff_30d {
            count_30d += 1;
        }
        if ts >= cutoff_7d {
            count_7d += 1;
        }
        if ts >= cutoff_24h {
            count_24h += 1;
        }
    }

    let ratio_7d_vs_30d_bp = ratio_bp(count_7d, count_30d);

    TopicTrend {
        topic,
        count_24h,
        count_7d,
        count_30d,
        ratio_7d_vs_30d_bp,
        computed_at: now,
    }
}

/// Compute `(count_7d * 30) / (count_30d * 7)` in basis points.
///
/// Returns 0 when `count_30d == 0`. Saturates at `u32::MAX`.
///
/// The result is NOT bounded to a "reasonable" range: a topic with
/// 30 recent publications and 1 historical publication will yield a
/// very large ratio. Callers that need a bounded signal must apply
/// their own threshold.
pub fn ratio_bp(count_7d: u32, count_30d: u32) -> u32 {
    if count_30d == 0 {
        return 0;
    }
    // (count_7d / 7) / (count_30d / 30) * 10_000
    // = (count_7d * 30 * 10_000) / (count_30d * 7)
    let numerator = (count_7d as u64) * 30u64 * (BP_ONE as u64);
    let denominator = (count_30d as u64) * 7u64;
    let raw = numerator / denominator;
    if raw > u32::MAX as u64 {
        u32::MAX
    } else {
        raw as u32
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Fixed "now" for all tests. Never use `Utc::now()` in tests:
    /// it makes them non-reproducible across runs and machines.
    fn fixed_now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    fn input(topics: &[Topic], age: Duration, now: DateTime<Utc>) -> TrendInput {
        TrendInput {
            topics: topics.to_vec(),
            effective_ts: now - age,
        }
    }

    // ─── compute_one: window boundaries ────────────────

    #[test]
    fn empty_input_yields_zero_trends() {
        let now = fixed_now();
        let trends = compute_all(&[], now);
        assert_eq!(trends.len(), Topic::ALL.len());
        for t in &trends {
            assert_eq!(t.count_24h, 0);
            assert_eq!(t.count_7d, 0);
            assert_eq!(t.count_30d, 0);
            assert_eq!(t.ratio_7d_vs_30d_bp, 0);
        }
    }

    #[test]
    fn single_recent_publication_counted_in_all_windows() {
        let now = fixed_now();
        let inputs = vec![input(&[Topic::Rust], Duration::hours(1), now)];
        let trend = compute_one(Topic::Rust, &inputs, now);
        assert_eq!(trend.count_24h, 1);
        assert_eq!(trend.count_7d, 1);
        assert_eq!(trend.count_30d, 1);
    }

    #[test]
    fn publication_between_24h_and_7d_not_in_24h() {
        let now = fixed_now();
        let inputs = vec![input(&[Topic::Rust], Duration::hours(48), now)];
        let trend = compute_one(Topic::Rust, &inputs, now);
        assert_eq!(trend.count_24h, 0);
        assert_eq!(trend.count_7d, 1);
        assert_eq!(trend.count_30d, 1);
    }

    #[test]
    fn publication_between_7d_and_30d_only_in_30d() {
        let now = fixed_now();
        let inputs = vec![input(&[Topic::Rust], Duration::days(20), now)];
        let trend = compute_one(Topic::Rust, &inputs, now);
        assert_eq!(trend.count_24h, 0);
        assert_eq!(trend.count_7d, 0);
        assert_eq!(trend.count_30d, 1);
    }

    #[test]
    fn publication_outside_30d_not_counted() {
        let now = fixed_now();
        let inputs = vec![input(&[Topic::Rust], Duration::days(40), now)];
        let trend = compute_one(Topic::Rust, &inputs, now);
        assert_eq!(trend.count_30d, 0);
        assert_eq!(trend.ratio_7d_vs_30d_bp, 0);
    }

    #[test]
    fn window_lower_bound_is_inclusive_7d() {
        let now = fixed_now();
        // Exactly 7 days old → must be counted in count_7d.
        let inputs = vec![input(&[Topic::Rust], Duration::days(7), now)];
        let trend = compute_one(Topic::Rust, &inputs, now);
        assert_eq!(trend.count_7d, 1);
        assert_eq!(trend.count_30d, 1);
    }

    #[test]
    fn window_lower_bound_is_inclusive_24h() {
        let now = fixed_now();
        // Exactly 24 hours old → must be counted in count_24h.
        let inputs = vec![input(&[Topic::Rust], Duration::hours(24), now)];
        let trend = compute_one(Topic::Rust, &inputs, now);
        assert_eq!(trend.count_24h, 1);
    }

    #[test]
    fn window_lower_bound_is_inclusive_30d() {
        let now = fixed_now();
        // Exactly 30 days old → must be counted in count_30d.
        let inputs = vec![input(&[Topic::Rust], Duration::days(30), now)];
        let trend = compute_one(Topic::Rust, &inputs, now);
        assert_eq!(trend.count_30d, 1);
        assert_eq!(trend.count_7d, 0);
    }

    #[test]
    fn future_publications_are_ignored() {
        let now = fixed_now();
        let inputs = vec![TrendInput {
            topics: vec![Topic::Rust],
            effective_ts: now + Duration::hours(5),
        }];
        let trend = compute_one(Topic::Rust, &inputs, now);
        assert_eq!(trend.count_24h, 0);
        assert_eq!(trend.count_30d, 0);
    }

    // ─── compute_one: topic filtering ──────────────────

    #[test]
    fn topic_isolation() {
        let now = fixed_now();
        let inputs = vec![
            input(&[Topic::Rust], Duration::hours(1), now),
            input(&[Topic::PostQuantum], Duration::hours(1), now),
        ];
        let rust = compute_one(Topic::Rust, &inputs, now);
        let pq = compute_one(Topic::PostQuantum, &inputs, now);
        assert_eq!(rust.count_24h, 1);
        assert_eq!(pq.count_24h, 1);
    }

    #[test]
    fn multi_topic_publication_counted_for_each_topic() {
        let now = fixed_now();
        let inputs = vec![input(
            &[Topic::Rust, Topic::GpuCompute],
            Duration::hours(1),
            now,
        )];
        let rust = compute_one(Topic::Rust, &inputs, now);
        let gpu = compute_one(Topic::GpuCompute, &inputs, now);
        assert_eq!(rust.count_24h, 1);
        assert_eq!(gpu.count_24h, 1);
    }

    #[test]
    fn duplicate_topics_do_not_double_count() {
        let now = fixed_now();
        // If the classifier ever emits a duplicated topic, the
        // publication MUST still count once.
        let inputs = vec![input(
            &[Topic::Rust, Topic::Rust, Topic::Rust],
            Duration::hours(1),
            now,
        )];
        let trend = compute_one(Topic::Rust, &inputs, now);
        assert_eq!(trend.count_24h, 1);
        assert_eq!(trend.count_7d, 1);
        assert_eq!(trend.count_30d, 1);
    }

    // ─── ratio_bp ──────────────────────────────────────

    #[test]
    fn ratio_zero_when_no_30d_data() {
        assert_eq!(ratio_bp(5, 0), 0);
        assert_eq!(ratio_bp(0, 0), 0);
        assert_eq!(ratio_bp(100, 0), 0);
    }

    #[test]
    fn ratio_zero_when_no_7d_data_but_30d_present() {
        // 0 recent against 30 historical → 0 bp.
        assert_eq!(ratio_bp(0, 30), 0);
    }

    #[test]
    fn ratio_is_1x_when_rate_is_stable() {
        // 7 in 7d (1/day), 30 in 30d (1/day) → 10_000 bp.
        assert_eq!(ratio_bp(7, 30), BP_ONE);
    }

    #[test]
    fn ratio_is_2x_when_recent_rate_doubles() {
        // 14 in 7d (2/day), 30 in 30d (1/day) → 20_000 bp.
        assert_eq!(ratio_bp(14, 30), 2 * BP_ONE);
    }

    #[test]
    fn ratio_is_half_when_recent_rate_halves() {
        // 7 in 7d (1/day), 60 in 30d (2/day) → 5_000 bp.
        assert_eq!(ratio_bp(7, 60), BP_ONE / 2);
    }

    #[test]
    fn ratio_floor_divides() {
        // 1 in 7d (1/7 per day), 30 in 30d (1/day) → 1428 bp.
        // (1 * 30 * 10000) / (30 * 7) = 300000 / 210 = 1428.57 → 1428.
        assert_eq!(ratio_bp(1, 30), 1428);
    }

    #[test]
    fn ratio_is_not_bounded_to_threshold() {
        // 30 in 7d, 30 in 30d → 42857 bp (4.28x), well above 2x.
        // (30 * 30 * 10000) / (30 * 7) = 9000000 / 210 = 42857.14 → 42857.
        assert_eq!(ratio_bp(30, 30), 42857);
    }

    #[test]
    fn ratio_saturates_at_u32_max() {
        assert_eq!(ratio_bp(u32::MAX, 1), u32::MAX);
    }

    // ─── compute_all: determinism ──────────────────────

    #[test]
    fn compute_all_is_deterministic() {
        let now = fixed_now();
        let inputs = vec![
            input(&[Topic::Rust], Duration::hours(1), now),
            input(&[Topic::StorageSystems], Duration::hours(100), now),
        ];
        let a = compute_all(&inputs, now);
        let b = compute_all(&inputs, now);
        assert_eq!(a.len(), b.len());
        for (x, y) in a.iter().zip(b.iter()) {
            assert_eq!(x.topic, y.topic);
            assert_eq!(x.count_24h, y.count_24h);
            assert_eq!(x.count_7d, y.count_7d);
            assert_eq!(x.count_30d, y.count_30d);
            assert_eq!(x.ratio_7d_vs_30d_bp, y.ratio_7d_vs_30d_bp);
        }
    }

    #[test]
    fn compute_all_returns_one_trend_per_topic() {
        let now = fixed_now();
        let trends = compute_all(&[], now);
        assert_eq!(trends.len(), Topic::ALL.len());
        for (trend, &topic) in trends.iter().zip(Topic::ALL.iter()) {
            assert_eq!(trend.topic, topic);
        }
    }
}
