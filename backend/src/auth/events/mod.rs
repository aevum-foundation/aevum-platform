//! Security events for the authentication subsystem.
//!
//! AUTH-16 will wire these into a persistent audit log.
//! The enum provides a stable vocabulary for rate limiter,
//! session rotation, and future security features.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub mod aevumdb;
pub mod storage;

/// Severity level for security events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecuritySeverity {
    Info,
    Warning,
    Critical,
}

/// Structured reason for login failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoginFailureReason {
    InvalidPassword,
    UnknownUser,
    AccountDisabled,
    SessionExpired,
    RateLimited,
}

impl LoginFailureReason {
    pub const fn as_str(self) -> &'static str {
        match self {
            LoginFailureReason::InvalidPassword => "invalid_password",
            LoginFailureReason::UnknownUser => "unknown_user",
            LoginFailureReason::AccountDisabled => "account_disabled",
            LoginFailureReason::SessionExpired => "session_expired",
            LoginFailureReason::RateLimited => "rate_limited",
        }
    }
}

/// Event kind for indexing and filtering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SecurityEventKind {
    UserRegistered,
    LoginSuccess,
    LoginFailed,
    LoginRateLimited,
    RegisterRateLimited,
    SessionCreated,
    SessionRevoked,
    AllSessionsRevoked,
    SessionRotated,
    PasswordChanged,
    PasswordResetRequested,
    PasswordResetCompleted,
    TwoFactorEnabled,
    TwoFactorDisabled,
    TwoFactorVerificationFailed,
    EmailChanged,
    AccountLocked,
    AccountUnlocked,
}

impl SecurityEventKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            SecurityEventKind::UserRegistered => "USER_REGISTERED",
            SecurityEventKind::LoginSuccess => "LOGIN_SUCCESS",
            SecurityEventKind::LoginFailed => "LOGIN_FAILED",
            SecurityEventKind::LoginRateLimited => "LOGIN_RATE_LIMITED",
            SecurityEventKind::RegisterRateLimited => "REGISTER_RATE_LIMITED",
            SecurityEventKind::SessionCreated => "SESSION_CREATED",
            SecurityEventKind::SessionRevoked => "SESSION_REVOKED",
            SecurityEventKind::AllSessionsRevoked => "ALL_SESSIONS_REVOKED",
            SecurityEventKind::SessionRotated => "SESSION_ROTATED",
            SecurityEventKind::PasswordChanged => "PASSWORD_CHANGED",
            SecurityEventKind::PasswordResetRequested => "PASSWORD_RESET_REQUESTED",
            SecurityEventKind::PasswordResetCompleted => "PASSWORD_RESET_COMPLETED",
            SecurityEventKind::TwoFactorEnabled => "TWO_FACTOR_ENABLED",
            SecurityEventKind::TwoFactorDisabled => "TWO_FACTOR_DISABLED",
            SecurityEventKind::TwoFactorVerificationFailed => "TWO_FACTOR_VERIFICATION_FAILED",
            SecurityEventKind::EmailChanged => "EMAIL_CHANGED",
            SecurityEventKind::AccountLocked => "ACCOUNT_LOCKED",
            SecurityEventKind::AccountUnlocked => "ACCOUNT_UNLOCKED",
        }
    }
}

/// Common metadata for all security events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityMetadata {
    pub event_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub client_ip: Option<String>,
    pub user_agent: Option<String>,
}

impl SecurityMetadata {
    pub fn new(client_ip: Option<String>, user_agent: Option<String>) -> Self {
        Self {
            event_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            client_ip,
            user_agent,
        }
    }
}

/// A stored security event with its own record ID and timestamp.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityEventRecord {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub event: SecurityEvent,
}

impl SecurityEventRecord {
    pub fn new(event: SecurityEvent) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: event.timestamp(),
            event,
        }
    }
}

