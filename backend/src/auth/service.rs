//! Aevum Platform authentication service.
//!
//! AUTH-05 — Auth Service

use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

use crate::auth::contracts::SESSION_DURATION_DAYS;
use crate::auth::models::{Session, SessionTokenHash, User};
use crate::auth::password::{PasswordHasher, SessionToken};
use crate::auth::api::AuthApi;
use crate::auth::authenticator::Authenticator;
use crate::auth::events::storage::SecurityEventStorage;
use crate::auth::events::{SecurityEvent, SecurityMetadata};
use crate::auth::rate_limit::LoginRateLimiter;
use crate::error::ApiError;

const MAX_SESSION_DURATION_DAYS: i64 = 30;
const MIN_SESSION_DURATION_DAYS: i64 = 1;

pub trait AuthStorage: Send + Sync {
    fn user_exists(
        &self,
        email: &str,
    ) -> impl std::future::Future<Output = Result<bool, ApiError>> + Send;
    fn get_user_by_email(
        &self,
        email: &str,
    ) -> impl std::future::Future<Output = Result<Option<User>, ApiError>> + Send;
    fn get_user_by_id(
        &self,
        user_id: &Uuid,
    ) -> impl std::future::Future<Output = Result<Option<User>, ApiError>> + Send;
    fn create_user(
        &self,
        user: &User,
    ) -> impl std::future::Future<Output = Result<(), ApiError>> + Send;
    fn create_session(
        &self,
        session: &Session,
    ) -> impl std::future::Future<Output = Result<(), ApiError>> + Send;
    fn get_session_by_token_hash(
        &self,
        token_hash: &SessionTokenHash,
    ) -> impl std::future::Future<Output = Result<Option<Session>, ApiError>> + Send;
    fn revoke_session_by_token_hash(
        &self,
        token_hash: &SessionTokenHash,
        revoked_at: DateTime<Utc>,
    ) -> impl std::future::Future<Output = Result<(), ApiError>> + Send;

    fn revoke_all_sessions_for_user(
        &self,
        user_id: &Uuid,
        revoked_at: DateTime<Utc>,
    ) -> impl std::future::Future<Output = Result<usize, ApiError>> + Send;
}

#[derive(Clone)]
pub struct AuthService<S>
where
    S: AuthStorage + 'static,
{
    storage: Arc<S>,
    password_hasher: PasswordHasher,
    session_duration_days: i64,
    rate_limiter: Arc<dyn LoginRateLimiter + Send + Sync>,
    events: Arc<dyn SecurityEventStorage + Send + Sync>,
}

impl<S> std::fmt::Debug for AuthService<S>
where
    S: AuthStorage + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuthService")
            .field("storage", &"<redacted>")
            .field("password_hasher", &self.password_hasher)
            .field("session_duration_days", &self.session_duration_days)
            .field("rate_limiter", &"<redacted>")
            .field("events", &"<redacted>")
            .finish()
    }
}

#[async_trait::async_trait]
impl<S> AuthApi for AuthService<S>
where
    S: AuthStorage + 'static,
{
    async fn register(
        &self,
        email: &str,
        password: &str,
        ip: &str,
    ) -> Result<User, ApiError> {
        AuthService::register(self, email, password, ip).await
    }

    async fn login(
        &self,
        email: &str,
        password: &str,
        ip: &str,
    ) -> Result<(User, SessionToken, crate::auth::csrf::CsrfToken), ApiError> {
        let (user, session_token) = AuthService::login(self, email, password, ip).await?;
        let csrf_token = crate::auth::csrf::CsrfToken::generate();
        Ok((user, session_token, csrf_token))
    }

    async fn logout(&self, token: &SessionToken) -> Result<(), ApiError> {
        AuthService::logout(self, token).await
    }

    async fn authenticate(&self, token: &SessionToken) -> Result<Option<User>, ApiError> {
        AuthService::authenticate(self, token).await
    }

    async fn rotate_session(
        &self,
        token: &SessionToken,
    ) -> Result<(SessionToken, crate::auth::csrf::CsrfToken), ApiError> {
        AuthService::rotate_session(self, token).await
    }
}

#[async_trait::async_trait]
impl<S> Authenticator for AuthService<S>
where
    S: AuthStorage + 'static,
{
    async fn authenticate(
        &self,
        token: &SessionToken,
    ) -> Result<Option<User>, ApiError> {
        AuthService::authenticate(self, token).await
    }
}

