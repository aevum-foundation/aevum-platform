//! Community HTTP/API contracts and stable validation error vocabulary.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::models::CommunityRole;
use super::validation::{
    ValidationError, BIO_MAX_LEN, DISPLAY_NAME_MAX_LEN, USERNAME_MAX_LEN, USERNAME_MIN_LEN,
};
use crate::error::ApiError;

pub const USERNAME_TOO_SHORT_CODE: &str = "USERNAME_TOO_SHORT";
pub const USERNAME_TOO_SHORT_MESSAGE: &str = "Username must be at least 3 characters";

pub const USERNAME_TOO_LONG_CODE: &str = "USERNAME_TOO_LONG";
pub const USERNAME_TOO_LONG_MESSAGE: &str = "Username must be at most 32 characters";

pub const USERNAME_INVALID_CHARSET_CODE: &str = "USERNAME_INVALID_CHARSET";
pub const USERNAME_INVALID_CHARSET_MESSAGE: &str = "Username may only contain a-z, 0-9, _ and -";

pub const USERNAME_RESERVED_CODE: &str = "USERNAME_RESERVED";
pub const USERNAME_RESERVED_MESSAGE: &str = "Username is reserved";

pub const USERNAME_REQUIRED_CODE: &str = "USERNAME_REQUIRED";
pub const USERNAME_REQUIRED_MESSAGE: &str = "Username is required for initial profile creation";

pub const USERNAME_IMMUTABLE_CODE: &str = "USERNAME_IMMUTABLE";
pub const USERNAME_IMMUTABLE_MESSAGE: &str = "Username cannot be changed";

pub const USERNAME_ALREADY_EXISTS_CODE: &str = "USERNAME_ALREADY_EXISTS";
pub const USERNAME_ALREADY_EXISTS_MESSAGE: &str = "Username is already taken";

pub const DISPLAY_NAME_TOO_LONG_CODE: &str = "DISPLAY_NAME_TOO_LONG";
pub const DISPLAY_NAME_TOO_LONG_MESSAGE: &str = "Display name must be at most 64 characters";

pub const DISPLAY_NAME_INVALID_CHARS_CODE: &str = "DISPLAY_NAME_INVALID_CHARS";
pub const DISPLAY_NAME_INVALID_CHARS_MESSAGE: &str = "Display name contains invalid characters";

pub const BIO_TOO_LONG_CODE: &str = "BIO_TOO_LONG";
pub const BIO_TOO_LONG_MESSAGE: &str = "Bio must be at most 500 characters";

