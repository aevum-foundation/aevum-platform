//! Notification domain models.
//!
//! B-2.1 — models + cursor encoding.

use base64::Engine;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A durable notification owned by a specific user.
///
/// `source_id` is deliberately not stored here — it is part of the
/// idempotency identity and lives in the idempotency index.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Notification {
    pub id: Uuid,
    pub user_id: Uuid,
    pub kind: NotificationKind,
    pub payload: serde_json::Value,
    pub read_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Notification kind. Exactly four kinds are supported in v1.
///
/// New kinds are added only when a real producer exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationKind {
    ReplyToTopic,
    ReplyToComment,
    MentionedInPost,
    System,
}

impl NotificationKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ReplyToTopic => "reply_to_topic",
            Self::ReplyToComment => "reply_to_comment",
            Self::MentionedInPost => "mentioned_in_post",
            Self::System => "system",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "reply_to_topic" => Some(Self::ReplyToTopic),
            "reply_to_comment" => Some(Self::ReplyToComment),
            "mentioned_in_post" => Some(Self::MentionedInPost),
            "system" => Some(Self::System),
            _ => None,
        }
    }
}

/// Cursor for paginated notification listing.
///
/// Encoded as base64(UTF-8 JSON {created_at, id}).
/// Ordering: `created_at DESC, id DESC`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotificationCursor {
    pub created_at: DateTime<Utc>,
    pub id: Uuid,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CursorError {
    InvalidBase64,
    InvalidJson,
    InvalidTimestamp,
    InvalidUuid,
}

impl NotificationCursor {
    pub fn encode(&self) -> String {
        let json = serde_json::json!({
            "created_at": self.created_at.to_rfc3339(),
            "id": self.id.to_string(),
        });
        let bytes = serde_json::to_vec(&json).expect("cursor json");
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
    }

    pub fn decode(raw: &str) -> Result<Self, CursorError> {
        let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(raw.as_bytes())
            .map_err(|_| CursorError::InvalidBase64)?;

        let value: serde_json::Value =
            serde_json::from_slice(&bytes).map_err(|_| CursorError::InvalidJson)?;

        let created_at_str = value
            .get("created_at")
            .and_then(|v| v.as_str())
            .ok_or(CursorError::InvalidJson)?;

        let id_str = value
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or(CursorError::InvalidJson)?;

        let created_at = DateTime::parse_from_rfc3339(created_at_str)
            .map_err(|_| CursorError::InvalidTimestamp)?
            .with_timezone(&Utc);

        let id = Uuid::parse_str(id_str).map_err(|_| CursorError::InvalidUuid)?;

        Ok(Self { created_at, id })
    }
}

/// Result of a `create` operation in the storage layer.
///
/// Repeat emit does **not** return an error. It returns the existing
/// notification so the caller can react idempotently.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CreateNotificationResult {
    Created(Notification),
    AlreadyExists(Notification),
}

/// A single page of notifications.
///
/// `next_cursor = None` means there is no further page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationPage {
    pub items: Vec<Notification>,
    pub next_cursor: Option<NotificationCursor>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_as_str_is_stable() {
        assert_eq!(NotificationKind::ReplyToTopic.as_str(), "reply_to_topic");
        assert_eq!(
            NotificationKind::ReplyToComment.as_str(),
            "reply_to_comment"
        );
        assert_eq!(
            NotificationKind::MentionedInPost.as_str(),
            "mentioned_in_post"
        );
        assert_eq!(NotificationKind::System.as_str(), "system");
    }

    #[test]
    fn kind_from_str_round_trips() {
        for kind in [
            NotificationKind::ReplyToTopic,
            NotificationKind::ReplyToComment,
            NotificationKind::MentionedInPost,
            NotificationKind::System,
        ] {
            assert_eq!(NotificationKind::from_str(kind.as_str()), Some(kind));
        }
    }

    #[test]
    fn kind_from_str_rejects_unknown() {
        assert_eq!(NotificationKind::from_str("unknown"), None);
        assert_eq!(NotificationKind::from_str(""), None);
        assert_eq!(NotificationKind::from_str("ReplyToTopic"), None);
    }

    #[test]
    fn kind_serializes_snake_case() {
        let json = serde_json::to_string(&NotificationKind::ReplyToTopic).unwrap();
        assert_eq!(json, "\"reply_to_topic\"");
    }

    #[test]
    fn cursor_encode_decode_round_trips() {
        let cursor = NotificationCursor {
            created_at: Utc::now(),
            id: Uuid::new_v4(),
        };

        let encoded = cursor.encode();
        let decoded = NotificationCursor::decode(&encoded).unwrap();

        assert_eq!(decoded.id, cursor.id);
        assert_eq!(
            decoded.created_at.timestamp(),
            cursor.created_at.timestamp()
        );
    }

    #[test]
    fn cursor_decode_rejects_invalid_base64() {
        let result = NotificationCursor::decode("not-base64!!!");
        assert!(matches!(result, Err(CursorError::InvalidBase64)));
    }

    #[test]
    fn cursor_decode_rejects_invalid_json() {
        let bytes = b"not json";
        let encoded = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes);
        let result = NotificationCursor::decode(&encoded);
        assert!(matches!(result, Err(CursorError::InvalidJson)));
    }

    #[test]
    fn cursor_decode_rejects_invalid_uuid() {
        let json = serde_json::json!({
            "created_at": Utc::now().to_rfc3339(),
            "id": "not-a-uuid",
        });
        let bytes = serde_json::to_vec(&json).unwrap();
        let encoded = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes);
        let result = NotificationCursor::decode(&encoded);
        assert!(matches!(result, Err(CursorError::InvalidUuid)));
    }
}
