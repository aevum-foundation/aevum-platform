//! Opportunity storage adapter.
//!
//! Key layout:
//!
//! ```text
//! growth:opportunity:id:{opportunity_id}
//! growth:opportunity:by_topic:{topic}:{inv_score}:{opportunity_id}
//! growth:opportunity:by_kind:{kind}:{inv_score}:{opportunity_id}
//! growth:opportunity:by_score:{inv_score}:{opportunity_id}
//! ```
//!
//! # Identity
//!
//! `OpportunityId` is UUID v4 — non-deterministic. Every detection is
//! a new record. There is no uniqueness constraint on Opportunity.
//!
//! # Ordering
//!
//! `inv_score = u64::MAX - score_bp`, encoded as 20-digit zero-padded
//! decimal. Natural lexicographic order of `by_topic`, `by_kind`, and
//! `by_score` therefore equals DESC score order. Ties on score are
//! broken by `opportunity_id` (UUID hyphenated), giving fully
//! deterministic ordering.
//!
//! # Performance note
//!
//! List methods currently materialize the full prefix via
//! `prefix_scan` and then truncate to `limit`. This is correctness-
//! neutral. A future AevumDB Tier-1 primitive (`prefix_scan_limited`)
//! will replace `prefix_scan` here without changing the public
//! contract. See `docs/architecture/growth-storage-design-v1.md`
//! section 19.

use async_trait::async_trait;
use uuid::Uuid;

use crate::error::ApiError;
use crate::growth::models::{Opportunity, OpportunityId, OpportunityKind, Topic};
use crate::growth::storage::OpportunityStorage;

use super::{
    deserialize, encode_inv_score, map_db_error, serialize, AevumDbGrowthStorage,
    OPPORTUNITY_BY_KIND_PREFIX, OPPORTUNITY_BY_SCORE_PREFIX, OPPORTUNITY_BY_TOPIC_PREFIX,
    OPPORTUNITY_PREFIX,
};

// ---------------------------------------------------------------------------
// Key builders (canonical, pub(crate) for Key Layout Contract tests)
// ---------------------------------------------------------------------------

pub(crate) fn opportunity_key(id: OpportunityId) -> String {
    format!("{}{}", OPPORTUNITY_PREFIX, id.0)
}

pub(crate) fn opportunity_by_topic_key(
    topic: Topic,
    inv_score: &str,
    id: OpportunityId,
) -> String {
    format!(
        "{}{}:{}:{}",
        OPPORTUNITY_BY_TOPIC_PREFIX,
        topic.as_str(),
        inv_score,
        id.0
    )
}

pub(crate) fn opportunity_by_topic_prefix(topic: Topic) -> String {
    format!("{}{}:", OPPORTUNITY_BY_TOPIC_PREFIX, topic.as_str())
}

pub(crate) fn opportunity_by_kind_key(
    kind: OpportunityKind,
    inv_score: &str,
    id: OpportunityId,
) -> String {
    format!(
        "{}{}:{}:{}",
        OPPORTUNITY_BY_KIND_PREFIX,
        kind.as_str(),
        inv_score,
        id.0
    )
}

pub(crate) fn opportunity_by_kind_prefix(kind: OpportunityKind) -> String {
    format!("{}{}:", OPPORTUNITY_BY_KIND_PREFIX, kind.as_str())
}

pub(crate) fn opportunity_by_score_key(inv_score: &str, id: OpportunityId) -> String {
    format!("{}{}:{}", OPPORTUNITY_BY_SCORE_PREFIX, inv_score, id.0)
}

// ---------------------------------------------------------------------------
// Trait implementation
// ---------------------------------------------------------------------------

#[async_trait]
impl OpportunityStorage for AevumDbGrowthStorage {
    async fn put_opportunity(&self, opportunity: &Opportunity) -> Result<(), ApiError> {
        let id = opportunity.id;
        let inv_score = encode_inv_score(opportunity.score_bp);

        let primary_key = opportunity_key(id);
        let primary_bytes = serialize(opportunity)?;

        let mut batch = self.db().batch();
        batch.put(primary_key.as_bytes(), &primary_bytes);

        let by_topic = opportunity_by_topic_key(opportunity.topic, &inv_score, id);
        batch.put(by_topic.as_bytes(), id.0.to_string().as_bytes());

        let by_kind = opportunity_by_kind_key(opportunity.kind, &inv_score, id);
        batch.put(by_kind.as_bytes(), id.0.to_string().as_bytes());

        let by_score = opportunity_by_score_key(&inv_score, id);
        batch.put(by_score.as_bytes(), id.0.to_string().as_bytes());

        batch.commit().map_err(map_db_error)
    }

