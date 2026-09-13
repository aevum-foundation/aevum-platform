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
    AuthContext, BackupCode, EmailVerificationToken, PasswordResetToken, PreAuthToken, Session,
    SessionTokenHash, User,
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

    fn create_pre_auth_token(
        &self,
        token: &PreAuthToken,
    ) -> impl std::future::Future<Output = Result<(), ApiError>> + Send;

    fn get_pre_auth_token_by_hash(
        &self,
        token_hash: &str,
    ) -> impl std::future::Future<Output = Result<Option<PreAuthToken>, ApiError>> + Send;

    fn consume_pre_auth_token(
        &self,
        token_hash: &str,
        consumed_at: DateTime<Utc>,
    ) -> impl std::future::Future<Output = Result<(), ApiError>> + Send;

    fn get_user_preferences(
        &self,
        user_id: &Uuid,
    ) -> impl std::future::Future<
        Output = Result<Option<crate::auth::preferences::UserPreferences>, ApiError>,
    > + Send;

    fn upsert_user_preferences(
        &self,
        preferences: &crate::auth::preferences::UserPreferences,
    ) -> impl std::future::Future<Output = Result<(), ApiError>> + Send;

    fn get_avatar(
        &self,
        user_id: &Uuid,
    ) -> impl std::future::Future<
        Output = Result<Option<crate::auth::avatar::Avatar>, ApiError>,
    > + Send;

    fn upsert_avatar(
        &self,
        avatar: &crate::auth::avatar::Avatar,
    ) -> impl std::future::Future<Output = Result<(), ApiError>> + Send;

    fn delete_avatar(
        &self,
        user_id: &Uuid,
    ) -> impl std::future::Future<Output = Result<(), ApiError>> + Send;

    fn get_avatar_blob(
        &self,
        blob_key: &str,
    ) -> impl std::future::Future<
        Output = Result<Option<zeroize::Zeroizing<Vec<u8>>>, ApiError>,
    > + Send;

    fn put_avatar_blob(
        &self,
        blob_key: &str,
        data: &[u8],
    ) -> impl std::future::Future<Output = Result<(), ApiError>> + Send;

    fn delete_avatar_blob(
        &self,
        blob_key: &str,
    ) -> impl std::future::Future<Output = Result<(), ApiError>> + Send;

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
    ) -> Result<crate::auth::models::LoginResult, ApiError> {
        AuthService::login(self, email, password, ip).await
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

    async fn enable_two_factor(&self, user_id: &Uuid, code: &str) -> Result<Vec<String>, ApiError> {
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

    async fn verify_two_factor(
        &self,
        pre_auth_token: &str,
        code: &str,
    ) -> Result<(User, SessionToken), ApiError> {
        AuthService::verify_two_factor(self, pre_auth_token, code).await
    }

    async fn get_preferences(
        &self,
        user_id: &Uuid,
    ) -> Result<crate::auth::preferences::UserPreferences, ApiError> {
        AuthService::get_preferences(self, user_id).await
    }

    async fn update_preferences(
        &self,
        user_id: &Uuid,
        update: crate::auth::preferences::UserPreferencesUpdate,
    ) -> Result<crate::auth::preferences::UserPreferences, ApiError> {
        AuthService::update_preferences(self, user_id, update).await
    }

    async fn upload_avatar(
        &self,
        user_id: &Uuid,
        content_type: &str,
        data: &[u8],
    ) -> Result<crate::auth::avatar::Avatar, ApiError> {
        AuthService::upload_avatar(self, user_id, content_type, data).await
    }

    async fn get_avatar(
        &self,
        user_id: &Uuid,
    ) -> Result<Option<(crate::auth::avatar::Avatar, zeroize::Zeroizing<Vec<u8>>)>, ApiError> {
        AuthService::get_avatar(self, user_id).await
    }

    async fn delete_avatar(&self, user_id: &Uuid) -> Result<(), ApiError> {
        AuthService::delete_avatar(self, user_id).await
    }

    async fn get_security_center(
        &self,
        user_id: &Uuid,
    ) -> Result<crate::auth::security_center::SecurityCenterResponse, ApiError> {
        AuthService::get_security_center(self, user_id).await
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
    ) -> Result<crate::auth::models::LoginResult, ApiError> {
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

        // Check 2FA
        let settings = self.storage.get_two_factor_settings(&user.id).await?;
        let requires_2fa = matches!(
            settings.map(|s| s.state),
            Some(crate::auth::two_factor::TwoFactorState::Enabled)
        );

        if requires_2fa {
            // Issue PreAuthToken
            let pre_auth_token = SessionToken::generate();
            let token_hash = SessionTokenHash::from_token(&pre_auth_token);
            let model = crate::auth::models::PreAuthToken::new(
                user.id,
                token_hash.as_str().to_owned(),
                crate::auth::contracts::PRE_AUTH_TOKEN_TTL_SECONDS,
            );

            self.storage.create_pre_auth_token(&model).await?;

            self.emit_event(SecurityEvent::TwoFactorChallengeIssued {
                metadata: metadata.clone(),
                user_id: user.id,
            })
            .await;

            return Ok(crate::auth::models::LoginResult::RequiresTwoFactor {
                pre_auth_token: pre_auth_token.expose().to_owned(),
                expires_in_seconds: crate::auth::contracts::PRE_AUTH_TOKEN_TTL_SECONDS,
            });
        }

        let (user, session_token) = self.create_session_for_user(user).await?;

        Ok(crate::auth::models::LoginResult::Session {
            user,
            session_token,
        })
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
    pub async fn enable_two_factor(&self, user_id: &Uuid, code: &str) -> Result<Vec<String>, ApiError> {
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

        // Enrollment confirmation: verify TOTP but do NOT consume the
        // time step. Replay protection applies to authentication
        // attempts (/2fa/verify), not to enrollment.
        let valid = self.check_totp_code(&settings, code).await?;

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
        // Plaintext codes are returned to the caller exactly once
        // and never persisted or logged.
        let plaintext_codes = self.generate_backup_codes(user_id).await?;

        self.emit_event(SecurityEvent::TwoFactorEnabled {
            metadata: SecurityMetadata::new(None, None),
            user_id: *user_id,
        })
        .await;

        Ok(plaintext_codes)
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
    /// Check a TOTP code without mutating replay state.
    ///
    /// Used by enrollment (`/2fa/enable`), where consuming a time step
    /// would incorrectly prevent the user from immediately logging in
    /// with the same authenticator code.
    async fn check_totp_code(
        &self,
        settings: &crate::auth::two_factor::TwoFactorSettings,
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

        Ok(totp.check(code, now))
    }

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

    /// Verify a 2FA challenge and create a real session.
    ///
    /// AUTH-25.4 — 2FA Verify
    ///
    /// Flow:
    /// 1. Load PreAuthToken by hash
    /// 2. Validate TTL and consumed state
    /// 3. Verify TOTP (with replay protection under user lock)
    /// 4. Fallback to backup code if TOTP fails
    /// 5. Create Session
    /// 6. Consume PreAuthToken (only after success)
    pub async fn verify_two_factor(
        &self,
        pre_auth_token: &str,
        code: &str,
    ) -> Result<(User, SessionToken), ApiError> {
        // Decode pre-auth token
        let token = SessionToken::from_secret(pre_auth_token.to_owned());
        let token_hash = SessionTokenHash::from_token(&token);
        let token_hash_str = token_hash.as_str().to_owned();

        // Load PreAuthToken
        let pre_auth = self
            .storage
            .get_pre_auth_token_by_hash(&token_hash_str)
            .await?
            .ok_or(ApiError::Unauthorized)?;

        if !pre_auth.is_valid_at(Utc::now()) {
            return Err(ApiError::Unauthorized);
        }

        let user_id = pre_auth.user_id;

        // Rate limit: per-user 2FA attempts
        self.rate_limiter.check_two_factor_attempts(&user_id).await?;

        // Acquire per-user lock for atomic verify + consume
        let lock = self.storage.lock_for_user(&user_id).await;
        let _guard = lock.lock().await;

        // Load user
        let user = self
            .storage
            .get_user_by_id(&user_id)
            .await?
            .ok_or(ApiError::Unauthorized)?;

        if !user.status.can_authenticate() {
            return Err(ApiError::Unauthorized);
        }

        // Try TOTP
        let totp_valid = if let Some(mut settings) =
            self.storage.get_two_factor_settings(&user_id).await?
        {
            if !matches!(
                settings.state,
                crate::auth::two_factor::TwoFactorState::Enabled
            ) {
                false
            } else {
                let valid = self.verify_totp_and_mark_step(&mut settings, code).await?;
                if valid {
                    self.storage.update_two_factor_settings(&settings).await?;
                }
                valid
            }
        } else {
            false
        };

        if totp_valid {
            self.emit_event(SecurityEvent::TwoFactorVerificationSucceeded {
                metadata: SecurityMetadata::new(None, None),
                user_id,
            })
            .await;
        } else {
            // Fallback to backup code
            let backup_valid = self.verify_backup_code(&user_id, code).await?;

            if !backup_valid {
                self.rate_limiter.record_two_factor_failure(&user_id).await;

                self.emit_event(SecurityEvent::TwoFactorVerificationFailed {
                    metadata: SecurityMetadata::new(None, None),
                    user_id,
                })
                .await;
                return Err(ApiError::Unauthorized);
            }
        }

        // Success: clear 2FA attempt counter
        self.rate_limiter.record_two_factor_success(&user_id).await;

        // Consume pre-auth token ONLY after successful verification
        self.storage
            .consume_pre_auth_token(&token_hash_str, Utc::now())
            .await?;

        // Create real session
        self.create_session_for_user(user).await
    }

    /// Get user preferences, returning defaults if none exist.
    ///
    /// AUTH-26 — User Preferences
    pub async fn get_preferences(
        &self,
        user_id: &Uuid,
    ) -> Result<crate::auth::preferences::UserPreferences, ApiError> {
        let stored = self.storage.get_user_preferences(user_id).await?;
        Ok(stored.unwrap_or_else(|| crate::auth::preferences::UserPreferences::defaults(*user_id)))
    }

    /// Update user preferences (partial update, server-owned fields preserved).
    pub async fn update_preferences(
        &self,
        user_id: &Uuid,
        update: crate::auth::preferences::UserPreferencesUpdate,
    ) -> Result<crate::auth::preferences::UserPreferences, ApiError> {
        let mut prefs = self.get_preferences(user_id).await?;

        if let Some(theme) = update.theme {
            prefs.theme = theme;
        }
        if let Some(language) = update.language {
            prefs.language = language;
        }
        if let Some(notifications_enabled) = update.notifications_enabled {
            prefs.notifications_enabled = notifications_enabled;
        }

        prefs.updated_at = Utc::now();

        self.storage.upsert_user_preferences(&prefs).await?;

        Ok(prefs)
    }

    /// Upload (or replace) the user's avatar.
    ///
    /// AUTH-27.3 — Avatar Upload
    ///
    /// Flow:
    /// 1. Validate payload (magic bytes + dimensions).
    /// 2. Encrypt payload via SecretCipher (CryptoDomain::Blob).
    /// 3. Write new blob to storage.
    /// 4. Atomically replace metadata (same blob_key, deterministic).
    /// 5. Old payload is overwritten by the new one (same key).
    ///
    /// Atomicity note: because blob_key is deterministic per user,
    /// the new payload overwrites the old one at the same key, so
    /// metadata and payload remain consistent even across failures.
    pub async fn upload_avatar(
        &self,
        user_id: &Uuid,
        content_type: &str,
        data: &[u8],
    ) -> Result<crate::auth::avatar::Avatar, ApiError> {
        let mime = crate::auth::avatar::validate_avatar(content_type, data)
            .map_err(|_| ApiError::BadRequest)?;

        let blob_key = crate::auth::avatar::Avatar::blob_key_for(user_id);

        // Derive object_id for envelope binding
        let object_id = {
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(b"AEVUM_AUTH_AVATAR_BLOB_OBJECT_ID_V1");
            hasher.update(user_id.as_bytes());
            let digest = hasher.finalize();
            u64::from_be_bytes([
                digest[0], digest[1], digest[2], digest[3],
                digest[4], digest[5], digest[6], digest[7],
            ])
        };

        let encrypted = self
            .secret_cipher
            .encrypt(object_id, data)
            .await?;

        self.storage
            .put_avatar_blob(&blob_key, &encrypted)
            .await?;

        let avatar = crate::auth::avatar::Avatar {
            user_id: *user_id,
            mime_type: mime,
            size: data.len() as u64,
            content_hash: crate::auth::avatar::Avatar::hash_bytes(data),
            blob_key,
            uploaded_at: Utc::now(),
        };

        self.storage.upsert_avatar(&avatar).await?;

        Ok(avatar)
    }

    /// Fetch the user's avatar metadata + decrypted payload.
    pub async fn get_avatar(
        &self,
        user_id: &Uuid,
    ) -> Result<Option<(crate::auth::avatar::Avatar, zeroize::Zeroizing<Vec<u8>>)>, ApiError> {
        let Some(avatar) = self.storage.get_avatar(user_id).await? else {
            return Ok(None);
        };

        let Some(encrypted) = self.storage.get_avatar_blob(&avatar.blob_key).await? else {
            return Ok(None);
        };

        let plaintext = self.secret_cipher.decrypt(&encrypted).await?;

        Ok(Some((avatar, plaintext)))
    }

    /// Delete the user's avatar (metadata + blob).
    pub async fn delete_avatar(&self, user_id: &Uuid) -> Result<(), ApiError> {
        let Some(avatar) = self.storage.get_avatar(user_id).await? else {
            return Ok(());
        };

        self.storage.delete_avatar_blob(&avatar.blob_key).await?;
        self.storage.delete_avatar(user_id).await?;

        Ok(())
    }

    /// Read-only security posture summary for the authenticated account.
    ///
    /// AUTH-28 — Security Center
    ///
    /// Pure aggregation: reads existing state, computes score.
    /// Performs no mutations, no new persistence.
    pub async fn get_security_center(
        &self,
        user_id: &Uuid,
    ) -> Result<crate::auth::security_center::SecurityCenterResponse, ApiError> {
        use crate::auth::events::SecurityEventKind;
        use crate::auth::security_center::{
            calculate_security_score, password_changed_recently, EventSummary,
            SecurityCenterResponse, SecurityScoreInput,
            SECURITY_CENTER_EVENT_LIMIT,
        };

        // User (email_verified)
        let user = self
            .storage
            .get_user_by_id(user_id)
            .await?
            .ok_or(ApiError::Unauthorized)?;

        // 2FA state
        let two_factor_enabled = matches!(
            self.storage
                .get_two_factor_settings(user_id)
                .await?
                .map(|s| s.state),
            Some(crate::auth::two_factor::TwoFactorState::Enabled)
        );

        // Backup codes remaining
        let backup_codes = self.storage.list_backup_codes(user_id).await?;
        let backup_codes_remaining = backup_codes.iter().filter(|c| c.is_active()).count();
        let has_backup_codes = backup_codes_remaining > 0;

        // Active sessions
        let active_sessions = self
            .storage
            .get_active_sessions_for_user(user_id, Utc::now())
            .await?
            .len();

        // Latest PasswordChanged event
        let password_changed_at = self
            .events
            .get_latest_event_for_user(*user_id, SecurityEventKind::PasswordChanged)
            .await?
            .map(|event| event.timestamp());

        let password_changed_recently =
            password_changed_recently(password_changed_at, Utc::now());

        // Recent events (client-safe summaries)
        let recent_events = self
            .events
            .get_events_for_user(*user_id, SECURITY_CENTER_EVENT_LIMIT)
            .await?
            .iter()
            .map(EventSummary::from_event)
            .collect();

        // Deterministic score
        let security_score = calculate_security_score(SecurityScoreInput {
            email_verified: user.email_verified,
            two_factor_enabled,
            has_backup_codes,
            password_changed_recently,
        });

        Ok(SecurityCenterResponse {
            email_verified: user.email_verified,
            two_factor_enabled,
            backup_codes_remaining,
            active_sessions,
            password_changed_at,
            security_score,
            recent_events,
        })
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
