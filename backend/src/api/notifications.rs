//! Notification HTTP API.
//!
//! B-2.4
//!
//! HTTP layer only:
//! - authentication context extraction;
//! - request deserialization;
//! - delegation to NotificationApi;
//! - HTTP response serialization.
//!
//! Business rules remain in NotificationService.
//!
//! Endpoints:
//! - GET  /api/v1/community/notifications               (paginated list)
//! - GET  /api/v1/community/notifications/unread-count  (unread count)
//! - POST /api/v1/community/notifications/{id}/read     (mark read, CSRF)

use std::sync::Arc;

use actix_web::{get, post, web, HttpMessage, HttpRequest, HttpResponse};
use uuid::Uuid;

use crate::auth::models::AuthContext;
use crate::community::notifications::api::NotificationApi;
use crate::community::notifications::contracts::ListNotificationsQuery;
use crate::error::ApiError;

/// Application-facing Notification service exposed to HTTP handlers.
///
/// The HTTP layer depends only on the domain API contract and never on a
/// concrete storage implementation.
pub type AppNotificationService = Arc<dyn NotificationApi>;

/// GET /api/v1/community/notifications
///
/// Returns the authenticated user's notifications, ordered by
/// `created_at DESC, id DESC`. Pagination is cursor-based.
#[get("/api/v1/community/notifications")]
pub async fn list_notifications(
    req: HttpRequest,
    service: web::Data<AppNotificationService>,
    query: web::Query<ListNotificationsQuery>,
) -> Result<HttpResponse, ApiError> {
    let auth = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(ApiError::Unauthorized)?;

    let query = query.into_inner();

    let response = service
        .list(&auth.user.id, query.cursor, query.limit)
        .await?;

    Ok(HttpResponse::Ok().json(response))
}

/// GET /api/v1/community/notifications/unread-count
///
/// Returns the number of unread notifications for the authenticated user.
#[get("/api/v1/community/notifications/unread-count")]
pub async fn unread_count(
    req: HttpRequest,
    service: web::Data<AppNotificationService>,
) -> Result<HttpResponse, ApiError> {
    let auth = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(ApiError::Unauthorized)?;

    let response = service.unread_count(&auth.user.id).await?;

    Ok(HttpResponse::Ok().json(response))
}

/// POST /api/v1/community/notifications/{id}/read
///
/// Marks the notification as read. Idempotent.
///
/// Returns `404` when the notification does not exist or belongs to
/// another user. CSRF applies because `POST` is mutating.
#[post("/api/v1/community/notifications/{id}/read")]
pub async fn mark_read(
    req: HttpRequest,
    service: web::Data<AppNotificationService>,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    let auth = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or(ApiError::Unauthorized)?;

    let notification_id = path.into_inner();

    let response = service.mark_read(&auth.user.id, &notification_id).await?;

    Ok(HttpResponse::Ok().json(response))
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(list_notifications)
        .service(unread_count)
        .service(mark_read);
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};

    use crate::community::notifications::service::NotificationService;
    use crate::community::notifications::storage::InMemoryNotificationStorage;

    fn test_service() -> AppNotificationService {
        Arc::new(NotificationService::with_noop_sink(
            InMemoryNotificationStorage::new(),
        ))
    }

    #[actix_web::test]
    async fn list_notifications_without_auth_is_401() {
        let service = test_service();
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(service))
                .configure(configure),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/v1/community/notifications")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn unread_count_without_auth_is_401() {
        let service = test_service();
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(service))
                .configure(configure),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/v1/community/notifications/unread-count")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn mark_read_without_auth_is_401() {
        let service = test_service();
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(service))
                .configure(configure),
        )
        .await;

        let req = test::TestRequest::post()
            .uri(&format!(
                "/api/v1/community/notifications/{}/read",
                Uuid::new_v4()
            ))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::UNAUTHORIZED);
    }
}
