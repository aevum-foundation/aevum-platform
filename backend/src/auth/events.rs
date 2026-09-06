//! Security events for the authentication subsystem.
//!
//! AUTH-16 will wire these into a persistent audit log.
//! The enum provides a stable vocabulary for rate limiter,
//! session rotation, and future security features.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
    pub fn event_name(&self) -> &'static str {
        match self {
            SecurityEvent::UserRegistered { .. } => "USER_REGISTERED",
            SecurityEvent::LoginSuccess { .. } => "LOGIN_SUCCESS",
            SecurityEvent::LoginFailed { .. } => "LOGIN_FAILED",
            SecurityEvent::LoginRateLimited { .. } => "LOGIN_RATE_LIMITED",
            SecurityEvent::RegisterRateLimited { .. } => "REGISTER_RATE_LIMITED",
            SecurityEvent::SessionCreated { .. } => "SESSION_CREATED",
            SecurityEvent::SessionRevoked { .. } => "SESSION_REVOKED",
            SecurityEvent::AllSessionsRevoked { .. } => "ALL_SESSIONS_REVOKED",
            SecurityEvent::SessionRotated { .. } => "SESSION_ROTATED",
            SecurityEvent::PasswordChanged { .. } => "PASSWORD_CHANGED",
            SecurityEvent::PasswordResetRequested { .. } => "PASSWORD_RESET_REQUESTED",
            SecurityEvent::PasswordResetCompleted { .. } => "PASSWORD_RESET_COMPLETED",
            SecurityEvent::TwoFactorEnabled { .. } => "TWO_FACTOR_ENABLED",
            SecurityEvent::TwoFactorDisabled { .. } => "TWO_FACTOR_DISABLED",
            SecurityEvent::TwoFactorVerificationFailed { .. } => "TWO_FACTOR_VERIFICATION_FAILED",
            SecurityEvent::EmailChanged { .. } => "EMAIL_CHANGED",
            SecurityEvent::AccountLocked { .. } => "ACCOUNT_LOCKED",
            SecurityEvent::AccountUnlocked { .. } => "ACCOUNT_UNLOCKED",
        }
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
        assert_eq!(
            LoginFailureReason::RateLimited.as_str(),
            "rate_limited"
        );
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
        assert!(json.contains("LOGIN_SUCCESS") || json.contains("LoginSuccess"));
    }
}