/// Security events tracked by the authentication subsystem.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecurityEvent {
    // Account lifecycle
    UserRegistered {
        metadata: SecurityMetadata,
        user_id: Uuid,
        email: String,
    },

    // Authentication
    LoginSuccess {
        metadata: SecurityMetadata,
        user_id: Uuid,
        email: String,
    },

    LoginFailed {
        metadata: SecurityMetadata,
        email: String,
        reason: LoginFailureReason,
    },

    LoginRateLimited {
        metadata: SecurityMetadata,
        email: Option<String>,
    },

    RegisterRateLimited {
        metadata: SecurityMetadata,
    },

    // Session lifecycle
    SessionCreated {
        metadata: SecurityMetadata,
        user_id: Uuid,
        session_id: Uuid,
    },

    SessionRevoked {
        metadata: SecurityMetadata,
        user_id: Uuid,
        session_id: Uuid,
    },

    AllSessionsRevoked {
        metadata: SecurityMetadata,
        user_id: Uuid,
        revoked_count: usize,
    },

    SessionRotated {
        metadata: SecurityMetadata,
        user_id: Uuid,
        old_session_id: Uuid,
        new_session_id: Uuid,
    },

    // Password lifecycle (future AUTH-20/AUTH-21)
    PasswordChanged {
        metadata: SecurityMetadata,
        user_id: Uuid,
    },

    PasswordResetRequested {
        metadata: SecurityMetadata,
        email: String,
    },

    PasswordResetCompleted {
        metadata: SecurityMetadata,
        user_id: Uuid,
    },

    // 2FA (future AUTH-23/AUTH-24)
    TwoFactorEnabled {
        metadata: SecurityMetadata,
        user_id: Uuid,
    },

    TwoFactorDisabled {
        metadata: SecurityMetadata,
        user_id: Uuid,
    },

    TwoFactorVerificationFailed {
        metadata: SecurityMetadata,
        user_id: Uuid,
    },

    // Account modifications
    EmailChanged {
        metadata: SecurityMetadata,
        user_id: Uuid,
        old_email: String,
        new_email: String,
    },

    AccountLocked {
        metadata: SecurityMetadata,
        user_id: Uuid,
    },

    AccountUnlocked {
        metadata: SecurityMetadata,
        user_id: Uuid,
    },
}

impl SecurityEvent {
    pub fn user_registered(user_id: Uuid, email: String, metadata: SecurityMetadata) -> Self {
        Self::UserRegistered {
            metadata,
            user_id,
            email,
        }
    }

    pub fn login_success(user_id: Uuid, email: String, metadata: SecurityMetadata) -> Self {
        Self::LoginSuccess {
            metadata,
            user_id,
            email,
        }
    }

    pub fn login_failed(
        email: String,
        reason: LoginFailureReason,
        metadata: SecurityMetadata,
    ) -> Self {
        Self::LoginFailed {
            metadata,
            email,
            reason,
        }
    }

    pub fn login_rate_limited(
        email: Option<String>,
        metadata: SecurityMetadata,
    ) -> Self {
        Self::LoginRateLimited { metadata, email }
    }

    pub fn register_rate_limited(metadata: SecurityMetadata) -> Self {
        Self::RegisterRateLimited { metadata }
    }

    pub fn session_created(
        user_id: Uuid,
        session_id: Uuid,
        metadata: SecurityMetadata,
    ) -> Self {
        Self::SessionCreated {
            metadata,
            user_id,
            session_id,
        }
    }

    pub fn session_revoked(
        user_id: Uuid,
        session_id: Uuid,
        metadata: SecurityMetadata,
    ) -> Self {
        Self::SessionRevoked {
            metadata,
            user_id,
            session_id,
        }
    }

    pub fn all_sessions_revoked(
        user_id: Uuid,
        revoked_count: usize,
        metadata: SecurityMetadata,
    ) -> Self {
        Self::AllSessionsRevoked {
            metadata,
            user_id,
            revoked_count,
        }
    }

