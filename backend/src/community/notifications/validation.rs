//! Notification input validation.
//!
//! B-2.1 — source_id, payload size, pagination limit.

use std::fmt;

use super::contracts::{MAX_PAGE_LIMIT, MAX_PAYLOAD_SIZE, MAX_SOURCE_ID_LEN, MIN_PAGE_LIMIT};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    SourceIdEmpty,
    SourceIdTooLong {
        max: usize,
        actual: usize,
    },
    SourceIdInvalidChar {
        code: u32,
    },

    PayloadTooLarge {
        max: usize,
        actual: usize,
    },

    LimitOutOfRange {
        min: usize,
        max: usize,
        actual: usize,
    },
}

impl fmt::Display for ValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SourceIdEmpty => formatter.write_str("source_id is empty"),
            Self::SourceIdTooLong { max, actual } => {
                write!(formatter, "source_id is {actual} chars; maximum is {max}")
            }
            Self::SourceIdInvalidChar { code } => {
                write!(
                    formatter,
                    "source_id contains control character U+{code:04X}"
                )
            }
            Self::PayloadTooLarge { max, actual } => {
                write!(formatter, "payload is {actual} bytes; maximum is {max}")
            }
            Self::LimitOutOfRange { min, max, actual } => {
                write!(formatter, "limit {actual} is outside {min}..={max}")
            }
        }
    }
}

impl std::error::Error for ValidationError {}

/// Validate a producer-supplied source_id.
///
/// Rules:
/// - 1..=256 Unicode scalar values;
/// - no control characters.
pub fn validate_source_id(raw: &str) -> Result<(), ValidationError> {
    if raw.is_empty() {
        return Err(ValidationError::SourceIdEmpty);
    }

    let length = raw.chars().count();

    if length > MAX_SOURCE_ID_LEN {
        return Err(ValidationError::SourceIdTooLong {
            max: MAX_SOURCE_ID_LEN,
            actual: length,
        });
    }

    for character in raw.chars() {
        if character.is_control() {
            return Err(ValidationError::SourceIdInvalidChar {
                code: character as u32,
            });
        }
    }

    Ok(())
}

/// Validate the serialized size of a notification payload.
///
/// The payload is treated as opaque JSON, but its serialized form must fit
/// within [`MAX_PAYLOAD_SIZE`].
pub fn validate_payload_size(payload: &serde_json::Value) -> Result<(), ValidationError> {
    let serialized = serde_json::to_vec(payload).map_err(|_| ValidationError::PayloadTooLarge {
        max: MAX_PAYLOAD_SIZE,
        actual: MAX_PAYLOAD_SIZE + 1,
    })?;

    let actual = serialized.len();

    if actual > MAX_PAYLOAD_SIZE {
        return Err(ValidationError::PayloadTooLarge {
            max: MAX_PAYLOAD_SIZE,
            actual,
        });
    }

    Ok(())
}

/// Validate a pagination limit.
///
/// Rules: `MIN_PAGE_LIMIT..=MAX_PAGE_LIMIT`.
pub fn validate_limit(limit: usize) -> Result<(), ValidationError> {
    if limit < MIN_PAGE_LIMIT || limit > MAX_PAGE_LIMIT {
        return Err(ValidationError::LimitOutOfRange {
            min: MIN_PAGE_LIMIT,
            max: MAX_PAGE_LIMIT,
            actual: limit,
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_id_empty_is_rejected() {
        assert_eq!(validate_source_id(""), Err(ValidationError::SourceIdEmpty));
    }

    #[test]
    fn source_id_single_char_is_ok() {
        assert!(validate_source_id("a").is_ok());
    }

    #[test]
    fn source_id_max_length_is_ok() {
        let raw = "a".repeat(MAX_SOURCE_ID_LEN);
        assert!(validate_source_id(&raw).is_ok());
    }

    #[test]
    fn source_id_over_max_is_rejected() {
        let raw = "a".repeat(MAX_SOURCE_ID_LEN + 1);
        assert!(matches!(
            validate_source_id(&raw),
            Err(ValidationError::SourceIdTooLong { .. })
        ));
    }

    #[test]
    fn source_id_with_newline_is_rejected() {
        assert!(matches!(
            validate_source_id("abc\ndef"),
            Err(ValidationError::SourceIdInvalidChar { .. })
        ));
    }

    #[test]
    fn source_id_with_tab_is_rejected() {
        assert!(matches!(
            validate_source_id("abc\tdef"),
            Err(ValidationError::SourceIdInvalidChar { .. })
        ));
    }

    #[test]
    fn source_id_with_unicode_is_ok() {
        assert!(validate_source_id("тема-1").is_ok());
    }

    #[test]
    fn payload_small_is_ok() {
        let payload = serde_json::json!({"topic": "abc"});
        assert!(validate_payload_size(&payload).is_ok());
    }

    #[test]
    fn payload_16kb_is_rejected_when_over() {
        // Build a payload of >16 KiB serialized.
        let big = "a".repeat(MAX_PAYLOAD_SIZE);
        let payload = serde_json::json!({ "data": big });
        assert!(matches!(
            validate_payload_size(&payload),
            Err(ValidationError::PayloadTooLarge { .. })
        ));
    }

    #[test]
    fn limit_zero_is_rejected() {
        assert!(matches!(
            validate_limit(0),
            Err(ValidationError::LimitOutOfRange { .. })
        ));
    }

    #[test]
    fn limit_one_is_ok() {
        assert!(validate_limit(1).is_ok());
    }

    #[test]
    fn limit_max_is_ok() {
        assert!(validate_limit(MAX_PAGE_LIMIT).is_ok());
    }

    #[test]
    fn limit_over_max_is_rejected() {
        assert!(matches!(
            validate_limit(MAX_PAGE_LIMIT + 1),
            Err(ValidationError::LimitOutOfRange { .. })
        ));
    }
}
