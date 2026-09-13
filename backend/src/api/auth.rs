//! Authentication API endpoints.
//!
//! AUTH-07 — Register
//! AUTH-08 — Login
//! AUTH-10 — /me
//! AUTH-11 — Logout

use uuid::Uuid;

use actix_web::{
    cookie::{Cookie, SameSite},
    get, post, web, HttpMessage, HttpRequest, HttpResponse,
};

use std::sync::Arc;

use crate::auth::api::AuthApi;
use crate::auth::contracts::{
    BackupCodeVerifyRequest, BackupCodesGenerateResponse, BackupCodeStatusResponse,
    EmailVerificationConfirm, EmailVerificationRequest, EmailVerificationResponse, LoginRequest,
    LoginResponse, LogoutResponse, MeResponse, PasswordChangeRequest, PasswordResetConfirm,
    PasswordResetRequest, RegisterRequest, RegisterResponse, SESSION_COOKIE_NAME,
    SESSION_DURATION_DAYS,
};
use crate::auth::csrf::{build_csrf_cookie, build_csrf_removal_cookie, CsrfConfig, CsrfToken};
use crate::auth::models::User;

use crate::error::{ApiError, ApiResult};

/// Application-wide authentication service type.
///
/// AUTH-17 — Storage agnostic API. The API layer works with the
/// AuthApi trait object and has no knowledge of the concrete storage.
pub type AppAuthService = Arc<dyn AuthApi>;

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

    let (user, session_token, csrf_token) = service.login(&req.email, &req.password, &ip).await?;

    let response = LoginResponse {
        user_id: user.id,
        email: user.email,
    };

    let session_cookie = build_session_cookie(session_token.expose());
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
        let token = crate::auth::password::SessionToken::from_secret(cookie.value().to_owned());

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

#[post("/api/v1/auth/rotate")]
pub async fn rotate(
    req: HttpRequest,
    service: web::Data<AppAuthService>,
) -> ApiResult<HttpResponse> {
    let cookie = req
        .cookie(crate::auth::contracts::SESSION_COOKIE_NAME)
        .ok_or(ApiError::Unauthorized)?;

    let session_token = crate::auth::password::SessionToken::from_secret(cookie.value().to_owned());

    let (new_session_token, new_csrf_token) = service.rotate_session(&session_token).await?;

    let session_cookie = build_session_cookie(new_session_token.expose());
    let csrf_cookie = build_csrf_cookie(&new_csrf_token, &CsrfConfig::default());

    let response = serde_json::json!({
        "success": true,
    });

    let mut http_response = HttpResponse::Ok().json(response);

    http_response
        .add_cookie(&session_cookie)
        .map_err(|_| ApiError::Internal)?;

    http_response
        .add_cookie(&csrf_cookie)
        .map_err(|_| ApiError::Internal)?;

    Ok(http_response)
}

#[post("/api/v1/auth/change-password")]
pub async fn change_password(
    req: web::Json<PasswordChangeRequest>,
    http_req: HttpRequest,
    service: web::Data<AppAuthService>,
) -> ApiResult<HttpResponse> {
    req.validate().map_err(|_| ApiError::BadRequest)?;

    let auth = http_req
        .extensions()
        .get::<crate::auth::models::AuthContext>()
        .cloned()
        .ok_or(ApiError::Unauthorized)?;
    let user = auth.user;

    let new_session_token = service
        .change_password(&user.id, &req.current_password, &req.new_password)
        .await?;

    let session_cookie = build_session_cookie(new_session_token.expose());
    let csrf_token = CsrfToken::generate();
    let csrf_cookie = build_csrf_cookie(&csrf_token, &CsrfConfig::default());

    let response = serde_json::json!({
        "success": true,
    });

    let mut http_response = HttpResponse::Ok().json(response);

    http_response
        .add_cookie(&session_cookie)
        .map_err(|_| ApiError::Internal)?;

    http_response
        .add_cookie(&csrf_cookie)
        .map_err(|_| ApiError::Internal)?;

    Ok(http_response)
}

