//! Community service — business logic over `CommunityStorage`.
//!
//! B-1.2b.4 — CommunityService<S>.
//!
//! Responsibilities:
//! - validate community profile input;
//! - distinguish create vs update;
//! - enforce immutable usernames;
//! - map storage-level username conflicts to the public API contract;
//! - build public/private response DTOs;
//! - never expose internal storage errors as successful operations.
//!
//! The service is generic over the storage backend and implements
//! `CommunityApi` used by HTTP handlers.

use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use crate::community::api::CommunityApi;
use crate::community::contracts::{
    PrivateProfileResponse, PublicProfileResponse, UpdateProfileRequest,
    USERNAME_ALREADY_EXISTS_CODE, USERNAME_ALREADY_EXISTS_MESSAGE, USERNAME_IMMUTABLE_CODE,
    USERNAME_IMMUTABLE_MESSAGE, USERNAME_REQUIRED_CODE, USERNAME_REQUIRED_MESSAGE,
};
use crate::community::models::{CommunityProfile, CommunityRole};
use crate::community::storage::CommunityStorage;
use crate::community::validation::{
    validate_bio, validate_display_name, validate_username, ValidUsername,
};
use crate::error::ApiError;

/// Application service for Community operations.
///
/// The storage backend is intentionally injected so the same business logic
/// can run against InMemoryCommunityStorage in tests and AevumDbCommunityStorage
/// in production.
pub struct CommunityService<S: CommunityStorage> {
    storage: S,
}

impl<S: CommunityStorage> CommunityService<S> {
    pub fn new(storage: S) -> Self {
        Self { storage }
    }
}

#[async_trait]
impl<S: CommunityStorage + 'static> CommunityApi for CommunityService<S> {
    async fn create_or_update_profile(
        &self,
        user_id: &Uuid,
        request: UpdateProfileRequest,
    ) -> Result<PrivateProfileResponse, ApiError> {
        // Validate fields before touching storage.
        //
        // `username` is intentionally handled separately because it is
        // optional on update but mandatory on initial creation.
        let display_name = validate_display_name(request.display_name.as_deref())?;
        let bio = validate_bio(request.bio.as_deref())?;

        let existing = self.storage.get_profile_by_user_id(user_id).await?;

        match existing {
            None => {
                self.create_profile(user_id, request, display_name, bio)
                    .await
            }

            Some(existing) => {
                self.update_existing_profile(existing, request, display_name, bio)
                    .await
            }
        }
    }

    async fn get_own_profile(&self, user_id: &Uuid) -> Result<PrivateProfileResponse, ApiError> {
        let profile = self
            .storage
            .get_profile_by_user_id(user_id)
            .await?
            .ok_or(ApiError::NotFound)?;

        let role = self.storage.get_role(user_id).await?;

        Ok(to_private_response(&profile, role))
    }

    async fn get_public_profile(&self, username: &str) -> Result<PublicProfileResponse, ApiError> {
        let valid = validate_username(username)?;

        let profile = self
            .storage
            .get_profile_by_username(&valid.normalized)
            .await?
            .ok_or(ApiError::NotFound)?;

        Ok(to_public_response(&profile))
    }
}

impl<S: CommunityStorage + 'static> CommunityService<S> {
    /// Create a brand-new community profile.
    async fn create_profile(
        &self,
        user_id: &Uuid,
        request: UpdateProfileRequest,
        display_name: Option<String>,
        bio: Option<String>,
    ) -> Result<PrivateProfileResponse, ApiError> {
        let raw_username = request.username.ok_or(ApiError::ValidationFailed {
            code: USERNAME_REQUIRED_CODE,
            message: USERNAME_REQUIRED_MESSAGE,
        })?;

        let valid: ValidUsername = validate_username(&raw_username)?;

        let now = Utc::now();

        let profile = CommunityProfile::new(
            *user_id,
            valid.canonical,
            valid.normalized,
            display_name,
            bio,
            now,
        );

        // Storage currently exposes a generic Conflict.
        //
        // Under the Phase-B single-authoritative-writer invariant, the service
        // has already established that this user_id has no profile. Therefore
        // a create conflict is interpreted as username ownership conflict.
        //
        // IMPORTANT: every non-Conflict error is propagated unchanged.
        match self.storage.create_profile(&profile).await {
            Ok(()) => {}

            Err(ApiError::Conflict) => {
                return Err(ApiError::ConflictDetailed {
                    code: USERNAME_ALREADY_EXISTS_CODE,
                    message: USERNAME_ALREADY_EXISTS_MESSAGE,
                });
            }

            Err(other) => {
                return Err(other);
            }
        }

        let role = self.storage.get_role(user_id).await?;

        Ok(to_private_response(&profile, role))
    }

    /// Update an existing profile while enforcing username immutability.
    async fn update_existing_profile(
        &self,
        existing: CommunityProfile,
        request: UpdateProfileRequest,
        display_name: Option<String>,
        bio: Option<String>,
    ) -> Result<PrivateProfileResponse, ApiError> {
        if let Some(raw_username) = request.username {
            let valid = validate_username(&raw_username)?;

            if valid.normalized != existing.normalized_username {
                return Err(ApiError::ConflictDetailed {
                    code: USERNAME_IMMUTABLE_CODE,
                    message: USERNAME_IMMUTABLE_MESSAGE,
                });
            }

            // Same normalized username is intentionally idempotent.
            //
            // Preserve the original canonical username rather than allowing
            // cosmetic case changes to mutate persisted identity.
        }

        let updated = CommunityProfile {
            user_id: existing.user_id,
            username: existing.username,
            normalized_username: existing.normalized_username,
            display_name,
            bio,
            created_at: existing.created_at,
            updated_at: Utc::now(),
        };

        self.storage.update_profile(&updated).await?;

        let role = self.storage.get_role(&updated.user_id).await?;

        Ok(to_private_response(&updated, role))
    }
}

