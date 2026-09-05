//! Authentication API endpoints.
//!
//! AUTH-07 — Register
//! AUTH-08 — Login
//! AUTH-10 — /me
//! AUTH-11 — Logout

use actix_web::{
    cookie::{Cookie, SameSite},
    get, post, web, HttpMessage, HttpRequest, HttpResponse,
};

use crate::auth::contracts::{
    LoginRequest, LoginResponse, LogoutResponse, MeResponse, RegisterRequest, RegisterResponse,
    SESSION_COOKIE_NAME, SESSION_DURATION_DAYS,
};
use crate::auth::csrf::{build_csrf_cookie, build_csrf_removal_cookie, CsrfConfig, CsrfToken};
use crate::auth::models::User;
use crate::auth::service::AuthService;
use crate::auth::storage::InMemoryAuthStorage;
use crate::error::{ApiError, ApiResult};

/// Application-wide authentication service type.
///
/// AUTH-17 will replace this alias with a storage-agnostic backend selection.
pub type AppAuthService = AuthService<InMemoryAuthStorage>;

fn build_session_cookie(token: &str) -> Cookie<'static> {
    Cookie::build(SESSION_COOKIE_NAME, token.to_owned())
        .http_only(true)
        .secure(cfg!(not(debug_assertions)))
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(actix_web::cookie::time::Duration::days(
            SESSION_DURATION_DAYS,
        ))
        .finish()
}

fn build_logout_cookie() -> Cookie<'static> {
    let mut cookie = Cookie::build(SESSION_COOKIE_NAME, "")
        .http_only(true)
        .secure(cfg!(not(debug_assertions)))
        .same_site(SameSite::Lax)
        .path("/")
        .finish();

    cookie.make_removal();
    cookie
}

#[post("/api/v1/auth/register")]
pub async fn register(
    req: web::Json<RegisterRequest>,
    http_req: HttpRequest,
    service: web::Data<AppAuthService>,
) -> ApiResult<HttpResponse> {
    req.validate().map_err(|_| ApiError::BadRequest)?;

    let ip = http_req
        .connection_info()
        .realip_remote_addr()
        .unwrap_or("unknown")
        .to_string();

    let user = service.register(&req.email, &req.password, &ip).await?;

    let response = RegisterResponse {
        user_id: user.id,
        email: user.email,
    };

    Ok(HttpResponse::Created().json(response))
}

#[post("/api/v1/auth/login")]
pub async fn login(
    req: web::Json<LoginRequest>,
    http_req: HttpRequest,
    service: web::Data<AppAuthService>,
) -> ApiResult<HttpResponse> {
    req.validate().map_err(|_| ApiError::BadRequest)?;

    let ip = http_req
        .connection_info()
        .realip_remote_addr()
        .unwrap_or("unknown")
        .to_string();

    let (user, session_token) = service.login(&req.email, &req.password, &ip).await?;

    let response = LoginResponse {
        user_id: user.id,
        email: user.email,
    };

    let session_cookie = build_session_cookie(session_token.expose());
    let csrf_token = CsrfToken::generate();
    let csrf_cookie = build_csrf_cookie(&csrf_token, &CsrfConfig::default());

    let mut http_response = HttpResponse::Ok().json(response);

    http_response
        .add_cookie(&session_cookie)
        .map_err(|_| ApiError::Internal)?;

    http_response
        .add_cookie(&csrf_cookie)
        .map_err(|_| ApiError::Internal)?;

    Ok(http_response)
}

