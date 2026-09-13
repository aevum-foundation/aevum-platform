//! Aevum Platform — AUTH-28 Security Center
//!
//! Read-only aggregation layer for the authenticated account.
//!
//! Security Center does not introduce or persist security state.
//! It aggregates existing authentication/security data and computes
//! a deterministic account-security posture score.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::auth::events::SecurityEvent;

/// Maximum number of security events exposed by the Security Center.
pub const SECURITY_CENTER_EVENT_LIMIT: usize = 10;

/// Password-change freshness window used by the security score.
pub const PASSWORD_RECENCY_DAYS: i64 = 90;

/// Maximum possible security posture score.
pub const SECURITY_SCORE_MAX: u8 = 100;

/// Client-safe summary of an internal security event.
///
/// Deliberately excludes internal identifiers, IP addresses,
/// user-agent strings and other operational metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventSummary {
    pub event_type: String,
    pub occurred_at: DateTime<Utc>,
    pub severity: String,
}

impl EventSummary {
    /// Convert an internal security event into a client-safe DTO.
    pub fn from_event(event: &SecurityEvent) -> Self {
        Self {
            event_type: event.event_name().to_owned(),
            occurred_at: event.timestamp(),
            severity: match event.severity() {
                crate::auth::events::SecuritySeverity::Info => "info".to_owned(),
                crate::auth::events::SecuritySeverity::Warning => "warning".to_owned(),
                crate::auth::events::SecuritySeverity::Critical => "critical".to_owned(),
            },
        }
    }
}

/// Input to the deterministic security posture calculation.
///
/// This is deliberately composed only of current security properties.
/// The score itself is never persisted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SecurityScoreInput {
    pub email_verified: bool,
    pub two_factor_enabled: bool,
    pub has_backup_codes: bool,
    pub password_changed_recently: bool,
}

/// Calculate the account-security posture score.
///
/// Policy:
/// - verified email: +20
/// - enabled 2FA: +35
/// - available backup codes: +15
/// - password changed within the configured freshness window: +30
///
/// Maximum score is always 100.
///
/// This function is pure and deterministic. It performs no I/O and
/// does not mutate authentication state.
///
/// `security_score` represents the configured account-security
/// posture policy. It must not be interpreted as a guarantee of
/// account security.
pub fn calculate_security_score(input: SecurityScoreInput) -> u8 {
    let mut score: u16 = 0;

    if input.email_verified {
        score += 20;
    }

    if input.two_factor_enabled {
        score += 35;
    }

    if input.has_backup_codes {
        score += 15;
    }

    if input.password_changed_recently {
        score += 30;
    }

    score.min(SECURITY_SCORE_MAX as u16) as u8
}

/// Determine whether a password-change timestamp falls inside the
/// configured freshness window.
///
/// Future timestamps are treated as not recent. This prevents a
/// malformed or clock-skewed timestamp from automatically granting
/// the freshness points.
pub fn password_changed_recently(changed_at: Option<DateTime<Utc>>, now: DateTime<Utc>) -> bool {
    let Some(changed_at) = changed_at else {
        return false;
    };

    if changed_at > now {
        return false;
    }

    changed_at >= now - Duration::days(PASSWORD_RECENCY_DAYS)
}

/// Read-only security posture summary for the authenticated account.
///
/// All values are derived from existing authentication state.
/// Nothing in this structure is itself persisted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityCenterResponse {
    pub email_verified: bool,
    pub two_factor_enabled: bool,
    pub backup_codes_remaining: usize,
    pub active_sessions: usize,
    pub password_changed_at: Option<DateTime<Utc>>,
    pub security_score: u8,
    pub recent_events: Vec<EventSummary>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(
        email_verified: bool,
        two_factor_enabled: bool,
        has_backup_codes: bool,
        password_changed_recently: bool,
    ) -> SecurityScoreInput {
        SecurityScoreInput {
            email_verified,
            two_factor_enabled,
            has_backup_codes,
            password_changed_recently,
        }
    }

    #[test]
    fn score_empty_account_is_zero() {
        assert_eq!(
            calculate_security_score(input(false, false, false, false)),
            0
        );
    }

    #[test]
    fn score_email_only_is_twenty() {
        assert_eq!(
            calculate_security_score(input(true, false, false, false)),
            20
        );
    }

    #[test]
    fn score_2fa_only_is_thirty_five() {
        assert_eq!(
            calculate_security_score(input(false, true, false, false)),
            35
        );
    }

    #[test]
    fn score_backup_codes_only_is_fifteen() {
        assert_eq!(
            calculate_security_score(input(false, false, true, false)),
            15
        );
    }

    #[test]
    fn score_recent_password_only_is_thirty() {
        assert_eq!(
            calculate_security_score(input(false, false, false, true)),
            30
        );
    }

    #[test]
    fn score_email_plus_2fa_is_fifty_five() {
        assert_eq!(
            calculate_security_score(input(true, true, false, false)),
            55
        );
    }

    #[test]
    fn score_all_controls_is_100() {
        assert_eq!(calculate_security_score(input(true, true, true, true)), 100);
    }

    #[test]
    fn score_never_exceeds_100() {
        let score = calculate_security_score(input(true, true, true, true));

        assert!(score <= SECURITY_SCORE_MAX);
        assert_eq!(score, SECURITY_SCORE_MAX);
    }

    #[test]
    fn password_within_90_days_is_recent() {
        let now = Utc::now();
        let changed_at = now - Duration::days(30);

        assert!(password_changed_recently(Some(changed_at), now));
    }

    #[test]
    fn password_exactly_on_boundary_is_recent() {
        let now = Utc::now();
        let changed_at = now - Duration::days(PASSWORD_RECENCY_DAYS);

        assert!(password_changed_recently(Some(changed_at), now));
    }

    #[test]
    fn password_older_than_90_days_is_not_recent() {
        let now = Utc::now();
        let changed_at = now - Duration::days(PASSWORD_RECENCY_DAYS + 1);

        assert!(!password_changed_recently(Some(changed_at), now));
    }

    #[test]
    fn missing_password_timestamp_is_not_recent() {
        assert!(!password_changed_recently(None, Utc::now()));
    }

    #[test]
    fn future_password_timestamp_is_not_recent() {
        let now = Utc::now();
        let changed_at = now + Duration::hours(1);

        assert!(!password_changed_recently(Some(changed_at), now));
    }

    #[test]
    fn event_limit_is_ten() {
        assert_eq!(SECURITY_CENTER_EVENT_LIMIT, 10);
    }

    #[test]
    fn password_recency_window_is_90_days() {
        assert_eq!(PASSWORD_RECENCY_DAYS, 90);
    }
}
