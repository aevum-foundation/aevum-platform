//! Aevum Platform authentication service.
//!
//! AUTH-05 — Auth Service

use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

use crate::auth::api::AuthApi;
use crate::auth::authenticator::Authenticator;
use crate::auth::contracts::SESSION_DURATION_DAYS;
use crate::auth::events::storage::SecurityEventStorage;
use crate::auth::events::{SecurityEvent, SecurityMetadata};
use crate::auth::models::{EmailVerificationToken, PasswordResetToken, Session, SessionTokenHash, User};
use crate::auth::password::{PasswordHasher, SessionToken};
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

    fn update_user(
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

    fn create_password_reset_token(
        &self,
        token: &PasswordResetToken,
    ) -> impl std::future::Future<Output = Result<(), ApiError>> + Send;

    fn get_password_reset_token_by_hash(
        &self,
        token_hash: &str,
    ) -> impl std::future::Future<Output = Result<Option<PasswordResetToken>, ApiError>> + Send;

    fn consume_password_reset_token(
        &self,
        token_hash: &str,
        used_at: DateTime<Utc>,
    ) -> impl std::future::Future<Output = Result<(), ApiError>> + Send;

    fn create_email_verification_token(
        &self,
        token: &EmailVerificationToken,
    ) -> impl std::future::Future<Output = Result<(), ApiError>> + Send;

    fn get_email_verification_token_by_hash(
        &self,
        token_hash: &str,
    ) -> impl std::future::Future<Output = Result<Option<EmailVerificationToken>, ApiError>> + Send;

    fn consume_email_verification_token(
        &self,
        token_hash: &str,
        used_at: DateTime<Utc>,
    ) -> impl std::future::Future<Output = Result<(), ApiError>> + Send;
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
    email_provider: Arc<dyn crate::auth::email::EmailProvider + Send + Sync>,
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
            .field("email_provider", &"<redacted>")
            .finish()
    }
}