fn to_public_response(profile: &CommunityProfile) -> PublicProfileResponse {
    PublicProfileResponse {
        username: profile.username.clone(),
        display_name: profile.display_name.clone(),
        bio: profile.bio.clone(),
        // Public avatar exposure is deliberately deferred until the avatar
        // access contract is explicitly defined.
        avatar_url: None,
        joined_at: profile.created_at,
    }
}

fn to_private_response(profile: &CommunityProfile, role: CommunityRole) -> PrivateProfileResponse {
    PrivateProfileResponse {
        username: profile.username.clone(),
        display_name: profile.display_name.clone(),
        bio: profile.bio.clone(),
        // Same v1 contract: no public avatar URL yet.
        avatar_url: None,
        joined_at: profile.created_at,
        role,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::community::storage::InMemoryCommunityStorage;

    fn service() -> CommunityService<InMemoryCommunityStorage> {
        CommunityService::new(InMemoryCommunityStorage::new())
    }

    fn request_create(username: &str) -> UpdateProfileRequest {
        UpdateProfileRequest {
            username: Some(username.to_owned()),
            display_name: Some("Display".to_owned()),
            bio: Some("bio".to_owned()),
        }
    }

    fn request_update(
        username: Option<&str>,
        display_name: Option<&str>,
        bio: Option<&str>,
    ) -> UpdateProfileRequest {
        UpdateProfileRequest {
            username: username.map(str::to_owned),
            display_name: display_name.map(str::to_owned),
            bio: bio.map(str::to_owned),
        }
    }

    #[tokio::test]
    async fn create_profile_happy_path() {
        let service = service();
        let user_id = Uuid::new_v4();

        let response = service
            .create_or_update_profile(&user_id, request_create("alice"))
            .await
            .unwrap();

        assert_eq!(response.username, "alice");
        assert_eq!(response.display_name.as_deref(), Some("Display"));
        assert_eq!(response.bio.as_deref(), Some("bio"));
        assert_eq!(response.role, CommunityRole::User);
        assert!(response.avatar_url.is_none());
    }

    #[tokio::test]
    async fn create_profile_without_username_is_validation_failed() {
        let service = service();
        let user_id = Uuid::new_v4();

        let error = service
            .create_or_update_profile(&user_id, request_update(None, Some("Display"), None))
            .await
            .unwrap_err();

        assert_eq!(error.code(), USERNAME_REQUIRED_CODE);
        assert_eq!(
            error.status_code(),
            actix_web::http::StatusCode::BAD_REQUEST
        );
    }

    #[tokio::test]
    async fn create_profile_duplicate_username_is_username_already_exists() {
        let service = service();
        let user_a = Uuid::new_v4();
        let user_b = Uuid::new_v4();

        service
            .create_or_update_profile(&user_a, request_create("alice"))
            .await
            .unwrap();

        let error = service
            .create_or_update_profile(&user_b, request_create("alice"))
            .await
            .unwrap_err();

        assert_eq!(error.code(), USERNAME_ALREADY_EXISTS_CODE);
        assert_eq!(error.status_code(), actix_web::http::StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn update_profile_changes_display_name_and_bio() {
        let service = service();
        let user_id = Uuid::new_v4();

        service
            .create_or_update_profile(&user_id, request_create("alice"))
            .await
            .unwrap();

        let response = service
            .create_or_update_profile(
                &user_id,
                request_update(None, Some("New Name"), Some("new bio")),
            )
            .await
            .unwrap();

        assert_eq!(response.username, "alice");
        assert_eq!(response.display_name.as_deref(), Some("New Name"));
        assert_eq!(response.bio.as_deref(), Some("new bio"));
    }

    #[tokio::test]
    async fn update_profile_without_username_succeeds() {
        let service = service();
        let user_id = Uuid::new_v4();

        service
            .create_or_update_profile(&user_id, request_create("alice"))
            .await
            .unwrap();

        let response = service
            .create_or_update_profile(&user_id, request_update(None, None, None))
            .await
            .unwrap();

        assert_eq!(response.username, "alice");
        assert!(response.display_name.is_none());
        assert!(response.bio.is_none());
    }

    #[tokio::test]
    async fn update_profile_with_same_username_is_idempotent() {
        let service = service();
        let user_id = Uuid::new_v4();

        service
            .create_or_update_profile(&user_id, request_create("Alice"))
            .await
            .unwrap();

        let response = service
            .create_or_update_profile(
                &user_id,
                request_update(Some("ALICE"), Some("Updated"), None),
            )
            .await
            .unwrap();

        // Canonical username remains unchanged.
        assert_eq!(response.username, "Alice");
        assert_eq!(response.display_name.as_deref(), Some("Updated"));
        assert!(response.bio.is_none());
    }

    #[tokio::test]
    async fn update_profile_with_different_username_is_username_immutable() {
        let service = service();
        let user_id = Uuid::new_v4();

        service
            .create_or_update_profile(&user_id, request_create("alice"))
            .await
            .unwrap();

        let error = service
            .create_or_update_profile(&user_id, request_update(Some("bob"), None, None))
            .await
            .unwrap_err();

        assert_eq!(error.code(), USERNAME_IMMUTABLE_CODE);
        assert_eq!(error.status_code(), actix_web::http::StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn update_profile_with_invalid_username_is_rejected() {
        let service = service();
        let user_id = Uuid::new_v4();

        service
            .create_or_update_profile(&user_id, request_create("alice"))
            .await
            .unwrap();

        let error = service
            .create_or_update_profile(&user_id, request_update(Some("bad username"), None, None))
            .await
            .unwrap_err();

        assert_eq!(error.code(), "USERNAME_INVALID_CHARSET");
        assert_eq!(
            error.status_code(),
            actix_web::http::StatusCode::BAD_REQUEST
        );
    }

    #[tokio::test]
    async fn get_own_profile_missing_returns_not_found() {
        let service = service();
        let user_id = Uuid::new_v4();

        let error = service.get_own_profile(&user_id).await.unwrap_err();

        assert_eq!(error.status_code(), actix_web::http::StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn get_own_profile_returns_response() {
        let service = service();
        let user_id = Uuid::new_v4();

        service
            .create_or_update_profile(&user_id, request_create("alice"))
            .await
            .unwrap();

        let response = service.get_own_profile(&user_id).await.unwrap();

        assert_eq!(response.username, "alice");
        assert_eq!(response.role, CommunityRole::User);
        assert_eq!(response.display_name.as_deref(), Some("Display"));
    }

    #[tokio::test]
    async fn get_public_profile_missing_returns_not_found() {
        let service = service();

        let error = service.get_public_profile("ghost").await.unwrap_err();

        assert_eq!(error.status_code(), actix_web::http::StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn get_public_profile_returns_response() {
        let service = service();
        let user_id = Uuid::new_v4();

        service
            .create_or_update_profile(&user_id, request_create("alice"))
            .await
            .unwrap();

        let response = service.get_public_profile("alice").await.unwrap();

        assert_eq!(response.username, "alice");
        assert_eq!(response.display_name.as_deref(), Some("Display"));
        assert_eq!(response.bio.as_deref(), Some("bio"));
        assert!(response.avatar_url.is_none());
    }

    #[tokio::test]
    async fn get_public_profile_normalizes_username() {
        let service = service();
        let user_id = Uuid::new_v4();

        service
            .create_or_update_profile(&user_id, request_create("Alice"))
            .await
            .unwrap();

        let response = service.get_public_profile("ALICE").await.unwrap();

        assert_eq!(response.username, "Alice");
    }

    #[tokio::test]
    async fn public_response_does_not_expose_private_identity_fields() {
        let service = service();
        let user_id = Uuid::new_v4();

        service
            .create_or_update_profile(&user_id, request_create("alice"))
            .await
            .unwrap();

        let response = service.get_public_profile("alice").await.unwrap();

        let json = serde_json::to_value(response).unwrap();

        assert!(json.get("user_id").is_none());
        assert!(json.get("email").is_none());
        assert!(json.get("role").is_none());
    }

    #[tokio::test]
    async fn create_profile_rejects_reserved_username() {
        let service = service();
        let user_id = Uuid::new_v4();

        let error = service
            .create_or_update_profile(&user_id, request_create("admin"))
            .await
            .unwrap_err();

        assert_eq!(error.code(), "USERNAME_RESERVED");
        assert_eq!(
            error.status_code(),
            actix_web::http::StatusCode::BAD_REQUEST
        );
    }

    #[tokio::test]
    async fn create_profile_rejects_invalid_bio() {
        let service = service();
        let user_id = Uuid::new_v4();

        let long_bio = "a".repeat(501);

        let error = service
            .create_or_update_profile(
                &user_id,
                UpdateProfileRequest {
                    username: Some("alice".to_owned()),
                    display_name: None,
                    bio: Some(long_bio),
                },
            )
            .await
            .unwrap_err();

        assert_eq!(error.code(), "BIO_TOO_LONG");
        assert_eq!(
            error.status_code(),
            actix_web::http::StatusCode::BAD_REQUEST
        );
    }
}
