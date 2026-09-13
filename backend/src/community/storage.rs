//! Community storage abstractions.
//!
//! B-1.2b.2 — CommunityStorage trait + InMemoryCommunityStorage.
//!
//! The storage layer enforces two invariants atomically under a per-username
//! lock:
//!
//! 1. one profile per `user_id`
//! 2. one user_id per `normalized_username`
//!
//! AevumDB does not currently expose a conditional write / CAS primitive, so
//! uniqueness is guaranteed by an application-level per-key mutex combined
//! with a single-instance deployment invariant. See docs/architecture/community-v1.md
//! for the multi-instance backlog (AevumDB-TX-1).

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::Mutex;
use uuid::Uuid;

use crate::community::models::{CommunityProfile, CommunityRole};
use crate::error::ApiError;

/// Persistent storage for community profiles and roles.
///
/// Implementations:
/// - InMemoryCommunityStorage (dev/test)
/// - AevumDbCommunityStorage (production)
pub trait CommunityStorage: Send + Sync {
    /// Create a brand-new profile.
    ///
    /// Fails with `ApiError::Conflict` if either:
    /// - a profile already exists for `profile.user_id`, or
    /// - `profile.normalized_username` is already claimed by another user.
    ///
    /// The check and the write are serialized by a per-username lock, so two
    /// concurrent calls for the same username cannot both succeed.
    fn create_profile(
        &self,
        profile: &CommunityProfile,
    ) -> impl std::future::Future<Output = Result<(), ApiError>> + Send;

    fn get_profile_by_user_id(
        &self,
        user_id: &Uuid,
    ) -> impl std::future::Future<Output = Result<Option<CommunityProfile>, ApiError>> + Send;

    fn get_profile_by_username(
        &self,
        normalized_username: &str,
    ) -> impl std::future::Future<Output = Result<Option<CommunityProfile>, ApiError>> + Send;

    /// Update mutable fields (display_name, bio, updated_at) of an existing
    /// profile. Does not touch the username index — username is immutable.
    fn update_profile(
        &self,
        profile: &CommunityProfile,
    ) -> impl std::future::Future<Output = Result<(), ApiError>> + Send;

    /// Return the community role for a user.
    ///
    /// Missing roles fall back to `CommunityRole::User` — a user without
    /// an explicit role record is not an error.
    fn get_role(
        &self,
        user_id: &Uuid,
    ) -> impl std::future::Future<Output = Result<CommunityRole, ApiError>> + Send;

    fn set_role(
        &self,
        user_id: &Uuid,
        role: CommunityRole,
    ) -> impl std::future::Future<Output = Result<(), ApiError>> + Send;
}

/// In-memory implementation for development and tests.
///
/// MUST NOT be treated as the production persistence layer.
#[derive(Clone, Debug, Default)]
pub struct InMemoryCommunityStorage {
    /// Profiles indexed by user_id.
    profiles: Arc<Mutex<HashMap<Uuid, CommunityProfile>>>,

    /// Username index: normalized_username -> user_id.
    username_index: Arc<Mutex<HashMap<String, Uuid>>>,

    /// Roles indexed by user_id. Absence means default `User`.
    roles: Arc<Mutex<HashMap<Uuid, CommunityRole>>>,

    /// Per-username locks for serializing atomic username claims.
    ///
    /// Policy: never hold two different username locks at the same time.
    username_locks: Arc<Mutex<HashMap<String, Arc<Mutex<()>>>>>,
}

impl InMemoryCommunityStorage {
    pub fn new() -> Self {
        Self::default()
    }

    /// Return the lock associated with a normalized username.
    ///
    /// The same `Arc<Mutex<()>>` is returned for the same key, so callers can
    /// serialize on it. Public for testing purposes only.
    pub async fn lock_for_username(&self, normalized: &str) -> Arc<Mutex<()>> {
        let mut locks = self.username_locks.lock().await;
        locks
            .entry(normalized.to_owned())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    /// Number of stored profiles. Useful for deterministic tests.
    pub async fn profile_count(&self) -> usize {
        self.profiles.lock().await.len()
    }
}

impl CommunityStorage for InMemoryCommunityStorage {
    async fn create_profile(&self, profile: &CommunityProfile) -> Result<(), ApiError> {
        let lock = self.lock_for_username(&profile.normalized_username).await;
        let _guard = lock.lock().await;

        // Invariant 1: user_id must not already have a profile.
        if self.profiles.lock().await.contains_key(&profile.user_id) {
            return Err(ApiError::Conflict);
        }

        // Invariant 2: normalized_username must not already be claimed.
        if self
            .username_index
            .lock()
            .await
            .contains_key(&profile.normalized_username)
        {
            return Err(ApiError::Conflict);
        }

        let mut profiles = self.profiles.lock().await;
        let mut index = self.username_index.lock().await;

        index.insert(profile.normalized_username.clone(), profile.user_id);
        profiles.insert(profile.user_id, profile.clone());

        Ok(())
    }