impl<S> AuthService<S>
where
    S: AuthStorage + 'static,
{
    pub fn new(storage: S) -> Self {
        Self {
            storage: Arc::new(storage),
            password_hasher: PasswordHasher::new(),
            session_duration_days: SESSION_DURATION_DAYS,
            rate_limiter: Arc::new(crate::auth::rate_limit::InMemoryRateLimiter::default()),
            events: Arc::new(crate::auth::events::storage::InMemorySecurityEventStorage::new()),
        }
    }

    pub fn with_session_duration(storage: S, days: i64) -> Result<Self, ApiError> {
        if !(MIN_SESSION_DURATION_DAYS..=MAX_SESSION_DURATION_DAYS).contains(&days) {
            return Err(ApiError::BadRequest);
        }

        Ok(Self {
            storage: Arc::new(storage),
            password_hasher: PasswordHasher::new(),
            session_duration_days: days,
            rate_limiter: Arc::new(crate::auth::rate_limit::InMemoryRateLimiter::default()),
            events: Arc::new(crate::auth::events::storage::InMemorySecurityEventStorage::new()),
        })
    }

    fn normalize_email(email: &str) -> String {
        email.trim().to_lowercase()
    }

    fn session_expiration(&self, now: DateTime<Utc>) -> DateTime<Utc> {
        now + Duration::days(self.session_duration_days)
    }

    pub async fn register(&self, email: &str, password: &str, ip: &str) -> Result<User, ApiError> {
        let normalized_email = Self::normalize_email(email);

        if normalized_email.is_empty() {
            return Err(ApiError::BadRequest);
        }

        // Rate limit: IP-based
        self.rate_limiter.check_register_ip(ip).await?;

        if self.storage.user_exists(&normalized_email).await? {
            return Err(ApiError::Conflict);
        }

        let password_hash = self
            .password_hasher
            .hash(password)
            .map_err(|_| ApiError::Internal)?;

        let user = User::new(normalized_email, password_hash);

        match self.storage.create_user(&user).await {
            Ok(()) => {
                self.rate_limiter.record_successful_registration(ip).await;

                self.emit_event(SecurityEvent::user_registered(
                    user.id,
                    user.email.clone(),
                    SecurityMetadata::new(Some(ip.to_string()), None),
                ))
                .await;

                Ok(user)
            }
            Err(ApiError::Conflict) => Err(ApiError::Conflict),
            Err(error) => Err(error),
        }
    }

    pub async fn login(
        &self,
        email: &str,
        password: &str,
        ip: &str,
    ) -> Result<(User, SessionToken), ApiError> {
        let normalized_email = Self::normalize_email(email);
        let metadata = SecurityMetadata::new(Some(ip.to_string()), None);

        // Rate limit: IP-based
        if let Err(error) = self.rate_limiter.check_login_ip(ip).await {
            self.emit_event(SecurityEvent::login_rate_limited(
                Some(normalized_email.clone()),
                metadata.clone(),
            ))
            .await;
            return Err(error);
        }

        // Rate limit: email-based
        if let Err(error) = self.rate_limiter.check_login_email(&normalized_email).await {
            self.emit_event(SecurityEvent::login_rate_limited(
                Some(normalized_email.clone()),
                metadata.clone(),
            ))
            .await;
            return Err(error);
        }

        let user = match self.storage.get_user_by_email(&normalized_email).await? {
            Some(user) => user,
            None => {
                self.rate_limiter
                    .record_login_failure(ip, &normalized_email)
                    .await;
                self.emit_event(SecurityEvent::login_failed(
                    normalized_email.clone(),
                    crate::auth::events::LoginFailureReason::UnknownUser,
                    metadata.clone(),
                ))
                .await;
                return Err(ApiError::Unauthorized);
            }
        };

        if !user.status.can_authenticate() {
            self.rate_limiter
                .record_login_failure(ip, &normalized_email)
                .await;
            self.emit_event(SecurityEvent::login_failed(
                normalized_email.clone(),
                crate::auth::events::LoginFailureReason::AccountDisabled,
                metadata.clone(),
            ))
            .await;
            return Err(ApiError::Unauthorized);
        }

        let password_valid = self
            .password_hasher
            .verify(password, &user.password_hash)
            .map_err(|_| ApiError::Unauthorized)?;

        if !password_valid {
            self.rate_limiter
                .record_login_failure(ip, &normalized_email)
                .await;
            self.emit_event(SecurityEvent::login_failed(
                normalized_email.clone(),
                crate::auth::events::LoginFailureReason::InvalidPassword,
                metadata.clone(),
            ))
            .await;
            return Err(ApiError::Unauthorized);
        }

        self.rate_limiter
            .record_login_success(ip, &normalized_email)
            .await;

        self.emit_event(SecurityEvent::login_success(
            user.id,
            normalized_email.clone(),
            metadata.clone(),
        ))
        .await;

        self.create_session_for_user(user).await
    }

    async fn create_session_for_user(&self, user: User) -> Result<(User, SessionToken), ApiError> {
        self.create_session_for_user_at(user, Utc::now()).await
    }

    async fn create_session_for_user_at(
        &self,
        user: User,
        now: DateTime<Utc>,
    ) -> Result<(User, SessionToken), ApiError> {
        let session_token = SessionToken::generate();
        let token_hash = SessionTokenHash::from_token(&session_token);
        let expires_at = self.session_expiration(now);

        let session = Session::new(user.id, token_hash, expires_at);

        self.storage.create_session(&session).await?;

        self.emit_event(SecurityEvent::session_created(
            session.user_id,
            session.id,
            SecurityMetadata::new(None, None),
        ))
        .await;

        Ok((user, session_token))
    }

    pub async fn authenticate(
        &self,
        token: &SessionToken,
    ) -> Result<Option<User>, ApiError> {
        self.authenticate_at(token, Utc::now()).await
    }

    pub async fn authenticate_at(
        &self,
        token: &SessionToken,
        now: DateTime<Utc>,
    ) -> Result<Option<User>, ApiError> {
        let token_hash = SessionTokenHash::from_token(token);

        let session = self.storage.get_session_by_token_hash(&token_hash).await?;

        let Some(session) = session else {
            return Ok(None);
        };

        if !session.is_valid_at(now) {
            return Ok(None);
        }

        let user = self
            .storage
            .get_user_by_id(&session.user_id)
            .await?
            .ok_or(ApiError::Internal)?;

        if !user.status.can_authenticate() {
            return Ok(None);
        }

        Ok(Some(user))
    }

    pub async fn logout(&self, token: &SessionToken) -> Result<(), ApiError> {
        self.logout_at(token, Utc::now()).await
    }

    pub async fn logout_at(
        &self,
        token: &SessionToken,
        now: DateTime<Utc>,
    ) -> Result<(), ApiError> {
        let token_hash = SessionTokenHash::from_token(token);

        if let Some(session) = self
            .storage
            .get_session_by_token_hash(&token_hash)
            .await?
        {
            self.storage
                .revoke_session_by_token_hash(&token_hash, now)
                .await?;

            self.emit_event(SecurityEvent::session_revoked(
                session.user_id,
                session.id,
                SecurityMetadata::new(None, None),
            ))
            .await;
        }

        Ok(())
    }

    pub async fn get_user(&self, user_id: &Uuid) -> Result<Option<User>, ApiError> {
        self.storage.get_user_by_id(user_id).await
    }

    /// Rotate the session token for an authenticated session.
    ///
    /// AUTH-15 — Session Rotation
    ///
    /// The old session is revoked and a new session with a fresh
    /// token and CSRF token is created. Returns the new tokens.
    pub async fn rotate_session(
        &self,
        token: &SessionToken,
    ) -> Result<(SessionToken, crate::auth::csrf::CsrfToken), ApiError> {
        let token_hash = SessionTokenHash::from_token(token);

        let session = self
            .storage
            .get_session_by_token_hash(&token_hash)
            .await?
            .ok_or(ApiError::Unauthorized)?;

        if !session.is_valid() {
            return Err(ApiError::Unauthorized);
        }

        // Revoke old session
        self.storage
            .revoke_session_by_token_hash(&token_hash, Utc::now())
            .await?;

        // Get user
        let user = self
            .storage
            .get_user_by_id(&session.user_id)
            .await?
            .ok_or(ApiError::Internal)?;

        // Create new session
        let (_, new_session_token) = self.create_session_for_user(user).await?;

        // Generate new CSRF token
        let new_csrf_token = crate::auth::csrf::CsrfToken::generate();

        self.emit_event(SecurityEvent::session_rotated(
            session.user_id,
            session.id,
            Uuid::new_v4(),
            SecurityMetadata::new(None, None),
        ))
        .await;

        Ok((new_session_token, new_csrf_token))
    }

    pub fn session_duration_days(&self) -> i64 {
        self.session_duration_days
    }

    /// Record a security event without failing the auth flow.
    async fn emit_event(&self, event: crate::auth::events::SecurityEvent) {
        if let Err(error) = self.events.record_event(event).await {
            log::error!("audit event failed: {}", error);
        }
    }
}
