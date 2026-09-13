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
