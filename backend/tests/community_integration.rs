//! B-1.2b.5 — Integration tests for the Community HTTP API.
//!
//! These tests exercise the full HTTP stack:
//! Request → AuthMiddleware → CSRF → Endpoint → CommunityService → Storage

use actix_web::{http::StatusCode, test, web, App};
use std::sync::Arc;

#[path = "support.rs"]
mod support;
use support::{cookie_header, create_test_context, extract_cookie};

use aevum_platform_api::{
    api,
    auth::{
        api::AuthApi, authenticator::Authenticator, csrf::CsrfConfig,
        csrf_middleware::CsrfMiddleware, middleware::AuthMiddleware,
    },
    community::api::CommunityApi,
};

const TEST_EMAIL_A: &str = "community-a@example.com";
const TEST_EMAIL_B: &str = "community-b@example.com";
const TEST_PASSWORD: &str = "correct-horse-battery-staple";

fn build_app(
    ctx: &support::TestContext,
) -> impl std::future::Future<
    Output = impl actix_web::dev::Service<
        actix_http::Request,
        Response = actix_web::dev::ServiceResponse<
            actix_web::body::EitherBody<actix_web::body::BoxBody>,
        >,
        Error = actix_web::Error,
    >,
> {
    let auth_api: Arc<dyn AuthApi> = ctx.auth_service.clone();
    let authenticator: Arc<dyn Authenticator> = ctx.auth_service.clone();
    let community_api: Arc<dyn CommunityApi> = ctx.community_service.clone();
    let app_state = ctx.app_state.clone();

    async move {
        test::init_service(
            App::new()
                .app_data(web::Data::new(app_state))
                .app_data(web::Data::new(auth_api))
                .app_data(web::Data::new(community_api))
                .wrap(AuthMiddleware::new(web::Data::new(authenticator)))
                .wrap(CsrfMiddleware::new(web::Data::new(CsrfConfig::default())))
                .configure(api::auth::configure)
                .configure(api::community::configure),
        )
        .await
    }
}

