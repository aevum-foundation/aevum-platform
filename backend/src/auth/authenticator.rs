//! Authentication middleware interface.
//!
//! AUTH-17 — Storage Agnostic Refactor
//!
//! This minimal trait provides only the authentication check
//! needed by middleware, keeping the middleware independent
//! from the broader AuthApi.

use async_trait::async_trait;

use crate::auth::models::User;
use crate::auth::password::SessionToken;
use crate::error::ApiError;

/// Verifies a session token and returns the associated user.
///
/// Implemented by AuthService<S> for any AuthStorage S.
#[async_trait]
pub trait Authenticator: Send + Sync {
    async fn authenticate(
        &self,
        token: &SessionToken,
    ) -> Result<Option<User>, ApiError>;
}
