//! Authentication storage implementations.
//!
//! AUTH-06
//!
//! The authentication service depends on the `AuthStorage` trait.
//! This module currently provides an in-memory implementation for
//! deterministic development and tests.
//!
//! Production storage will implement the same trait using AevumDB.
//!
//! Security invariants:
//! - raw passwords are never stored here
//! - raw session tokens are never stored here
//! - session lookup uses token hashes only
//! - email uniqueness is enforced atomically
//! - revoked sessions remain persisted so revocation survives
//!   subsequent authentication attempts

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::auth::models::{PasswordResetToken, Session, SessionTokenHash, User};
use crate::auth::service::AuthStorage;
use crate::error::ApiError;

/// In-memory authentication storage.
///
/// This implementation is intentionally simple and exists for:
/// - unit tests
/// - integration tests
/// - local development
/// - service wiring before AevumDB integration
///
/// It MUST NOT be treated as the production persistence layer.
#[derive(Clone, Debug, Default)]
pub struct InMemoryAuthStorage {
    /// All user records indexed by normalized email.
    users: Arc<Mutex<HashMap<String, User>>>,

    /// Reverse user index.
    users_by_id: Arc<Mutex<HashMap<Uuid, String>>>,

    /// Sessions indexed by token hash.
    sessions: Arc<Mutex<HashMap<String, Session>>>,

    /// Password reset tokens indexed by token hash.
    password_reset_tokens: Arc<Mutex<HashMap<String, PasswordResetToken>>>,
}

impl InMemoryAuthStorage {
    /// Create an empty storage.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return the number of stored users.
    ///
    /// Primarily useful for deterministic tests.
    pub async fn user_count(&self) -> usize {
        self.users.lock().await.len()
    }

    /// Return the number of stored sessions.
    ///
    /// Primarily useful for deterministic tests.
    pub async fn session_count(&self) -> usize {
        self.sessions.lock().await.len()
    }
}

impl AuthStorage for InMemoryAuthStorage {
    async fn user_exists(&self, email: &str) -> Result<bool, ApiError> {
        let users = self.users.lock().await;
        Ok(users.contains_key(email))
    }

    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>, ApiError> {
        let users = self.users.lock().await;
        Ok(users.get(email).cloned())
    }

    async fn get_user_by_id(&self, user_id: &Uuid) -> Result<Option<User>, ApiError> {
        // IMPORTANT:
        // Lock order is always:
        // users -> users_by_id
        //
        // This avoids the inverse-lock-order deadlock that would occur
        // if another operation acquired users_by_id first.
        let users = self.users.lock().await;

        let users_by_id = self.users_by_id.lock().await;

        let Some(email) = users_by_id.get(user_id) else {
            return Ok(None);
        };

        Ok(users.get(email).cloned())
    }

    async fn update_user(&self, user: &User) -> Result<(), ApiError> {
        let mut users = self.users.lock().await;

        if let Some(existing) = users.get_mut(&user.email) {
            *existing = user.clone();
            Ok(())
        } else {
            Err(ApiError::NotFound)
        }
    }

    async fn create_user(&self, user: &User) -> Result<(), ApiError> {
        // IMPORTANT:
        // Lock order is identical to get_user_by_id():
        // users -> users_by_id
        let mut users = self.users.lock().await;
        let mut users_by_id = self.users_by_id.lock().await;

        // The storage layer remains authoritative for uniqueness.
        //
        // AuthService also performs a fast pre-check, but this check
        // protects against concurrent registration requests.
        if users.contains_key(&user.email) {
            return Err(ApiError::Conflict);
        }

        if users_by_id.contains_key(&user.id) {
            return Err(ApiError::Conflict);
        }

        users.insert(user.email.clone(), user.clone());

        users_by_id.insert(user.id, user.email.clone());

        Ok(())
    }

    async fn create_session(&self, session: &Session) -> Result<(), ApiError> {
        let mut sessions = self.sessions.lock().await;

        let key = session.token_hash.as_str().to_owned();

        // Token hashes are expected to be unique because the raw token
        // has cryptographically strong random entropy.
        //
        // Treating an existing hash as a conflict protects against
        // accidental overwrites.
        if sessions.contains_key(&key) {
            return Err(ApiError::Conflict);
        }

        sessions.insert(key, session.clone());

        Ok(())
    }

    async fn get_session_by_token_hash(
        &self,
        token_hash: &SessionTokenHash,
    ) -> Result<Option<Session>, ApiError> {
        let sessions = self.sessions.lock().await;

        Ok(sessions.get(token_hash.as_str()).cloned())
    }

    async fn create_password_reset_token(&self, token: &PasswordResetToken) -> Result<(), ApiError> {
        let mut tokens = self.password_reset_tokens.lock().await;
        tokens.insert(token.token_hash.clone(), token.clone());
        Ok(())
    }

    async fn get_password_reset_token_by_hash(
        &self,
        token_hash: &str,
    ) -> Result<Option<PasswordResetToken>, ApiError> {
        let tokens = self.password_reset_tokens.lock().await;
        Ok(tokens.get(token_hash).cloned())
    }

