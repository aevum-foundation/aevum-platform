//! Notification service — business logic over `NotificationStorage`.
//!
//! B-2.3 — NotificationService<S>.
//!
//! Responsibilities:
//! - validation of `source_id`, payload size, pagination cursor, limit;
//! - UUID / timestamp generation;
//! - idempotency orchestration;
//! - authorization by authenticated `user_id`;
//! - post-commit sink delivery (best-effort, never breaks emit).
//!
//! The service is generic over storage. The sink is a trait object so it
//! can be swapped without recomposition (SSE / WebSocket / push later).

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use super::api::NotificationApi;
use super::contracts::{
    NotificationPageResponse, NotificationResponse, UnreadCountResponse, DEFAULT_PAGE_LIMIT,
    INVALID_CURSOR_CODE, INVALID_CURSOR_MESSAGE, MAX_CURSOR_LEN,
};
use super::models::{CreateNotificationResult, Notification, NotificationCursor, NotificationKind};
use super::sink::{NoopNotificationSink, NotificationSink};
use super::storage::NotificationStorage;
use super::validation::{validate_limit, validate_payload_size, validate_source_id};
use crate::error::ApiError;

pub struct NotificationService<S: NotificationStorage> {
    storage: S,
    sink: Arc<dyn NotificationSink>,
}

impl<S: NotificationStorage> NotificationService<S> {
    pub fn new(storage: S, sink: Arc<dyn NotificationSink>) -> Self {
        Self { storage, sink }
    }

    pub fn with_noop_sink(storage: S) -> Self {
        Self {
            storage,
            sink: Arc::new(NoopNotificationSink::new()),
        }
    }
}

#[async_trait]
impl<S: NotificationStorage + 'static> NotificationApi for NotificationService<S> {
    async fn emit(
        &self,
        user_id: &Uuid,
        kind: NotificationKind,
        source_id: &str,
        payload: serde_json::Value,
    ) -> Result<NotificationResponse, ApiError> {
        // Validate inputs before touching storage.
        validate_source_id(source_id)?;
        validate_payload_size(&payload)?;

        let notification = Notification {
            id: Uuid::new_v4(),
            user_id: *user_id,
            kind,
            payload,
            read_at: None,
            created_at: Utc::now(),
        };

        let result = self.storage.create(&notification, source_id).await?;

        match result {
            CreateNotificationResult::Created(created) => {
                // Storage commit happened. Sink delivery is best-effort.
                // A sink failure MUST NOT roll back the notification.
                if let Err(error) = self.sink.deliver(&created).await {
                    tracing::warn!(
                        notification_id = %created.id,
                        user_id = %created.user_id,
                        error = %error,
                        "notification delivery failed"
                    );
                }

                Ok(to_response(&created))
            }
            CreateNotificationResult::AlreadyExists(existing) => {
                // Idempotent repeat: no new delivery attempt.
                Ok(to_response(&existing))
            }
        }
    }

    async fn list(
        &self,
        user_id: &Uuid,
        cursor: Option<String>,
        limit: Option<usize>,
    ) -> Result<NotificationPageResponse, ApiError> {
        let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT);
        validate_limit(limit)?;

        let decoded_cursor = match cursor {
            Some(raw) => {
                if raw.len() > MAX_CURSOR_LEN {
                    return Err(ApiError::ValidationFailed {
                        code: INVALID_CURSOR_CODE,
                        message: INVALID_CURSOR_MESSAGE,
                    });
                }

                Some(
                    NotificationCursor::decode(&raw).map_err(|_| ApiError::ValidationFailed {
                        code: INVALID_CURSOR_CODE,
                        message: INVALID_CURSOR_MESSAGE,
                    })?,
                )
            }
            None => None,
        };

        let page = self.storage.list(user_id, decoded_cursor, limit).await?;

        let items: Vec<NotificationResponse> = page.items.iter().map(to_response).collect();
        let next_cursor = page.next_cursor.as_ref().map(|c| c.encode());

        Ok(NotificationPageResponse { items, next_cursor })
    }

    async fn unread_count(&self, user_id: &Uuid) -> Result<UnreadCountResponse, ApiError> {
        let count = self.storage.unread_count(user_id).await?;
        Ok(UnreadCountResponse { count })
    }

    async fn mark_read(
        &self,
        user_id: &Uuid,
        notification_id: &Uuid,
    ) -> Result<NotificationResponse, ApiError> {
        let notification = self.storage.mark_read(user_id, notification_id).await?;
        Ok(to_response(&notification))
    }
}