    async fn get_opportunity(
        &self,
        id: OpportunityId,
    ) -> Result<Option<Opportunity>, ApiError> {
        let key = opportunity_key(id);
        match self.db().get(key.as_bytes()).map_err(map_db_error)? {
            None => Ok(None),
            Some(bytes) => Ok(Some(deserialize::<Opportunity>(&bytes)?)),
        }
    }

    async fn list_opportunities_by_topic(
        &self,
        topic: Topic,
        limit: usize,
    ) -> Result<Vec<Opportunity>, ApiError> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        let prefix = opportunity_by_topic_prefix(topic);
        self.list_opportunities_via_index(&prefix, limit).await
    }

    async fn list_opportunities_by_kind(
        &self,
        kind: OpportunityKind,
        limit: usize,
    ) -> Result<Vec<Opportunity>, ApiError> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        let prefix = opportunity_by_kind_prefix(kind);
        self.list_opportunities_via_index(&prefix, limit).await
    }

    async fn list_top_opportunities(
        &self,
        limit: usize,
    ) -> Result<Vec<Opportunity>, ApiError> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        self.list_opportunities_via_index(OPPORTUNITY_BY_SCORE_PREFIX, limit)
            .await
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

impl AevumDbGrowthStorage {
    /// Generic index-driven opportunity listing.
    ///
    /// Scans `prefix` in natural (lexicographic) order, which for
    /// `by_topic`, `by_kind`, and `by_score` corresponds to DESC
    /// score order, with `opportunity_id` as deterministic
    /// tie-breaker.
    ///
    /// Returns at most `limit` records.
    ///
    /// TODO(AevumDB Tier-1): switch to `prefix_scan_limited(prefix, limit)`
    /// when the primitive becomes available.
    async fn list_opportunities_via_index(
        &self,
        prefix: &str,
        limit: usize,
    ) -> Result<Vec<Opportunity>, ApiError> {
        let raw = self
            .db()
            .prefix_scan(prefix.as_bytes())
            .map_err(map_db_error)?;

        let mut opportunities = Vec::with_capacity(raw.len().min(limit));
        for (_key, value) in raw {
            if opportunities.len() >= limit {
                break;
            }
            let id = parse_opportunity_id_value(&value)?;
            if let Some(opportunity) = self.get_opportunity(id).await? {
                opportunities.push(opportunity);
            }
        }

        Ok(opportunities)
    }
}

