//! Authentication login endpoint.
//!
//! AUTH-08 — Login

use actix_web::{
    cookie::{Cookie, SameSite},
    post, web, HttpResponse,
};

use crate::auth::contracts::{
    LoginRequest, LoginResponse, SESSION_COOKIE_NAME, SESSION_DURATION_DAYS,
};
use crate::auth::service::AuthService;
use crate::auth::storage::InMemoryAuthStorage;
use crate::error::{ApiError, ApiResult};

#[post("/api/v1/auth/login")]
pub async fn login(
    req: web::Json<LoginRequest>,
    service: web::Data<AuthService<InMemoryAuthStorage>>,
) -> ApiResult<HttpResponse> {
    req.validate().map_err(|_| ApiError::BadRequest)?;

    let (user, session_token) = service.login(&req.email, &req.password).await?;

    let response = LoginResponse {
        user_id: user.id,
        email: user.email,
    };

    let cookie = Cookie::build(SESSION_COOKIE_NAME, session_token.expose().to_owned())
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(actix_web::cookie::time::Duration::days(
            SESSION_DURATION_DAYS,
        ))
        .finish();

    let mut http_response = HttpResponse::Ok().json(response);

    http_response
        .add_cookie(&cookie)
        .map_err(|_| ApiError::Internal)?;

    Ok(http_response)
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(login);
}



#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{http::StatusCode, test, App};
    use crate::auth::service::AuthService;
    use crate::auth::storage::InMemoryAuthStorage;

    const TEST_EMAIL: &str = "test@example.com";
    const TEST_PASSWORD: &str = "correct-horse-battery-staple";

    fn test_service() -> AuthService<InMemoryAuthStorage> {
        AuthService::new(InMemoryAuthStorage::new())
    }

    async fn register_test_user(service: &AuthService<InMemoryAuthStorage>) {
        service.register(TEST_EMAIL, TEST_PASSWORD).await.unwrap();
    }

    #[actix_web::test]
    async fn login_returns_ok_and_session_cookie() {
        let service = web::Data::new(test_service());
        register_test_user(&service).await;

        let app = test::init_service(
            App::new().app_data(service).service(login),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/v1/auth/login")
            .set_json(serde_json::json!({
                "email": TEST_EMAIL,
                "password": TEST_PASSWORD
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let cookie = resp.headers().get("set-cookie").unwrap();
        assert!(cookie.to_str().unwrap().contains("aevum_session="));
    }

    #[actix_web::test]
    async fn login_rejects_invalid_password() {
        let service = web::Data::new(test_service());
        register_test_user(&service).await;

        let app = test::init_service(
            App::new().app_data(service).service(login),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/v1/auth/login")
            .set_json(serde_json::json!({
                "email": TEST_EMAIL,
                "password": "wrong-password"
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn login_rejects_unknown_user() {
        let service = web::Data::new(test_service());

        let app = test::init_service(
            App::new().app_data(service).service(login),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/v1/auth/login")
            .set_json(serde_json::json!({
                "email": "unknown@example.com",
                "password": TEST_PASSWORD
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn login_rejects_invalid_request() {
        let service = web::Data::new(test_service());

        let app = test::init_service(
            App::new().app_data(service).service(login),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/v1/auth/login")
            .set_json(serde_json::json!({
                "email": "",
                "password": ""
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[actix_web::test]
    async fn login_creates_session() {
        let storage = InMemoryAuthStorage::new();
        let service = AuthService::new(storage.clone());
        register_test_user(&service).await;

        let (user, token) = service.login(TEST_EMAIL, TEST_PASSWORD).await.unwrap();
        assert!(!user.id.is_nil());

        let authenticated = service.authenticate(token.expose()).await.unwrap().unwrap();
        assert_eq!(authenticated.id, user.id);
    }
}
