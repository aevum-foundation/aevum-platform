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
use crate::auth::models::{
    AuthContext, BackupCode, EmailVerificationToken, PasswordResetToken, Session, SessionTokenHash,
    User,
};
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

    fn get_active_sessions_for_user(
        &self,
        user_id: &Uuid,
        now: DateTime<Utc>,
    ) -> impl std::future::Future<Output = Result<Vec<Session>, ApiError>> + Send;

    fn create_backup_code(
        &self,
        code: &BackupCode,
    ) -> impl std::future::Future<Output = Result<(), ApiError>> + Send;

    fn create_two_factor_settings(
        &self,
        settings: &crate::auth::two_factor::TwoFactorSettings,
    ) -> impl std::future::Future<Output = Result<(), ApiError>> + Send;

    fn get_two_factor_settings(
        &self,
        user_id: &Uuid,
    ) -> impl std::future::Future<Output = Result<Option<crate::auth::two_factor::TwoFactorSettings>, ApiError>> + Send;

    fn update_two_factor_settings(
        &self,
        settings: &crate::auth::two_factor::TwoFactorSettings,
    ) -> impl std::future::Future<Output = Result<(), ApiError>> + Send;

    fn delete_two_factor_settings(
        &self,
        user_id: &Uuid,
    ) -> impl std::future::Future<Output = Result<(), ApiError>> + Send;

    /// Get a per-user lock for serializing critical atomic operations.
    ///
    /// Returns the same `Arc<Mutex<()>>` for the same user_id.
    ///
    /// Policy: never hold two different user locks at the same time.
    fn lock_for_user(
        &self,
        user_id: &Uuid,
    ) -> impl std::future::Future<Output = Arc<tokio::sync::Mutex<()>>> + Send;

    fn list_backup_codes(
        &self,
        user_id: &Uuid,
    ) -> impl std::future::Future<Output = Result<Vec<BackupCode>, ApiError>> + Send;

    fn consume_backup_code(
        &self,
        user_id: &Uuid,
        code_hash: &str,
        used_at: DateTime<Utc>,
    ) -> impl std::future::Future<Output = Result<bool, ApiError>> + Send;

    fn revoke_all_backup_codes(
        &self,
        user_id: &Uuid,
        revoked_at: DateTime<Utc>,
    ) -> impl std::future::Future<Output = Result<usize, ApiError>> + Send;

    fn revoke_session_by_id(
        &self,
        session_id: &Uuid,
        user_id: &Uuid,
        revoked_at: DateTime<Utc>,
    ) -> impl std::future::Future<Output = Result<(), ApiError>> + Send;

    fn revoke_all_sessions_except(
        &self,
        user_id: &Uuid,
        except_session_id: &Uuid,
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
    secret_cipher: Arc<dyn crate::auth::secret_cipher::SecretCipher + Send + Sync>,
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
            .field("secret_cipher", &"<redacted>")
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

    async fn authenticate(&self, token: &SessionToken) -> Result<Option<AuthContext>, ApiError> {
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

    async fn request_email_verification(&self, user_id: &Uuid) -> Result<(), ApiError> {
        AuthService::request_email_verification(self, user_id).await
    }

    async fn verify_email(&self, token: &str) -> Result<(), ApiError> {
        AuthService::verify_email(self, token).await
    }

    async fn list_sessions(
        &self,
        user_id: &Uuid,
        current_session_id: Option<Uuid>,
    ) -> Result<Vec<crate::auth::contracts::ActiveSessionResponse>, ApiError> {
        AuthService::list_sessions(self, user_id, current_session_id).await
    }

    async fn revoke_session(&self, session_id: &Uuid, user_id: &Uuid) -> Result<(), ApiError> {
        AuthService::revoke_session(self, session_id, user_id).await
    }

    async fn revoke_other_sessions(
        &self,
        user_id: &Uuid,
        current_session_id: &Uuid,
    ) -> Result<usize, ApiError> {
        AuthService::revoke_other_sessions(self, user_id, current_session_id).await
    }

    async fn generate_backup_codes(&self, user_id: &Uuid) -> Result<Vec<String>, ApiError> {
        AuthService::generate_backup_codes(self, user_id).await
    }

    async fn verify_backup_code(&self, user_id: &Uuid, code: &str) -> Result<bool, ApiError> {
        AuthService::verify_backup_code(self, user_id, code).await
    }

    async fn backup_codes_status(
        &self,
        user_id: &Uuid,
    ) -> Result<crate::auth::contracts::BackupCodeStatusResponse, ApiError> {
        AuthService::backup_codes_status(self, user_id).await
    }

    async fn setup_two_factor(
        &self,
        user_id: &Uuid,
        email: &str,
    ) -> Result<crate::auth::contracts::TwoFactorSetupResponse, ApiError> {
        AuthService::setup_two_factor(self, user_id, email).await
    }

    async fn enable_two_factor(&self, user_id: &Uuid, code: &str) -> Result<(), ApiError> {
        AuthService::enable_two_factor(self, user_id, code).await
    }

    async fn disable_two_factor(
        &self,
        user_id: &Uuid,
        password: &str,
        code: &str,
    ) -> Result<(), ApiError> {
        AuthService::disable_two_factor(self, user_id, password, code).await
    }

    async fn two_factor_status(
        &self,
        user_id: &Uuid,
    ) -> Result<crate::auth::contracts::TwoFactorStatusResponse, ApiError> {
        AuthService::two_factor_status(self, user_id).await
    }
}

#[async_trait::async_trait]
impl<S> Authenticator for AuthService<S>
where
    S: AuthStorage + 'static,
{
    async fn authenticate(&self, token: &SessionToken) -> Result<Option<AuthContext>, ApiError> {
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
            secret_cipher: Arc::new(crate::auth::secret_cipher::TestSecretCipher::new()),
        }
    }

    /// Full constructor with all dependencies.
    ///
    /// Production MUST provide a non-passthrough SecretCipher.
    pub fn with_dependencies(
        storage: S,
        email_provider: Arc<dyn crate::auth::email::EmailProvider + Send + Sync>,
        secret_cipher: Arc<dyn crate::auth::secret_cipher::SecretCipher + Send + Sync>,
    ) -> Self {
        Self {
            storage: Arc::new(storage),
            password_hasher: PasswordHasher::new(),
            session_duration_days: SESSION_DURATION_DAYS,
            rate_limiter: Arc::new(crate::auth::rate_limit::InMemoryRateLimiter::default()),
            events: Arc::new(crate::auth::events::storage::InMemorySecurityEventStorage::new()),
            email_provider,
            secret_cipher,
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
            secret_cipher: Arc::new(crate::auth::secret_cipher::TestSecretCipher::new()),
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
    ) -> Result<Option<AuthContext>, ApiError> {
        self.authenticate_at(token, Utc::now()).await
    }

    pub async fn authenticate_at(
        &self,
        token: &SessionToken,
        now: DateTime<Utc>,
    ) -> Result<Option<AuthContext>, ApiError> {
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

        Ok(Some(AuthContext { user, session }))
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
    pub async fn request_email_verification(&self, user_id: &Uuid) -> Result<(), ApiError> {
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

    pub async fn list_sessions(
        &self,
        user_id: &Uuid,
        current_session_id: Option<Uuid>,
    ) -> Result<Vec<crate::auth::contracts::ActiveSessionResponse>, ApiError> {
        let sessions = self
            .storage
            .get_active_sessions_for_user(user_id, Utc::now())
            .await?;

        Ok(sessions
            .into_iter()
            .map(|session| crate::auth::contracts::ActiveSessionResponse {
                id: session.id,
                created_at: session.created_at,
                expires_at: session.expires_at,
                last_seen_at: session.last_seen_at,
                ip_address: session.ip_address.clone(),
                user_agent: session.user_agent.clone(),
                current: current_session_id == Some(session.id),
            })
            .collect())
    }

    pub async fn revoke_session(&self, session_id: &Uuid, user_id: &Uuid) -> Result<(), ApiError> {
        self.storage
            .revoke_session_by_id(session_id, user_id, Utc::now())
            .await?;

        self.emit_event(SecurityEvent::SessionRevoked {
            metadata: SecurityMetadata::new(None, None),
            user_id: *user_id,
            session_id: *session_id,
        })
        .await;

        metrics::gauge!("auth_active_sessions").decrement(1.0);

        Ok(())
    }

    pub async fn revoke_other_sessions(
        &self,
        user_id: &Uuid,
        current_session_id: &Uuid,
    ) -> Result<usize, ApiError> {
        let revoked = self
            .storage
            .revoke_all_sessions_except(user_id, current_session_id, Utc::now())
            .await?;

        self.emit_event(SecurityEvent::AllSessionsRevoked {
            metadata: SecurityMetadata::new(None, None),
            user_id: *user_id,
            revoked_count: revoked,
        })
        .await;

        for _ in 0..revoked {
            metrics::gauge!("auth_active_sessions").decrement(1.0);
        }

        Ok(revoked)
    }

    /// Generate a new set of backup codes for the user.
    ///
    /// AUTH-24 — Backup Codes
    ///
    /// Old active codes are revoked, new codes are hashed with Argon2id,
    /// and plaintext codes are returned to the caller exactly once.
    pub async fn generate_backup_codes(&self, user_id: &Uuid) -> Result<Vec<String>, ApiError> {
        // Revoke all existing active backup codes
        let revoked = self
            .storage
            .revoke_all_backup_codes(user_id, Utc::now())
            .await?;

        if revoked > 0 {
            self.emit_event(SecurityEvent::BackupCodesRevoked {
                metadata: SecurityMetadata::new(None, None),
                user_id: *user_id,
            })
            .await;
        }

        let plaintext_codes = crate::auth::backup_codes::generate_set();
        let set_id = Uuid::new_v4();
        let mut count = 0;

        for plaintext in &plaintext_codes {
            let normalized = crate::auth::backup_codes::normalize_code(plaintext);
            let code_hash = self
                .password_hasher
                .hash(&normalized)
                .map_err(|_| ApiError::Internal)?;

            let code = BackupCode::new(*user_id, set_id, code_hash);
            self.storage.create_backup_code(&code).await?;
            count += 1;
        }

        self.emit_event(SecurityEvent::BackupCodesGenerated {
            metadata: SecurityMetadata::new(None, None),
            user_id: *user_id,
            count,
        })
        .await;

        Ok(plaintext_codes)
    }

    /// Verify and consume a backup code.
    pub async fn verify_backup_code(&self, user_id: &Uuid, code: &str) -> Result<bool, ApiError> {
        let normalized = crate::auth::backup_codes::normalize_code(code);

        if !crate::auth::backup_codes::is_valid_normalized_code(&normalized) {
            self.emit_event(SecurityEvent::BackupCodeVerificationFailed {
                metadata: SecurityMetadata::new(None, None),
                user_id: *user_id,
            })
            .await;
            return Ok(false);
        }

        let codes = self.storage.list_backup_codes(user_id).await?;

        for stored in codes {
            if !stored.is_active() {
                continue;
            }

            let matches = self
                .password_hasher
                .verify(&normalized, &stored.code_hash)
                .map_err(|_| ApiError::Internal)?;

            if matches {
                let consumed = self
                    .storage
                    .consume_backup_code(user_id, &stored.code_hash, Utc::now())
                    .await?;

                if consumed {
                    let remaining = self
                        .storage
                        .list_backup_codes(user_id)
                        .await?
                        .iter()
                        .filter(|c| c.is_active())
                        .count();

                    self.emit_event(SecurityEvent::BackupCodeUsed {
                        metadata: SecurityMetadata::new(None, None),
                        user_id: *user_id,
                        remaining,
                    })
                    .await;

                    return Ok(true);
                }
            }
        }

        self.emit_event(SecurityEvent::BackupCodeVerificationFailed {
            metadata: SecurityMetadata::new(None, None),
            user_id: *user_id,
        })
        .await;

        Ok(false)
    }

    /// Get backup codes status for the user.
    pub async fn backup_codes_status(
        &self,
        user_id: &Uuid,
    ) -> Result<crate::auth::contracts::BackupCodeStatusResponse, ApiError> {
        let codes = self.storage.list_backup_codes(user_id).await?;

        let total = codes.len();
        let remaining = codes.iter().filter(|c| c.is_active()).count();
        let generated_at = codes.iter().map(|c| c.created_at).max();

        Ok(crate::auth::contracts::BackupCodeStatusResponse {
            enabled: total > 0,
            remaining,
            total,
            generated_at,
        })
    }

    /// Start TOTP enrollment.
    ///
    /// AUTH-25.3 — Two-Factor Setup
    ///
    /// Generates a 160-bit CSPRNG secret, stores it as Pending (encrypted),
    /// and returns the otpauth URI + base32 secret to the caller.
    ///
    /// Plaintext secret is returned ONCE and never logged.
    pub async fn setup_two_factor(
        &self,
        user_id: &Uuid,
        email: &str,
    ) -> Result<crate::auth::contracts::TwoFactorSetupResponse, ApiError> {
        use rand::rngs::OsRng;
        use rand::RngCore;
        use totp_rs::TOTP;

        use crate::auth::two_factor::profile;

        // Generate 160-bit secret via OS CSPRNG
        let mut secret_bytes = zeroize::Zeroizing::new([0u8; profile::SECRET_BYTES]);
        OsRng
            .try_fill_bytes(&mut secret_bytes[..])
            .map_err(|_| ApiError::Internal)?;

        // Build TOTP object for URI generation
        let totp = TOTP::new(
            profile::ALGORITHM,
            profile::DIGITS,
            profile::SKEW,
            profile::STEP_SECONDS,
            secret_bytes.to_vec(),
            Some(profile::ISSUER.to_string()),
            email.to_string(),
        )
        .map_err(|_| ApiError::Internal)?;

        let otpauth_uri = totp.get_url();
        let secret_base32 = totp.get_secret_base32();

        // Derive domain-separated object_id
        let object_id = crate::auth::two_factor::TotpSecretObjectId::for_user(user_id);

        // Encrypt secret via SecretCipher
        let encrypted_secret = self
            .secret_cipher
            .encrypt(object_id, &secret_bytes[..])
            .await?;

        // Store as Pending (replaces any existing pending)
        let settings = crate::auth::two_factor::TwoFactorSettings::new_pending(
            *user_id,
            encrypted_secret,
        );
        self.storage.create_two_factor_settings(&settings).await?;

        Ok(crate::auth::contracts::TwoFactorSetupResponse {
            otpauth_uri,
            secret_base32,
        })
    }

    /// Confirm TOTP enrollment with the first valid code.
    ///
    /// AUTH-25.3 — Two-Factor Enable
    ///
    /// On success, transitions Pending → Enabled and generates backup codes.
    pub async fn enable_two_factor(&self, user_id: &Uuid, code: &str) -> Result<(), ApiError> {
        // Serialize per-user atomic operations.
        let lock = self.storage.lock_for_user(user_id).await;
        let _guard = lock.lock().await;

        let mut settings = self
            .storage
            .get_two_factor_settings(user_id)
            .await?
            .ok_or(ApiError::Unauthorized)?;

        if !matches!(
            settings.state,
            crate::auth::two_factor::TwoFactorState::Pending
        ) {
            return Err(ApiError::BadRequest);
        }

        if settings.is_pending_expired(Utc::now()) {
            self.storage.delete_two_factor_settings(user_id).await?;
            return Err(ApiError::Unauthorized);
        }

        let valid = self.verify_totp_and_mark_step(&mut settings, code).await?;

        if !valid {
            self.emit_event(SecurityEvent::TwoFactorVerificationFailed {
                metadata: SecurityMetadata::new(None, None),
                user_id: *user_id,
            })
            .await;
            return Err(ApiError::Unauthorized);
        }

        // Transition Pending → Enabled.
        settings.state = crate::auth::two_factor::TwoFactorState::Enabled;
        settings.enabled_at = Some(Utc::now());
        settings.expires_at = None;

        self.storage.update_two_factor_settings(&settings).await?;

        // Generate backup codes atomically with enable.
        // Storage failure here leaves 2FA enabled without recovery codes;
        // generate_backup_codes is best-effort but must not be skipped.
        let _plaintext_codes = self.generate_backup_codes(user_id).await?;

        self.emit_event(SecurityEvent::TwoFactorEnabled {
            metadata: SecurityMetadata::new(None, None),
            user_id: *user_id,
        })
        .await;

        Ok(())
    }

    /// Disable two-factor authentication.
    ///
    /// AUTH-25.3 — Two-Factor Disable
    ///
    /// Requires password + current valid TOTP code.
    pub async fn disable_two_factor(
        &self,
        user_id: &Uuid,
        password: &str,
        code: &str,
    ) -> Result<(), ApiError> {
        // Verify password first (no need to hold the lock for this).
        let user = self
            .storage
            .get_user_by_id(user_id)
            .await?
            .ok_or(ApiError::Unauthorized)?;

        let password_valid = self
            .password_hasher
            .verify(password, &user.password_hash)
            .map_err(|_| ApiError::Unauthorized)?;

        if !password_valid {
            return Err(ApiError::Unauthorized);
        }

        // Serialize per-user atomic operations.
        let lock = self.storage.lock_for_user(user_id).await;
        let _guard = lock.lock().await;

        let mut settings = self
            .storage
            .get_two_factor_settings(user_id)
            .await?
            .ok_or(ApiError::Unauthorized)?;

        if !matches!(
            settings.state,
            crate::auth::two_factor::TwoFactorState::Enabled
        ) {
            return Err(ApiError::BadRequest);
        }

        let valid = self.verify_totp_and_mark_step(&mut settings, code).await?;

        if !valid {
            self.emit_event(SecurityEvent::TwoFactorVerificationFailed {
                metadata: SecurityMetadata::new(None, None),
                user_id: *user_id,
            })
            .await;
            return Err(ApiError::Unauthorized);
        }

        // Atomic disable: delete settings + revoke backup codes.
        self.storage.delete_two_factor_settings(user_id).await?;
        self.storage
            .revoke_all_backup_codes(user_id, Utc::now())
            .await?;

        self.emit_event(SecurityEvent::TwoFactorDisabled {
            metadata: SecurityMetadata::new(None, None),
            user_id: *user_id,
        })
        .await;

        Ok(())
    }

    /// Get current two-factor status.
    pub async fn two_factor_status(
        &self,
        user_id: &Uuid,
    ) -> Result<crate::auth::contracts::TwoFactorStatusResponse, ApiError> {
        let settings = self.storage.get_two_factor_settings(user_id).await?;

        let (enabled, state) = match settings {
            Some(s) if matches!(s.state, crate::auth::two_factor::TwoFactorState::Enabled) => {
                (true, "enabled".to_string())
            }
            Some(s) if matches!(s.state, crate::auth::two_factor::TwoFactorState::Pending) => {
                (false, "pending".to_string())
            }
            _ => (false, "disabled".to_string()),
        };

        Ok(crate::auth::contracts::TwoFactorStatusResponse { enabled, state })
    }

    /// Verify a TOTP code and mark its time step as accepted.
    ///
    /// AUTH-25 — Replay protection.
    ///
    /// Rejects codes whose time step has already been accepted.
    ///
    /// IMPORTANT: The caller MUST:
    /// 1. Hold `storage.lock_for_user(user_id)` for the entire
    ///    read → verify → update → persist cycle.
    /// 2. Persist the modified settings via `update_two_factor_settings`
    ///    before releasing the lock.
    ///
    /// Without both, replay protection is not enforced.
    async fn verify_totp_and_mark_step(
        &self,
        settings: &mut crate::auth::two_factor::TwoFactorSettings,
        code: &str,
    ) -> Result<bool, ApiError> {
        use totp_rs::TOTP;

        use crate::auth::two_factor::profile;

        let secret_bytes = self
            .secret_cipher
            .decrypt(&settings.encrypted_secret)
            .await?;

        let totp = TOTP::new(
            profile::ALGORITHM,
            profile::DIGITS,
            profile::SKEW,
            profile::STEP_SECONDS,
            secret_bytes.to_vec(),
            Some(profile::ISSUER.to_string()),
            settings.user_id.to_string(),
        )
        .map_err(|_| ApiError::Internal)?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| ApiError::Internal)?
            .as_secs();

        if !totp.check(code, now) {
            return Ok(false);
        }

        // Current time step.
        let step = now / profile::STEP_SECONDS;

        // Replay protection.
        if let Some(last) = settings.last_accepted_step {
            if step <= last {
                return Ok(false);
            }
        }

        settings.last_accepted_step = Some(step);
        Ok(true)
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
