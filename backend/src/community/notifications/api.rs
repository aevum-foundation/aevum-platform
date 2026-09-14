//! Application-facing notification API.
//!
//! B-2.3 — NotificationApi trait used by HTTP handlers.
//!
//! The HTTP layer depends only on this trait and does not know which
//! storage backend or sink is active.

use async_trait::async_trait;
use uuid::Uuid;

use super::contracts::{NotificationPageResponse, NotificationResponse, UnreadCountResponse};
use super::models::NotificationKind;
use crate::error::ApiError;

/// High-level notification API used by HTTP handlers.
///
/// Implementations:
/// - NotificationService<InMemoryNotificationStorage>
/// - NotificationService<AevumDbNotificationStorage>
#[async_trait]
pub trait NotificationApi: Send + Sync {
    /// Emit a notification idempotently.
    ///
    /// Identity is `(user_id, kind, sha256(source_id))`. Repeat emit
    /// returns the existing notification instead of creating a duplicate.
    ///
    /// Sink delivery happens after durable storage commit and does not
    /// affect the success of this call.
    async fn emit(
        &self,
        user_id: &Uuid,
        kind: NotificationKind,
        source_id: &str,
        payload: serde_json::Value,
    ) -> Result<NotificationResponse, ApiError>;

    /// List notifications for the authenticated user.
    ///
    /// The `cursor` is an opaque string previously returned as
    /// `next_cursor`. Invalid cursors yield `400 INVALID_CURSOR`.
    async fn list(
        &self,
        user_id: &Uuid,
        cursor: Option<String>,
        limit: Option<usize>,
    ) -> Result<NotificationPageResponse, ApiError>;

    /// Return the number of unread notifications for the authenticated user.
    async fn unread_count(&self, user_id: &Uuid) -> Result<UnreadCountResponse, ApiError>;

    /// Mark a notification as read. Idempotent.
    ///
    /// Returns `404` if the notification does not exist or belongs to
    /// another user.
    async fn mark_read(
        &self,
        user_id: &Uuid,
        notification_id: &Uuid,
    ) -> Result<NotificationResponse, ApiError>;
}
