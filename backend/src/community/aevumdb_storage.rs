//! AevumDB-backed community storage.
//!
//! B-1.2b.3 — production storage layer for Community profiles.
//!
//! Username uniqueness is guaranteed by a per-username application mutex
//! combined with a single-instance deployment invariant. AevumDB does not
//! currently expose conditional writes / CAS; see docs/architecture/community-v1.md
//! and the AevumDB-TX-1 backlog item.

use std::collections::HashMap;
use std::sync::Arc;

use aevum_db::{AevumDb, DbConfig, DbError, DbRuntime};
use serde::de::DeserializeOwned;
use serde::Serialize;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::community::models::{CommunityProfile, CommunityRole};
use crate::community::storage::CommunityStorage;
use crate::error::ApiError;

const PROFILE_PREFIX: &str = "platform:community:profile:";
const USERNAME_INDEX_PREFIX: &str = "platform:community:profile:username:";
const ROLE_PREFIX: &str = "platform:community:role:";

#[derive(Clone)]
pub struct AevumDbCommunityStorage {
    db: Arc<AevumDb>,
    /// Per-username locks for serializing atomic username claims.
    /// Policy: never hold two different username locks at the same time.
    username_locks: Arc<Mutex<HashMap<String, Arc<Mutex<()>>>>>,
}

impl std::fmt::Debug for AevumDbCommunityStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AevumDbCommunityStorage")
            .field("db", &"<redacted>")
            .field("username_locks", &"<redacted>")
            .finish()
    }
}

