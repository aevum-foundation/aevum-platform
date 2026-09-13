//! Application-facing authentication API.
//!
//! AUTH-17 — Storage Agnostic Refactor
//!
//! This trait defines the interface used by HTTP handlers.
//! Concrete AuthService instances implement this trait, allowing
//! the API layer to work without knowledge of the underlying storage.

use async_trait::async_trait;
use uuid::Uuid;

use crate::auth::csrf::CsrfToken;
use crate::auth::models::{AuthContext, User};
use crate::auth::password::SessionToken;
use crate::error::ApiError;

/// High-level authentication API used by HTTP handlers.
///
/// Implementations:
/// - AuthService<InMemoryAuthStorage>
/// - AuthService<AevumDbAuthStorage>
#[async_trait]
pub trait AuthApi: Send + Sync {
    async fn register(&self, email: &str, password: &str, ip: &str) -> Result<User, ApiError>;

    async fn login(
        &self,
        email: &str,
        password: &str,
        ip: &str,
    ) -> Result<crate::auth::models::LoginResult, ApiError>;

    async fn logout(&self, token: &SessionToken) -> Result<(), ApiError>;

    async fn authenticate(&self, token: &SessionToken) -> Result<Option<AuthContext>, ApiError>;

    async fn rotate_session(
        &self,
        token: &SessionToken,
    ) -> Result<(SessionToken, CsrfToken), ApiError>;

    async fn change_password(
        &self,
        user_id: &Uuid,
        current_password: &str,
        new_password: &str,
    ) -> Result<SessionToken, ApiError>;

    async fn request_password_reset(&self, email: &str, ip: &str) -> Result<(), ApiError>;

    async fn confirm_password_reset(&self, token: &str, new_password: &str)
        -> Result<(), ApiError>;

    async fn request_email_verification(&self, user_id: &Uuid) -> Result<(), ApiError>;

    async fn verify_email(&self, token: &str) -> Result<(), ApiError>;

    async fn list_sessions(
        &self,
        user_id: &Uuid,
        current_session_id: Option<Uuid>,
    ) -> Result<Vec<crate::auth::contracts::ActiveSessionResponse>, ApiError>;

    async fn revoke_session(&self, session_id: &Uuid, user_id: &Uuid) -> Result<(), ApiError>;

    async fn revoke_other_sessions(
        &self,
        user_id: &Uuid,
        current_session_id: &Uuid,
    ) -> Result<usize, ApiError>;

    async fn generate_backup_codes(
        &self,
        user_id: &Uuid,
    ) -> Result<Vec<String>, ApiError>;

    async fn verify_backup_code(
        &self,
        user_id: &Uuid,
        code: &str,
    ) -> Result<bool, ApiError>;

    async fn backup_codes_status(
        &self,
        user_id: &Uuid,
    ) -> Result<crate::auth::contracts::BackupCodeStatusResponse, ApiError>;

    async fn setup_two_factor(
        &self,
        user_id: &Uuid,
        email: &str,
    ) -> Result<crate::auth::contracts::TwoFactorSetupResponse, ApiError>;

    async fn enable_two_factor(&self, user_id: &Uuid, code: &str) -> Result<Vec<String>, ApiError>;

    async fn disable_two_factor(
        &self,
        user_id: &Uuid,
        password: &str,
        code: &str,
    ) -> Result<(), ApiError>;

    async fn two_factor_status(
        &self,
        user_id: &Uuid,
    ) -> Result<crate::auth::contracts::TwoFactorStatusResponse, ApiError>;

    async fn verify_two_factor(
        &self,
        pre_auth_token: &str,
        code: &str,
    ) -> Result<(crate::auth::models::User, SessionToken), ApiError>;

    async fn get_preferences(
        &self,
        user_id: &Uuid,
    ) -> Result<crate::auth::preferences::UserPreferences, ApiError>;

    async fn update_preferences(
        &self,
        user_id: &Uuid,
        update: crate::auth::preferences::UserPreferencesUpdate,
    ) -> Result<crate::auth::preferences::UserPreferences, ApiError>;

    async fn upload_avatar(
        &self,
        user_id: &Uuid,
        content_type: &str,
        data: &[u8],
    ) -> Result<crate::auth::avatar::Avatar, ApiError>;

    async fn get_avatar(
        &self,
        user_id: &Uuid,
    ) -> Result<Option<(crate::auth::avatar::Avatar, zeroize::Zeroizing<Vec<u8>>)>, ApiError>;

    async fn delete_avatar(&self, user_id: &Uuid) -> Result<(), ApiError>;
}
