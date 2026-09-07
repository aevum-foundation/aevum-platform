//! Application metrics for the Aevum Platform.
//!
//! AUTH-19.3 — Metrics
//!
//! Centralizes metric names and provides typed helpers
//! for counters, gauges, and histograms.

use metrics::{counter, gauge, histogram};
use std::time::Instant;

// ─── Auth Metrics ─────────────────────────────────────────

pub mod auth {
    pub const LOGIN_SUCCESS_TOTAL: &str = "auth_login_success_total";
    pub const LOGIN_FAILED_TOTAL: &str = "auth_login_failed_total";
    pub const REGISTER_TOTAL: &str = "auth_register_total";
    pub const ACTIVE_SESSIONS: &str = "auth_active_sessions";
    pub const SESSION_CREATED_TOTAL: &str = "auth_session_created_total";
    pub const SESSION_REVOKED_TOTAL: &str = "auth_session_revoked_total";
    pub const SESSION_ROTATED_TOTAL: &str = "auth_session_rotated_total";

    pub const LOGIN_DURATION_SECONDS: &str = "auth_login_duration_seconds";
    pub const REGISTER_DURATION_SECONDS: &str = "auth_register_duration_seconds";
    pub const ROTATE_DURATION_SECONDS: &str = "auth_rotate_duration_seconds";
}

// ─── Audit Metrics ────────────────────────────────────────

pub mod audit {
    pub const EVENTS_WRITTEN_TOTAL: &str = "audit_events_written_total";
    pub const EVENTS_FAILED_TOTAL: &str = "audit_events_failed_total";
    pub const WRITE_DURATION_SECONDS: &str = "audit_write_duration_seconds";
}

// ─── Storage Metrics ──────────────────────────────────────

pub mod storage {
    pub const READS_TOTAL: &str = "auth_storage_reads_total";
    pub const WRITES_TOTAL: &str = "auth_storage_writes_total";
    pub const ERRORS_TOTAL: &str = "auth_storage_errors_total";
    pub const READ_DURATION_SECONDS: &str = "auth_storage_read_duration_seconds";
    pub const WRITE_DURATION_SECONDS: &str = "auth_storage_write_duration_seconds";
}

// ─── Rate Limiter Metrics ────────────────────────────────

pub mod rate_limit {
    pub const LOGIN_RATE_LIMITED_TOTAL: &str = "auth_login_rate_limited_total";
    pub const REGISTER_RATE_LIMITED_TOTAL: &str = "auth_register_rate_limited_total";
}

// ─── Security Metrics ─────────────────────────────────────

pub mod security {
    pub const INVALID_PASSWORD_TOTAL: &str = "security_invalid_password_total";
    pub const UNKNOWN_USER_TOTAL: &str = "security_unknown_user_total";
    pub const ACCOUNT_DISABLED_TOTAL: &str = "security_account_disabled_total";
}

// ─── Typed Helpers ───────────────────────────────────────

/// Record a counter metric.
pub fn count(name: &'static str) {
    counter!(name).increment(1);
}

/// Record a counter with a reason label.
pub fn count_with_reason(name: &'static str, reason: &'static str) {
    counter!(name, "reason" => reason).increment(1);
}

/// Record a gauge increment.
pub fn gauge_inc(name: &'static str) {
    gauge!(name).increment(1.0);
}

/// Record a gauge decrement.
pub fn gauge_dec(name: &'static str) {
    gauge!(name).decrement(1.0);
}

/// Record a duration histogram in seconds.
pub fn time(name: &'static str, start: Instant) {
    histogram!(name).record(start.elapsed().as_secs_f64());
}

/// RAII timer that records duration on drop.
///
/// Usage:
/// ```rust,ignore
/// let _timer = metrics::Timer::start(metrics::auth::LOGIN_DURATION_SECONDS);
/// // ... logic ...
/// ```
#[must_use]
pub struct Timer {
    name: &'static str,
    start: Instant,
}

impl Timer {
    pub fn start(name: &'static str) -> Self {
        Self {
            name,
            start: Instant::now(),
        }
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        histogram!(self.name).record(self.start.elapsed().as_secs_f64());
    }
}

// ─── Domain-Specific Helpers ─────────────────────────────

/// Record session creation.
pub fn session_created() {
    gauge_inc(auth::ACTIVE_SESSIONS);
}

/// Record session revocation.
pub fn session_revoked() {
    gauge_dec(auth::ACTIVE_SESSIONS);
}

/// Record audit event written.
pub fn audit_written() {
    count(audit::EVENTS_WRITTEN_TOTAL);
}

/// Record audit event failure.
pub fn audit_failed() {
    count(audit::EVENTS_FAILED_TOTAL);
}

/// Record storage read.
pub fn storage_read() {
    count(storage::READS_TOTAL);
}

/// Record storage write.
pub fn storage_write() {
    count(storage::WRITES_TOTAL);
}

/// Record storage error.
pub fn storage_error() {
    count(storage::ERRORS_TOTAL);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_are_stable() {
        assert_eq!(auth::LOGIN_SUCCESS_TOTAL, "auth_login_success_total");
        assert_eq!(auth::ACTIVE_SESSIONS, "auth_active_sessions");
        assert_eq!(audit::EVENTS_WRITTEN_TOTAL, "audit_events_written_total");
        assert_eq!(storage::READS_TOTAL, "auth_storage_reads_total");
        assert_eq!(
            rate_limit::LOGIN_RATE_LIMITED_TOTAL,
            "auth_login_rate_limited_total"
        );
    }

    #[test]
    fn count_does_not_panic() {
        count("test_counter");
    }

    #[test]
    fn count_with_reason_does_not_panic() {
        count_with_reason("test_counter", "test_reason");
    }

    #[test]
    fn gauge_operations_do_not_panic() {
        gauge_inc("test_gauge");
        gauge_dec("test_gauge");
    }

    #[test]
    fn timer_records_on_drop() {
        let _timer = Timer::start("test_timer");
    }

    #[test]
    fn domain_helpers_do_not_panic() {
        session_created();
        session_revoked();
        audit_written();
        audit_failed();
        storage_read();
        storage_write();
        storage_error();
    }
}