impl AevumDbCommunityStorage {
    pub fn open(config: DbConfig, runtime: DbRuntime) -> Result<Self, ApiError> {
        let db = AevumDb::open(config, runtime).map_err(|error| {
            log::error!("AevumDB open failed: {}", error);
            ApiError::Internal
        })?;
        Ok(Self {
            db: Arc::new(db),
            username_locks: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub async fn lock_for_username(&self, normalized: &str) -> Arc<Mutex<()>> {
        let mut locks = self.username_locks.lock().await;
        locks
            .entry(normalized.to_owned())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    pub(crate) fn profile_key(user_id: &Uuid) -> String {
        format!("{}{}", PROFILE_PREFIX, user_id)
    }

    pub(crate) fn username_index_key(normalized_username: &str) -> String {
        format!("{}{}", USERNAME_INDEX_PREFIX, normalized_username)
    }

    pub(crate) fn role_key(user_id: &Uuid) -> String {
        format!("{}{}", ROLE_PREFIX, user_id)
    }

    fn serialize<T: Serialize>(value: &T) -> Result<Vec<u8>, ApiError> {
        serde_json::to_vec(value).map_err(|error| {
            log::error!("community serialize failed: {}", error);
            ApiError::Internal
        })
    }

    fn deserialize<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, ApiError> {
        serde_json::from_slice(bytes).map_err(|error| {
            log::error!("community deserialize failed: {}", error);
            ApiError::Internal
        })
    }

    fn map_db_error(error: DbError) -> ApiError {
        log::error!("AevumDB error: {}", error);
        ApiError::Internal
    }
}

impl CommunityStorage for AevumDbCommunityStorage {
    async fn create_profile(&self, profile: &CommunityProfile) -> Result<(), ApiError> {
        let lock = self.lock_for_username(&profile.normalized_username).await;
        let _guard = lock.lock().await;

        // Invariant 1: user_id must not already have a profile.
        let profile_key = Self::profile_key(&profile.user_id);
        if self
            .db
            .get(profile_key.as_bytes())
            .map_err(Self::map_db_error)?
            .is_some()
        {
            return Err(ApiError::Conflict);
        }

        // Invariant 2: normalized_username must not already be claimed.
        let index_key = Self::username_index_key(&profile.normalized_username);
        if self
            .db
            .get(index_key.as_bytes())
            .map_err(Self::map_db_error)?
            .is_some()
        {
            return Err(ApiError::Conflict);
        }

        let data = Self::serialize(profile)?;
        let user_id_str = profile.user_id.to_string();

        let mut batch = self.db.batch();
        batch.put(profile_key.as_bytes(), &data);
        batch.put(index_key.as_bytes(), user_id_str.as_bytes());
        batch.commit().map_err(Self::map_db_error)?;

        Ok(())
    }

    async fn get_profile_by_user_id(
        &self,
        user_id: &Uuid,
    ) -> Result<Option<CommunityProfile>, ApiError> {
        let key = Self::profile_key(user_id);
        let data = self.db.get(key.as_bytes()).map_err(Self::map_db_error)?;
        data.map(|bytes| Self::deserialize(&bytes)).transpose()
    }

    async fn get_profile_by_username(
        &self,
        normalized_username: &str,
    ) -> Result<Option<CommunityProfile>, ApiError> {
        let index_key = Self::username_index_key(normalized_username);
        let Some(user_id_bytes) = self
            .db
            .get(index_key.as_bytes())
            .map_err(Self::map_db_error)?
        else {
            return Ok(None);
        };

        let user_id_str = std::str::from_utf8(&user_id_bytes).map_err(|error| {
            log::error!("username index utf8 decode failed: {}", error);
            ApiError::Internal
        })?;

        let user_id = Uuid::parse_str(user_id_str).map_err(|error| {
            log::error!("username index uuid parse failed: {}", error);
            ApiError::Internal
        })?;

        self.get_profile_by_user_id(&user_id).await
    }

    async fn update_profile(&self, profile: &CommunityProfile) -> Result<(), ApiError> {
        let key = Self::profile_key(&profile.user_id);

        let existing_bytes = self
            .db
            .get(key.as_bytes())
            .map_err(Self::map_db_error)?
            .ok_or(ApiError::NotFound)?;

        let existing: CommunityProfile = Self::deserialize(&existing_bytes)?;

        // Username is immutable — refuse silent mismatch.
        if existing.normalized_username != profile.normalized_username {
            return Err(ApiError::Conflict);
        }

        let data = Self::serialize(profile)?;
        self.db
            .put(key.as_bytes(), &data)
            .map_err(Self::map_db_error)?;

        Ok(())
    }

    async fn get_role(&self, user_id: &Uuid) -> Result<CommunityRole, ApiError> {
        let key = Self::role_key(user_id);
        let Some(bytes) = self.db.get(key.as_bytes()).map_err(Self::map_db_error)? else {
            return Ok(CommunityRole::default());
        };

        let raw = std::str::from_utf8(&bytes).map_err(|error| {
            log::error!("role utf8 decode failed: {}", error);
            ApiError::Internal
        })?;

        Ok(CommunityRole::from_str(raw).unwrap_or_default())
    }

    async fn set_role(&self, user_id: &Uuid, role: CommunityRole) -> Result<(), ApiError> {
        let key = Self::role_key(user_id);
        let value = role.as_str();

        self.db
            .put(key.as_bytes(), value.as_bytes())
            .map_err(Self::map_db_error)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aevum_db::config::SyncMode;
    use chrono::Utc;
    use tempfile::TempDir;

    fn test_db() -> (AevumDbCommunityStorage, TempDir) {
        let temp = TempDir::new().unwrap();
        let config = DbConfig {
            path: temp.path().to_path_buf(),
            memtable_max_bytes: 64 * 1024 * 1024,
            wal_segment_bytes: 64 * 1024 * 1024,
            sync_mode: SyncMode::Always,
            max_open_files: 1000,
            block_size: 4096,
            storage_mode: aevum_db::config::StorageMode::default(),
        };
        let runtime = DbRuntime::plaintext();
        let storage = AevumDbCommunityStorage::open(config, runtime).unwrap();
        (storage, temp)
    }

    fn profile(user_id: Uuid, username: &str) -> CommunityProfile {
        let normalized = username.trim().to_ascii_lowercase();
        CommunityProfile::new(
            user_id,
            username.to_owned(),
            normalized,
            None,
            None,
            Utc::now(),
        )
    }

    #[test]
    fn key_layout_is_stable() {
        let user_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();

        assert_eq!(
            AevumDbCommunityStorage::profile_key(&user_id),
            "platform:community:profile:550e8400-e29b-41d4-a716-446655440000"
        );
        assert_eq!(
            AevumDbCommunityStorage::username_index_key("alice"),
            "platform:community:profile:username:alice"
        );
        assert_eq!(
            AevumDbCommunityStorage::role_key(&user_id),
            "platform:community:role:550e8400-e29b-41d4-a716-446655440000"
        );
    }

    #[tokio::test]
    async fn create_and_get_by_user_id() {
        let (storage, _temp) = test_db();
        let user_id = Uuid::new_v4();
        let profile = profile(user_id, "alice");

        storage.create_profile(&profile).await.unwrap();

        let fetched = storage
            .get_profile_by_user_id(&user_id)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(fetched.user_id, user_id);
        assert_eq!(fetched.username, "alice");
        assert_eq!(fetched.normalized_username, "alice");
    }

    #[tokio::test]
    async fn create_and_get_by_username() {
        let (storage, _temp) = test_db();
        let user_id = Uuid::new_v4();
        let profile = profile(user_id, "Alice");

        storage.create_profile(&profile).await.unwrap();

        let fetched = storage
            .get_profile_by_username("alice")
            .await
            .unwrap()
            .unwrap();

        assert_eq!(fetched.user_id, user_id);
        assert_eq!(fetched.username, "Alice");
    }

    #[tokio::test]
    async fn duplicate_username_is_conflict() {
        let (storage, _temp) = test_db();

        storage
            .create_profile(&profile(Uuid::new_v4(), "alice"))
            .await
            .unwrap();

        let result = storage
            .create_profile(&profile(Uuid::new_v4(), "Alice"))
            .await;

        assert!(matches!(result, Err(ApiError::Conflict)));
    }

    #[tokio::test]
    async fn duplicate_user_id_is_conflict() {
        let (storage, _temp) = test_db();
        let user_id = Uuid::new_v4();

        storage
            .create_profile(&profile(user_id, "alice"))
            .await
            .unwrap();

        let result = storage.create_profile(&profile(user_id, "bob")).await;

        assert!(matches!(result, Err(ApiError::Conflict)));
    }

    #[tokio::test]
    async fn get_role_defaults_to_user() {
        let (storage, _temp) = test_db();
        let user_id = Uuid::new_v4();

        assert_eq!(
            storage.get_role(&user_id).await.unwrap(),
            CommunityRole::User
        );
    }

    #[tokio::test]
    async fn set_role_round_trips() {
        let (storage, _temp) = test_db();
        let user_id = Uuid::new_v4();

        storage
            .set_role(&user_id, CommunityRole::Moderator)
            .await
            .unwrap();

        assert_eq!(
            storage.get_role(&user_id).await.unwrap(),
            CommunityRole::Moderator
        );

        storage
            .set_role(&user_id, CommunityRole::Admin)
            .await
            .unwrap();

        assert_eq!(
            storage.get_role(&user_id).await.unwrap(),
            CommunityRole::Admin
        );
    }

    #[tokio::test]
    async fn update_profile_preserves_username_index() {
        let (storage, _temp) = test_db();
        let user_id = Uuid::new_v4();
        let mut profile = profile(user_id, "alice");

        storage.create_profile(&profile).await.unwrap();

        profile.display_name = Some("Alice".to_owned());
        profile.bio = Some("hello".to_owned());
        profile.updated_at = Utc::now();

        storage.update_profile(&profile).await.unwrap();

        let by_username = storage
            .get_profile_by_username("alice")
            .await
            .unwrap()
            .unwrap();

        assert_eq!(by_username.display_name.as_deref(), Some("Alice"));
        assert_eq!(by_username.bio.as_deref(), Some("hello"));
        assert_eq!(by_username.user_id, user_id);
    }

    #[tokio::test]
    async fn update_profile_rejects_username_change() {
        let (storage, _temp) = test_db();
        let user_id = Uuid::new_v4();
        let profile = profile(user_id, "alice");

        storage.create_profile(&profile).await.unwrap();

        let mut tampered = profile.clone();
        tampered.username = "bob".to_owned();
        tampered.normalized_username = "bob".to_owned();

        let result = storage.update_profile(&tampered).await;
        assert!(matches!(result, Err(ApiError::Conflict)));

        let fetched = storage
            .get_profile_by_user_id(&user_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(fetched.username, "alice");
    }

    #[tokio::test]
    async fn update_profile_missing_returns_not_found() {
        let (storage, _temp) = test_db();
        let profile = profile(Uuid::new_v4(), "alice");

        let result = storage.update_profile(&profile).await;
        assert!(matches!(result, Err(ApiError::NotFound)));
    }

    #[tokio::test]
    async fn get_profile_by_username_missing_returns_none() {
        let (storage, _temp) = test_db();

        let result = storage.get_profile_by_username("ghost").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn get_profile_by_user_id_missing_returns_none() {
        let (storage, _temp) = test_db();

        let result = storage
            .get_profile_by_user_id(&Uuid::new_v4())
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn concurrent_create_same_username_one_wins() {
        let (storage, _temp) = test_db();

        let profile_a = profile(Uuid::new_v4(), "alice");
        let profile_b = profile(Uuid::new_v4(), "ALICE");

        let (r1, r2) = tokio::join!(
            storage.create_profile(&profile_a),
            storage.create_profile(&profile_b),
        );

        let results = [r1, r2];
        let ok_count = results.iter().filter(|r| r.is_ok()).count();
        let conflict_count = results
            .iter()
            .filter(|r| matches!(r, Err(ApiError::Conflict)))
            .count();

        assert_eq!(ok_count, 1, "exactly one create must succeed");
        assert_eq!(conflict_count, 1, "exactly one create must conflict");
    }
}