#[post("/api/v1/auth/logout")]
pub async fn logout(
    req: HttpRequest,
    service: web::Data<AppAuthService>,
) -> ApiResult<HttpResponse> {
    if let Some(cookie) = req.cookie(SESSION_COOKIE_NAME) {
        let token = cookie.value().to_owned();

        match service.logout(&token).await {
            Ok(()) => {}
            Err(ApiError::Unauthorized) | Err(ApiError::NotFound) => {
                // Logout is intentionally idempotent:
                // unknown/already-revoked sessions do not leak information.
            }
            Err(error) => return Err(error),
        }
    }

    let response = LogoutResponse { success: true };

    let removal_cookie = build_logout_cookie();
    let csrf_removal_cookie = build_csrf_removal_cookie(&CsrfConfig::default());

    let mut http_response = HttpResponse::Ok().json(response);

    http_response
        .add_cookie(&removal_cookie)
        .map_err(|_| ApiError::Internal)?;

    http_response
        .add_cookie(&csrf_removal_cookie)
        .map_err(|_| ApiError::Internal)?;

    Ok(http_response)
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
    cfg.service(register)
        .service(login)
        .service(logout)
        .service(me);
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{http::StatusCode, test, App};

    const TEST_EMAIL: &str = "test@example.com";
    const TEST_PASSWORD: &str = "correct-horse-battery-staple";

    fn test_service() -> AppAuthService {
        AppAuthService::new(InMemoryAuthStorage::new())
    }

    async fn register_test_user(service: &AppAuthService) {
        service
            .register(TEST_EMAIL, TEST_PASSWORD, "127.0.0.1")
            .await
            .unwrap();
    }

    #[actix_web::test]
    async fn register_returns_created() {
        let service = web::Data::new(test_service());
        let app = test::init_service(App::new().app_data(service).service(register)).await;

        let req = test::TestRequest::post()
            .uri("/api/v1/auth/register")
            .set_json(serde_json::json!({
                "email": "test@example.com",
                "password": TEST_PASSWORD
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
            "password": TEST_PASSWORD
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
    async fn login_returns_ok_and_session_cookie() {
        let service = web::Data::new(test_service());
        register_test_user(&service).await;

        let app = test::init_service(App::new().app_data(service).service(login)).await;

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
        assert!(cookie.to_str().unwrap().contains("__Host-aevum_session="));
    }

    #[actix_web::test]
    async fn login_rejects_invalid_password() {
        let service = web::Data::new(test_service());
        register_test_user(&service).await;

        let app = test::init_service(App::new().app_data(service).service(login)).await;

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

        let app = test::init_service(App::new().app_data(service).service(login)).await;

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

        let app = test::init_service(App::new().app_data(service).service(login)).await;

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
        let service = AppAuthService::new(storage.clone());
        register_test_user(&service).await;

        let (user, token) = service.login(TEST_EMAIL, TEST_PASSWORD, "127.0.0.1").await.unwrap();
        assert!(!user.id.is_nil());

        let authenticated = service.authenticate(token.expose()).await.unwrap().unwrap();
        assert_eq!(authenticated.id, user.id);
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

        let user = User::new(
            "me-test@example.com".to_string(),
            "hashed_password".to_string(),
        );

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

        let mut user = User::new(
            "suspended@example.com".to_string(),
            "hashed_password".to_string(),
        );
        user.status = crate::auth::models::UserStatus::Suspended;

        let mut req = test::TestRequest::get().uri("/api/v1/auth/me").to_request();
        req.extensions_mut().insert(user);

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn logout_returns_ok_without_cookie() {
        let service = web::Data::new(test_service());

        let app = test::init_service(App::new().app_data(service).service(logout)).await;

        let req = test::TestRequest::post()
            .uri("/api/v1/auth/logout")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: LogoutResponse = test::read_body_json(resp).await;
        assert!(body.success);
    }

    #[actix_web::test]
    async fn logout_removes_session_cookie() {
        let storage = InMemoryAuthStorage::new();
        let service = web::Data::new(AppAuthService::new(storage.clone()));
        register_test_user(&service).await;

        let (_, token) = service.login(TEST_EMAIL, TEST_PASSWORD, "127.0.0.1").await.unwrap();

        let app = test::init_service(App::new().app_data(service.clone()).service(logout)).await;

        let cookie = build_session_cookie(token.expose());

        let req = test::TestRequest::post()
            .uri("/api/v1/auth/logout")
            .cookie(cookie)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let set_cookie = resp.headers().get("set-cookie").unwrap();
        let set_cookie_str = set_cookie.to_str().unwrap();
        assert!(set_cookie_str.contains("__Host-aevum_session=;"));
    }

    #[actix_web::test]
    async fn logout_is_idempotent() {
        let storage = InMemoryAuthStorage::new();
        let service = web::Data::new(AppAuthService::new(storage.clone()));
        register_test_user(&service).await;

        let (_, token) = service.login(TEST_EMAIL, TEST_PASSWORD, "127.0.0.1").await.unwrap();

        let app = test::init_service(App::new().app_data(service.clone()).service(logout)).await;

        let cookie = build_session_cookie(token.expose());

        // First logout
        let req1 = test::TestRequest::post()
            .uri("/api/v1/auth/logout")
            .cookie(cookie.clone())
            .to_request();
        let resp1 = test::call_service(&app, req1).await;
        assert_eq!(resp1.status(), StatusCode::OK);

        // Second logout with same cookie
        let req2 = test::TestRequest::post()
            .uri("/api/v1/auth/logout")
            .cookie(cookie.clone())
            .to_request();
        let resp2 = test::call_service(&app, req2).await;
        assert_eq!(resp2.status(), StatusCode::OK);
    }
}