    pub fn session_rotated(
        user_id: Uuid,
        old_session_id: Uuid,
        new_session_id: Uuid,
        metadata: SecurityMetadata,
    ) -> Self {
        Self::SessionRotated {
            metadata,
            user_id,
            old_session_id,
            new_session_id,
        }
    }

    pub fn kind(&self) -> SecurityEventKind {
        match self {
            SecurityEvent::UserRegistered { .. } => SecurityEventKind::UserRegistered,
            SecurityEvent::LoginSuccess { .. } => SecurityEventKind::LoginSuccess,
            SecurityEvent::LoginFailed { .. } => SecurityEventKind::LoginFailed,
            SecurityEvent::LoginRateLimited { .. } => SecurityEventKind::LoginRateLimited,
            SecurityEvent::RegisterRateLimited { .. } => SecurityEventKind::RegisterRateLimited,
            SecurityEvent::SessionCreated { .. } => SecurityEventKind::SessionCreated,
            SecurityEvent::SessionRevoked { .. } => SecurityEventKind::SessionRevoked,
            SecurityEvent::AllSessionsRevoked { .. } => SecurityEventKind::AllSessionsRevoked,
            SecurityEvent::SessionRotated { .. } => SecurityEventKind::SessionRotated,
            SecurityEvent::PasswordChanged { .. } => SecurityEventKind::PasswordChanged,
            SecurityEvent::PasswordResetRequested { .. } => {
                SecurityEventKind::PasswordResetRequested
            }
            SecurityEvent::PasswordResetCompleted { .. } => {
                SecurityEventKind::PasswordResetCompleted
            }
            SecurityEvent::TwoFactorEnabled { .. } => SecurityEventKind::TwoFactorEnabled,
            SecurityEvent::TwoFactorDisabled { .. } => SecurityEventKind::TwoFactorDisabled,
            SecurityEvent::TwoFactorVerificationFailed { .. } => {
                SecurityEventKind::TwoFactorVerificationFailed
            }
            SecurityEvent::EmailChanged { .. } => SecurityEventKind::EmailChanged,
            SecurityEvent::AccountLocked { .. } => SecurityEventKind::AccountLocked,
            SecurityEvent::AccountUnlocked { .. } => SecurityEventKind::AccountUnlocked,
        }
    }

