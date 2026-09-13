//! Community HTTP API.
//!
//! B-1.2b.5
//!
//! HTTP layer only:
//! - authentication context extraction;
//! - request deserialization;
//! - delegation to CommunityApi;
//! - HTTP response serialization.
//!
//! Business rules remain in CommunityService.

use std::sync::Arc;

use actix_web::{get, put, web, HttpMessage, HttpRequest, HttpResponse};

use crate::auth::models::AuthContext;
use crate::community::api::CommunityApi;
use crate::community::contracts::UpdateProfileRequest;
use crate::error::ApiError;

/// Application-facing Community service exposed to HTTP handlers.
///
/// The HTTP layer depends only on the domain API contract and never on a
/// concrete storage implementation.
pub type AppCommunityService = Arc<dyn CommunityApi>;

/// GET /api/v1/community/u/{username}
///
/// Public profile lookup. No authentication is required.
#[get("/api/v1/community/u/{username}")]
pub async fn get_public_profile(
    service: web::Data<AppCommunityService>,
    path: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let username = path.into_inner();

    let profile = service.get_public_profile(&username).await?;

    Ok(HttpResponse::Ok().json(profile))
}

/// GET /api/v1/community/me/profile
///
/// Returns the authenticated user's private Community profile.
#[get("/api/v1/community/me/profile")]
pub async fn get_own_profile(
    req: HttpRequest,
    service: web::Data<AppCommunityService>,
) -> Result<HttpResponse, ApiError> {
    let auth = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(ApiError::Unauthorized)?;

    let profile = service.get_own_profile(&auth.user.id).await?;

    Ok(HttpResponse::Ok().json(profile))
}

/// PUT /api/v1/community/me/profile
///
/// Creates the authenticated user's Community profile or updates its mutable
/// fields. CSRF enforcement is intentionally delegated to the global
/// CsrfMiddleware.
#[put("/api/v1/community/me/profile")]
pub async fn update_own_profile(
    req: HttpRequest,
    service: web::Data<AppCommunityService>,
    body: web::Json<UpdateProfileRequest>,
) -> Result<HttpResponse, ApiError> {
    let auth = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(ApiError::Unauthorized)?;

    let profile = service
        .create_or_update_profile(&auth.user.id, body.into_inner())
        .await?;

    Ok(HttpResponse::Ok().json(profile))
}

/// Register Community HTTP routes.
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(get_public_profile)
        .service(get_own_profile)
        .service(update_own_profile);
}