/// Register + login a user, returning `(session_token, csrf_token)`.
async fn register_and_login<S>(app: &S, email: &str) -> (String, String)
where
    S: actix_web::dev::Service<
        actix_http::Request,
        Response = actix_web::dev::ServiceResponse<
            actix_web::body::EitherBody<actix_web::body::BoxBody>,
        >,
        Error = actix_web::Error,
    >,
{
    // Register
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/register")
        .set_json(serde_json::json!({
            "email": email,
            "password": TEST_PASSWORD
        }))
        .to_request();
    let resp = test::call_service(app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    // Login
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .set_json(serde_json::json!({
            "email": email,
            "password": TEST_PASSWORD
        }))
        .to_request();
    let resp = test::call_service(app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let session_token = extract_cookie(&resp, "__Host-aevum_session").unwrap();
    let csrf_token = extract_cookie(&resp, "__Host-aevum_csrf").unwrap();

    (session_token, csrf_token)
}

// ─────────────────────────────────────────────────────────
// Public endpoint
// ─────────────────────────────────────────────────────────

#[actix_web::test]
async fn get_public_profile_not_found_returns_404() {
    let ctx = create_test_context().await;
    let app = build_app(&ctx).await;

    let req = test::TestRequest::get()
        .uri("/api/v1/community/u/ghost")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn get_public_profile_after_create_returns_200() {
    let ctx = create_test_context().await;
    let app = build_app(&ctx).await;

    let (session_token, csrf_token) = register_and_login(&app, TEST_EMAIL_A).await;

    // Create profile
    let req = test::TestRequest::put()
        .uri("/api/v1/community/me/profile")
        .cookie(cookie_header("__Host-aevum_session", &session_token))
        .cookie(cookie_header("__Host-aevum_csrf", &csrf_token))
        .insert_header(("X-CSRF-Token", csrf_token.as_str()))
        .set_json(serde_json::json!({
            "username": "alice",
            "display_name": "Alice",
            "bio": "hello"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Public lookup, no auth
    let req = test::TestRequest::get()
        .uri("/api/v1/community/u/alice")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

// ─────────────────────────────────────────────────────────
// Auth requirements
// ─────────────────────────────────────────────────────────

#[actix_web::test]
async fn get_own_profile_without_auth_is_401() {
    let ctx = create_test_context().await;
    let app = build_app(&ctx).await;

    let req = test::TestRequest::get()
        .uri("/api/v1/community/me/profile")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn get_own_profile_without_profile_returns_404() {
    let ctx = create_test_context().await;
    let app = build_app(&ctx).await;

    let (session_token, _csrf) = register_and_login(&app, TEST_EMAIL_A).await;

    let req = test::TestRequest::get()
        .uri("/api/v1/community/me/profile")
        .cookie(cookie_header("__Host-aevum_session", &session_token))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn put_profile_without_auth_is_rejected_by_outer_csrf() {
    let ctx = create_test_context().await;
    let app = build_app(&ctx).await;

    // CsrfMiddleware wraps AuthMiddleware, so an unauthenticated PUT
    // without CSRF cookie/header is rejected at the CSRF layer first.
    let req = test::TestRequest::put()
        .uri("/api/v1/community/me/profile")
        .set_json(serde_json::json!({
            "username": "alice",
            "display_name": null,
            "bio": null
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

// ─────────────────────────────────────────────────────────
// CSRF
// ─────────────────────────────────────────────────────────

#[actix_web::test]
async fn put_profile_without_csrf_is_403() {
    let ctx = create_test_context().await;
    let app = build_app(&ctx).await;

    let (session_token, _csrf_token) = register_and_login(&app, TEST_EMAIL_A).await;

    let req = test::TestRequest::put()
        .uri("/api/v1/community/me/profile")
        .cookie(cookie_header("__Host-aevum_session", &session_token))
        .set_json(serde_json::json!({
            "username": "alice",
            "display_name": null,
            "bio": null
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[actix_web::test]
async fn put_profile_with_wrong_csrf_is_403_and_profile_not_created() {
    let ctx = create_test_context().await;
    let app = build_app(&ctx).await;

    let (session_token, _csrf_token) = register_and_login(&app, TEST_EMAIL_A).await;

    let req = test::TestRequest::put()
        .uri("/api/v1/community/me/profile")
        .cookie(cookie_header("__Host-aevum_session", &session_token))
        .insert_header(("X-CSRF-Token", "definitely-wrong-token"))
        .set_json(serde_json::json!({
            "username": "alice",
            "display_name": null,
            "bio": null
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    // Side effect must not have happened.
    let req = test::TestRequest::get()
        .uri("/api/v1/community/u/alice")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// ─────────────────────────────────────────────────────────
// Happy path
// ─────────────────────────────────────────────────────────

#[actix_web::test]
async fn put_profile_create_happy_path() {
    let ctx = create_test_context().await;
    let app = build_app(&ctx).await;

    let (session_token, csrf_token) = register_and_login(&app, TEST_EMAIL_A).await;

    let req = test::TestRequest::put()
        .uri("/api/v1/community/me/profile")
        .cookie(cookie_header("__Host-aevum_session", &session_token))
        .cookie(cookie_header("__Host-aevum_csrf", &csrf_token))
        .insert_header(("X-CSRF-Token", csrf_token.as_str()))
        .set_json(serde_json::json!({
            "username": "alice",
            "display_name": "Alice",
            "bio": "hello"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["username"], "alice");
    assert_eq!(body["display_name"], "Alice");
    assert_eq!(body["bio"], "hello");
    assert_eq!(body["role"], "user");
    assert!(body["avatar_url"].is_null());

    // Own profile visible
    let req = test::TestRequest::get()
        .uri("/api/v1/community/me/profile")
        .cookie(cookie_header("__Host-aevum_session", &session_token))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[actix_web::test]
async fn put_profile_update_happy_path() {
    let ctx = create_test_context().await;
    let app = build_app(&ctx).await;

    let (session_token, csrf_token) = register_and_login(&app, TEST_EMAIL_A).await;

    // Create
    let req = test::TestRequest::put()
        .uri("/api/v1/community/me/profile")
        .cookie(cookie_header("__Host-aevum_session", &session_token))
        .cookie(cookie_header("__Host-aevum_csrf", &csrf_token))
        .insert_header(("X-CSRF-Token", csrf_token.as_str()))
        .set_json(serde_json::json!({
            "username": "alice",
            "display_name": "Alice",
            "bio": "first"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Update
    let req = test::TestRequest::put()
        .uri("/api/v1/community/me/profile")
        .cookie(cookie_header("__Host-aevum_session", &session_token))
        .cookie(cookie_header("__Host-aevum_csrf", &csrf_token))
        .insert_header(("X-CSRF-Token", csrf_token.as_str()))
        .set_json(serde_json::json!({
            "username": null,
            "display_name": "Alice Updated",
            "bio": "second"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["username"], "alice");
    assert_eq!(body["display_name"], "Alice Updated");
    assert_eq!(body["bio"], "second");
}

// ─────────────────────────────────────────────────────────
// Conflicts
// ─────────────────────────────────────────────────────────

#[actix_web::test]
async fn put_profile_duplicate_username_is_409() {
    let ctx = create_test_context().await;
    let app = build_app(&ctx).await;

    let (session_a, csrf_a) = register_and_login(&app, TEST_EMAIL_A).await;
    let (session_b, csrf_b) = register_and_login(&app, TEST_EMAIL_B).await;

    let req = test::TestRequest::put()
        .uri("/api/v1/community/me/profile")
        .cookie(cookie_header("__Host-aevum_session", &session_a))
        .cookie(cookie_header("__Host-aevum_csrf", &csrf_a))
        .insert_header(("X-CSRF-Token", csrf_a.as_str()))
        .set_json(serde_json::json!({
            "username": "alice",
            "display_name": null,
            "bio": null
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let req = test::TestRequest::put()
        .uri("/api/v1/community/me/profile")
        .cookie(cookie_header("__Host-aevum_session", &session_b))
        .cookie(cookie_header("__Host-aevum_csrf", &csrf_b))
        .insert_header(("X-CSRF-Token", csrf_b.as_str()))
        .set_json(serde_json::json!({
            "username": "alice",
            "display_name": null,
            "bio": null
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CONFLICT);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["error"]["code"], "USERNAME_ALREADY_EXISTS");

    // A's profile must be intact.
    let req = test::TestRequest::get()
        .uri("/api/v1/community/u/alice")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[actix_web::test]
async fn put_profile_username_change_is_409() {
    let ctx = create_test_context().await;
    let app = build_app(&ctx).await;

    let (session_token, csrf_token) = register_and_login(&app, TEST_EMAIL_A).await;

    let req = test::TestRequest::put()
        .uri("/api/v1/community/me/profile")
        .cookie(cookie_header("__Host-aevum_session", &session_token))
        .cookie(cookie_header("__Host-aevum_csrf", &csrf_token))
        .insert_header(("X-CSRF-Token", csrf_token.as_str()))
        .set_json(serde_json::json!({
            "username": "alice",
            "display_name": null,
            "bio": null
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let req = test::TestRequest::put()
        .uri("/api/v1/community/me/profile")
        .cookie(cookie_header("__Host-aevum_session", &session_token))
        .cookie(cookie_header("__Host-aevum_csrf", &csrf_token))
        .insert_header(("X-CSRF-Token", csrf_token.as_str()))
        .set_json(serde_json::json!({
            "username": "bob",
            "display_name": null,
            "bio": null
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CONFLICT);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["error"]["code"], "USERNAME_IMMUTABLE");
}

// ─────────────────────────────────────────────────────────
// Case-insensitive lookup
// ─────────────────────────────────────────────────────────

#[actix_web::test]
async fn public_username_lookup_is_case_insensitive() {
    let ctx = create_test_context().await;
    let app = build_app(&ctx).await;

    let (session_token, csrf_token) = register_and_login(&app, TEST_EMAIL_A).await;

    let req = test::TestRequest::put()
        .uri("/api/v1/community/me/profile")
        .cookie(cookie_header("__Host-aevum_session", &session_token))
        .cookie(cookie_header("__Host-aevum_csrf", &csrf_token))
        .insert_header(("X-CSRF-Token", csrf_token.as_str()))
        .set_json(serde_json::json!({
            "username": "Alice",
            "display_name": null,
            "bio": null
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    for lookup in ["alice", "ALICE", "Alice"] {
        let req = test::TestRequest::get()
            .uri(&format!("/api/v1/community/u/{}", lookup))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK, "lookup failed for {lookup}");
    }
}

// ─────────────────────────────────────────────────────────
// Visibility
// ─────────────────────────────────────────────────────────

#[actix_web::test]
async fn public_response_does_not_leak_private_fields() {
    let ctx = create_test_context().await;
    let app = build_app(&ctx).await;

    let (session_token, csrf_token) = register_and_login(&app, TEST_EMAIL_A).await;

    let req = test::TestRequest::put()
        .uri("/api/v1/community/me/profile")
        .cookie(cookie_header("__Host-aevum_session", &session_token))
        .cookie(cookie_header("__Host-aevum_csrf", &csrf_token))
        .insert_header(("X-CSRF-Token", csrf_token.as_str()))
        .set_json(serde_json::json!({
            "username": "alice",
            "display_name": "Alice",
            "bio": "hello"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let req = test::TestRequest::get()
        .uri("/api/v1/community/u/alice")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    let object = body
        .as_object()
        .expect("public profile must be a JSON object");

    let allowed = ["username", "display_name", "bio", "avatar_url", "joined_at"];

    assert_eq!(
        object.len(),
        allowed.len(),
        "public profile JSON must contain exactly the whitelisted fields, got: {:?}",
        object.keys().collect::<Vec<_>>()
    );

    for key in allowed {
        assert!(object.contains_key(key), "missing public field: {key}");
    }

    assert!(!object.contains_key("user_id"));
    assert!(!object.contains_key("email"));
    assert!(!object.contains_key("role"));
}

#[actix_web::test]
async fn private_response_contains_role_and_no_identity_leak() {
    let ctx = create_test_context().await;
    let app = build_app(&ctx).await;

    let (session_token, csrf_token) = register_and_login(&app, TEST_EMAIL_A).await;

    let req = test::TestRequest::put()
        .uri("/api/v1/community/me/profile")
        .cookie(cookie_header("__Host-aevum_session", &session_token))
        .cookie(cookie_header("__Host-aevum_csrf", &csrf_token))
        .insert_header(("X-CSRF-Token", csrf_token.as_str()))
        .set_json(serde_json::json!({
            "username": "alice",
            "display_name": "Alice",
            "bio": "hello"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let req = test::TestRequest::get()
        .uri("/api/v1/community/me/profile")
        .cookie(cookie_header("__Host-aevum_session", &session_token))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    let object = body
        .as_object()
        .expect("private profile must be a JSON object");

    let allowed = [
        "username",
        "display_name",
        "bio",
        "avatar_url",
        "joined_at",
        "role",
    ];

    assert_eq!(
        object.len(),
        allowed.len(),
        "private profile JSON must contain exactly the whitelisted fields, got: {:?}",
        object.keys().collect::<Vec<_>>()
    );

    for key in allowed {
        assert!(object.contains_key(key), "missing private field: {key}");
    }

    assert!(!object.contains_key("user_id"));
    assert!(!object.contains_key("email"));
}