#[post("/api/v1/auth/password-reset/request")]
pub async fn request_password_reset(
    req: web::Json<PasswordResetRequest>,
    http_req: HttpRequest,
    service: web::Data<AppAuthService>,
) -> ApiResult<HttpResponse> {
    req.validate().map_err(|_| ApiError::BadRequest)?;

    let ip = http_req
        .connection_info()
        .realip_remote_addr()
        .unwrap_or("unknown")
        .to_string();

    service.request_password_reset(&req.email, &ip).await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "If the account exists, reset instructions have been sent."
    })))
}

#[post("/api/v1/auth/password-reset/confirm")]
pub async fn confirm_password_reset(
    req: web::Json<PasswordResetConfirm>,
    service: web::Data<AppAuthService>,
) -> ApiResult<HttpResponse> {
    req.validate().map_err(|_| ApiError::BadRequest)?;

    service
        .confirm_password_reset(&req.token, &req.new_password)
        .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true
    })))
}

#[post("/api/v1/auth/email/verification/request")]
pub async fn request_email_verification(
    req: web::Json<EmailVerificationRequest>,
    http_req: HttpRequest,
    service: web::Data<AppAuthService>,
) -> ApiResult<HttpResponse> {
    req.validate().map_err(|_| ApiError::BadRequest)?;

    let auth = http_req
        .extensions()
        .get::<crate::auth::models::AuthContext>()
        .cloned()
        .ok_or(ApiError::Unauthorized)?;
    let user = auth.user;

    service.request_email_verification(&user.id).await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true
    })))
}

#[post("/api/v1/auth/email/verification/confirm")]
pub async fn confirm_email_verification(
    req: web::Json<EmailVerificationConfirm>,
    service: web::Data<AppAuthService>,
) -> ApiResult<HttpResponse> {
    req.validate().map_err(|_| ApiError::BadRequest)?;

    service.verify_email(&req.token).await?;

    Ok(HttpResponse::Ok().json(EmailVerificationResponse { success: true }))
}

#[get("/api/v1/auth/sessions")]
pub async fn list_sessions(
    http_req: HttpRequest,
    service: web::Data<AppAuthService>,
) -> ApiResult<HttpResponse> {
    let auth = http_req
        .extensions()
        .get::<crate::auth::models::AuthContext>()
        .cloned()
        .ok_or(ApiError::Unauthorized)?;
    let user = auth.user;

    let current_session_id = http_req.extensions().get::<Uuid>().copied();

    let sessions = service.list_sessions(&user.id, current_session_id).await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "sessions": sessions
    })))
}

#[post("/api/v1/auth/sessions/{id}/revoke")]
pub async fn revoke_session(
    path: web::Path<Uuid>,
    http_req: HttpRequest,
    service: web::Data<AppAuthService>,
) -> ApiResult<HttpResponse> {
    let auth = http_req
        .extensions()
        .get::<crate::auth::models::AuthContext>()
        .cloned()
        .ok_or(ApiError::Unauthorized)?;
    let user = auth.user;

    let session_id = path.into_inner();

    service.revoke_session(&session_id, &user.id).await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true
    })))
}

#[post("/api/v1/auth/sessions/revoke-all")]
pub async fn revoke_other_sessions(
    http_req: HttpRequest,
    service: web::Data<AppAuthService>,
) -> ApiResult<HttpResponse> {
    let auth = http_req
        .extensions()
        .get::<crate::auth::models::AuthContext>()
        .cloned()
        .ok_or(ApiError::Unauthorized)?;
    let user = auth.user;
    let current_session_id = auth.session.id;

    let revoked = service
        .revoke_other_sessions(&user.id, &current_session_id)
        .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "revoked_count": revoked
    })))
}

#[post("/api/v1/auth/backup-codes/generate")]
pub async fn generate_backup_codes(
    http_req: HttpRequest,
    service: web::Data<AppAuthService>,
) -> ApiResult<HttpResponse> {
    let auth = http_req
        .extensions()
        .get::<crate::auth::models::AuthContext>()
        .cloned()
        .ok_or(ApiError::Unauthorized)?;

    let codes = service.generate_backup_codes(&auth.user.id).await?;

    Ok(HttpResponse::Ok().json(BackupCodesGenerateResponse { codes }))
}