/// Decode a secondary-index value (UUID hyphenated string) into an
/// `OpportunityId`.
fn parse_opportunity_id_value(bytes: &[u8]) -> Result<OpportunityId, ApiError> {
    let uuid_str = std::str::from_utf8(bytes).map_err(|error| {
        log::error!("Growth: opportunity id utf8 decode failed: {}", error);
        ApiError::Internal
    })?;

    Uuid::parse_str(uuid_str)
        .map(OpportunityId)
        .map_err(|error| {
            log::error!("Growth: opportunity id uuid parse failed: {}", error);
            ApiError::Internal
        })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    use chrono::Utc;
    use tempfile::TempDir;

    use crate::growth::models::PublicationId;

    fn test_storage() -> (AevumDbGrowthStorage, TempDir) {
        let temp = TempDir::new().unwrap();
        let config = aevum_db::DbConfig::plaintext(temp.path().to_path_buf());
        let runtime = aevum_db::DbRuntime::plaintext();
        let storage = AevumDbGrowthStorage::open(config, runtime).unwrap();
        (storage, temp)
    }

    fn dummy_publication_id() -> PublicationId {
        PublicationId::from_hex(&"0".repeat(32)).unwrap()
    }

    fn make_opportunity(
        topic: Topic,
        kind: OpportunityKind,
        score_bp: u32,
    ) -> Opportunity {
        Opportunity {
            id: OpportunityId::new(),
            kind,
            topic,
            score_bp,
            evidence: vec![dummy_publication_id()],
            detected_at: Utc::now(),
            payload: serde_json::json!({}),
        }
    }

    // ─── Key Layout Contract ────────────────────────────

    #[test]
    fn opportunity_key_is_stable() {
        let opp = make_opportunity(Topic::Rust, OpportunityKind::TopicAccelerating, 5000);
        let key = opportunity_key(opp.id);
        assert!(key.starts_with("growth:opportunity:id:"));
        assert!(key.ends_with(&opp.id.0.to_string()));
    }

    #[test]
    fn opportunity_by_topic_key_is_stable() {
        let opp = make_opportunity(Topic::Rust, OpportunityKind::TopicAccelerating, 5000);
        let key = opportunity_by_topic_key(Topic::Rust, "00000000000000000000", opp.id);
        assert!(key.starts_with("growth:opportunity:by_topic:rust:"));
        assert!(key.ends_with(&opp.id.0.to_string()));
    }

    #[test]
    fn opportunity_by_kind_key_is_stable() {
        let opp = make_opportunity(Topic::Rust, OpportunityKind::TopicAccelerating, 5000);
        let key = opportunity_by_kind_key(
            OpportunityKind::TopicAccelerating,
            "00000000000000000000",
            opp.id,
        );
        assert!(key.starts_with("growth:opportunity:by_kind:topic_accelerating:"));
        assert!(key.ends_with(&opp.id.0.to_string()));
    }

    #[test]
    fn opportunity_by_score_key_is_stable() {
        let opp = make_opportunity(Topic::Rust, OpportunityKind::TopicAccelerating, 5000);
        let key = opportunity_by_score_key("00000000000000000000", opp.id);
        assert!(key.starts_with("growth:opportunity:by_score:"));
        assert!(key.ends_with(&opp.id.0.to_string()));
    }

    #[test]
    fn primary_prefix_does_not_match_secondary_keys() {
        let opp = make_opportunity(Topic::Rust, OpportunityKind::TopicAccelerating, 5000);
        let primary = opportunity_key(opp.id);
        let by_topic =
            opportunity_by_topic_key(Topic::Rust, "00000000000000000000", opp.id);
        let by_score = opportunity_by_score_key("00000000000000000000", opp.id);
        assert!(!by_topic.starts_with(&primary));
        assert!(!by_score.starts_with(&primary));
    }

    // ─── Score ordering / inversion ─────────────────────

    #[test]
    fn inverted_score_is_descending() {
        // For score_a < score_b, inv(score_a) > inv(score_b).
        let low = encode_inv_score(100);
        let high = encode_inv_score(9000);
        assert!(low > high, "lower score must sort AFTER higher score");
        assert_eq!(low.len(), 20);
        assert_eq!(high.len(), 20);
    }

    // ─── Behaviour ──────────────────────────────────────

    #[tokio::test]
    async fn put_get_roundtrip() {
        let (storage, _temp) = test_storage();
        let opp = make_opportunity(Topic::Rust, OpportunityKind::TopicAccelerating, 7500);

        storage.put_opportunity(&opp).await.unwrap();
        let loaded = storage
            .get_opportunity(opp.id)
            .await
            .unwrap()
            .expect("must exist");

        assert_eq!(loaded.id, opp.id);
        assert_eq!(loaded.score_bp, 7500);
        assert_eq!(loaded.topic, Topic::Rust);
    }

    #[tokio::test]
    async fn get_missing_returns_none() {
        let (storage, _temp) = test_storage();
        let opp = make_opportunity(Topic::Rust, OpportunityKind::TopicAccelerating, 5000);
        assert!(storage.get_opportunity(opp.id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn list_by_topic_filters_correctly() {
        let (storage, _temp) = test_storage();
        let rust = make_opportunity(Topic::Rust, OpportunityKind::TopicAccelerating, 5000);
        let pq = make_opportunity(
            Topic::PostQuantum,
            OpportunityKind::TopicEmerging,
            6000,
        );

        storage.put_opportunity(&rust).await.unwrap();
        storage.put_opportunity(&pq).await.unwrap();

        let rust_list = storage
            .list_opportunities_by_topic(Topic::Rust, 10)
            .await
            .unwrap();
        assert_eq!(rust_list.len(), 1);
        assert_eq!(rust_list[0].id, rust.id);

        let pq_list = storage
            .list_opportunities_by_topic(Topic::PostQuantum, 10)
            .await
            .unwrap();
        assert_eq!(pq_list.len(), 1);
        assert_eq!(pq_list[0].id, pq.id);
    }

    #[tokio::test]
    async fn list_by_kind_filters_correctly() {
        let (storage, _temp) = test_storage();
        let accel = make_opportunity(Topic::Rust, OpportunityKind::TopicAccelerating, 5000);
        let emerging = make_opportunity(Topic::Rust, OpportunityKind::TopicEmerging, 6000);

        storage.put_opportunity(&accel).await.unwrap();
        storage.put_opportunity(&emerging).await.unwrap();

        let accel_list = storage
            .list_opportunities_by_kind(OpportunityKind::TopicAccelerating, 10)
            .await
            .unwrap();
        assert_eq!(accel_list.len(), 1);
        assert_eq!(accel_list[0].id, accel.id);

        let emerging_list = storage
            .list_opportunities_by_kind(OpportunityKind::TopicEmerging, 10)
            .await
            .unwrap();
        assert_eq!(emerging_list.len(), 1);
        assert_eq!(emerging_list[0].id, emerging.id);
    }

    #[tokio::test]
    async fn list_top_opportunities_is_desc_by_score() {
        let (storage, _temp) = test_storage();
        let low = make_opportunity(Topic::Rust, OpportunityKind::TopicAccelerating, 2000);
        let high = make_opportunity(Topic::Rust, OpportunityKind::TopicAccelerating, 9000);
        let mid = make_opportunity(Topic::Rust, OpportunityKind::TopicAccelerating, 5000);

        storage.put_opportunity(&low).await.unwrap();
        storage.put_opportunity(&high).await.unwrap();
        storage.put_opportunity(&mid).await.unwrap();

        let list = storage.list_top_opportunities(10).await.unwrap();
        assert_eq!(list.len(), 3);
        assert_eq!(list[0].id, high.id, "highest score first");
        assert_eq!(list[1].id, mid.id);
        assert_eq!(list[2].id, low.id);
    }

    #[tokio::test]
    async fn same_score_has_deterministic_order() {
        let (storage, _temp) = test_storage();
        let a = make_opportunity(Topic::Rust, OpportunityKind::TopicAccelerating, 5000);
        let b = make_opportunity(Topic::Rust, OpportunityKind::TopicAccelerating, 5000);

        storage.put_opportunity(&a).await.unwrap();
        storage.put_opportunity(&b).await.unwrap();

        let first_run = storage.list_top_opportunities(10).await.unwrap();
        let second_run = storage.list_top_opportunities(10).await.unwrap();
        assert_eq!(first_run.len(), 2);
        assert_eq!(first_run[0].id, second_run[0].id);
        assert_eq!(first_run[1].id, second_run[1].id);
    }

    #[tokio::test]
    async fn limit_is_respected() {
        let (storage, _temp) = test_storage();
        for _ in 0..5 {
            let opp = make_opportunity(Topic::Rust, OpportunityKind::TopicAccelerating, 5000);
            storage.put_opportunity(&opp).await.unwrap();
        }
        let list = storage.list_top_opportunities(2).await.unwrap();
        assert_eq!(list.len(), 2);
    }

    #[tokio::test]
    async fn limit_zero_returns_empty() {
        let (storage, _temp) = test_storage();
        let opp = make_opportunity(Topic::Rust, OpportunityKind::TopicAccelerating, 5000);
        storage.put_opportunity(&opp).await.unwrap();

        assert!(storage
            .list_opportunities_by_topic(Topic::Rust, 0)
            .await
            .unwrap()
            .is_empty());
        assert!(storage
            .list_opportunities_by_kind(OpportunityKind::TopicAccelerating, 0)
            .await
            .unwrap()
            .is_empty());
        assert!(storage.list_top_opportunities(0).await.unwrap().is_empty());
    }
}