    async fn get_profile_by_user_id(
        &self,
        user_id: &Uuid,
    ) -> Result<Option<CommunityProfile>, ApiError> {
        Ok(self.profiles.lock().await.get(user_id).cloned())
    }

    async fn get_profile_by_username(
        &self,
        normalized_username: &str,
    ) -> Result<Option<CommunityProfile>, ApiError> {
        let user_id = match self
            .username_index
            .lock()
            .await
            .get(normalized_username)
            .copied()
        {
            Some(id) => id,
            None => return Ok(None),
        };

        Ok(self.profiles.lock().await.get(&user_id).cloned())
    }

    async fn update_profile(&self, profile: &CommunityProfile) -> Result<(), ApiError> {
        let mut profiles = self.profiles.lock().await;

        let existing = profiles.get(&profile.user_id).ok_or(ApiError::NotFound)?;

        // Username is immutable — refuse silent mismatch.
        if existing.normalized_username != profile.normalized_username {
            return Err(ApiError::Conflict);
        }

        profiles.insert(profile.user_id, profile.clone());
        Ok(())
    }

    async fn get_role(&self, user_id: &Uuid) -> Result<CommunityRole, ApiError> {
        Ok(self
            .roles
            .lock()
            .await
            .get(user_id)
            .copied()
            .unwrap_or_default())
    }

    async fn set_role(&self, user_id: &Uuid, role: CommunityRole) -> Result<(), ApiError> {
        self.roles.lock().await.insert(*user_id, role);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

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

    #[tokio::test]
    async fn create_and_get_by_user_id() {
        let storage = InMemoryCommunityStorage::new();
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
        let storage = InMemoryCommunityStorage::new();
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
        let storage = InMemoryCommunityStorage::new();

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
        let storage = InMemoryCommunityStorage::new();
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
        let storage = InMemoryCommunityStorage::new();
        let user_id = Uuid::new_v4();

        let role = storage.get_role(&user_id).await.unwrap();
        assert_eq!(role, CommunityRole::User);
    }

    #[tokio::test]
    async fn set_role_round_trips() {
        let storage = InMemoryCommunityStorage::new();
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
        let storage = InMemoryCommunityStorage::new();
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
        let storage = InMemoryCommunityStorage::new();
        let user_id = Uuid::new_v4();
        let profile = profile(user_id, "alice");

        storage.create_profile(&profile).await.unwrap();

        let mut tampered = profile.clone();
        tampered.username = "bob".to_owned();
        tampered.normalized_username = "bob".to_owned();

        let result = storage.update_profile(&tampered).await;
        assert!(matches!(result, Err(ApiError::Conflict)));

        // Original still intact.
        let fetched = storage
            .get_profile_by_user_id(&user_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(fetched.username, "alice");
    }

    #[tokio::test]
    async fn update_profile_missing_returns_not_found() {
        let storage = InMemoryCommunityStorage::new();
        let profile = profile(Uuid::new_v4(), "alice");

        let result = storage.update_profile(&profile).await;
        assert!(matches!(result, Err(ApiError::NotFound)));
    }

    #[tokio::test]
    async fn get_profile_by_username_missing_returns_none() {
        let storage = InMemoryCommunityStorage::new();

        let result = storage.get_profile_by_username("ghost").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn get_profile_by_user_id_missing_returns_none() {
        let storage = InMemoryCommunityStorage::new();

        let result = storage
            .get_profile_by_user_id(&Uuid::new_v4())
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn concurrent_create_same_username_one_wins() {
        let storage = InMemoryCommunityStorage::new();

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

        assert_eq!(storage.profile_count().await, 1);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn concurrent_create_same_user_id_one_wins() {
        let storage = InMemoryCommunityStorage::new();
        let user_id = Uuid::new_v4();

        let profile_a = profile(user_id, "alice");
        let profile_b = profile(user_id, "bob");

        // Different usernames — different locks. The user_id invariant must
        // still hold because both check `profiles.contains_key(user_id)`
        // inside the shared profiles mutex.
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
        assert_eq!(storage.profile_count().await, 1);
    }
}
