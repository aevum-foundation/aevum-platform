//! AevumDB-backed authentication storage.

use crate::auth::models::{Session, SessionTokenHash, User};
use crate::auth::service::AuthStorage;
use crate::error::ApiError;
use aevum_db::{AevumDb, DbConfig, DbError, DbRuntime};
use chrono::{DateTime, Utc};
use std::sync::Arc;
use uuid::Uuid;

const USER_EMAIL_PREFIX: &str = "platform:user:email:";
const USER_ID_PREFIX: &str = "platform:user:id:";
const SESSION_TOKEN_PREFIX: &str = "platform:session:token:";
const SESSION_ID_PREFIX: &str = "platform:session:id:";
const SESSION_BY_USER_PREFIX: &str = "platform:session:by_user:";

#[derive(Clone)]
pub struct AevumDbAuthStorage {
    db: Arc<AevumDb>,
}

impl std::fmt::Debug for AevumDbAuthStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AevumDbAuthStorage")
            .field("db", &"<redacted>")
            .finish()
    }
}

impl AevumDbAuthStorage {
    pub fn open(config: DbConfig, runtime: DbRuntime) -> Result<Self, ApiError> {
        let db = AevumDb::open(config, runtime).map_err(|error| {
            log::error!("AevumDB open failed: {}", error);
            ApiError::Internal
        })?;
        Ok(Self { db: Arc::new(db) })
    }

    fn user_email_key(email: &str) -> String {
        format!("{}{}", USER_EMAIL_PREFIX, email)
    }

    fn user_id_key(user_id: &Uuid) -> String {
        format!("{}{}", USER_ID_PREFIX, user_id)
    }

    fn session_token_key(token_hash: &SessionTokenHash) -> String {
        format!("{}{}", SESSION_TOKEN_PREFIX, token_hash.as_str())
    }

    fn session_token_key_from_hash(hash: &str) -> String {
        format!("{}{}", SESSION_TOKEN_PREFIX, hash)
    }

    fn session_id_key(session_id: &Uuid) -> String {
        format!("{}{}", SESSION_ID_PREFIX, session_id)
    }

    fn session_by_user_key(user_id: &Uuid, session_id: &Uuid) -> String {
        format!("{}{}:{}", SESSION_BY_USER_PREFIX, user_id, session_id)
    }

    fn session_by_user_prefix(user_id: &Uuid) -> String {
        format!("{}{}:", SESSION_BY_USER_PREFIX, user_id)
    }

    fn serialize<T: serde::Serialize>(value: &T) -> Result<Vec<u8>, ApiError> {
        serde_json::to_vec(value).map_err(|error| {
            log::error!("Serialization failed: {}", error);
            ApiError::Internal
        })
    }

    fn deserialize<T: serde::de::DeserializeOwned>(data: &[u8]) -> Result<T, ApiError> {
        serde_json::from_slice(data).map_err(|error| {
            log::error!("Deserialization failed: {}", error);
            ApiError::Internal
        })
    }

    fn map_db_error(error: DbError) -> ApiError {
        log::error!("AevumDB error: {}", error);
        ApiError::Internal
    }
}

impl AuthStorage for AevumDbAuthStorage {
    async fn user_exists(&self, email: &str) -> Result<bool, ApiError> {
        let key = Self::user_email_key(email);
        self.db
            .get(key.as_bytes())
            .map(|v| v.is_some())
            .map_err(Self::map_db_error)
    }

    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>, ApiError> {
        let key = Self::user_email_key(email);
        let data = self.db.get(key.as_bytes()).map_err(Self::map_db_error)?;
        data.map(|d| Self::deserialize(&d)).transpose()
    }

    async fn get_user_by_id(&self, user_id: &Uuid) -> Result<Option<User>, ApiError> {
        let key = Self::user_id_key(user_id);
        let data = self.db.get(key.as_bytes()).map_err(Self::map_db_error)?;
        data.map(|d| Self::deserialize(&d)).transpose()
    }

    async fn create_user(&self, user: &User) -> Result<(), ApiError> {
        let email_key = Self::user_email_key(&user.email);
        let id_key = Self::user_id_key(&user.id);
        let user_data = Self::serialize(user)?;

        // Проверяем существование email
        if self
            .db
            .get(email_key.as_bytes())
            .map_err(Self::map_db_error)?
            .is_some()
        {
            return Err(ApiError::Conflict);
        }

        let mut batch = self.db.batch();
        batch.put(email_key.as_bytes(), &user_data);
        batch.put(id_key.as_bytes(), &user_data);
        batch.commit().map_err(Self::map_db_error)?;
        Ok(())
    }

    async fn create_session(&self, session: &Session) -> Result<(), ApiError> {
        let token_key = Self::session_token_key(&session.token_hash);
        let id_key = Self::session_id_key(&session.id);
        let session_data = Self::serialize(session)?;

        let mut batch = self.db.batch();
        batch.put(token_key.as_bytes(), &session_data);
        batch.put(id_key.as_bytes(), &session_data);
        batch.commit().map_err(Self::map_db_error)?;
        Ok(())
    }