pub const BIO_INVALID_CHARS_CODE: &str = "BIO_INVALID_CHARS";
pub const BIO_INVALID_CHARS_MESSAGE: &str = "Bio contains invalid characters";

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateProfileRequest {
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub bio: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PublicProfileResponse {
    pub username: String,
    pub display_name: Option<String>,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
    pub joined_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PrivateProfileResponse {
    pub username: String,
    pub display_name: Option<String>,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
    pub joined_at: chrono::DateTime<chrono::Utc>,
    pub role: CommunityRole,
}

impl From<ValidationError> for ApiError {
    fn from(error: ValidationError) -> Self {
        let (code, message) = match error {
            ValidationError::UsernameTooShort { .. } => {
                (USERNAME_TOO_SHORT_CODE, USERNAME_TOO_SHORT_MESSAGE)
            }
            ValidationError::UsernameTooLong { .. } => {
                (USERNAME_TOO_LONG_CODE, USERNAME_TOO_LONG_MESSAGE)
            }
            ValidationError::UsernameInvalidCharset { .. } => (
                USERNAME_INVALID_CHARSET_CODE,
                USERNAME_INVALID_CHARSET_MESSAGE,
            ),
            ValidationError::UsernameReserved => {
                (USERNAME_RESERVED_CODE, USERNAME_RESERVED_MESSAGE)
            }
            ValidationError::DisplayNameTooLong { .. } => {
                (DISPLAY_NAME_TOO_LONG_CODE, DISPLAY_NAME_TOO_LONG_MESSAGE)
            }
            ValidationError::DisplayNameContainsControl { .. }
            | ValidationError::DisplayNameContainsBidi { .. } => (
                DISPLAY_NAME_INVALID_CHARS_CODE,
                DISPLAY_NAME_INVALID_CHARS_MESSAGE,
            ),
            ValidationError::BioTooLong { .. } => (BIO_TOO_LONG_CODE, BIO_TOO_LONG_MESSAGE),
            ValidationError::BioContainsControl { .. }
            | ValidationError::BioContainsBidi { .. } => {
                (BIO_INVALID_CHARS_CODE, BIO_INVALID_CHARS_MESSAGE)
            }
        };

        ApiError::ValidationFailed { code, message }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frozen_messages_match_validation_limits() {
        assert_eq!(
            USERNAME_TOO_SHORT_MESSAGE,
            "Username must be at least 3 characters"
        );
        assert_eq!(
            USERNAME_TOO_LONG_MESSAGE,
            "Username must be at most 32 characters"
        );
        assert_eq!(
            DISPLAY_NAME_TOO_LONG_MESSAGE,
            "Display name must be at most 64 characters"
        );
        assert_eq!(BIO_TOO_LONG_MESSAGE, "Bio must be at most 500 characters");

        assert_eq!(USERNAME_MIN_LEN, 3);
        assert_eq!(USERNAME_MAX_LEN, 32);
        assert_eq!(DISPLAY_NAME_MAX_LEN, 64);
        assert_eq!(BIO_MAX_LEN, 500);
    }

    #[test]
    fn validation_error_maps_without_leaking_offending_character() {
        let error = ApiError::from(ValidationError::UsernameInvalidCharset { offending: '@' });

        assert_eq!(error.code(), USERNAME_INVALID_CHARSET_CODE);
        assert_eq!(error.message(), USERNAME_INVALID_CHARSET_MESSAGE);
    }

    #[test]
    fn update_request_rejects_unknown_fields() {
        let result = serde_json::from_str::<UpdateProfileRequest>(
            r#"{"username":"alice","unknown":"value"}"#,
        );

        assert!(result.is_err());
    }

    #[test]
    fn conflict_codes_are_stable() {
        assert_eq!(USERNAME_REQUIRED_CODE, "USERNAME_REQUIRED");
        assert_eq!(USERNAME_IMMUTABLE_CODE, "USERNAME_IMMUTABLE");
        assert_eq!(USERNAME_ALREADY_EXISTS_CODE, "USERNAME_ALREADY_EXISTS");

        assert_eq!(
            USERNAME_REQUIRED_MESSAGE,
            "Username is required for initial profile creation"
        );
        assert_eq!(USERNAME_IMMUTABLE_MESSAGE, "Username cannot be changed");
        assert_eq!(USERNAME_ALREADY_EXISTS_MESSAGE, "Username is already taken");
    }

    #[test]
    fn conflict_detailed_renders_as_409_with_custom_payload() {
        let error = ApiError::ConflictDetailed {
            code: USERNAME_IMMUTABLE_CODE,
            message: USERNAME_IMMUTABLE_MESSAGE,
        };

        assert_eq!(error.code(), "USERNAME_IMMUTABLE");
        assert_eq!(error.message(), "Username cannot be changed");
        assert_eq!(error.status_code(), actix_web::http::StatusCode::CONFLICT);
    }

    #[test]
    fn legacy_conflict_still_renders_generic_payload() {
        let error = ApiError::Conflict;

        assert_eq!(error.code(), "CONFLICT");
        assert_eq!(error.message(), "Resource conflict");
        assert_eq!(error.status_code(), actix_web::http::StatusCode::CONFLICT);
    }

    #[test]
    fn response_contract_does_not_contain_user_id_or_email() {
        let public = serde_json::json!({
            "username": "alice",
            "display_name": "Alice",
            "bio": null,
            "avatar_url": null,
            "joined_at": "2026-01-01T00:00:00Z"
        });

        assert!(!public.as_object().unwrap().contains_key("user_id"));
        assert!(!public.as_object().unwrap().contains_key("email"));

        let _ = Uuid::nil();
    }
}
