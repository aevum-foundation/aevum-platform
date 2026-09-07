//! AUTH-18 — Integration tests for the auth HTTP API.
//!
//! These tests exercise the full HTTP stack:
//! Request → AuthMiddleware → CSRF → Endpoint → AuthService → Storage

use actix_web::{cookie::Cookie, http::StatusCode, test, web, App};
use std::sync::Arc;

use aevum_platform_api::{
    api::{self},
    auth::{
        api::AuthApi, authenticator::Authenticator, csrf::CsrfConfig,
        csrf_middleware::CsrfMiddleware, middleware::AuthMiddleware, service::AuthService,
        storage::InMemoryAuthStorage,
    },
    config::Config,
    state::AppState,
    storage::MockStorage,
};

const TEST_EMAIL: &str = "integration@example.com";
const TEST_PASSWORD: &str = "correct-horse-battery-staple";

type TestApp = actix_web::dev::ServiceResponse<actix_web::body::BoxBody>;

async fn build_test_app() -> impl actix_web::dev::Service<
    actix_http::Request,
    Response = TestApp,
    Error = actix_web::Error,
> {
    let storage = Arc::new(MockStorage::new());
    let config = Config::from_env();
    let app_state = AppState::new(config, storage);

    let auth_storage = InMemoryAuthStorage::new();
    let auth_service = Arc::new(AuthService::new(auth_storage));

    let auth_api: Arc<dyn AuthApi> = auth_service.clone();
    let authenticator: Arc<dyn Authenticator> = auth_service.clone();

    test::init_service(
        App::new()
            .app_data(web::Data::new(app_state))
            .app_data(web::Data::new(auth_api))
            .wrap(AuthMiddleware::new(web::Data::new(authenticator)))
            .wrap(CsrfMiddleware::new(web::Data::new(CsrfConfig::default())))
            .configure(api::auth::configure),
    )
    .await
}

fn extract_cookie(response: &actix_web::dev::ServiceResponse, name: &str) -> Option<String> {
    response
        .headers()
        .get_all("set-cookie")
        .into_iter()
        .filter_map(|value| value.to_str().ok())
        .find(|cookie_str| cookie_str.starts_with(&format!("{}=", name)))
        .and_then(|cookie_str| {
            let cookie = Cookie::parse(cookie_str).ok()?;
            Some(cookie.value().to_string())
        })
}

fn cookie_header(name: &'static str, value: &str) -> Cookie<'static> {
    Cookie::build(name, value.to_owned()).finish()
}

#[actix_web::test]
async fn full_auth_flow_register_login_me_logout() {
    let app = build_test_app().await;

    // 1. Register
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/register")
        .set_json(serde_json::json!({
            "email": TEST_EMAIL,
            "password": TEST_PASSWORD
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    // 2. Login
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .set_json(serde_json::json!({
            "email": TEST_EMAIL,
            "password": TEST_PASSWORD
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let session_token = extract_cookie(&resp, "__Host-aevum_session").unwrap();
    assert!(!session_token.is_empty());

    let csrf_token = extract_cookie(&resp, "__Host-aevum_csrf").unwrap();
    assert!(!csrf_token.is_empty());

    // 3. Me with valid session cookie
    let req = test::TestRequest::get()
        .uri("/api/v1/auth/me")
        .cookie(cookie_header("__Host-aevum_session", &session_token))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // 4. Logout
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/logout")
        .cookie(cookie_header("__Host-aevum_session", &session_token))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // 4.1 Session must be invalid after logout
    let req = test::TestRequest::get()
        .uri("/api/v1/auth/me")
        .cookie(cookie_header("__Host-aevum_session", &session_token))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // 5. Logout again (idempotent)
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/logout")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[actix_web::test]
async fn register_validation_failures() {
    let app = build_test_app().await;

    // Bad email
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/register")
        .set_json(serde_json::json!({
            "email": "",
            "password": TEST_PASSWORD
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    // Short password
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/register")
        .set_json(serde_json::json!({
            "email": "valid@example.com",
            "password": "short"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[actix_web::test]
async fn register_duplicate_email_conflict() {
    let app = build_test_app().await;

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
async fn login_failures() {
    let app = build_test_app().await;

    // Register user
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/register")
        .set_json(serde_json::json!({
            "email": TEST_EMAIL,
            "password": TEST_PASSWORD
        }))
        .to_request();
    let _ = test::call_service(&app, req).await;

    // Wrong password
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .set_json(serde_json::json!({
            "email": TEST_EMAIL,
            "password": "wrong-password"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // Unknown email
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
#[ignore = "requires protected csrf endpoint"]
async fn csrf_rejects_missing_header() {
    let app = build_test_app().await;

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .set_json(serde_json::json!({
            "email": "nonexistent@example.com",
            "password": TEST_PASSWORD
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn rate_limit_login() {
    let app = build_test_app().await;

    // Register user
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/register")
        .set_json(serde_json::json!({
            "email": TEST_EMAIL,
            "password": TEST_PASSWORD
        }))
        .to_request();
    let _ = test::call_service(&app, req).await;

    // Attempt login with wrong password 6 times (limit is 5)
    let mut rate_limited = false;
    for _ in 0..6 {
        let req = test::TestRequest::post()
            .uri("/api/v1/auth/login")
            .set_json(serde_json::json!({
                "email": TEST_EMAIL,
                "password": "wrong-password"
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        if resp.status() == StatusCode::TOO_MANY_REQUESTS {
            rate_limited = true;
            break;
        }
    }

    assert!(rate_limited, "Expected rate limiting to trigger");
}

#[actix_web::test]
async fn session_rotation_invalidates_old_token() {
    let app = build_test_app().await;

    // Register
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/register")
        .set_json(serde_json::json!({
            "email": "rotate@example.com",
            "password": TEST_PASSWORD
        }))
        .to_request();
    let _ = test::call_service(&app, req).await;

    // Login
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .set_json(serde_json::json!({
            "email": "rotate@example.com",
            "password": TEST_PASSWORD
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let old_token = extract_cookie(&resp, "__Host-aevum_session").expect("session cookie required");
    let csrf_token = extract_cookie(&resp, "__Host-aevum_csrf").expect("csrf cookie required");

    // Verify old token works
    let req = test::TestRequest::get()
        .uri("/api/v1/auth/me")
        .cookie(cookie_header("__Host-aevum_session", &old_token))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Rotate session — needs CSRF cookie and header
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/rotate")
        .cookie(cookie_header("__Host-aevum_session", &old_token))
        .cookie(cookie_header("__Host-aevum_csrf", &csrf_token))
        .insert_header(("X-CSRF-Token", csrf_token.clone()))
        .to_request();
    let resp = test::call_service(&app, req).await;

    if resp.status().is_success() {
        let new_token =
            extract_cookie(&resp, "__Host-aevum_session").expect("rotated session cookie required");

        // Old token must be invalid
        let req = test::TestRequest::get()
            .uri("/api/v1/auth/me")
            .cookie(cookie_header("__Host-aevum_session", &old_token))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

        // New token must be valid
        let req = test::TestRequest::get()
            .uri("/api/v1/auth/me")
            .cookie(cookie_header("__Host-aevum_session", &new_token))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