fn to_response(notification: &Notification) -> NotificationResponse {
    NotificationResponse {
        id: notification.id,
        kind: notification.kind,
        payload: notification.payload.clone(),
        read_at: notification.read_at,
        created_at: notification.created_at,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::community::notifications::sink::{FailingSink, NotificationSinkError};
    use crate::community::notifications::storage::InMemoryNotificationStorage;

    fn service() -> NotificationService<InMemoryNotificationStorage> {
        NotificationService::with_noop_sink(InMemoryNotificationStorage::new())
    }

    fn service_with_failing_sink() -> NotificationService<InMemoryNotificationStorage> {
        NotificationService::new(InMemoryNotificationStorage::new(), Arc::new(FailingSink))
    }

    #[tokio::test]
    async fn emit_creates_notification() {
        let service = service();
        let user_id = Uuid::new_v4();

        let response = service
            .emit(
                &user_id,
                NotificationKind::System,
                "src-1",
                serde_json::json!({"text": "hi"}),
            )
            .await
            .unwrap();

        assert_eq!(response.kind, NotificationKind::System);
        assert!(response.read_at.is_none());
        assert_eq!(response.payload["text"], "hi");
    }

    #[tokio::test]
    async fn emit_repeat_is_idempotent() {
        let service = service();
        let user_id = Uuid::new_v4();

        let first = service
            .emit(
                &user_id,
                NotificationKind::System,
                "src-1",
                serde_json::json!({"text": "hi"}),
            )
            .await
            .unwrap();

        let second = service
            .emit(
                &user_id,
                NotificationKind::System,
                "src-1",
                serde_json::json!({"text": "different"}),
            )
            .await
            .unwrap();

        assert_eq!(first.id, second.id);
        assert_eq!(first.payload, second.payload);
    }

    #[tokio::test]
    async fn emit_different_kind_creates_two() {
        let service = service();
        let user_id = Uuid::new_v4();

        service
            .emit(
                &user_id,
                NotificationKind::ReplyToTopic,
                "src-1",
                serde_json::json!({}),
            )
            .await
            .unwrap();

        service
            .emit(
                &user_id,
                NotificationKind::MentionedInPost,
                "src-1",
                serde_json::json!({}),
            )
            .await
            .unwrap();

        let page = service.list(&user_id, None, None).await.unwrap();
        assert_eq!(page.items.len(), 2);
    }

    #[tokio::test]
    async fn emit_empty_source_id_is_validation_failed() {
        let service = service();
        let user_id = Uuid::new_v4();

        let error = service
            .emit(
                &user_id,
                NotificationKind::System,
                "",
                serde_json::json!({}),
            )
            .await
            .unwrap_err();

        assert_eq!(error.code(), "SOURCE_ID_EMPTY");
        assert_eq!(
            error.status_code(),
            actix_web::http::StatusCode::BAD_REQUEST
        );
    }

    #[tokio::test]
    async fn emit_source_id_too_long_is_validation_failed() {
        let service = service();
        let user_id = Uuid::new_v4();
        let long = "a".repeat(257);

        let error = service
            .emit(
                &user_id,
                NotificationKind::System,
                &long,
                serde_json::json!({}),
            )
            .await
            .unwrap_err();

        assert_eq!(error.code(), "SOURCE_ID_TOO_LONG");
    }

    #[tokio::test]
    async fn emit_payload_too_large_is_validation_failed() {
        let service = service();
        let user_id = Uuid::new_v4();
        let big = "a".repeat(17 * 1024);
        let payload = serde_json::json!({"data": big});

        let error = service
            .emit(&user_id, NotificationKind::System, "src-1", payload)
            .await
            .unwrap_err();

        assert_eq!(error.code(), "PAYLOAD_TOO_LARGE");
    }

    #[tokio::test]
    async fn emit_with_failing_sink_returns_ok_and_persists() {
        let service = service_with_failing_sink();
        let user_id = Uuid::new_v4();

        // Sink will fail. emit must still succeed and the notification
        // must remain persisted — this is the critical invariant.
        let response = service
            .emit(
                &user_id,
                NotificationKind::System,
                "src-1",
                serde_json::json!({"text": "hi"}),
            )
            .await
            .unwrap();

        // Persisted and visible through list.
        let page = service.list(&user_id, None, None).await.unwrap();
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].id, response.id);
    }

    #[tokio::test]
    async fn list_empty_returns_empty_page() {
        let service = service();
        let page = service.list(&Uuid::new_v4(), None, None).await.unwrap();
        assert!(page.items.is_empty());
        assert!(page.next_cursor.is_none());
    }

    #[tokio::test]
    async fn list_default_limit_is_20() {
        let service = service();
        let user_id = Uuid::new_v4();

        // Emit 25 notifications.
        for i in 0..25 {
            service
                .emit(
                    &user_id,
                    NotificationKind::System,
                    &format!("src-{i}"),
                    serde_json::json!({}),
                )
                .await
                .unwrap();
        }

        let page = service.list(&user_id, None, None).await.unwrap();
        assert_eq!(page.items.len(), 20);
        assert!(page.next_cursor.is_some());
    }

    #[tokio::test]
    async fn list_pagination_uses_cursor() {
        let service = service();
        let user_id = Uuid::new_v4();

        for i in 0..5 {
            service
                .emit(
                    &user_id,
                    NotificationKind::System,
                    &format!("src-{i}"),
                    serde_json::json!({}),
                )
                .await
                .unwrap();
        }

        let page1 = service.list(&user_id, None, Some(2)).await.unwrap();
        assert_eq!(page1.items.len(), 2);
        assert!(page1.next_cursor.is_some());

        let page2 = service
            .list(&user_id, page1.next_cursor.clone(), Some(2))
            .await
            .unwrap();
        assert_eq!(page2.items.len(), 2);

        let page3 = service
            .list(&user_id, page2.next_cursor, Some(2))
            .await
            .unwrap();
        assert_eq!(page3.items.len(), 1);
        assert!(page3.next_cursor.is_none());
    }

    #[tokio::test]
    async fn list_invalid_cursor_is_validation_failed() {
        let service = service();
        let user_id = Uuid::new_v4();

        let error = service
            .list(&user_id, Some("not-a-real-cursor".to_owned()), None)
            .await
            .unwrap_err();

        assert_eq!(error.code(), "INVALID_CURSOR");
    }

    #[tokio::test]
    async fn list_limit_zero_is_validation_failed() {
        let service = service();
        let user_id = Uuid::new_v4();

        let error = service.list(&user_id, None, Some(0)).await.unwrap_err();

        assert_eq!(error.code(), "LIMIT_OUT_OF_RANGE");
    }

    #[tokio::test]
    async fn list_limit_too_large_is_validation_failed() {
        let service = service();
        let user_id = Uuid::new_v4();

        let error = service.list(&user_id, None, Some(101)).await.unwrap_err();

        assert_eq!(error.code(), "LIMIT_OUT_OF_RANGE");
    }

    #[tokio::test]
    async fn unread_count_tracks_state() {
        let service = service();
        let user_id = Uuid::new_v4();

        for i in 0..3 {
            service
                .emit(
                    &user_id,
                    NotificationKind::System,
                    &format!("src-{i}"),
                    serde_json::json!({}),
                )
                .await
                .unwrap();
        }

        let response = service.unread_count(&user_id).await.unwrap();
        assert_eq!(response.count, 3);
    }

    #[tokio::test]
    async fn mark_read_first_call_sets_read_at() {
        let service = service();
        let user_id = Uuid::new_v4();

        let emitted = service
            .emit(
                &user_id,
                NotificationKind::System,
                "s1",
                serde_json::json!({}),
            )
            .await
            .unwrap();

        let marked = service.mark_read(&user_id, &emitted.id).await.unwrap();
        assert!(marked.read_at.is_some());
    }

    #[tokio::test]
    async fn mark_read_repeat_is_idempotent() {
        let service = service();
        let user_id = Uuid::new_v4();

        let emitted = service
            .emit(
                &user_id,
                NotificationKind::System,
                "s1",
                serde_json::json!({}),
            )
            .await
            .unwrap();

        let first = service.mark_read(&user_id, &emitted.id).await.unwrap();
        let second = service.mark_read(&user_id, &emitted.id).await.unwrap();

        assert_eq!(first.read_at, second.read_at);
    }

    #[tokio::test]
    async fn mark_read_wrong_user_is_not_found() {
        let service = service();
        let owner = Uuid::new_v4();
        let other = Uuid::new_v4();

        let emitted = service
            .emit(
                &owner,
                NotificationKind::System,
                "s1",
                serde_json::json!({}),
            )
            .await
            .unwrap();

        let error = service.mark_read(&other, &emitted.id).await.unwrap_err();
        assert_eq!(error.status_code(), actix_web::http::StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn mark_read_missing_is_not_found() {
        let service = service();
        let error = service
            .mark_read(&Uuid::new_v4(), &Uuid::new_v4())
            .await
            .unwrap_err();
        assert_eq!(error.status_code(), actix_web::http::StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn response_does_not_contain_user_id() {
        let service = service();
        let user_id = Uuid::new_v4();

        let response = service
            .emit(
                &user_id,
                NotificationKind::System,
                "s1",
                serde_json::json!({"text": "hi"}),
            )
            .await
            .unwrap();

        let json = serde_json::to_value(response).unwrap();
        let object = json.as_object().unwrap();
        assert!(!object.contains_key("user_id"));
        assert!(object.contains_key("id"));
        assert!(object.contains_key("kind"));
        assert!(object.contains_key("payload"));
        assert!(object.contains_key("read_at"));
        assert!(object.contains_key("created_at"));
    }

    // Silence unused warning for the FailingSink import helper.
    #[allow(dead_code)]
    fn _assert_sink_error_type(_: NotificationSinkError) {}
}
