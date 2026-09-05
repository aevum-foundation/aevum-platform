//! Authentication API endpoints.
//!
//! AUTH-07 — Register endpoint
//! AUTH-10 — /me endpoint

use actix_web::{get, post, web, HttpMessage, HttpRequest, HttpResponse};

use crate::auth::contracts::{MeResponse, RegisterRequest, RegisterResponse};
use crate::auth::models::User;
use crate::auth::service::AuthService;
use crate::auth::storage::InMemoryAuthStorage;
use crate::error::{ApiError, ApiResult};

#[post("/api/v1/auth/register")]
pub async fn register(
    req: web::Json<RegisterRequest>,
    service: web::Data<AuthService<InMemoryAuthStorage>>,
) -> ApiResult<HttpResponse> {
    req.validate().map_err(|_| ApiError::BadRequest)?;

    let user = service.register(&req.email, &req.password).await?;

    let response = RegisterResponse {
        user_id: user.id,
        email: user.email,
    };

    Ok(HttpResponse::Created().json(response))
}

#[get("/api/v1/auth/me")]
pub async fn me(req: HttpRequest) -> ApiResult<HttpResponse> {
    let user = req
        .extensions()
        .get::<User>()
        .cloned()
        .ok_or(ApiError::Unauthorized)?;

    if !user.status.can_authenticate() {
        return Err(ApiError::Unauthorized);
    }

    let response = MeResponse {
        user_id: user.id,
        email: user.email.clone(),
        status: user.status,
        created_at: user.created_at,
    };

    Ok(HttpResponse::Ok()
        .insert_header(("Cache-Control", "no-store"))
        .json(response))
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(register).service(me);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::service::AuthService;
    use crate::auth::storage::InMemoryAuthStorage;
    use actix_web::{http::StatusCode, test, App};

    fn test_service() -> AuthService<InMemoryAuthStorage> {
        AuthService::new(InMemoryAuthStorage::new())
    }

    #[actix_web::test]
    async fn register_returns_created() {
        let service = web::Data::new(test_service());
        let app = test::init_service(App::new().app_data(service).service(register)).await;

        let req = test::TestRequest::post()
            .uri("/api/v1/auth/register")
            .set_json(serde_json::json!({
                "email": "test@example.com",
                "password": "correct-horse-battery-staple"
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    #[actix_web::test]
    async fn register_rejects_invalid_request() {
        let service = web::Data::new(test_service());
        let app = test::init_service(App::new().app_data(service).service(register)).await;

        let req = test::TestRequest::post()
            .uri("/api/v1/auth/register")
            .set_json(serde_json::json!({
                "email": "",
                "password": "short"
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[actix_web::test]
    async fn register_rejects_duplicate_email() {
        let service = web::Data::new(test_service());
        let app = test::init_service(App::new().app_data(service).service(register)).await;

        let payload = serde_json::json!({
            "email": "duplicate@example.com",
            "password": "correct-horse-battery-staple"
        });

        let req1 = test::TestRequest::post()
            .uri("/api/v1/auth/register")
            .set_json(&payload)
            .to_request();

        let resp1 = test::call_service(&app, req1).await;
        assert_eq!(resp1.status(), StatusCode::CREATED);

        let req2 = test::TestRequest::post()
            .uri("/api/v1/auth/register")
            .set_json(&payload)
            .to_request();

        let resp2 = test::call_service(&app, req2).await;
        assert_eq!(resp2.status(), StatusCode::CONFLICT);
    }

    #[actix_web::test]
    async fn me_returns_unauthorized_without_user() {
        let service = web::Data::new(test_service());
        let app = test::init_service(App::new().app_data(service).service(me)).await;

        let req = test::TestRequest::get().uri("/api/v1/auth/me").to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn me_returns_user_for_authenticated_request() {
        let service = web::Data::new(test_service());
        let app = test::init_service(App::new().app_data(service).service(me)).await;

        let user = crate::auth::models::User::new(
            "me-test@example.com".to_string(),
            "hashed_password".to_string(),
        );

        let req = test::TestRequest::get().uri("/api/v1/auth/me").to_request();

        // Insert User into request extensions
        let mut req = test::TestRequest::get().uri("/api/v1/auth/me").to_request();
        req.extensions_mut().insert(user.clone());

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: MeResponse = test::read_body_json(resp).await;
        assert_eq!(body.user_id, user.id);
        assert_eq!(body.email, "me-test@example.com");
        assert_eq!(body.status, crate::auth::models::UserStatus::Active);
    }

    #[actix_web::test]
    async fn me_rejects_suspended_user() {
        let service = web::Data::new(test_service());
        let app = test::init_service(App::new().app_data(service).service(me)).await;

        let mut user = crate::auth::models::User::new(
            "suspended@example.com".to_string(),
            "hashed_password".to_string(),
        );
        user.status = crate::auth::models::UserStatus::Suspended;

        let mut req = test::TestRequest::get().uri("/api/v1/auth/me").to_request();
        req.extensions_mut().insert(user);

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }
}
