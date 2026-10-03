//! TopicState storage adapter.
//!
//! One record per Topic. `Topic::ALL` is a fixed set of six, so
//! `list_topic_states` has no `limit` parameter (see
//! `growth/storage.rs` for the justification).

use async_trait::async_trait;

use crate::error::ApiError;
use crate::growth::models::{Topic, TopicState};
use crate::growth::storage::TopicStateStorage;

use super::{deserialize, map_db_error, serialize, AevumDbGrowthStorage, TOPIC_STATE_PREFIX};

// ---------------------------------------------------------------------------
// Key builder (canonical, pub(crate) for Key Layout Contract tests)
// ---------------------------------------------------------------------------

pub(crate) fn topic_state_key(topic: Topic) -> String {
    format!("{}{}", TOPIC_STATE_PREFIX, topic.as_str())
}

// ---------------------------------------------------------------------------
// Trait implementation
// ---------------------------------------------------------------------------

#[async_trait]
impl TopicStateStorage for AevumDbGrowthStorage {
    async fn put_topic_state(&self, state: &TopicState) -> Result<(), ApiError> {
        let key = topic_state_key(state.topic);
        let bytes = serialize(state)?;
        self.db().put(key.as_bytes(), &bytes).map_err(map_db_error)
    }

    async fn get_topic_state(&self, topic: Topic) -> Result<Option<TopicState>, ApiError> {
        let key = topic_state_key(topic);
        match self.db().get(key.as_bytes()).map_err(map_db_error)? {
            None => Ok(None),
            Some(bytes) => Ok(Some(deserialize::<TopicState>(&bytes)?)),
        }
    }

    async fn list_topic_states(&self) -> Result<Vec<TopicState>, ApiError> {
        let raw = self
            .db()
            .prefix_scan(TOPIC_STATE_PREFIX.as_bytes())
            .map_err(map_db_error)?;

        let mut states = Vec::with_capacity(raw.len());
        for (_key, value) in raw {
            states.push(deserialize::<TopicState>(&value)?);
        }

        // Deterministic order: topic ASC.
        states.sort_by_key(|s| s.topic.as_str());

        Ok(states)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    use chrono::Utc;
    use tempfile::TempDir;

    use crate::growth::aevumdb::AevumDbGrowthStorage;

    fn test_storage() -> (AevumDbGrowthStorage, TempDir) {
        let temp = TempDir::new().unwrap();
        let config = aevum_db::DbConfig::plaintext(temp.path().to_path_buf());
        let runtime = aevum_db::DbRuntime::plaintext();
        let storage = AevumDbGrowthStorage::open(config, runtime).unwrap();
        (storage, temp)
    }

    // ─── Key Layout Contract ────────────────────────────

    #[test]
    fn topic_state_key_is_stable() {
        assert_eq!(
            topic_state_key(Topic::PostQuantum),
            "growth:topic:state:post_quantum"
        );
        assert_eq!(
            topic_state_key(Topic::DistributedSystems),
            "growth:topic:state:distributed_systems"
        );
        assert_eq!(topic_state_key(Topic::Rust), "growth:topic:state:rust");
        assert_eq!(
            topic_state_key(Topic::BlockchainArchitecture),
            "growth:topic:state:blockchain_architecture"
        );
        assert_eq!(
            topic_state_key(Topic::GpuCompute),
            "growth:topic:state:gpu_compute"
        );
        assert_eq!(
            topic_state_key(Topic::StorageSystems),
            "growth:topic:state:storage_systems"
        );
    }

    // ─── Behaviour ──────────────────────────────────────

    #[tokio::test]
    async fn put_get_roundtrip() {
        let (storage, _temp) = test_storage();
        let now = Utc::now();
        let state = TopicState {
            topic: Topic::Rust,
            publication_count: 42,
            source_count: 7,
            first_seen_at: Some(now),
            last_seen_at: Some(now),
            updated_at: now,
        };

        storage.put_topic_state(&state).await.unwrap();

        let loaded = storage
            .get_topic_state(Topic::Rust)
            .await
            .unwrap()
            .expect("record must exist");

        assert_eq!(loaded.topic, Topic::Rust);
        assert_eq!(loaded.publication_count, 42);
        assert_eq!(loaded.source_count, 7);
    }

    #[tokio::test]
    async fn get_missing_returns_none() {
        let (storage, _temp) = test_storage();
        let loaded = storage.get_topic_state(Topic::Rust).await.unwrap();
        assert!(loaded.is_none());
    }

    #[tokio::test]
    async fn list_returns_all_six_in_topic_order() {
        let (storage, _temp) = test_storage();
        let now = Utc::now();

        // Write in a scrambled order to prove sorting.
        for topic in [
            Topic::StorageSystems,
            Topic::Rust,
            Topic::GpuCompute,
            Topic::PostQuantum,
            Topic::DistributedSystems,
            Topic::BlockchainArchitecture,
        ] {
            let state = TopicState::empty(topic, now);
            storage.put_topic_state(&state).await.unwrap();
        }

        let states = storage.list_topic_states().await.unwrap();
        assert_eq!(states.len(), 6);

        let returned: Vec<&str> = states.iter().map(|s| s.topic.as_str()).collect();
        let expected: Vec<&str> = Topic::ALL.iter().map(|t| t.as_str()).collect();
        let mut expected_sorted = expected.clone();
        expected_sorted.sort();
        assert_eq!(returned, expected_sorted);
    }

    #[tokio::test]
    async fn put_overwrites_previous() {
        let (storage, _temp) = test_storage();
        let now = Utc::now();

        let first = TopicState {
            topic: Topic::Rust,
            publication_count: 1,
            source_count: 1,
            first_seen_at: Some(now),
            last_seen_at: Some(now),
            updated_at: now,
        };
        storage.put_topic_state(&first).await.unwrap();

        let second = TopicState {
            topic: Topic::Rust,
            publication_count: 99,
            source_count: 5,
            first_seen_at: Some(now),
            last_seen_at: Some(now),
            updated_at: now,
        };
        storage.put_topic_state(&second).await.unwrap();

        let loaded = storage.get_topic_state(Topic::Rust).await.unwrap().unwrap();
        assert_eq!(loaded.publication_count, 99);
        assert_eq!(loaded.source_count, 5);
    }
}