    async fn get_session_by_token_hash(
        &self,
        token_hash: &SessionTokenHash,
    ) -> Result<Option<Session>, ApiError> {
        let key = Self::session_token_key(token_hash);
        let data = self.db.get(key.as_bytes()).map_err(Self::map_db_error)?;
        data.map(|d| Self::deserialize(&d)).transpose()
    }

    async fn revoke_session_by_token_hash(
        &self,
        token_hash: &SessionTokenHash,
        revoked_at: DateTime<Utc>,
    ) -> Result<(), ApiError> {
        let token_key = Self::session_token_key(token_hash);
        let data = self
            .db
            .get(token_key.as_bytes())
            .map_err(Self::map_db_error)?;
        let data = data.ok_or(ApiError::NotFound)?;
        let mut session: Session = Self::deserialize(&data)?;
        session.revoke(revoked_at);

        let session_data = Self::serialize(&session)?;
        let id_key = Self::session_id_key(&session.id);

        let mut batch = self.db.batch();
        batch.put(token_key.as_bytes(), &session_data);
        batch.put(id_key.as_bytes(), &session_data);
        batch.commit().map_err(Self::map_db_error)?;
        Ok(())
    }

    async fn revoke_all_sessions_for_user(
        &self,
        user_id: &Uuid,
        revoked_at: DateTime<Utc>,
    ) -> Result<usize, ApiError> {
        let prefix = Self::session_by_user_prefix(user_id);
        let entries = self
            .db
            .prefix_scan(prefix.as_bytes())
            .map_err(Self::map_db_error)?;

        let mut batch = self.db.batch();
        let mut revoked_count = 0;

        for (index_key, token_hash_bytes) in entries {
            let token_hash_str = String::from_utf8_lossy(&token_hash_bytes);

            let token_key = Self::session_token_key_from_hash(&token_hash_str);
            let Some(data) = self.db.get(token_key.as_bytes()).map_err(Self::map_db_error)? else {
                // Stale index: no corresponding session exists
                batch.delete(&index_key);
                continue;
            };

            let mut session: Session = Self::deserialize(&data)?;
            if session.revoked_at.is_some() {
                continue;
            }

            session.revoke(revoked_at);

            let session_data = Self::serialize(&session)?;
            let id_key = Self::session_id_key(&session.id);

            batch.put(token_key.as_bytes(), &session_data);
            batch.put(id_key.as_bytes(), &session_data);
            revoked_count += 1;
        }

        batch.commit().map_err(Self::map_db_error)?;
        Ok(revoked_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aevum_db::config::SyncMode;
    use chrono::Duration;
    use tempfile::TempDir;

    fn test_db() -> (AevumDbAuthStorage, TempDir) {
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
        let storage = AevumDbAuthStorage::open(config, runtime).unwrap();
        (storage, temp)
    }

    #[tokio::test]
    async fn user_lifecycle() {
        let (storage, _temp) = test_db();
        let user = User::new("test@example.com".to_string(), "hashed".to_string());

        storage.create_user(&user).await.unwrap();
        assert!(storage.user_exists("test@example.com").await.unwrap());

        let by_email = storage
            .get_user_by_email("test@example.com")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(by_email.id, user.id);

        let by_id = storage.get_user_by_id(&user.id).await.unwrap().unwrap();
        assert_eq!(by_id.email, user.email);
    }

    #[tokio::test]
    async fn duplicate_email_is_conflict() {
        let (storage, _temp) = test_db();
        let user1 = User::new("test@example.com".to_string(), "hash1".to_string());
        let user2 = User::new("test@example.com".to_string(), "hash2".to_string());

        storage.create_user(&user1).await.unwrap();
        let result = storage.create_user(&user2).await;
        assert!(matches!(result, Err(ApiError::Conflict)));
    }

    #[tokio::test]
    async fn session_lifecycle() {
        let (storage, _temp) = test_db();
        let user = User::new("test@example.com".to_string(), "hashed".to_string());
        storage.create_user(&user).await.unwrap();

        let token_hash = SessionTokenHash::new("test_hash".to_string());
        let expires_at = Utc::now() + Duration::hours(1);
        let session = Session::new(user.id, token_hash.clone(), expires_at);

        storage.create_session(&session).await.unwrap();

        let found = storage
            .get_session_by_token_hash(&token_hash)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(found.id, session.id);
        assert!(found.revoked_at.is_none());

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
        let (storage, _temp) = test_db();
        let token_hash = SessionTokenHash::new("unknown".to_string());
        let result = storage
            .revoke_session_by_token_hash(&token_hash, Utc::now())
            .await;
        assert!(matches!(result, Err(ApiError::NotFound)));
    }

    #[test]
    fn key_layout_is_stable() {
        let user_id = Uuid::nil();
        assert_eq!(
            AevumDbAuthStorage::user_email_key("alice@example.com"),
            "platform:user:email:alice@example.com"
        );
        assert_eq!(
            AevumDbAuthStorage::user_id_key(&user_id),
            format!("platform:user:id:{}", user_id)
        );
        let hash = SessionTokenHash::new("abc".to_string());
        assert_eq!(
            AevumDbAuthStorage::session_token_key(&hash),
            "platform:session:token:abc"
        );
    }
}