    async fn consume_password_reset_token(
        &self,
        token_hash: &str,
        used_at: DateTime<Utc>,
    ) -> Result<(), ApiError> {
        let mut tokens = self.password_reset_tokens.lock().await;
        if let Some(token) = tokens.get_mut(token_hash) {
            if token.used_at.is_none() {
                token.used_at = Some(used_at);
            }
        }
        Ok(())
    }

    async fn revoke_session_by_token_hash(
        &self,
        token_hash: &SessionTokenHash,
        revoked_at: DateTime<Utc>,
    ) -> Result<(), ApiError> {
        let mut sessions = self.sessions.lock().await;

        if let Some(session) = sessions.get_mut(token_hash.as_str()) {
            // Session::revoke() is expected to be idempotent.
            session.revoke(revoked_at);
        }

        // Logout is intentionally idempotent:
        // unknown/already-revoked sessions do not leak information.
        Ok(())
    }

    async fn revoke_all_sessions_for_user(
        &self,
        user_id: &Uuid,
        revoked_at: DateTime<Utc>,
    ) -> Result<usize, ApiError> {
        let mut sessions = self.sessions.lock().await;
        let mut revoked_count = 0;

        for session in sessions.values_mut() {
            if &session.user_id == user_id && session.revoked_at.is_none() {
                session.revoke(revoked_at);
                revoked_count += 1;
            }
        }

        Ok(revoked_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[tokio::test]
    async fn user_lifecycle() {
        let storage = InMemoryAuthStorage::new();

        let user = User::new(
            "test@example.com".to_string(),
            "hashed_password".to_string(),
        );

        assert!(!storage.user_exists("test@example.com").await.unwrap());

        storage.create_user(&user).await.unwrap();

        assert!(storage.user_exists("test@example.com").await.unwrap());

        let by_email = storage
            .get_user_by_email("test@example.com")
            .await
            .unwrap()
            .unwrap();

        assert_eq!(by_email.id, user.id);
        assert_eq!(by_email.email, user.email);

        let by_id = storage.get_user_by_id(&user.id).await.unwrap().unwrap();

        assert_eq!(by_id.id, user.id);

        assert_eq!(storage.user_count().await, 1);
    }

    #[tokio::test]
    async fn duplicate_email_is_conflict() {
        let storage = InMemoryAuthStorage::new();

        let user1 = User::new("test@example.com".to_string(), "hash1".to_string());

        let user2 = User::new("test@example.com".to_string(), "hash2".to_string());

        storage.create_user(&user1).await.unwrap();

        let result = storage.create_user(&user2).await;

        assert!(matches!(result, Err(ApiError::Conflict)));
    }

    #[tokio::test]
    async fn duplicate_user_id_is_conflict() {
        let storage = InMemoryAuthStorage::new();

        let user = User::new("first@example.com".to_string(), "hash".to_string());

        storage.create_user(&user).await.unwrap();

        let duplicate_id = User {
            id: user.id,
            email: "second@example.com".to_string(),
            password_hash: "hash".to_string(),
            status: user.status,
            created_at: user.created_at,
            updated_at: user.updated_at,
        };

        let result = storage.create_user(&duplicate_id).await;

        assert!(matches!(result, Err(ApiError::Conflict)));
    }

    #[tokio::test]
    async fn session_lifecycle() {
        let storage = InMemoryAuthStorage::new();

        let user = User::new(
            "test@example.com".to_string(),
            "hashed_password".to_string(),
        );

        storage.create_user(&user).await.unwrap();

        let token_hash = SessionTokenHash::new("test_token_hash".to_string());

        let expires_at = Utc::now() + Duration::hours(1);

        let session = Session::new(user.id, token_hash.clone(), expires_at);

        storage.create_session(&session).await.unwrap();

        assert_eq!(storage.session_count().await, 1);

        let retrieved = storage
            .get_session_by_token_hash(&token_hash)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(retrieved.user_id, user.id);

        assert!(retrieved.revoked_at.is_none());

        let revoked_at = Utc::now();

        storage
            .revoke_session_by_token_hash(&token_hash, revoked_at)
            .await
            .unwrap();

        let revoked = storage
            .get_session_by_token_hash(&token_hash)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(revoked.revoked_at, Some(revoked_at));
    }

    #[tokio::test]
    async fn logout_unknown_session_is_idempotent() {
        let storage = InMemoryAuthStorage::new();

        let token_hash = SessionTokenHash::new("unknown_token".to_string());

        let result = storage
            .revoke_session_by_token_hash(&token_hash, Utc::now())
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn duplicate_session_hash_is_conflict() {
        let storage = InMemoryAuthStorage::new();

        let user = User::new("test@example.com".to_string(), "hash".to_string());

        storage.create_user(&user).await.unwrap();

        let token_hash = SessionTokenHash::new("same_hash".to_string());

        let expires_at = Utc::now() + Duration::hours(1);

        let session1 = Session::new(user.id, token_hash.clone(), expires_at);

        let session2 = Session::new(user.id, token_hash.clone(), expires_at);

        storage.create_session(&session1).await.unwrap();

        let result = storage.create_session(&session2).await;

        assert!(matches!(result, Err(ApiError::Conflict)));
    }
}