#[post("/api/v1/auth/backup-codes/verify")]
pub async fn verify_backup_code(
    req: web::Json<BackupCodeVerifyRequest>,
    http_req: HttpRequest,
    service: web::Data<AppAuthService>,
) -> ApiResult<HttpResponse> {
    req.validate().map_err(|_| ApiError::BadRequest)?;

    let auth = http_req
        .extensions()
        .get::<crate::auth::models::AuthContext>()
        .cloned()
        .ok_or(ApiError::Unauthorized)?;

    let valid = service
        .verify_backup_code(&auth.user.id, &req.code)
        .await?;

    if valid {
        Ok(HttpResponse::Ok().json(serde_json::json!({ "valid": true })))
    } else {
        Err(ApiError::Unauthorized)
    }
}

#[get("/api/v1/auth/backup-codes/status")]
pub async fn backup_codes_status(
    http_req: HttpRequest,
    service: web::Data<AppAuthService>,
) -> ApiResult<HttpResponse> {
    let auth = http_req
        .extensions()
        .get::<crate::auth::models::AuthContext>()
        .cloned()
        .ok_or(ApiError::Unauthorized)?;

    let status: BackupCodeStatusResponse = service.backup_codes_status(&auth.user.id).await?;

    Ok(HttpResponse::Ok().json(status))
}

#[get("/api/v1/auth/me")]
pub async fn me(req: HttpRequest) -> ApiResult<HttpResponse> {
    let auth = req
        .extensions()
        .get::<crate::auth::models::AuthContext>()
        .cloned()
        .ok_or(ApiError::Unauthorized)?;
    let user = auth.user;

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
        .service(rotate)
        .service(change_password)
        .service(request_password_reset)
        .service(confirm_password_reset)
        .service(request_email_verification)
        .service(confirm_email_verification)
        .service(list_sessions)
        .service(revoke_session)
        .service(revoke_other_sessions)
        .service(generate_backup_codes)
        .service(verify_backup_code)
        .service(backup_codes_status)
        .service(me);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::service::AuthService;
    use crate::auth::storage::InMemoryAuthStorage;
    use uuid::Uuid;

    use actix_web::{http::StatusCode, test, App};

    const TEST_EMAIL: &str = "test@example.com";
    const TEST_PASSWORD: &str = "correct-horse-battery-staple";

    fn test_service() -> AppAuthService {
        Arc::new(AuthService::new(InMemoryAuthStorage::new()))
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
        let auth_service = Arc::new(AuthService::new(storage.clone()));
        let service: Arc<dyn AuthApi> = auth_service.clone();
        register_test_user(&service).await;

        let (user, token, _csrf_token) = service
            .login(TEST_EMAIL, TEST_PASSWORD, "127.0.0.1")
            .await
            .unwrap();
        assert!(!user.id.is_nil());

        let authenticated = service.authenticate(&token).await.unwrap().unwrap();
        assert_eq!(authenticated.user.id, user.id);
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
        let auth_service = Arc::new(AuthService::new(storage.clone()));
        let service: Arc<dyn AuthApi> = auth_service.clone();
        register_test_user(&service).await;

        let (_, token, _csrf_token) = service
            .login(TEST_EMAIL, TEST_PASSWORD, "127.0.0.1")
            .await
            .unwrap();

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(service.clone()))
                .service(logout),
        )
        .await;

        let cookie = build_session_cookie(token.expose());
        let session_token =
            crate::auth::password::SessionToken::from_secret(token.expose().to_string());

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
        let auth_service = Arc::new(AuthService::new(storage.clone()));
        let service: Arc<dyn AuthApi> = auth_service.clone();
        register_test_user(&service).await;

        let (_, token, _csrf_token) = service
            .login(TEST_EMAIL, TEST_PASSWORD, "127.0.0.1")
            .await
            .unwrap();

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(service.clone()))
                .service(logout),
        )
        .await;

        let cookie = build_session_cookie(token.expose());
        let session_token =
            crate::auth::password::SessionToken::from_secret(token.expose().to_string());

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
