//! Notification delivery sink abstraction.
//!
//! B-2.3 — Sink is an extension point for future realtime delivery
//! (SSE, WebSocket, push). In v1 only `NoopNotificationSink` exists.
//!
//! ## Critical rule
//!
//! Storage commit happens **before** sink delivery.
//! A sink failure MUST NOT roll back an already committed notification.
//! Sink failures are logged by the service, not surfaced to the caller.
//!
//! Delivery is best-effort. Notifications are considered successfully
//! emitted once they are durably stored. Sink delivery is an optional
//! secondary step.

use async_trait::async_trait;

use super::models::Notification;

/// Error returned by a `NotificationSink::deliver` implementation.
///
/// Treated as opaque by the service — logged but never propagated to
/// `NotificationService::emit` callers.
///
/// Kept as an enum so that future delivery-specific variants
/// (`WebSocketClosed`, `SseDisconnected`, `PushProviderUnavailable`, ...)
/// can be added without a breaking change.
#[derive(Debug, thiserror::Error)]
pub enum NotificationSinkError {
    #[error("{0}")]
    Delivery(String),
}

impl NotificationSinkError {
    pub fn new(message: impl Into<String>) -> Self {
        Self::Delivery(message.into())
    }
}

impl From<String> for NotificationSinkError {
    fn from(value: String) -> Self {
        Self::Delivery(value)
    }
}

impl From<&str> for NotificationSinkError {
    fn from(value: &str) -> Self {
        Self::Delivery(value.to_owned())
    }
}

/// Delivery mechanism for notifications.
///
/// Implementations:
/// - NoopNotificationSink (v1)
/// - future: SseNotificationSink, WebSocketNotificationSink, ...
///
/// ## Object safety
///
/// This trait MUST remain object-safe so that `Arc<dyn NotificationSink>`
/// can be used as the service's sink field. Do NOT add methods taking
/// `self` by value (`fn into_inner(self)`), `Self` return types
/// (`fn clone(&self) -> Self`), or generic methods.
#[async_trait]
pub trait NotificationSink: Send + Sync {
    /// Deliver a notification.
    ///
    /// Called by the service **after** durable storage commit. Returning
    /// an error does not roll back the notification; it is logged and
    /// ignored.
    ///
    /// Delivery is best-effort.
    async fn deliver(&self, notification: &Notification) -> Result<(), NotificationSinkError>;
}

/// A no-op sink. Used when no realtime delivery is configured.
///
/// Always returns `Ok(())`.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopNotificationSink;

impl NoopNotificationSink {
    pub const fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NotificationSink for NoopNotificationSink {
    async fn deliver(&self, _notification: &Notification) -> Result<(), NotificationSinkError> {
        Ok(())
    }
}

/// Failing sink fixture for service-level tests.
///
/// Declared outside `mod tests` so that `service.rs` can import it as
/// `crate::community::notifications::sink::FailingSink`.
///
/// Verifies the critical invariant:
///
///   storage.create() → Ok
///   sink.deliver()   → Err
///   emit()           → Ok (notification remains persisted)
#[cfg(test)]
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct FailingSink;

#[cfg(test)]
#[async_trait]
impl NotificationSink for FailingSink {
    async fn deliver(&self, _: &Notification) -> Result<(), NotificationSinkError> {
        Err(NotificationSinkError::from("boom"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    fn sample() -> Notification {
        Notification {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            kind: super::super::models::NotificationKind::System,
            payload: serde_json::json!({"text": "hello"}),
            read_at: None,
            created_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn noop_sink_returns_ok() {
        let sink = NoopNotificationSink::new();
        let result = sink.deliver(&sample()).await;
        assert!(result.is_ok());
    }

    #[test]
    fn sink_error_new_constructs_delivery_variant() {
        let error = NotificationSinkError::new("boom");
        match error {
            NotificationSinkError::Delivery(message) => assert_eq!(message, "boom"),
        }
    }

    #[test]
    fn sink_error_displays_message() {
        let error = NotificationSinkError::new("boom");
        assert_eq!(error.to_string(), "boom");
    }

    #[test]
    fn sink_error_from_string() {
        let error: NotificationSinkError = String::from("boom").into();
        match error {
            NotificationSinkError::Delivery(message) => assert_eq!(message, "boom"),
        }
    }

    #[test]
    fn sink_error_from_str() {
        let error: NotificationSinkError = "boom".into();
        match error {
            NotificationSinkError::Delivery(message) => assert_eq!(message, "boom"),
        }
    }

    // ─── FailingSink fixture tests ───
    //
    // The FailingSink type itself is declared at the top of this module
    // (before `#[cfg(test)] mod tests`) so that `service.rs` tests can
    // import it as `crate::community::notifications::sink::FailingSink`.

    #[tokio::test]
    async fn failing_sink_returns_error() {
        let sink = FailingSink;
        let result = sink.deliver(&sample()).await;
        assert!(result.is_err());
    }
}
