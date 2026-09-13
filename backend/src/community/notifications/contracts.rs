//! Notification HTTP contracts and stable error vocabulary.
//!
//! B-2.1 — DTOs, constants, error codes.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::models::NotificationKind;
use crate::error::ApiError;

// ─── Limits ────────────────────────────────────────────────

pub const DEFAULT_PAGE_LIMIT: usize = 20;
pub const MIN_PAGE_LIMIT: usize = 1;
pub const MAX_PAGE_LIMIT: usize = 100;

pub const MAX_PAYLOAD_SIZE: usize = 16 * 1024;
pub const MAX_SOURCE_ID_LEN: usize = 256;

// ─── Error codes ───────────────────────────────────────────

pub const INVALID_CURSOR_CODE: &str = "INVALID_CURSOR";
pub const INVALID_CURSOR_MESSAGE: &str = "Pagination cursor is invalid";

pub const SOURCE_ID_EMPTY_CODE: &str = "SOURCE_ID_EMPTY";
pub const SOURCE_ID_EMPTY_MESSAGE: &str = "Source identifier is empty";

pub const SOURCE_ID_TOO_LONG_CODE: &str = "SOURCE_ID_TOO_LONG";
pub const SOURCE_ID_TOO_LONG_MESSAGE: &str = "Source identifier is too long";

pub const SOURCE_ID_INVALID_CODE: &str = "SOURCE_ID_INVALID";
pub const SOURCE_ID_INVALID_MESSAGE: &str = "Source identifier contains invalid characters";

pub const PAYLOAD_TOO_LARGE_CODE: &str = "PAYLOAD_TOO_LARGE";
pub const PAYLOAD_TOO_LARGE_MESSAGE: &str = "Notification payload exceeds maximum size";

pub const LIMIT_OUT_OF_RANGE_CODE: &str = "LIMIT_OUT_OF_RANGE";
pub const LIMIT_OUT_OF_RANGE_MESSAGE: &str = "Page limit is out of allowed range";

// ─── DTOs ──────────────────────────────────────────────────

/// Public notification DTO.
///
/// `user_id` is deliberately omitted: all notification endpoints require
/// authentication, and the caller always knows their own user_id.
#[derive(Debug, Clone, Serialize)]
pub struct NotificationResponse {
    pub id: Uuid,
    pub kind: NotificationKind,
    pub payload: serde_json::Value,
    pub read_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct NotificationPageResponse {
    pub items: Vec<NotificationResponse>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UnreadCountResponse {
    pub count: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListNotificationsQuery {
    pub limit: Option<usize>,
    pub cursor: Option<String>,
}

// ─── ValidationError → ApiError ────────────────────────────

use super::validation::ValidationError;

impl From<ValidationError> for ApiError {
    fn from(error: ValidationError) -> Self {
        let (code, message) = match error {
            ValidationError::SourceIdEmpty => (SOURCE_ID_EMPTY_CODE, SOURCE_ID_EMPTY_MESSAGE),
            ValidationError::SourceIdTooLong { .. } => {
                (SOURCE_ID_TOO_LONG_CODE, SOURCE_ID_TOO_LONG_MESSAGE)
            }
            ValidationError::SourceIdInvalidChar { .. } => {
                (SOURCE_ID_INVALID_CODE, SOURCE_ID_INVALID_MESSAGE)
            }
            ValidationError::PayloadTooLarge { .. } => {
                (PAYLOAD_TOO_LARGE_CODE, PAYLOAD_TOO_LARGE_MESSAGE)
            }
            ValidationError::LimitOutOfRange { .. } => {
                (LIMIT_OUT_OF_RANGE_CODE, LIMIT_OUT_OF_RANGE_MESSAGE)
            }
        };

        ApiError::ValidationFailed { code, message }
    }
}

// ─── Tests ─────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::community::notifications::validation::ValidationError;

    #[test]
    fn limits_are_stable() {
        assert_eq!(DEFAULT_PAGE_LIMIT, 20);
        assert_eq!(MIN_PAGE_LIMIT, 1);
        assert_eq!(MAX_PAGE_LIMIT, 100);
        assert_eq!(MAX_PAYLOAD_SIZE, 16 * 1024);
        assert_eq!(MAX_SOURCE_ID_LEN, 256);
    }

    #[test]
    fn error_codes_are_stable() {
        assert_eq!(INVALID_CURSOR_CODE, "INVALID_CURSOR");
        assert_eq!(SOURCE_ID_EMPTY_CODE, "SOURCE_ID_EMPTY");
        assert_eq!(SOURCE_ID_TOO_LONG_CODE, "SOURCE_ID_TOO_LONG");
        assert_eq!(SOURCE_ID_INVALID_CODE, "SOURCE_ID_INVALID");
        assert_eq!(PAYLOAD_TOO_LARGE_CODE, "PAYLOAD_TOO_LARGE");
        assert_eq!(LIMIT_OUT_OF_RANGE_CODE, "LIMIT_OUT_OF_RANGE");
    }

    #[test]
    fn source_id_empty_maps_to_validation_failed() {
        let error = ApiError::from(ValidationError::SourceIdEmpty);
        assert_eq!(error.code(), SOURCE_ID_EMPTY_CODE);
        assert_eq!(
            error.status_code(),
            actix_web::http::StatusCode::BAD_REQUEST
        );
    }

    #[test]
    fn payload_too_large_maps_to_validation_failed() {
        let error = ApiError::from(ValidationError::PayloadTooLarge {
            max: MAX_PAYLOAD_SIZE,
            actual: MAX_PAYLOAD_SIZE + 1,
        });
        assert_eq!(error.code(), PAYLOAD_TOO_LARGE_CODE);
        assert_eq!(
            error.status_code(),
            actix_web::http::StatusCode::BAD_REQUEST
        );
    }

    #[test]
    fn limit_out_of_range_maps_to_validation_failed() {
        let error = ApiError::from(ValidationError::LimitOutOfRange {
            min: MIN_PAGE_LIMIT,
            max: MAX_PAGE_LIMIT,
            actual: 0,
        });
        assert_eq!(error.code(), LIMIT_OUT_OF_RANGE_CODE);
        assert_eq!(
            error.status_code(),
            actix_web::http::StatusCode::BAD_REQUEST
        );
    }

    #[test]
    fn list_query_rejects_unknown_fields() {
        let result =
            serde_json::from_str::<ListNotificationsQuery>(r#"{"limit":10,"unknown":"x"}"#);
        assert!(result.is_err());
    }

    #[test]
    fn list_query_accepts_empty() {
        let result = serde_json::from_str::<ListNotificationsQuery>("{}").unwrap();
        assert_eq!(result.limit, None);
        assert_eq!(result.cursor, None);
    }

    #[test]
    fn notification_response_serializes_without_user_id() {
        let response = NotificationResponse {
            id: Uuid::nil(),
            kind: NotificationKind::System,
            payload: serde_json::json!({"message": "hello"}),
            read_at: None,
            created_at: Utc::now(),
        };

        let json = serde_json::to_value(response).unwrap();
        let object = json.as_object().unwrap();

        assert!(!object.contains_key("user_id"));
        assert!(object.contains_key("id"));
        assert!(object.contains_key("kind"));
        assert!(object.contains_key("payload"));
        assert!(object.contains_key("read_at"));
        assert!(object.contains_key("created_at"));
    }

    #[test]
    fn unread_count_response_serializes() {
        let response = UnreadCountResponse { count: 3 };
        let json = serde_json::to_value(response).unwrap();
        assert_eq!(json["count"], 3);
    }
}
