//! Aevum Platform — AUTH-25 Two-Factor Authentication
//!
//! TOTP settings and domain-separated object-id derivation.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Lifecycle state of two-factor authentication for a user.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TwoFactorState {
    /// 2FA is not configured.
    Disabled,
    /// Secret has been generated but not yet confirmed by the user.
    Pending,
    /// 2FA is active and required for login.
    Enabled,
}

/// Persistent two-factor settings for a user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwoFactorSettings {
    pub user_id: Uuid,
    /// Encrypted TOTP secret (never plaintext at rest).
    pub encrypted_secret: Vec<u8>,
    pub state: TwoFactorState,
    pub created_at: DateTime<Utc>,
    /// Pending secrets expire after a fixed TTL.
    /// Once enabled, this field is unused.
    pub expires_at: Option<DateTime<Utc>>,
    pub enabled_at: Option<DateTime<Utc>>,
    /// Highest TOTP step accepted so far; used for replay protection.
    pub last_accepted_step: Option<u64>,
}

impl TwoFactorSettings {
    /// Pending secrets must be confirmed within this window.
    pub const PENDING_TTL_SECONDS: i64 = 10 * 60;

    pub fn new_pending(user_id: Uuid, encrypted_secret: Vec<u8>) -> Self {
        let now = Utc::now();
        Self {
            user_id,
            encrypted_secret,
            state: TwoFactorState::Pending,
            created_at: now,
            expires_at: Some(now + chrono::Duration::seconds(Self::PENDING_TTL_SECONDS)),
            enabled_at: None,
            last_accepted_step: None,
        }
    }

    pub fn is_pending_expired(&self, now: DateTime<Utc>) -> bool {
        matches!(self.state, TwoFactorState::Pending)
            && self.expires_at.map_or(false, |exp| now >= exp)
    }
}

/// Domain-separated object-id derivation for TOTP secrets.
///
/// Each secret purpose receives its own derivation domain so that
/// the same user_id cannot collide across different secret types
/// (TOTP, API keys, OAuth secrets, etc.).
pub struct TotpSecretObjectId;

impl TotpSecretObjectId {
    /// Derivation domain for TOTP secret storage.
    const DOMAIN: &'static [u8] = b"AEVUM_AUTH_TOTP_SECRET_V1";

    /// Deterministic object_id for a user's TOTP secret.
    ///
    /// Same user always maps to the same object_id.
    /// Different secret purposes map to different object_ids.
    pub fn for_user(user_id: &Uuid) -> u64 {
        let mut hasher = Sha256::new();
        hasher.update(Self::DOMAIN);
        hasher.update(user_id.as_bytes());
        let digest = hasher.finalize();

        u64::from_be_bytes([
            digest[0], digest[1], digest[2], digest[3],
            digest[4], digest[5], digest[6], digest[7],
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn object_id_is_deterministic() {
        let user_id = Uuid::new_v4();
        let a = TotpSecretObjectId::for_user(&user_id);
        let b = TotpSecretObjectId::for_user(&user_id);
        assert_eq!(a, b);
    }

    #[test]
    fn object_id_differs_between_users() {
        let u1 = Uuid::new_v4();
        let u2 = Uuid::new_v4();
        assert_ne!(
            TotpSecretObjectId::for_user(&u1),
            TotpSecretObjectId::for_user(&u2)
        );
    }
}