    pub fn event_name(&self) -> &'static str {
        self.kind().as_str()
    }

    pub fn severity(&self) -> SecuritySeverity {
        match self {
            SecurityEvent::UserRegistered { .. }
            | SecurityEvent::LoginSuccess { .. }
            | SecurityEvent::SessionCreated { .. }
            | SecurityEvent::SessionRotated { .. }
            | SecurityEvent::PasswordChanged { .. }
            | SecurityEvent::PasswordResetCompleted { .. }
            | SecurityEvent::TwoFactorEnabled { .. }
            | SecurityEvent::TwoFactorDisabled { .. }
            | SecurityEvent::EmailChanged { .. }
            | SecurityEvent::AccountUnlocked { .. } => SecuritySeverity::Info,

            SecurityEvent::LoginFailed { .. }
            | SecurityEvent::LoginRateLimited { .. }
            | SecurityEvent::RegisterRateLimited { .. }
            | SecurityEvent::TwoFactorVerificationFailed { .. }
            | SecurityEvent::PasswordResetRequested { .. } => SecuritySeverity::Warning,

            SecurityEvent::SessionRevoked { .. }
            | SecurityEvent::AllSessionsRevoked { .. }
            | SecurityEvent::AccountLocked { .. } => SecuritySeverity::Critical,
        }
    }

    pub fn metadata(&self) -> &SecurityMetadata {
        match self {
            SecurityEvent::UserRegistered { metadata, .. }
            | SecurityEvent::LoginSuccess { metadata, .. }
            | SecurityEvent::LoginFailed { metadata, .. }
            | SecurityEvent::LoginRateLimited { metadata, .. }
            | SecurityEvent::RegisterRateLimited { metadata, .. }
            | SecurityEvent::SessionCreated { metadata, .. }
            | SecurityEvent::SessionRevoked { metadata, .. }
            | SecurityEvent::AllSessionsRevoked { metadata, .. }
            | SecurityEvent::SessionRotated { metadata, .. }
            | SecurityEvent::PasswordChanged { metadata, .. }
            | SecurityEvent::PasswordResetRequested { metadata, .. }
            | SecurityEvent::PasswordResetCompleted { metadata, .. }
            | SecurityEvent::TwoFactorEnabled { metadata, .. }
            | SecurityEvent::TwoFactorDisabled { metadata, .. }
            | SecurityEvent::TwoFactorVerificationFailed { metadata, .. }
            | SecurityEvent::EmailChanged { metadata, .. }
            | SecurityEvent::AccountLocked { metadata, .. }
            | SecurityEvent::AccountUnlocked { metadata, .. } => metadata,
        }
    }

    pub fn timestamp(&self) -> DateTime<Utc> {
        self.metadata().timestamp
    }

    pub fn user_id(&self) -> Option<Uuid> {
        match self {
            SecurityEvent::UserRegistered { user_id, .. }
            | SecurityEvent::LoginSuccess { user_id, .. }
            | SecurityEvent::SessionCreated { user_id, .. }
            | SecurityEvent::SessionRevoked { user_id, .. }
            | SecurityEvent::AllSessionsRevoked { user_id, .. }
            | SecurityEvent::SessionRotated { user_id, .. }
            | SecurityEvent::PasswordChanged { user_id, .. }
            | SecurityEvent::PasswordResetCompleted { user_id, .. }
            | SecurityEvent::TwoFactorEnabled { user_id, .. }
            | SecurityEvent::TwoFactorDisabled { user_id, .. }
            | SecurityEvent::TwoFactorVerificationFailed { user_id, .. }
            | SecurityEvent::EmailChanged { user_id, .. }
            | SecurityEvent::AccountLocked { user_id, .. }
            | SecurityEvent::AccountUnlocked { user_id, .. } => Some(*user_id),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_names_are_stable() {
        let metadata = SecurityMetadata::new(None, None);
        let user_id = Uuid::nil();
        let session_id = Uuid::nil();

        let event = SecurityEvent::SessionRotated {
            metadata,
            user_id,
            old_session_id: session_id,
            new_session_id: session_id,
        };

        assert_eq!(event.event_name(), "SESSION_ROTATED");
        assert_eq!(event.severity(), SecuritySeverity::Info);
        assert_eq!(event.kind(), SecurityEventKind::SessionRotated);
    }

    #[test]
    fn critical_events_have_critical_severity() {
        let metadata = SecurityMetadata::new(None, None);
        let event = SecurityEvent::AccountLocked {
            metadata,
            user_id: Uuid::nil(),
        };

        assert_eq!(event.severity(), SecuritySeverity::Critical);
    }

    #[test]
    fn login_failure_reason_is_structured() {
        assert_eq!(
            LoginFailureReason::InvalidPassword.as_str(),
            "invalid_password"
        );
        assert_eq!(LoginFailureReason::UnknownUser.as_str(), "unknown_user");
        assert_eq!(LoginFailureReason::RateLimited.as_str(), "rate_limited");
    }

    #[test]
    fn events_are_serializable() {
        let metadata = SecurityMetadata::new(None, None);
        let event = SecurityEvent::LoginSuccess {
            metadata,
            user_id: Uuid::nil(),
            email: "test@example.com".to_string(),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("LoginSuccess"));
    }

    #[test]
    fn security_event_record_has_id_and_timestamp() {
        let metadata = SecurityMetadata::new(None, None);
        let event = SecurityEvent::LoginSuccess {
            metadata,
            user_id: Uuid::nil(),
            email: "test@example.com".to_string(),
        };

        let record = SecurityEventRecord::new(event.clone());
        assert!(!record.id.is_nil());
        assert_eq!(record.timestamp, event.timestamp());
    }
}
