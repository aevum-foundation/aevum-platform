//! Rate limiting for authentication endpoints.
//!
//! AUTH-14 — Rate Limiting
//!
//! This module provides brute-force protection for login and registration.
//! The rate limiter is intentionally transport-agnostic and lives inside
//! AuthService, so all entry points (HTTP, gRPC, CLI, future APIs) share
//! the same protection.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use tokio::sync::Mutex;

use crate::error::ApiError;

/// Rate limit configuration.
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum login attempts per IP in the tracking window.
    pub login_ip_limit: u32,
    /// Maximum login attempts per email in the tracking window.
    pub login_email_limit: u32,
    /// Maximum registration attempts per IP in the tracking window.
    pub register_ip_limit: u32,
    /// Tracking window for login attempts.
    pub login_window: Duration,
    /// Tracking window for registration attempts.
    pub register_window: Duration,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            login_ip_limit: 20,
            login_email_limit: 5,
            register_ip_limit: 5,
            login_window: Duration::from_secs(15 * 60),
            register_window: Duration::from_secs(15 * 60),
        }
    }
}

/// Reason why a rate limit was triggered.
///
/// Used by future AUTH-16 audit logging.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimitReason {
    LoginIp,
    LoginEmail,
    RegisterIp,
}

/// A single rate-limit bucket with count, first attempt, and optional lock.
#[derive(Debug, Clone)]
pub struct AttemptBucket {
    pub count: u32,
    pub first_attempt: Instant,
    pub locked_until: Option<Instant>,
}

impl AttemptBucket {
    fn new() -> Self {
        Self {
            count: 0,
            first_attempt: Instant::now(),
            locked_until: None,
        }
    }

    fn record_attempt(&mut self) {
        self.count += 1;
    }

    fn reset(&mut self) {
        self.count = 0;
        self.first_attempt = Instant::now();
        self.locked_until = None;
    }

    fn is_locked_at(&self, now: Instant) -> bool {
        match self.locked_until {
            Some(until) => now < until,
            None => false,
        }
    }

    fn window_elapsed(&self, window: Duration, now: Instant) -> bool {
        now.duration_since(self.first_attempt) >= window
    }

    fn unlock_if_expired(&mut self, now: Instant) -> bool {
        if let Some(until) = self.locked_until {
            if now >= until {
                self.reset();
                return true;
            }
        }
        false
    }
}

/// Transport-agnostic login rate limiter.
///
/// Implementations:
/// - InMemoryRateLimiter (current)
/// - AevumDbRateLimiter (future)
/// - RedisRateLimiter (future)
#[async_trait]
pub trait LoginRateLimiter: Send + Sync {
    async fn check_login_ip(&self, ip: &str) -> Result<(), ApiError>;

    async fn check_login_email(&self, email: &str) -> Result<(), ApiError>;

    async fn record_login_failure(&self, ip: &str, email: &str);

    async fn record_login_success(&self, ip: &str, email: &str);

    async fn check_register_ip(&self, ip: &str) -> Result<(), ApiError>;

    async fn record_successful_registration(&self, ip: &str);
}

/// In-memory rate limiter for development and tests.
///
/// AUTH-17 will introduce a persistent backend using AevumDB.
#[derive(Clone)]
pub struct InMemoryRateLimiter {
    login_ip: Arc<Mutex<HashMap<String, AttemptBucket>>>,
    login_email: Arc<Mutex<HashMap<String, AttemptBucket>>>,
    register_ip: Arc<Mutex<HashMap<String, AttemptBucket>>>,
    config: RateLimitConfig,
}

impl InMemoryRateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            login_ip: Arc::new(Mutex::new(HashMap::new())),
            login_email: Arc::new(Mutex::new(HashMap::new())),
            register_ip: Arc::new(Mutex::new(HashMap::new())),
            config,
        }
    }

    /// Remove stale buckets that are no longer relevant.
    fn cleanup_expired(map: &mut HashMap<String, AttemptBucket>, window: Duration, now: Instant) {
        map.retain(|_, bucket| !bucket.is_locked_at(now) && !bucket.window_elapsed(window, now));
    }
}

impl Default for InMemoryRateLimiter {
    fn default() -> Self {
        Self::new(RateLimitConfig::default())
    }
}

#[async_trait]
impl LoginRateLimiter for InMemoryRateLimiter {
    async fn check_login_ip(&self, ip: &str) -> Result<(), ApiError> {
        let mut map = self.login_ip.lock().await;
        let now = Instant::now();

        Self::cleanup_expired(&mut map, self.config.login_window, now);

        let bucket = map.entry(ip.to_owned()).or_insert_with(AttemptBucket::new);

        if bucket.unlock_if_expired(now) {
            return Ok(());
        }

        if bucket.is_locked_at(now) {
            return Err(ApiError::RateLimited);
        }

        if bucket.window_elapsed(self.config.login_window, now) {
            bucket.reset();
            return Ok(());
        }

        if bucket.count >= self.config.login_ip_limit {
            bucket.locked_until = Some(now + self.config.login_window);
            return Err(ApiError::RateLimited);
        }

        Ok(())
    }

    async fn check_login_email(&self, email: &str) -> Result<(), ApiError> {
        let mut map = self.login_email.lock().await;
        let now = Instant::now();

        Self::cleanup_expired(&mut map, self.config.login_window, now);

        let bucket = map
            .entry(email.to_owned())
            .or_insert_with(AttemptBucket::new);

        if bucket.unlock_if_expired(now) {
            return Ok(());
        }

        if bucket.is_locked_at(now) {
            return Err(ApiError::RateLimited);
        }

        if bucket.window_elapsed(self.config.login_window, now) {
            bucket.reset();
            return Ok(());
        }

        if bucket.count >= self.config.login_email_limit {
            bucket.locked_until = Some(now + self.config.login_window);
            return Err(ApiError::RateLimited);
        }

        Ok(())
    }

