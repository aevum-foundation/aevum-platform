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
use crate::auth::models::User;
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
    ) -> Result<(User, SessionToken, CsrfToken), ApiError>;

    async fn logout(&self, token: &SessionToken) -> Result<(), ApiError>;

    async fn authenticate(&self, token: &SessionToken) -> Result<Option<User>, ApiError>;

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
}