#[async_trait::async_trait]
impl<S> AuthApi for AuthService<S>
where
    S: AuthStorage + 'static,
{
    async fn register(&self, email: &str, password: &str, ip: &str) -> Result<User, ApiError> {
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

    async fn change_password(
        &self,
        user_id: &Uuid,
        current_password: &str,
        new_password: &str,
    ) -> Result<SessionToken, ApiError> {
        AuthService::change_password(self, user_id, current_password, new_password).await
    }

    async fn request_password_reset(&self, email: &str, ip: &str) -> Result<(), ApiError> {
        AuthService::request_password_reset(self, email, ip).await
    }

    async fn confirm_password_reset(
        &self,
        token: &str,
        new_password: &str,
    ) -> Result<(), ApiError> {
        AuthService::confirm_password_reset(self, token, new_password).await
    }

    async fn request_email_verification(
        &self,
        user_id: &Uuid,
    ) -> Result<(), ApiError> {
        AuthService::request_email_verification(self, user_id).await
    }

    async fn verify_email(&self, token: &str) -> Result<(), ApiError> {
        AuthService::verify_email(self, token).await
    }
}

#[async_trait::async_trait]
impl<S> Authenticator for AuthService<S>
where
    S: AuthStorage + 'static,
{
    async fn authenticate(&self, token: &SessionToken) -> Result<Option<User>, ApiError> {
        AuthService::authenticate(self, token).await
    }
}

impl<S> AuthService<S>
where
    S: AuthStorage + 'static,
{
    pub fn new(storage: S) -> Self {
        Self::new_with_email_provider(
            storage,
            Arc::new(crate::auth::email::MockEmailProvider::new()),
        )
    }

    pub fn new_with_email_provider(
        storage: S,
        email_provider: Arc<dyn crate::auth::email::EmailProvider + Send + Sync>,
    ) -> Self {
        Self {
            storage: Arc::new(storage),
            password_hasher: PasswordHasher::new(),
            session_duration_days: SESSION_DURATION_DAYS,
            rate_limiter: Arc::new(crate::auth::rate_limit::InMemoryRateLimiter::default()),
            events: Arc::new(crate::auth::events::storage::InMemorySecurityEventStorage::new()),
            email_provider,
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
            email_provider: Arc::new(crate::auth::email::MockEmailProvider::new()),
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

    pub async fn authenticate(&self, token: &SessionToken) -> Result<Option<User>, ApiError> {
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

        if let Some(session) = self.storage.get_session_by_token_hash(&token_hash).await? {
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

    /// Change the password for the currently authenticated user.
    ///
    /// AUTH-20 — Password Change
    pub async fn change_password(
        &self,
        user_id: &Uuid,
        current_password: &str,
        new_password: &str,
    ) -> Result<SessionToken, ApiError> {
        let user = self
            .storage
            .get_user_by_id(user_id)
            .await?
            .ok_or(ApiError::Unauthorized)?;

        // Verify current password
        let password_valid = self
            .password_hasher
            .verify(current_password, &user.password_hash)
            .map_err(|_| ApiError::Unauthorized)?;

        if !password_valid {
            return Err(ApiError::Unauthorized);
        }

        // Hash new password
        let new_password_hash = self
            .password_hasher
            .hash(new_password)
            .map_err(|_| ApiError::Internal)?;

        // Update user
        let mut updated_user = user.clone();
        updated_user.password_hash = new_password_hash;
        updated_user.updated_at = Utc::now();

        self.storage.update_user(&updated_user).await?;

        // Revoke all sessions
        let revoked_count = self
            .storage
            .revoke_all_sessions_for_user(user_id, Utc::now())
            .await?;

        // Create new session
        let (_, session_token) = self.create_session_for_user(updated_user).await?;

        // Emit security event
        self.emit_event(SecurityEvent::PasswordChanged {
            metadata: SecurityMetadata::new(None, None),
            user_id: *user_id,
        })
        .await;

        // Update active sessions gauge
        for _ in 0..revoked_count {
            metrics::gauge!("auth_active_sessions").decrement(1.0);
        }

        Ok(session_token)
    }

    /// Request a password reset.
    ///
    /// AUTH-21 — Password Reset
    ///
    /// Always returns Ok(()) to prevent account enumeration.
    pub async fn request_password_reset(&self, email: &str, ip: &str) -> Result<(), ApiError> {
        let normalized_email = Self::normalize_email(email);

        // Rate limit: IP-based
        self.rate_limiter.check_login_ip(ip).await?;

        let user = self.storage.get_user_by_email(&normalized_email).await?;

        if let Some(user) = user {
            // Generate reset token
            let reset_token = SessionToken::generate();
            let reset_hash = SessionTokenHash::from_token(&reset_token);
            let reset_model = PasswordResetToken::new(
                user.id,
                reset_hash.as_str().to_owned(),
                crate::auth::contracts::PASSWORD_RESET_TOKEN_TTL_MINUTES,
            );

            // Store hashed token
            self.storage
                .create_password_reset_token(&reset_model)
                .await?;

            // Send email with raw token
            let _ = self
                .email_provider
                .send_password_reset(&normalized_email, reset_token.expose())
                .await;

            // Audit event
            self.emit_event(SecurityEvent::PasswordResetRequested {
                metadata: SecurityMetadata::new(Some(ip.to_string()), None),
                email: normalized_email,
            })
            .await;
        }

        Ok(())
    }

    /// Confirm a password reset using the emailed token.
    ///
    /// AUTH-21 — Password Reset
    pub async fn confirm_password_reset(
        &self,
        token: &str,
        new_password: &str,
    ) -> Result<(), ApiError> {
        let temp_token = SessionToken::from_secret(token.to_owned());
        let token_hash = SessionTokenHash::from_token(&temp_token);
        let token_hash_str = token_hash.as_str();

        let reset_token = self
            .storage
            .get_password_reset_token_by_hash(token_hash_str)
            .await?
            .ok_or(ApiError::Unauthorized)?;

        if !reset_token.is_valid_at(Utc::now()) {
            return Err(ApiError::Unauthorized);
        }

        // Consume token (mark as used)
        self.storage
            .consume_password_reset_token(token_hash_str, Utc::now())
            .await?;

        // Get user
        let user = self
            .storage
            .get_user_by_id(&reset_token.user_id)
            .await?
            .ok_or(ApiError::Unauthorized)?;

        // Hash new password
        let new_password_hash = self
            .password_hasher
            .hash(new_password)
            .map_err(|_| ApiError::Internal)?;

        // Update user
        let mut updated_user = user.clone();
        updated_user.password_hash = new_password_hash;
        updated_user.updated_at = Utc::now();
        self.storage.update_user(&updated_user).await?;

        // Revoke all sessions
        let revoked_count = self
            .storage
            .revoke_all_sessions_for_user(&user.id, Utc::now())
            .await?;

        for _ in 0..revoked_count {
            metrics::gauge!("auth_active_sessions").decrement(1.0);
        }

        // Audit event
        self.emit_event(SecurityEvent::PasswordResetCompleted {
            metadata: SecurityMetadata::new(None, None),
            user_id: user.id,
        })
        .await;

        Ok(())
    }

    /// Request email verification.
    ///
    /// AUTH-22 — Email Verification
    ///
    /// If already verified, returns Ok without creating a token.
    pub async fn request_email_verification(
        &self,
        user_id: &Uuid,
    ) -> Result<(), ApiError> {
        let user = self
            .storage
            .get_user_by_id(user_id)
            .await?
            .ok_or(ApiError::Unauthorized)?;

        if user.email_verified {
            return Ok(());
        }

        // Generate verification token
        let verification_token = SessionToken::generate();
        let verification_hash = SessionTokenHash::from_token(&verification_token);
        let verification_model = EmailVerificationToken::new(
            user.id,
            verification_hash.as_str().to_owned(),
            24 * 60, // 24 hours TTL
        );

        self.storage
            .create_email_verification_token(&verification_model)
            .await?;

        let _ = self
            .email_provider
            .send_email_verification(&user.email, verification_token.expose())
            .await;

        self.emit_event(SecurityEvent::EmailVerificationRequested {
            metadata: SecurityMetadata::new(None, None),
            user_id: user.id,
            email: user.email.clone(),
        })
        .await;

        Ok(())
    }

    /// Verify email using the emailed token.
    ///
    /// AUTH-22 — Email Verification
    pub async fn verify_email(&self, token: &str) -> Result<(), ApiError> {
        let temp_token = SessionToken::from_secret(token.to_owned());
        let token_hash = SessionTokenHash::from_token(&temp_token);
        let token_hash_str = token_hash.as_str();

        let verification_token = self
            .storage
            .get_email_verification_token_by_hash(token_hash_str)
            .await?
            .ok_or(ApiError::Unauthorized)?;

        if !verification_token.is_valid_at(Utc::now()) {
            return Err(ApiError::Unauthorized);
        }

        // Consume token
        self.storage
            .consume_email_verification_token(token_hash_str, Utc::now())
            .await?;

        // Update user
        let user = self
            .storage
            .get_user_by_id(&verification_token.user_id)
            .await?
            .ok_or(ApiError::Unauthorized)?;

        let mut updated_user = user.clone();
        updated_user.email_verified = true;
        updated_user.email_verified_at = Some(Utc::now());
        updated_user.updated_at = Utc::now();
        self.storage.update_user(&updated_user).await?;

        self.emit_event(SecurityEvent::EmailVerified {
            metadata: SecurityMetadata::new(None, None),
            user_id: user.id,
        })
        .await;

        Ok(())
    }

    pub fn session_duration_days(&self) -> i64 {
        self.session_duration_days
    }

    /// Record a security event without failing the auth flow.
    async fn emit_event(&self, event: crate::auth::events::SecurityEvent) {
        let event_name = event.event_name();
        if let Err(error) = self.events.record_event(event).await {
            tracing::error!(
                error = %error,
                event = %event_name,
                "audit event failed"
            );
        }
    }
}
