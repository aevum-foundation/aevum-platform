//! B-2.5 — Integration tests for the Notification HTTP API.
//!
//! These tests exercise the full HTTP stack:
//! Request → AuthMiddleware → CSRF → Endpoint → NotificationService → Storage

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
    community::notifications::{api::NotificationApi, models::NotificationKind},
};

const TEST_EMAIL_A: &str = "notif-a@example.com";
const TEST_EMAIL_B: &str = "notif-b@example.com";
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
    let notification_api: Arc<dyn NotificationApi> = ctx.notification_service.clone();
    let app_state = ctx.app_state.clone();

    async move {
        test::init_service(
            App::new()
                .app_data(web::Data::new(app_state))
                .app_data(web::Data::new(auth_api))
                .app_data(web::Data::new(notification_api))
                .wrap(AuthMiddleware::new(web::Data::new(authenticator)))
                .wrap(CsrfMiddleware::new(web::Data::new(CsrfConfig::default())))
                .configure(api::auth::configure)
                .configure(api::notifications::configure),
        )
        .await
    }
}

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
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/register")
        .set_json(serde_json::json!({
            "email": email,
            "password": TEST_PASSWORD
        }))
        .to_request();
    let resp = test::call_service(app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .set_json(serde_json::json!({
            "email": email,
            "password": TEST_PASSWORD
        }))
        .to_request();
    let resp = test::call_service(app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let session = extract_cookie(&resp, "__Host-aevum_session").unwrap();
    let csrf = extract_cookie(&resp, "__Host-aevum_csrf").unwrap();

    (session, csrf)
}

// ─────────────────────────────────────────────────────────
// Auth requirements
// ─────────────────────────────────────────────────────────

#[actix_web::test]
async fn list_without_auth_is_401() {
    let ctx = create_test_context().await;
    let app = build_app(&ctx).await;

    let req = test::TestRequest::get()
        .uri("/api/v1/community/notifications")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn unread_count_without_auth_is_401() {
    let ctx = create_test_context().await;
    let app = build_app(&ctx).await;

    let req = test::TestRequest::get()
        .uri("/api/v1/community/notifications/unread-count")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn mark_read_without_auth_is_rejected_by_outer_csrf() {
    let ctx = create_test_context().await;
    let app = build_app(&ctx).await;

    // CsrfMiddleware wraps AuthMiddleware, so an unauthenticated POST
    // without CSRF cookie/header is rejected at the CSRF layer first.
    let req = test::TestRequest::post()
        .uri(&format!(
            "/api/v1/community/notifications/{}/read",
            uuid_like()
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

// ─────────────────────────────────────────────────────────
// CSRF
// ─────────────────────────────────────────────────────────

#[actix_web::test]
async fn mark_read_without_csrf_is_403() {
    let ctx = create_test_context().await;
    let app = build_app(&ctx).await;

    let (session, _csrf) = register_and_login(&app, TEST_EMAIL_A).await;

    let req = test::TestRequest::post()
        .uri(&format!(
            "/api/v1/community/notifications/{}/read",
            uuid_like()
        ))
        .cookie(cookie_header("__Host-aevum_session", &session))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[actix_web::test]
async fn mark_read_with_wrong_csrf_is_403() {
    let ctx = create_test_context().await;
    let app = build_app(&ctx).await;

    let (session, _csrf) = register_and_login(&app, TEST_EMAIL_A).await;

    let req = test::TestRequest::post()
        .uri(&format!(
            "/api/v1/community/notifications/{}/read",
            uuid_like()
        ))
        .cookie(cookie_header("__Host-aevum_session", &session))
        .insert_header(("X-CSRF-Token", "wrong-token"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

// ─────────────────────────────────────────────────────────
// Happy paths — using NotificationApi directly for emit
// ─────────────────────────────────────────────────────────

#[actix_web::test]
async fn emit_then_list_via_http() {
    let ctx = create_test_context().await;
    let app = build_app(&ctx).await;

    let (session, _csrf) = register_and_login(&app, TEST_EMAIL_A).await;

    // We need a real user_id. Fetch via /me.
    let req = test::TestRequest::get()
        .uri("/api/v1/auth/me")
        .cookie(cookie_header("__Host-aevum_session", &session))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let me: serde_json::Value = test::read_body_json(resp).await;
    let user_id_str = me["user_id"].as_str().unwrap().to_owned();
    let user_id = uuid::Uuid::parse_str(&user_id_str).unwrap();

    // Emit directly through the service.
    ctx.notification_service
        .emit(
            &user_id,
            NotificationKind::System,
            "src-int-1",
            serde_json::json!({"text": "hello"}),
        )
        .await
        .unwrap();

    // List via HTTP.
    let req = test::TestRequest::get()
        .uri("/api/v1/community/notifications")
        .cookie(cookie_header("__Host-aevum_session", &session))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["items"].as_array().unwrap().len(), 1);
    assert_eq!(body["items"][0]["kind"], "system");
    assert!(body["items"][0].get("user_id").is_none());
}

#[actix_web::test]
async fn emit_repeat_same_source_does_not_duplicate() {
    let ctx = create_test_context().await;
    let app = build_app(&ctx).await;

    let (session, _csrf) = register_and_login(&app, TEST_EMAIL_A).await;

    let req = test::TestRequest::get()
        .uri("/api/v1/auth/me")
        .cookie(cookie_header("__Host-aevum_session", &session))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let me: serde_json::Value = test::read_body_json(resp).await;
    let user_id = uuid::Uuid::parse_str(me["user_id"].as_str().unwrap()).unwrap();

    for _ in 0..3 {
        ctx.notification_service
            .emit(
                &user_id,
                NotificationKind::System,
                "same-src",
                serde_json::json!({}),
            )
            .await
            .unwrap();
    }

    let req = test::TestRequest::get()
        .uri("/api/v1/community/notifications")
        .cookie(cookie_header("__Host-aevum_session", &session))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["items"].as_array().unwrap().len(), 1);
}

// ─────────────────────────────────────────────────────────
// Pagination
// ─────────────────────────────────────────────────────────

#[actix_web::test]
async fn list_pagination_via_http() {
    let ctx = create_test_context().await;
    let app = build_app(&ctx).await;

    let (session, _csrf) = register_and_login(&app, TEST_EMAIL_A).await;

    let req = test::TestRequest::get()
        .uri("/api/v1/auth/me")
        .cookie(cookie_header("__Host-aevum_session", &session))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let me: serde_json::Value = test::read_body_json(resp).await;
    let user_id = uuid::Uuid::parse_str(me["user_id"].as_str().unwrap()).unwrap();

    for i in 0..5 {
        ctx.notification_service
            .emit(
                &user_id,
                NotificationKind::System,
                &format!("src-{i}"),
                serde_json::json!({}),
            )
            .await
            .unwrap();
    }

    // Page 1
    let req = test::TestRequest::get()
        .uri("/api/v1/community/notifications?limit=2")
        .cookie(cookie_header("__Host-aevum_session", &session))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["items"].as_array().unwrap().len(), 2);
    let next = body["next_cursor"].as_str().unwrap().to_owned();

    // Page 2
    let req = test::TestRequest::get()
        .uri(&format!(
            "/api/v1/community/notifications?limit=2&cursor={}",
            next
        ))
        .cookie(cookie_header("__Host-aevum_session", &session))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["items"].as_array().unwrap().len(), 2);
}

#[actix_web::test]
async fn list_invalid_cursor_is_400() {
    let ctx = create_test_context().await;
    let app = build_app(&ctx).await;

    let (session, _csrf) = register_and_login(&app, TEST_EMAIL_A).await;

    let req = test::TestRequest::get()
        .uri("/api/v1/community/notifications?cursor=not-base64!!!")
        .cookie(cookie_header("__Host-aevum_session", &session))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["error"]["code"], "INVALID_CURSOR");
}

#[actix_web::test]
async fn list_limit_out_of_range_is_400() {
    let ctx = create_test_context().await;
    let app = build_app(&ctx).await;

    let (session, _csrf) = register_and_login(&app, TEST_EMAIL_A).await;

    let req = test::TestRequest::get()
        .uri("/api/v1/community/notifications?limit=0")
        .cookie(cookie_header("__Host-aevum_session", &session))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["error"]["code"], "LIMIT_OUT_OF_RANGE");
}

// ─────────────────────────────────────────────────────────
// Unread count
// ─────────────────────────────────────────────────────────

#[actix_web::test]
async fn unread_count_tracks_state() {
    let ctx = create_test_context().await;
    let app = build_app(&ctx).await;

    let (session, _csrf) = register_and_login(&app, TEST_EMAIL_A).await;

    let req = test::TestRequest::get()
        .uri("/api/v1/auth/me")
        .cookie(cookie_header("__Host-aevum_session", &session))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let me: serde_json::Value = test::read_body_json(resp).await;
    let user_id = uuid::Uuid::parse_str(me["user_id"].as_str().unwrap()).unwrap();

    for i in 0..3 {
        ctx.notification_service
            .emit(
                &user_id,
                NotificationKind::System,
                &format!("src-{i}"),
                serde_json::json!({}),
            )
            .await
            .unwrap();
    }

    let req = test::TestRequest::get()
        .uri("/api/v1/community/notifications/unread-count")
        .cookie(cookie_header("__Host-aevum_session", &session))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["count"], 3);
}

// ─────────────────────────────────────────────────────────
// Mark read
// ─────────────────────────────────────────────────────────

#[actix_web::test]
async fn mark_read_first_and_repeat() {
    let ctx = create_test_context().await;
    let app = build_app(&ctx).await;

    let (session, csrf) = register_and_login(&app, TEST_EMAIL_A).await;

    let req = test::TestRequest::get()
        .uri("/api/v1/auth/me")
        .cookie(cookie_header("__Host-aevum_session", &session))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let me: serde_json::Value = test::read_body_json(resp).await;
    let user_id = uuid::Uuid::parse_str(me["user_id"].as_str().unwrap()).unwrap();

    let notification = ctx
        .notification_service
        .emit(
            &user_id,
            NotificationKind::System,
            "src-read-1",
            serde_json::json!({}),
        )
        .await
        .unwrap();

    let notification_id = notification.id;

    // First mark read
    let req = test::TestRequest::post()
        .uri(&format!(
            "/api/v1/community/notifications/{}/read",
            notification_id
        ))
        .cookie(cookie_header("__Host-aevum_session", &session))
        .cookie(cookie_header("__Host-aevum_csrf", &csrf))
        .insert_header(("X-CSRF-Token", csrf.as_str()))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let first: serde_json::Value = test::read_body_json(resp).await;
    assert!(first["read_at"].is_string());

    // Repeat — idempotent
    let req = test::TestRequest::post()
        .uri(&format!(
            "/api/v1/community/notifications/{}/read",
            notification_id
        ))
        .cookie(cookie_header("__Host-aevum_session", &session))
        .cookie(cookie_header("__Host-aevum_csrf", &csrf))
        .insert_header(("X-CSRF-Token", csrf.as_str()))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let second: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(first["read_at"], second["read_at"]);
}

#[actix_web::test]
async fn mark_read_wrong_user_is_404() {
    let ctx = create_test_context().await;
    let app = build_app(&ctx).await;

    let (session_a, _csrf_a) = register_and_login(&app, TEST_EMAIL_A).await;
    let (_session_b, csrf_b) = register_and_login(&app, TEST_EMAIL_B).await;

    // Get user A id
    let req = test::TestRequest::get()
        .uri("/api/v1/auth/me")
        .cookie(cookie_header("__Host-aevum_session", &session_a))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let me: serde_json::Value = test::read_body_json(resp).await;
    let user_a_id = uuid::Uuid::parse_str(me["user_id"].as_str().unwrap()).unwrap();

    // Emit for user A
    let notification = ctx
        .notification_service
        .emit(
            &user_a_id,
            NotificationKind::System,
            "src-cross",
            serde_json::json!({}),
        )
        .await
        .unwrap();

    // User B attempts to mark A's notification as read via B's session
    let req = test::TestRequest::post()
        .uri(&format!(
            "/api/v1/community/notifications/{}/read",
            notification.id
        ))
        .cookie(cookie_header("__Host-aevum_session", &_session_b))
        .cookie(cookie_header("__Host-aevum_csrf", &csrf_b))
        .insert_header(("X-CSRF-Token", csrf_b.as_str()))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

fn uuid_like() -> uuid::Uuid {
    uuid::Uuid::new_v4()
}
