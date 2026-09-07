// Aevum Platform — authentication data models.

use base64::Engine;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::auth::csrf::CsrfTokenHash;
use crate::auth::password::{PasswordHasher, SessionToken};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub password_hash: String,
    pub status: UserStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserStatus {
    Active,
    Suspended,
    Deleted,
}

impl UserStatus {
    pub const fn can_authenticate(self) -> bool {
        matches!(self, Self::Active)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token_hash: SessionTokenHash,
    pub csrf_token_hash: Option<CsrfTokenHash>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub issued_at: DateTime<Utc>,
    pub last_rotated_at: DateTime<Utc>,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
}

impl Session {
    pub fn new(user_id: Uuid, token_hash: SessionTokenHash, expires_at: DateTime<Utc>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            user_id,
            token_hash,
            csrf_token_hash: None,
            created_at: now,
            expires_at,
            revoked_at: None,
            issued_at: now,
            last_rotated_at: now,
            last_seen_at: None,
            user_agent: None,
            ip_address: None,
        }
    }

    pub fn is_expired_at(&self, now: DateTime<Utc>) -> bool {
        now >= self.expires_at
    }

    pub const fn is_revoked(&self) -> bool {
        self.revoked_at.is_some()
    }

    pub fn is_valid_at(&self, now: DateTime<Utc>) -> bool {
        !self.is_revoked() && !self.is_expired_at(now)
    }

    pub fn is_valid(&self) -> bool {
        self.is_valid_at(Utc::now())
    }

    pub fn revoke(&mut self, now: DateTime<Utc>) {
        if self.revoked_at.is_none() {
            self.revoked_at = Some(now);
        }
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionTokenHash(String);

impl SessionTokenHash {
    pub(crate) fn new(hash: String) -> Self {
        Self(hash)
    }

    pub fn from_token(token: &SessionToken) -> Self {
        let digest = Sha256::digest(token.expose().as_bytes());
        Self(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for SessionTokenHash {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("SessionTokenHash(REDACTED)")
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoginCredentials {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordResetToken {
    pub user_id: Uuid,
    pub token_hash: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub used_at: Option<DateTime<Utc>>,
}

impl PasswordResetToken {
    pub fn new(user_id: Uuid, token_hash: String, ttl_minutes: i64) -> Self {
        let now = Utc::now();
        Self {
            user_id,
            token_hash,
            created_at: now,
            expires_at: now + chrono::Duration::minutes(ttl_minutes),
            used_at: None,
        }
    }

    pub fn is_expired_at(&self, now: DateTime<Utc>) -> bool {
        now >= self.expires_at
    }

    pub fn is_used(&self) -> bool {
        self.used_at.is_some()
    }

    pub fn is_valid_at(&self, now: DateTime<Utc>) -> bool {
        !self.is_expired_at(now) && !self.is_used()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterResponse {
    pub user_id: Uuid,
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    pub user_id: Uuid,
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeResponse {
    pub user_id: Uuid,
    pub email: String,
    pub status: UserStatus,
    pub created_at: DateTime<Utc>,
}

impl User {
    pub fn new(email: String, password_hash: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            email,
            password_hash,
            status: UserStatus::Active,
            created_at: now,
            updated_at: now,
        }
    }
}
