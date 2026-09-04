//! Authentication API endpoints.
//!
//! AUTH-07 — Register endpoint

use actix_web::{post, web, HttpResponse};

use crate::auth::contracts::{RegisterRequest, RegisterResponse};
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

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(register);
}



#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{http::StatusCode, test, App};
    use crate::auth::service::AuthService;
    use crate::auth::storage::InMemoryAuthStorage;

    fn test_service() -> AuthService<InMemoryAuthStorage> {
        AuthService::new(InMemoryAuthStorage::new())
    }

    #[actix_web::test]
    async fn register_returns_created() {
        let service = web::Data::new(test_service());
        let app = test::init_service(
            App::new().app_data(service).service(register),
        )
        .await;

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
        let app = test::init_service(
            App::new().app_data(service).service(register),
        )
        .await;

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
        let app = test::init_service(
            App::new().app_data(service).service(register),
        )
        .await;

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
}