    async fn record_login_failure(&self, ip: &str, email: &str) {
        let mut ip_map = self.login_ip.lock().await;
        let ip_bucket = ip_map
            .entry(ip.to_owned())
            .or_insert_with(AttemptBucket::new);
        ip_bucket.record_attempt();

        let mut email_map = self.login_email.lock().await;
        let email_bucket = email_map
            .entry(email.to_owned())
            .or_insert_with(AttemptBucket::new);
        email_bucket.record_attempt();
    }

    async fn record_login_success(&self, ip: &str, email: &str) {
        let mut ip_map = self.login_ip.lock().await;
        ip_map.remove(ip);

        let mut email_map = self.login_email.lock().await;
        email_map.remove(email);
    }

    async fn check_register_ip(&self, ip: &str) -> Result<(), ApiError> {
        let mut map = self.register_ip.lock().await;
        let now = Instant::now();

        Self::cleanup_expired(&mut map, self.config.register_window, now);

        let bucket = map.entry(ip.to_owned()).or_insert_with(AttemptBucket::new);

        if bucket.unlock_if_expired(now) {
            return Ok(());
        }

        if bucket.is_locked_at(now) {
            return Err(ApiError::RateLimited);
        }

        if bucket.window_elapsed(self.config.register_window, now) {
            bucket.reset();
            return Ok(());
        }

        if bucket.count >= self.config.register_ip_limit {
            bucket.locked_until = Some(now + self.config.register_window);
            return Err(ApiError::RateLimited);
        }

        Ok(())
    }

    async fn record_successful_registration(&self, ip: &str) {
        let mut map = self.register_ip.lock().await;
        let bucket = map.entry(ip.to_owned()).or_insert_with(AttemptBucket::new);
        bucket.record_attempt();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_defaults_are_reasonable() {
        let config = RateLimitConfig::default();
        assert_eq!(config.login_ip_limit, 20);
        assert_eq!(config.login_email_limit, 5);
        assert_eq!(config.register_ip_limit, 5);
        assert_eq!(config.login_window, Duration::from_secs(15 * 60));
        assert_eq!(config.register_window, Duration::from_secs(15 * 60));
    }

    #[test]
    fn attempt_bucket_window_elapse() {
        let mut bucket = AttemptBucket::new();
        bucket.record_attempt();
        assert!(!bucket.window_elapsed(Duration::from_secs(60), Instant::now()));
        assert!(bucket.window_elapsed(Duration::from_millis(0), Instant::now()));
    }

    #[test]
    fn attempt_bucket_reset_works() {
        let mut bucket = AttemptBucket::new();
        bucket.record_attempt();
        bucket.record_attempt();
        bucket.locked_until = Some(Instant::now() + Duration::from_secs(60));
        bucket.reset();
        assert_eq!(bucket.count, 0);
        assert!(bucket.locked_until.is_none());
    }

    #[tokio::test]
    async fn login_ip_limit_enforced() {
        let limiter = InMemoryRateLimiter::new(RateLimitConfig {
            login_ip_limit: 2,
            ..Default::default()
        });

        assert!(limiter.check_login_ip("127.0.0.1").await.is_ok());
        limiter
            .record_login_failure("127.0.0.1", "test@example.com")
            .await;
        assert!(limiter.check_login_ip("127.0.0.1").await.is_ok());
        limiter
            .record_login_failure("127.0.0.1", "test@example.com")
            .await;
        assert!(limiter.check_login_ip("127.0.0.1").await.is_err());
    }

    #[tokio::test]
    async fn login_email_limit_enforced() {
        let limiter = InMemoryRateLimiter::new(RateLimitConfig {
            login_email_limit: 2,
            ..Default::default()
        });

        assert!(limiter.check_login_email("test@example.com").await.is_ok());
        limiter
            .record_login_failure("127.0.0.1", "test@example.com")
            .await;
        assert!(limiter.check_login_email("test@example.com").await.is_ok());
        limiter
            .record_login_failure("127.0.0.1", "test@example.com")
            .await;
        assert!(limiter.check_login_email("test@example.com").await.is_err());
    }

    #[tokio::test]
    async fn register_ip_limit_enforced() {
        let limiter = InMemoryRateLimiter::new(RateLimitConfig {
            register_ip_limit: 2,
            ..Default::default()
        });

        assert!(limiter.check_register_ip("127.0.0.1").await.is_ok());
        limiter.record_successful_registration("127.0.0.1").await;
        assert!(limiter.check_register_ip("127.0.0.1").await.is_ok());
        limiter.record_successful_registration("127.0.0.1").await;
        assert!(limiter.check_register_ip("127.0.0.1").await.is_err());
    }

    #[tokio::test]
    async fn successful_login_clears_bucket() {
        let limiter = InMemoryRateLimiter::new(RateLimitConfig {
            login_email_limit: 5,
            ..Default::default()
        });

        limiter
            .record_login_failure("127.0.0.1", "test@example.com")
            .await;
        assert!(limiter.check_login_email("test@example.com").await.is_ok());

        limiter
            .record_login_success("127.0.0.1", "test@example.com")
            .await;
        assert!(limiter.check_login_email("test@example.com").await.is_ok());
    }
}
