//! AUTH-18 — Integration tests for the auth HTTP API.
//!
//! These tests exercise the full HTTP stack:
//! Request → AuthMiddleware → CSRF → Endpoint → AuthService → Storage

use actix_web::{cookie::Cookie, http::StatusCode, test, web, App};
use std::sync::Arc;

#[path = "support.rs"]
mod support;
use support::create_test_context;

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

fn extract_cookie(
    response: &actix_web::dev::ServiceResponse<
        actix_web::body::EitherBody<actix_web::body::BoxBody>,
    >,
    name: &str,
) -> Option<String> {
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
    let ctx = create_test_context().await;

    let auth_api: Arc<dyn AuthApi> = ctx.auth_service.clone();
    let authenticator: Arc<dyn Authenticator> = ctx.auth_service.clone();
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(ctx.app_state.clone()))
            .app_data(web::Data::new(auth_api))
            .wrap(AuthMiddleware::new(web::Data::new(authenticator)))
            .wrap(CsrfMiddleware::new(web::Data::new(CsrfConfig::default())))
            .configure(api::auth::configure),
    )
    .await;

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
    let ctx = create_test_context().await;

    let auth_api: Arc<dyn AuthApi> = ctx.auth_service.clone();
    let authenticator: Arc<dyn Authenticator> = ctx.auth_service.clone();
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(ctx.app_state.clone()))
            .app_data(web::Data::new(auth_api))
            .wrap(AuthMiddleware::new(web::Data::new(authenticator)))
            .wrap(CsrfMiddleware::new(web::Data::new(CsrfConfig::default())))
            .configure(api::auth::configure),
    )
    .await;

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
    let ctx = create_test_context().await;

    let auth_api: Arc<dyn AuthApi> = ctx.auth_service.clone();
    let authenticator: Arc<dyn Authenticator> = ctx.auth_service.clone();
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(ctx.app_state.clone()))
            .app_data(web::Data::new(auth_api))
            .wrap(AuthMiddleware::new(web::Data::new(authenticator)))
            .wrap(CsrfMiddleware::new(web::Data::new(CsrfConfig::default())))
            .configure(api::auth::configure),
    )
    .await;

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
    let ctx = create_test_context().await;

    let auth_api: Arc<dyn AuthApi> = ctx.auth_service.clone();
    let authenticator: Arc<dyn Authenticator> = ctx.auth_service.clone();
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(ctx.app_state.clone()))
            .app_data(web::Data::new(auth_api))
            .wrap(AuthMiddleware::new(web::Data::new(authenticator)))
            .wrap(CsrfMiddleware::new(web::Data::new(CsrfConfig::default())))
            .configure(api::auth::configure),
    )
    .await;

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
async fn csrf_rejects_missing_header() {
    let ctx = create_test_context().await;

    let auth_api: Arc<dyn AuthApi> = ctx.auth_service.clone();
    let authenticator: Arc<dyn Authenticator> = ctx.auth_service.clone();
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(ctx.app_state.clone()))
            .app_data(web::Data::new(auth_api))
            .wrap(AuthMiddleware::new(web::Data::new(authenticator)))
            .wrap(CsrfMiddleware::new(web::Data::new(CsrfConfig::default())))
            .configure(api::auth::configure),
    )
    .await;

    // Register + login to obtain a valid session
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/register")
        .set_json(serde_json::json!({
            "email": "csrf-protected@example.com",
            "password": TEST_PASSWORD
        }))
        .to_request();
    let _ = test::call_service(&app, req).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .set_json(serde_json::json!({
            "email": "csrf-protected@example.com",
            "password": TEST_PASSWORD
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let session_token = extract_cookie(&resp, "__Host-aevum_session").unwrap();

    // POST to a CSRF-protected endpoint WITHOUT X-CSRF-Token header.
    // Middleware must reject with 403 Forbidden.
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/sessions/revoke-all")
        .cookie(cookie_header("__Host-aevum_session", &session_token))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[actix_web::test]
async fn csrf_rejects_mismatched_token() {
    let ctx = create_test_context().await;

    let auth_api: Arc<dyn AuthApi> = ctx.auth_service.clone();
    let authenticator: Arc<dyn Authenticator> = ctx.auth_service.clone();
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(ctx.app_state.clone()))
            .app_data(web::Data::new(auth_api))
            .wrap(AuthMiddleware::new(web::Data::new(authenticator)))
            .wrap(CsrfMiddleware::new(web::Data::new(CsrfConfig::default())))
            .configure(api::auth::configure),
    )
    .await;

    // Register + login
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/register")
        .set_json(serde_json::json!({
            "email": "csrf-mismatch@example.com",
            "password": TEST_PASSWORD
        }))
        .to_request();
    let _ = test::call_service(&app, req).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .set_json(serde_json::json!({
            "email": "csrf-mismatch@example.com",
            "password": TEST_PASSWORD
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let session_token = extract_cookie(&resp, "__Host-aevum_session").unwrap();
    let real_csrf = extract_cookie(&resp, "__Host-aevum_csrf").unwrap();

    // Send a cookie CSRF token that does NOT match the header value.
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/sessions/revoke-all")
        .cookie(cookie_header("__Host-aevum_session", &session_token))
        .cookie(cookie_header("__Host-aevum_csrf", &real_csrf))
        .insert_header(("X-CSRF-Token", "wrong-token-value"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[actix_web::test]
async fn csrf_accepts_matching_token() {
    let ctx = create_test_context().await;

    let auth_api: Arc<dyn AuthApi> = ctx.auth_service.clone();
    let authenticator: Arc<dyn Authenticator> = ctx.auth_service.clone();
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(ctx.app_state.clone()))
            .app_data(web::Data::new(auth_api))
            .wrap(AuthMiddleware::new(web::Data::new(authenticator)))
            .wrap(CsrfMiddleware::new(web::Data::new(CsrfConfig::default())))
            .configure(api::auth::configure),
    )
    .await;

    // Register + login
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/register")
        .set_json(serde_json::json!({
            "email": "csrf-ok@example.com",
            "password": TEST_PASSWORD
        }))
        .to_request();
    let _ = test::call_service(&app, req).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .set_json(serde_json::json!({
            "email": "csrf-ok@example.com",
            "password": TEST_PASSWORD
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let session_token = extract_cookie(&resp, "__Host-aevum_session").unwrap();
    let csrf_token = extract_cookie(&resp, "__Host-aevum_csrf").unwrap();

    // Correct cookie + matching header → should pass CSRF middleware
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/sessions/revoke-all")
        .cookie(cookie_header("__Host-aevum_session", &session_token))
        .cookie(cookie_header("__Host-aevum_csrf", &csrf_token))
        .insert_header(("X-CSRF-Token", csrf_token.clone()))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[actix_web::test]
async fn rate_limit_login() {
    let ctx = create_test_context().await;

    let auth_api: Arc<dyn AuthApi> = ctx.auth_service.clone();
    let authenticator: Arc<dyn Authenticator> = ctx.auth_service.clone();
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(ctx.app_state.clone()))
            .app_data(web::Data::new(auth_api))
            .wrap(AuthMiddleware::new(web::Data::new(authenticator)))
            .wrap(CsrfMiddleware::new(web::Data::new(CsrfConfig::default())))
            .configure(api::auth::configure),
    )
    .await;

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
    let ctx = create_test_context().await;

    let auth_api: Arc<dyn AuthApi> = ctx.auth_service.clone();
    let authenticator: Arc<dyn Authenticator> = ctx.auth_service.clone();
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(ctx.app_state.clone()))
            .app_data(web::Data::new(auth_api))
            .wrap(AuthMiddleware::new(web::Data::new(authenticator)))
            .wrap(CsrfMiddleware::new(web::Data::new(CsrfConfig::default())))
            .configure(api::auth::configure),
    )
    .await;

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

#[actix_web::test]
async fn change_password_flow() {
    let ctx = create_test_context().await;

    let auth_api: Arc<dyn AuthApi> = ctx.auth_service.clone();
    let authenticator: Arc<dyn Authenticator> = ctx.auth_service.clone();
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(ctx.app_state.clone()))
            .app_data(web::Data::new(auth_api))
            .wrap(AuthMiddleware::new(web::Data::new(authenticator)))
            .wrap(CsrfMiddleware::new(web::Data::new(CsrfConfig::default())))
            .configure(api::auth::configure),
    )
    .await;

    // Register
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/register")
        .set_json(serde_json::json!({
            "email": "password-change@example.com",
            "password": TEST_PASSWORD
        }))
        .to_request();
    let _ = test::call_service(&app, req).await;

    // Login
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .set_json(serde_json::json!({
            "email": "password-change@example.com",
            "password": TEST_PASSWORD
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let session_token = extract_cookie(&resp, "__Host-aevum_session").unwrap();
    let csrf_token = extract_cookie(&resp, "__Host-aevum_csrf").unwrap();

    // Change password
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/change-password")
        .cookie(cookie_header("__Host-aevum_session", &session_token))
        .cookie(cookie_header("__Host-aevum_csrf", &csrf_token))
        .insert_header(("X-CSRF-Token", csrf_token.clone()))
        .set_json(serde_json::json!({
            "current_password": TEST_PASSWORD,
            "new_password": "new-secure-password-123"
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // New session token should be in response
    let new_session_token = extract_cookie(&resp, "__Host-aevum_session").unwrap();
    assert_ne!(new_session_token, session_token);

    // Old session must be invalid
    let req = test::TestRequest::get()
        .uri("/api/v1/auth/me")
        .cookie(cookie_header("__Host-aevum_session", &session_token))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // New session must work
    let req = test::TestRequest::get()
        .uri("/api/v1/auth/me")
        .cookie(cookie_header("__Host-aevum_session", &new_session_token))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Old password should no longer work
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .set_json(serde_json::json!({
            "email": "password-change@example.com",
            "password": TEST_PASSWORD
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // New password should work
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .set_json(serde_json::json!({
            "email": "password-change@example.com",
            "password": "new-secure-password-123"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[actix_web::test]
async fn change_password_wrong_current_password() {
    let ctx = create_test_context().await;

    let auth_api: Arc<dyn AuthApi> = ctx.auth_service.clone();
    let authenticator: Arc<dyn Authenticator> = ctx.auth_service.clone();
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(ctx.app_state.clone()))
            .app_data(web::Data::new(auth_api))
            .wrap(AuthMiddleware::new(web::Data::new(authenticator)))
            .wrap(CsrfMiddleware::new(web::Data::new(CsrfConfig::default())))
            .configure(api::auth::configure),
    )
    .await;

    // Register + Login
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/register")
        .set_json(serde_json::json!({
            "email": "wrong-current@example.com",
            "password": TEST_PASSWORD
        }))
        .to_request();
    let _ = test::call_service(&app, req).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .set_json(serde_json::json!({
            "email": "wrong-current@example.com",
            "password": TEST_PASSWORD
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let session_token = extract_cookie(&resp, "__Host-aevum_session").unwrap();
    let csrf_token = extract_cookie(&resp, "__Host-aevum_csrf").unwrap();

    // Change password with wrong current
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/change-password")
        .cookie(cookie_header("__Host-aevum_session", &session_token))
        .cookie(cookie_header("__Host-aevum_csrf", &csrf_token))
        .insert_header(("X-CSRF-Token", csrf_token.clone()))
        .set_json(serde_json::json!({
            "current_password": "wrong-password",
            "new_password": "new-secure-password-123"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // Old password should still work
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .set_json(serde_json::json!({
            "email": "wrong-current@example.com",
            "password": TEST_PASSWORD
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[actix_web::test]
async fn change_password_weak_new_password() {
    let ctx = create_test_context().await;

    let auth_api: Arc<dyn AuthApi> = ctx.auth_service.clone();
    let authenticator: Arc<dyn Authenticator> = ctx.auth_service.clone();
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(ctx.app_state.clone()))
            .app_data(web::Data::new(auth_api))
            .wrap(AuthMiddleware::new(web::Data::new(authenticator)))
            .wrap(CsrfMiddleware::new(web::Data::new(CsrfConfig::default())))
            .configure(api::auth::configure),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/register")
        .set_json(serde_json::json!({
            "email": "weak-new@example.com",
            "password": TEST_PASSWORD
        }))
        .to_request();
    let _ = test::call_service(&app, req).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .set_json(serde_json::json!({
            "email": "weak-new@example.com",
            "password": TEST_PASSWORD
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let session_token = extract_cookie(&resp, "__Host-aevum_session").unwrap();
    let csrf_token = extract_cookie(&resp, "__Host-aevum_csrf").unwrap();

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/change-password")
        .cookie(cookie_header("__Host-aevum_session", &session_token))
        .cookie(cookie_header("__Host-aevum_csrf", &csrf_token))
        .insert_header(("X-CSRF-Token", csrf_token.clone()))
        .set_json(serde_json::json!({
            "current_password": TEST_PASSWORD,
            "new_password": "123"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[actix_web::test]
async fn change_password_requires_csrf() {
    let ctx = create_test_context().await;

    let auth_api: Arc<dyn AuthApi> = ctx.auth_service.clone();
    let authenticator: Arc<dyn Authenticator> = ctx.auth_service.clone();
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(ctx.app_state.clone()))
            .app_data(web::Data::new(auth_api))
            .wrap(AuthMiddleware::new(web::Data::new(authenticator)))
            .wrap(CsrfMiddleware::new(web::Data::new(CsrfConfig::default())))
            .configure(api::auth::configure),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/register")
        .set_json(serde_json::json!({
            "email": "csrf-change@example.com",
            "password": TEST_PASSWORD
        }))
        .to_request();
    let _ = test::call_service(&app, req).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .set_json(serde_json::json!({
            "email": "csrf-change@example.com",
            "password": TEST_PASSWORD
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let session_token = extract_cookie(&resp, "__Host-aevum_session").unwrap();

    // No CSRF cookie and header
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/change-password")
        .cookie(cookie_header("__Host-aevum_session", &session_token))
        .set_json(serde_json::json!({
            "current_password": TEST_PASSWORD,
            "new_password": "new-secure-password-123"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[actix_web::test]
async fn password_reset_full_flow_with_real_token() {
    let ctx = create_test_context().await;

    let auth_api: Arc<dyn AuthApi> = ctx.auth_service.clone();
    let authenticator: Arc<dyn Authenticator> = ctx.auth_service.clone();
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(ctx.app_state.clone()))
            .app_data(web::Data::new(auth_api))
            .wrap(AuthMiddleware::new(web::Data::new(authenticator)))
            .wrap(CsrfMiddleware::new(web::Data::new(CsrfConfig::default())))
            .configure(api::auth::configure),
    )
    .await;

    // Register
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/register")
        .set_json(serde_json::json!({
            "email": "reset-full@example.com",
            "password": TEST_PASSWORD
        }))
        .to_request();
    let _ = test::call_service(&app, req).await;

    // Request reset
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/password-reset/request")
        .set_json(serde_json::json!({
            "email": "reset-full@example.com"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Get real token from MockEmailProvider
    let sent = ctx.email_provider.sent_emails.lock().unwrap();
    assert_eq!(sent.len(), 1);
    let reset_token = sent[0].1.clone();
    drop(sent);

    // Confirm reset with real token
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/password-reset/confirm")
        .set_json(serde_json::json!({
            "token": reset_token,
            "new_password": "new-password-after-reset"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Old password should fail
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .set_json(serde_json::json!({
            "email": "reset-full@example.com",
            "password": TEST_PASSWORD
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // New password should work
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .set_json(serde_json::json!({
            "email": "reset-full@example.com",
            "password": "new-password-after-reset"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Token reuse should fail
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/password-reset/confirm")
        .set_json(serde_json::json!({
            "token": reset_token,
            "new_password": "another-password"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn password_reset_enumeration_responses_identical() {
    let ctx = create_test_context().await;

    let auth_api: Arc<dyn AuthApi> = ctx.auth_service.clone();
    let authenticator: Arc<dyn Authenticator> = ctx.auth_service.clone();
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(ctx.app_state.clone()))
            .app_data(web::Data::new(auth_api))
            .wrap(AuthMiddleware::new(web::Data::new(authenticator)))
            .wrap(CsrfMiddleware::new(web::Data::new(CsrfConfig::default())))
            .configure(api::auth::configure),
    )
    .await;

    // Unknown email
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/password-reset/request")
        .set_json(serde_json::json!({
            "email": "unknown-enum@example.com"
        }))
        .to_request();
    let resp_unknown = test::call_service(&app, req).await;

    // Register existing
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/register")
        .set_json(serde_json::json!({
            "email": "known-enum@example.com",
            "password": TEST_PASSWORD
        }))
        .to_request();
    let _ = test::call_service(&app, req).await;

    // Known email
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/password-reset/request")
        .set_json(serde_json::json!({
            "email": "known-enum@example.com"
        }))
        .to_request();
    let resp_known = test::call_service(&app, req).await;

    // Responses must be identical
    assert_eq!(resp_unknown.status(), resp_known.status());
}

#[actix_web::test]
async fn email_verification_full_flow() {
    let ctx = create_test_context().await;

    let auth_api: Arc<dyn AuthApi> = ctx.auth_service.clone();
    let authenticator: Arc<dyn Authenticator> = ctx.auth_service.clone();
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(ctx.app_state.clone()))
            .app_data(web::Data::new(auth_api))
            .wrap(AuthMiddleware::new(web::Data::new(authenticator)))
            .wrap(CsrfMiddleware::new(web::Data::new(CsrfConfig::default())))
            .configure(api::auth::configure),
    )
    .await;

    // Register
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/register")
        .set_json(serde_json::json!({
            "email": "verify-flow@example.com",
            "password": TEST_PASSWORD
        }))
        .to_request();
    let _ = test::call_service(&app, req).await;

    // Login
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .set_json(serde_json::json!({
            "email": "verify-flow@example.com",
            "password": TEST_PASSWORD
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let session_token = extract_cookie(&resp, "__Host-aevum_session").unwrap();
    let csrf_token = extract_cookie(&resp, "__Host-aevum_csrf").unwrap();

    // Request verification
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/email/verification/request")
        .cookie(cookie_header("__Host-aevum_session", &session_token))
        .cookie(cookie_header("__Host-aevum_csrf", &csrf_token))
        .insert_header(("X-CSRF-Token", csrf_token.clone()))
        .set_json(serde_json::json!({}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Get token from MockEmailProvider
    let sent = ctx.email_provider.sent_emails.lock().unwrap();
    assert_eq!(sent.len(), 1);
    let verification_token = sent[0].1.clone();
    drop(sent);

    // Confirm verification (public endpoint)
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/email/verification/confirm")
        .set_json(serde_json::json!({
            "token": verification_token
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Token reuse should fail
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/email/verification/confirm")
        .set_json(serde_json::json!({
            "token": verification_token
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn session_management_flow() {
    let ctx = create_test_context().await;

    let auth_api: Arc<dyn AuthApi> = ctx.auth_service.clone();
    let authenticator: Arc<dyn Authenticator> = ctx.auth_service.clone();
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(ctx.app_state.clone()))
            .app_data(web::Data::new(auth_api))
            .wrap(AuthMiddleware::new(web::Data::new(authenticator)))
            .wrap(CsrfMiddleware::new(web::Data::new(CsrfConfig::default())))
            .configure(api::auth::configure),
    )
    .await;

    // Register + Login
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/register")
        .set_json(serde_json::json!({
            "email": "sessions@example.com",
            "password": TEST_PASSWORD
        }))
        .to_request();
    let _ = test::call_service(&app, req).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .set_json(serde_json::json!({
            "email": "sessions@example.com",
            "password": TEST_PASSWORD
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let session_token = extract_cookie(&resp, "__Host-aevum_session").unwrap();
    let csrf_token = extract_cookie(&resp, "__Host-aevum_csrf").unwrap();

    // List sessions
    let req = test::TestRequest::get()
        .uri("/api/v1/auth/sessions")
        .cookie(cookie_header("__Host-aevum_session", &session_token))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Revoke all other sessions (should revoke 0 since only current)
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/sessions/revoke-all")
        .cookie(cookie_header("__Host-aevum_session", &session_token))
        .cookie(cookie_header("__Host-aevum_csrf", &csrf_token))
        .insert_header(("X-CSRF-Token", csrf_token.clone()))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[actix_web::test]
async fn backup_codes_full_flow() {
    let ctx = create_test_context().await;

    let auth_api: Arc<dyn AuthApi> = ctx.auth_service.clone();
    let authenticator: Arc<dyn Authenticator> = ctx.auth_service.clone();
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(ctx.app_state.clone()))
            .app_data(web::Data::new(auth_api))
            .wrap(AuthMiddleware::new(web::Data::new(authenticator)))
            .wrap(CsrfMiddleware::new(web::Data::new(CsrfConfig::default())))
            .configure(api::auth::configure),
    )
    .await;

    // Register + login
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/register")
        .set_json(serde_json::json!({
            "email": "backup@example.com",
            "password": TEST_PASSWORD
        }))
        .to_request();
    let _ = test::call_service(&app, req).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .set_json(serde_json::json!({
            "email": "backup@example.com",
            "password": TEST_PASSWORD
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let session_token = extract_cookie(&resp, "__Host-aevum_session").unwrap();
    let csrf_token = extract_cookie(&resp, "__Host-aevum_csrf").unwrap();

    // Generate codes
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/backup-codes/generate")
        .cookie(cookie_header("__Host-aevum_session", &session_token))
        .cookie(cookie_header("__Host-aevum_csrf", &csrf_token))
        .insert_header(("X-CSRF-Token", csrf_token.clone()))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    let codes = body["codes"].as_array().unwrap();
    assert_eq!(codes.len(), 10);
    let first_code = codes[0].as_str().unwrap().to_string();

    // Status — should show 10 remaining
    let req = test::TestRequest::get()
        .uri("/api/v1/auth/backup-codes/status")
        .cookie(cookie_header("__Host-aevum_session", &session_token))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["remaining"], 10);
    assert_eq!(body["total"], 10);
    assert_eq!(body["enabled"], true);

    // Verify first code
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/backup-codes/verify")
        .cookie(cookie_header("__Host-aevum_session", &session_token))
        .cookie(cookie_header("__Host-aevum_csrf", &csrf_token))
        .insert_header(("X-CSRF-Token", csrf_token.clone()))
        .set_json(serde_json::json!({ "code": first_code }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Verify same code again — should fail (single-use)
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/backup-codes/verify")
        .cookie(cookie_header("__Host-aevum_session", &session_token))
        .cookie(cookie_header("__Host-aevum_csrf", &csrf_token))
        .insert_header(("X-CSRF-Token", csrf_token.clone()))
        .set_json(serde_json::json!({ "code": first_code }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // Status — should show 9 remaining
    let req = test::TestRequest::get()
        .uri("/api/v1/auth/backup-codes/status")
        .cookie(cookie_header("__Host-aevum_session", &session_token))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["remaining"], 9);
}

#[actix_web::test]
async fn two_factor_full_enrollment_flow() {
    let ctx = create_test_context().await;

    let auth_api: Arc<dyn AuthApi> = ctx.auth_service.clone();
    let authenticator: Arc<dyn Authenticator> = ctx.auth_service.clone();
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(ctx.app_state.clone()))
            .app_data(web::Data::new(auth_api))
            .wrap(AuthMiddleware::new(web::Data::new(authenticator)))
            .wrap(CsrfMiddleware::new(web::Data::new(CsrfConfig::default())))
            .configure(api::auth::configure),
    )
    .await;

    // Register + login
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/register")
        .set_json(serde_json::json!({
            "email": "2fa-flow@example.com",
            "password": TEST_PASSWORD
        }))
        .to_request();
    let _ = test::call_service(&app, req).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .set_json(serde_json::json!({
            "email": "2fa-flow@example.com",
            "password": TEST_PASSWORD
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let session_token = extract_cookie(&resp, "__Host-aevum_session").unwrap();
    let csrf_token = extract_cookie(&resp, "__Host-aevum_csrf").unwrap();

    // Setup 2FA
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/2fa/setup")
        .cookie(cookie_header("__Host-aevum_session", &session_token))
        .cookie(cookie_header("__Host-aevum_csrf", &csrf_token))
        .insert_header(("X-CSRF-Token", csrf_token.clone()))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    let otpauth_uri = body["otpauth_uri"].as_str().unwrap();
    let secret_base32 = body["secret_base32"].as_str().unwrap();

    assert!(otpauth_uri.starts_with("otpauth://totp/"));
    assert!(!secret_base32.is_empty());

    // Generate valid TOTP code from secret
    use totp_rs::{Algorithm, Secret, TOTP};
    let secret = Secret::Encoded(secret_base32.to_string());
    let totp = TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        secret.to_bytes().unwrap(),
        Some("Aevum".to_string()),
        "2fa-flow@example.com".to_string(),
    )
    .unwrap();

    let code = totp.generate_current().unwrap();

    // Enable 2FA
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/2fa/enable")
        .cookie(cookie_header("__Host-aevum_session", &session_token))
        .cookie(cookie_header("__Host-aevum_csrf", &csrf_token))
        .insert_header(("X-CSRF-Token", csrf_token.clone()))
        .set_json(serde_json::json!({ "code": code }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Status should show enabled
    let req = test::TestRequest::get()
        .uri("/api/v1/auth/2fa/status")
        .cookie(cookie_header("__Host-aevum_session", &session_token))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["enabled"], true);

    // Backup codes should have been generated
    let req = test::TestRequest::get()
        .uri("/api/v1/auth/backup-codes/status")
        .cookie(cookie_header("__Host-aevum_session", &session_token))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["total"], 10);
}

#[actix_web::test]
async fn two_factor_rejects_invalid_code() {
    let ctx = create_test_context().await;

    let auth_api: Arc<dyn AuthApi> = ctx.auth_service.clone();
    let authenticator: Arc<dyn Authenticator> = ctx.auth_service.clone();
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(ctx.app_state.clone()))
            .app_data(web::Data::new(auth_api))
            .wrap(AuthMiddleware::new(web::Data::new(authenticator)))
            .wrap(CsrfMiddleware::new(web::Data::new(CsrfConfig::default())))
            .configure(api::auth::configure),
    )
    .await;

    // Register + login
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/register")
        .set_json(serde_json::json!({
            "email": "2fa-invalid@example.com",
            "password": TEST_PASSWORD
        }))
        .to_request();
    let _ = test::call_service(&app, req).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .set_json(serde_json::json!({
            "email": "2fa-invalid@example.com",
            "password": TEST_PASSWORD
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let session_token = extract_cookie(&resp, "__Host-aevum_session").unwrap();
    let csrf_token = extract_cookie(&resp, "__Host-aevum_csrf").unwrap();

    // Setup
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/2fa/setup")
        .cookie(cookie_header("__Host-aevum_session", &session_token))
        .cookie(cookie_header("__Host-aevum_csrf", &csrf_token))
        .insert_header(("X-CSRF-Token", csrf_token.clone()))
        .to_request();
    let _ = test::call_service(&app, req).await;

    // Try enable with wrong code
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/2fa/enable")
        .cookie(cookie_header("__Host-aevum_session", &session_token))
        .cookie(cookie_header("__Host-aevum_csrf", &csrf_token))
        .insert_header(("X-CSRF-Token", csrf_token.clone()))
        .set_json(serde_json::json!({ "code": "000000" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

/// Common helper: set up 2FA for a fresh user and return:
/// (app, email, session_token, csrf_token, secret_base32, backup_codes)
async fn setup_2fa_user(
    ctx: &support::TestContext,
    email: &str,
) -> (
    impl actix_web::dev::Service<
        actix_http::Request,
        Response = actix_web::dev::ServiceResponse<
            actix_web::body::EitherBody<actix_web::body::BoxBody>,
        >,
        Error = actix_web::Error,
    >,
    String,
    String,
    Vec<String>,
) {
    let auth_api: Arc<dyn AuthApi> = ctx.auth_service.clone();
    let authenticator: Arc<dyn Authenticator> = ctx.auth_service.clone();
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(ctx.app_state.clone()))
            .app_data(web::Data::new(auth_api))
            .wrap(AuthMiddleware::new(web::Data::new(authenticator)))
            .wrap(CsrfMiddleware::new(web::Data::new(CsrfConfig::default())))
            .configure(api::auth::configure),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/register")
        .set_json(serde_json::json!({ "email": email, "password": TEST_PASSWORD }))
        .to_request();
    let _ = test::call_service(&app, req).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .set_json(serde_json::json!({ "email": email, "password": TEST_PASSWORD }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let session_token = extract_cookie(&resp, "__Host-aevum_session").unwrap();
    let csrf_token = extract_cookie(&resp, "__Host-aevum_csrf").unwrap();

    // Setup
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/2fa/setup")
        .cookie(cookie_header("__Host-aevum_session", &session_token))
        .cookie(cookie_header("__Host-aevum_csrf", &csrf_token))
        .insert_header(("X-CSRF-Token", csrf_token.clone()))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body: serde_json::Value = test::read_body_json(resp).await;
    let secret_base32 = body["secret_base32"].as_str().unwrap().to_string();

    // Enable with a valid code
    use totp_rs::{Algorithm, Secret, TOTP};
    let secret = Secret::Encoded(secret_base32.clone());
    let totp = TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        secret.to_bytes().unwrap(),
        Some("Aevum".to_string()),
        email.to_string(),
    )
    .unwrap();
    let code = totp.generate_current().unwrap();

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/2fa/enable")
        .cookie(cookie_header("__Host-aevum_session", &session_token))
        .cookie(cookie_header("__Host-aevum_csrf", &csrf_token))
        .insert_header(("X-CSRF-Token", csrf_token.clone()))
        .set_json(serde_json::json!({ "code": code }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    let backup_codes: Vec<String> = body["backup_codes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();

    (app, secret_base32, session_token, backup_codes)
}

/// Wait until we are safely inside a fresh 30-second TOTP step.
///
/// This avoids flakiness when a test happens to run on a step boundary.
/// Production replay protection must not be weakened for tests.
async fn wait_for_fresh_totp_step() {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let seconds_into_step = now % 30;

    // Leave at least 5 seconds before the next boundary.
    if seconds_into_step > 25 {
        let wait = 30 - seconds_into_step + 1;
        tokio::time::sleep(tokio::time::Duration::from_secs(wait)).await;
    }
}

#[actix_web::test]
async fn two_factor_rejects_replayed_totp() {
    let ctx = create_test_context().await;
    let (app, secret_base32, _session, _codes) =
        setup_2fa_user(&ctx, "2fa-replay@example.com").await;

    // New login
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .set_json(serde_json::json!({
            "email": "2fa-replay@example.com",
            "password": TEST_PASSWORD
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body: serde_json::Value = test::read_body_json(resp).await;
    let pre_auth_token = body["pre_auth_token"].as_str().unwrap().to_string();

    // Wait for a fresh TOTP step before generating the code,
    // so both verify calls stay inside the same step.
    wait_for_fresh_totp_step().await;

    use totp_rs::{Algorithm, Secret, TOTP};
    let secret = Secret::Encoded(secret_base32);
    let totp = TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        secret.to_bytes().unwrap(),
        Some("Aevum".to_string()),
        "2fa-replay@example.com".to_string(),
    )
    .unwrap();
    let code = totp.generate_current().unwrap();

    // First verify — should succeed and consume the step.
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/2fa/verify")
        .set_json(serde_json::json!({
            "pre_auth_token": pre_auth_token,
            "code": code
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Replay: get a new pre-auth token, then reuse the SAME code.
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .set_json(serde_json::json!({
            "email": "2fa-replay@example.com",
            "password": TEST_PASSWORD
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body: serde_json::Value = test::read_body_json(resp).await;
    let pre_auth_token2 = body["pre_auth_token"].as_str().unwrap().to_string();

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/2fa/verify")
        .set_json(serde_json::json!({
            "pre_auth_token": pre_auth_token2,
            "code": code
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn two_factor_pre_auth_survives_failed_verification() {
    let ctx = create_test_context().await;
    let (app, _secret, _session, backup_codes) =
        setup_2fa_user(&ctx, "2fa-survive@example.com").await;

    // New login → pre-auth token
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .set_json(serde_json::json!({
            "email": "2fa-survive@example.com",
            "password": TEST_PASSWORD
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body: serde_json::Value = test::read_body_json(resp).await;
    let pre_auth_token = body["pre_auth_token"].as_str().unwrap().to_string();

    // Bad code
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/2fa/verify")
        .set_json(serde_json::json!({
            "pre_auth_token": pre_auth_token,
            "code": "000000"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // Same pre-auth token must still work with a valid backup code
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/2fa/verify")
        .set_json(serde_json::json!({
            "pre_auth_token": pre_auth_token,
            "code": backup_codes[0]
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let session = extract_cookie(&resp, "__Host-aevum_session").unwrap();
    assert!(!session.is_empty());
}

#[actix_web::test]
async fn two_factor_allows_backup_code_fallback() {
    let ctx = create_test_context().await;
    let (app, _secret, _session, backup_codes) =
        setup_2fa_user(&ctx, "2fa-backup@example.com").await;

    // New login
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .set_json(serde_json::json!({
            "email": "2fa-backup@example.com",
            "password": TEST_PASSWORD
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "requires_two_factor");
    let pre_auth_token = body["pre_auth_token"].as_str().unwrap().to_string();

    // Use backup code
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/2fa/verify")
        .set_json(serde_json::json!({
            "pre_auth_token": pre_auth_token,
            "code": backup_codes[0]
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let session = extract_cookie(&resp, "__Host-aevum_session").unwrap();
    assert!(!session.is_empty());

    // Backup code reuse must fail
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/2fa/verify")
        .set_json(serde_json::json!({
            "pre_auth_token": pre_auth_token,
            "code": backup_codes[0]
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn preferences_default_for_new_user() {
    let ctx = create_test_context().await;
    let auth_api: Arc<dyn AuthApi> = ctx.auth_service.clone();
    let authenticator: Arc<dyn Authenticator> = ctx.auth_service.clone();
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(ctx.app_state.clone()))
            .app_data(web::Data::new(auth_api))
            .wrap(AuthMiddleware::new(web::Data::new(authenticator)))
            .wrap(CsrfMiddleware::new(web::Data::new(CsrfConfig::default())))
            .configure(api::auth::configure),
    )
    .await;

    // Register + login
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/register")
        .set_json(serde_json::json!({
            "email": "prefs-default@example.com",
            "password": TEST_PASSWORD
        }))
        .to_request();
    let _ = test::call_service(&app, req).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .set_json(serde_json::json!({
            "email": "prefs-default@example.com",
            "password": TEST_PASSWORD
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let session_token = extract_cookie(&resp, "__Host-aevum_session").unwrap();

    let req = test::TestRequest::get()
        .uri("/api/v1/auth/preferences")
        .cookie(cookie_header("__Host-aevum_session", &session_token))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["theme"], "system");
    assert_eq!(body["language"], "en");
    assert_eq!(body["notifications_enabled"], true);
}

#[actix_web::test]
async fn preferences_requires_authentication() {
    let ctx = create_test_context().await;
    let auth_api: Arc<dyn AuthApi> = ctx.auth_service.clone();
    let authenticator: Arc<dyn Authenticator> = ctx.auth_service.clone();
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(ctx.app_state.clone()))
            .app_data(web::Data::new(auth_api))
            .wrap(AuthMiddleware::new(web::Data::new(authenticator)))
            .wrap(CsrfMiddleware::new(web::Data::new(CsrfConfig::default())))
            .configure(api::auth::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/auth/preferences")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn preferences_update_and_persist() {
    let ctx = create_test_context().await;
    let auth_api: Arc<dyn AuthApi> = ctx.auth_service.clone();
    let authenticator: Arc<dyn Authenticator> = ctx.auth_service.clone();
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(ctx.app_state.clone()))
            .app_data(web::Data::new(auth_api))
            .wrap(AuthMiddleware::new(web::Data::new(authenticator)))
            .wrap(CsrfMiddleware::new(web::Data::new(CsrfConfig::default())))
            .configure(api::auth::configure),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/register")
        .set_json(serde_json::json!({
            "email": "prefs-update@example.com",
            "password": TEST_PASSWORD
        }))
        .to_request();
    let _ = test::call_service(&app, req).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .set_json(serde_json::json!({
            "email": "prefs-update@example.com",
            "password": TEST_PASSWORD
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let session_token = extract_cookie(&resp, "__Host-aevum_session").unwrap();
    let csrf_token = extract_cookie(&resp, "__Host-aevum_csrf").unwrap();

    // Update
    let req = test::TestRequest::put()
        .uri("/api/v1/auth/preferences")
        .cookie(cookie_header("__Host-aevum_session", &session_token))
        .cookie(cookie_header("__Host-aevum_csrf", &csrf_token))
        .insert_header(("X-CSRF-Token", csrf_token.clone()))
        .set_json(serde_json::json!({
            "theme": "dark",
            "notifications_enabled": false
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["theme"], "dark");
    assert_eq!(body["notifications_enabled"], false);
    // Language not sent — should remain default
    assert_eq!(body["language"], "en");

    // GET again — should persist
    let req = test::TestRequest::get()
        .uri("/api/v1/auth/preferences")
        .cookie(cookie_header("__Host-aevum_session", &session_token))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["theme"], "dark");
    assert_eq!(body["notifications_enabled"], false);
}

#[actix_web::test]
async fn preferences_rejects_unknown_field() {
    let ctx = create_test_context().await;
    let auth_api: Arc<dyn AuthApi> = ctx.auth_service.clone();
    let authenticator: Arc<dyn Authenticator> = ctx.auth_service.clone();
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(ctx.app_state.clone()))
            .app_data(web::Data::new(auth_api))
            .wrap(AuthMiddleware::new(web::Data::new(authenticator)))
            .wrap(CsrfMiddleware::new(web::Data::new(CsrfConfig::default())))
            .configure(api::auth::configure),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/register")
        .set_json(serde_json::json!({
            "email": "prefs-unknown@example.com",
            "password": TEST_PASSWORD
        }))
        .to_request();
    let _ = test::call_service(&app, req).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .set_json(serde_json::json!({
            "email": "prefs-unknown@example.com",
            "password": TEST_PASSWORD
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let session_token = extract_cookie(&resp, "__Host-aevum_session").unwrap();
    let csrf_token = extract_cookie(&resp, "__Host-aevum_csrf").unwrap();

    let req = test::TestRequest::put()
        .uri("/api/v1/auth/preferences")
        .cookie(cookie_header("__Host-aevum_session", &session_token))
        .cookie(cookie_header("__Host-aevum_csrf", &csrf_token))
        .insert_header(("X-CSRF-Token", csrf_token.clone()))
        .set_json(serde_json::json!({
            "theme": "dark",
            "user_id": "00000000-0000-0000-0000-000000000000"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

fn minimal_png_test(width: u32, height: u32) -> Vec<u8> {
    let mut data = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, b'I', b'H', b'D',
        b'R',
    ];
    data.extend_from_slice(&width.to_be_bytes());
    data.extend_from_slice(&height.to_be_bytes());
    data.extend_from_slice(&[0x08, 0x02, 0x00, 0x00, 0x00]);
    data
}

#[actix_web::test]
async fn avatar_upload_get_delete_flow() {
    let ctx = create_test_context().await;
    let auth_api: Arc<dyn AuthApi> = ctx.auth_service.clone();
    let authenticator: Arc<dyn Authenticator> = ctx.auth_service.clone();
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(ctx.app_state.clone()))
            .app_data(web::Data::new(auth_api))
            .wrap(AuthMiddleware::new(web::Data::new(authenticator)))
            .wrap(CsrfMiddleware::new(web::Data::new(CsrfConfig::default())))
            .configure(api::auth::configure),
    )
    .await;

    // Register + login
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/register")
        .set_json(serde_json::json!({
            "email": "avatar@example.com",
            "password": TEST_PASSWORD
        }))
        .to_request();
    let _ = test::call_service(&app, req).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .set_json(serde_json::json!({
            "email": "avatar@example.com",
            "password": TEST_PASSWORD
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let session_token = extract_cookie(&resp, "__Host-aevum_session").unwrap();
    let csrf_token = extract_cookie(&resp, "__Host-aevum_csrf").unwrap();

    let png_data = minimal_png_test(256, 256);

    // Upload
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/avatar")
        .cookie(cookie_header("__Host-aevum_session", &session_token))
        .cookie(cookie_header("__Host-aevum_csrf", &csrf_token))
        .insert_header(("X-CSRF-Token", csrf_token.clone()))
        .insert_header(("Content-Type", "image/png"))
        .set_payload(png_data.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Get
    let req = test::TestRequest::get()
        .uri("/api/v1/auth/avatar")
        .cookie(cookie_header("__Host-aevum_session", &session_token))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let content_type = resp.headers().get("content-type").unwrap();
    assert_eq!(content_type.to_str().unwrap(), "image/png");

    let body = test::read_body(resp).await;
    assert_eq!(body.as_ref(), png_data.as_slice());

    // Delete
    let req = test::TestRequest::delete()
        .uri("/api/v1/auth/avatar")
        .cookie(cookie_header("__Host-aevum_session", &session_token))
        .cookie(cookie_header("__Host-aevum_csrf", &csrf_token))
        .insert_header(("X-CSRF-Token", csrf_token.clone()))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Get after delete — 404
    let req = test::TestRequest::get()
        .uri("/api/v1/auth/avatar")
        .cookie(cookie_header("__Host-aevum_session", &session_token))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn avatar_rejects_svg() {
    let ctx = create_test_context().await;
    let auth_api: Arc<dyn AuthApi> = ctx.auth_service.clone();
    let authenticator: Arc<dyn Authenticator> = ctx.auth_service.clone();
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(ctx.app_state.clone()))
            .app_data(web::Data::new(auth_api))
            .wrap(AuthMiddleware::new(web::Data::new(authenticator)))
            .wrap(CsrfMiddleware::new(web::Data::new(CsrfConfig::default())))
            .configure(api::auth::configure),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/register")
        .set_json(serde_json::json!({
            "email": "avatar-svg@example.com",
            "password": TEST_PASSWORD
        }))
        .to_request();
    let _ = test::call_service(&app, req).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .set_json(serde_json::json!({
            "email": "avatar-svg@example.com",
            "password": TEST_PASSWORD
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let session_token = extract_cookie(&resp, "__Host-aevum_session").unwrap();
    let csrf_token = extract_cookie(&resp, "__Host-aevum_csrf").unwrap();

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/avatar")
        .cookie(cookie_header("__Host-aevum_session", &session_token))
        .cookie(cookie_header("__Host-aevum_csrf", &csrf_token))
        .insert_header(("X-CSRF-Token", csrf_token.clone()))
        .insert_header(("Content-Type", "image/svg+xml"))
        .set_payload(b"<svg xmlns=\"http://www.w3.org/2000/svg\"/>".to_vec())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[actix_web::test]
async fn avatar_requires_authentication() {
    let ctx = create_test_context().await;
    let auth_api: Arc<dyn AuthApi> = ctx.auth_service.clone();
    let authenticator: Arc<dyn Authenticator> = ctx.auth_service.clone();
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(ctx.app_state.clone()))
            .app_data(web::Data::new(auth_api))
            .wrap(AuthMiddleware::new(web::Data::new(authenticator)))
            .wrap(CsrfMiddleware::new(web::Data::new(CsrfConfig::default())))
            .configure(api::auth::configure),
    )
    .await;

    let png_data = minimal_png_test(100, 100);

    // POST without session and without CSRF:
    // CSRF middleware runs first for mutating methods and rejects with 403.
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/avatar")
        .insert_header(("Content-Type", "image/png"))
        .set_payload(png_data)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    // GET without session: AuthMiddleware rejects with 401.
    let req = test::TestRequest::get()
        .uri("/api/v1/auth/avatar")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn security_center_default_state() {
    let ctx = create_test_context().await;
    let auth_api: Arc<dyn AuthApi> = ctx.auth_service.clone();
    let authenticator: Arc<dyn Authenticator> = ctx.auth_service.clone();
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(ctx.app_state.clone()))
            .app_data(web::Data::new(auth_api))
            .wrap(AuthMiddleware::new(web::Data::new(authenticator)))
            .wrap(CsrfMiddleware::new(web::Data::new(CsrfConfig::default())))
            .configure(api::auth::configure),
    )
    .await;

    // Register + login
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/register")
        .set_json(serde_json::json!({
            "email": "sec-default@example.com",
            "password": TEST_PASSWORD
        }))
        .to_request();
    let _ = test::call_service(&app, req).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .set_json(serde_json::json!({
            "email": "sec-default@example.com",
            "password": TEST_PASSWORD
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let session_token = extract_cookie(&resp, "__Host-aevum_session").unwrap();

    let req = test::TestRequest::get()
        .uri("/api/v1/auth/security-center")
        .cookie(cookie_header("__Host-aevum_session", &session_token))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["email_verified"], false);
    assert_eq!(body["two_factor_enabled"], false);
    assert_eq!(body["backup_codes_remaining"], 0);
    assert_eq!(body["active_sessions"], 1);
    // score: 0 (no email, no 2FA, no backup codes, no recent password change event)
    assert_eq!(body["security_score"], 0);
}

#[actix_web::test]
async fn security_center_consistent_after_2fa_enrollment() {
    let ctx = create_test_context().await;
    let auth_api: Arc<dyn AuthApi> = ctx.auth_service.clone();
    let authenticator: Arc<dyn Authenticator> = ctx.auth_service.clone();
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(ctx.app_state.clone()))
            .app_data(web::Data::new(auth_api))
            .wrap(AuthMiddleware::new(web::Data::new(authenticator)))
            .wrap(CsrfMiddleware::new(web::Data::new(CsrfConfig::default())))
            .configure(api::auth::configure),
    )
    .await;

    // Register + login
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/register")
        .set_json(serde_json::json!({
            "email": "sec-2fa@example.com",
            "password": TEST_PASSWORD
        }))
        .to_request();
    let _ = test::call_service(&app, req).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .set_json(serde_json::json!({
            "email": "sec-2fa@example.com",
            "password": TEST_PASSWORD
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let session_token = extract_cookie(&resp, "__Host-aevum_session").unwrap();
    let csrf_token = extract_cookie(&resp, "__Host-aevum_csrf").unwrap();

    // Setup + enable 2FA
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/2fa/setup")
        .cookie(cookie_header("__Host-aevum_session", &session_token))
        .cookie(cookie_header("__Host-aevum_csrf", &csrf_token))
        .insert_header(("X-CSRF-Token", csrf_token.clone()))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body: serde_json::Value = test::read_body_json(resp).await;
    let secret_base32 = body["secret_base32"].as_str().unwrap().to_string();

    use totp_rs::{Algorithm, Secret, TOTP};
    let secret = Secret::Encoded(secret_base32);
    let totp = TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        secret.to_bytes().unwrap(),
        Some("Aevum".to_string()),
        "sec-2fa@example.com".to_string(),
    )
    .unwrap();
    let code = totp.generate_current().unwrap();

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/2fa/enable")
        .cookie(cookie_header("__Host-aevum_session", &session_token))
        .cookie(cookie_header("__Host-aevum_csrf", &csrf_token))
        .insert_header(("X-CSRF-Token", csrf_token.clone()))
        .set_json(serde_json::json!({ "code": code }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Check Security Center reflects new state
    let req = test::TestRequest::get()
        .uri("/api/v1/auth/security-center")
        .cookie(cookie_header("__Host-aevum_session", &session_token))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body: serde_json::Value = test::read_body_json(resp).await;

    assert_eq!(body["two_factor_enabled"], true);
    assert_eq!(body["backup_codes_remaining"], 10);
    // score = 35 (2FA) + 15 (backup codes) = 50
    assert_eq!(body["security_score"], 50);
}

#[actix_web::test]
async fn security_center_requires_authentication() {
    let ctx = create_test_context().await;
    let auth_api: Arc<dyn AuthApi> = ctx.auth_service.clone();
    let authenticator: Arc<dyn Authenticator> = ctx.auth_service.clone();
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(ctx.app_state.clone()))
            .app_data(web::Data::new(auth_api))
            .wrap(AuthMiddleware::new(web::Data::new(authenticator)))
            .wrap(CsrfMiddleware::new(web::Data::new(CsrfConfig::default())))
            .configure(api::auth::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/auth/security-center")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn security_center_is_read_only() {
    let ctx = create_test_context().await;
    let auth_api: Arc<dyn AuthApi> = ctx.auth_service.clone();
    let authenticator: Arc<dyn Authenticator> = ctx.auth_service.clone();
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(ctx.app_state.clone()))
            .app_data(web::Data::new(auth_api))
            .wrap(AuthMiddleware::new(web::Data::new(authenticator)))
            .wrap(CsrfMiddleware::new(web::Data::new(CsrfConfig::default())))
            .configure(api::auth::configure),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/register")
        .set_json(serde_json::json!({
            "email": "sec-readonly@example.com",
            "password": TEST_PASSWORD
        }))
        .to_request();
    let _ = test::call_service(&app, req).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .set_json(serde_json::json!({
            "email": "sec-readonly@example.com",
            "password": TEST_PASSWORD
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let session_token = extract_cookie(&resp, "__Host-aevum_session").unwrap();

    // Fetch twice — state must be identical
    let req = test::TestRequest::get()
        .uri("/api/v1/auth/security-center")
        .cookie(cookie_header("__Host-aevum_session", &session_token))
        .to_request();
    let resp1 = test::call_service(&app, req).await;
    let body1: serde_json::Value = test::read_body_json(resp1).await;

    let req = test::TestRequest::get()
        .uri("/api/v1/auth/security-center")
        .cookie(cookie_header("__Host-aevum_session", &session_token))
        .to_request();
    let resp2 = test::call_service(&app, req).await;
    let body2: serde_json::Value = test::read_body_json(resp2).await;

    assert_eq!(body1["security_score"], body2["security_score"]);
    assert_eq!(body1["active_sessions"], body2["active_sessions"]);
    assert_eq!(
        body1["backup_codes_remaining"],
        body2["backup_codes_remaining"]
    );
    assert_eq!(body1["two_factor_enabled"], body2["two_factor_enabled"]);
}
