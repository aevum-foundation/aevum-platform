//! Application-facing Community API.
//!
//! B-1.2b.4 — stable application contract between HTTP handlers and the
//! Community service layer.

use async_trait::async_trait;
use uuid::Uuid;

use crate::community::contracts::{
    PrivateProfileResponse, PublicProfileResponse, UpdateProfileRequest,
};
use crate::error::ApiError;

/// High-level Community API used by HTTP handlers.
///
/// The HTTP layer depends only on this trait and therefore does not know
/// which storage backend is active.
#[async_trait]
pub trait CommunityApi: Send + Sync {
    /// Create or update the authenticated user's profile.
    ///
    /// Creation requires a username.
    ///
    /// On update:
    /// - username omitted => unchanged;
    /// - same normalized username => idempotent;
    /// - different normalized username => conflict.
    async fn create_or_update_profile(
        &self,
        user_id: &Uuid,
        request: UpdateProfileRequest,
    ) -> Result<PrivateProfileResponse, ApiError>;

    /// Return the authenticated user's private community profile.
    async fn get_own_profile(&self, user_id: &Uuid) -> Result<PrivateProfileResponse, ApiError>;

    /// Return a public profile by username.
    ///
    /// Username lookup is normalized according to the Community validation
    /// contract.
    async fn get_public_profile(&self, username: &str) -> Result<PublicProfileResponse, ApiError>;
}
