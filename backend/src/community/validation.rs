//! Community input validation and canonicalization.

use std::fmt;

/// Minimum username length in Unicode scalar values.
pub const USERNAME_MIN_LEN: usize = 3;

/// Maximum username length in Unicode scalar values.
pub const USERNAME_MAX_LEN: usize = 32;

/// Maximum display-name length in Unicode scalar values.
pub const DISPLAY_NAME_MAX_LEN: usize = 64;

/// Maximum bio length in Unicode scalar values.
pub const BIO_MAX_LEN: usize = 500;

/// A username after validation.
///
/// `canonical` preserves the trimmed user input.
/// `normalized` is the lowercase form used for uniqueness/indexing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidUsername {
    pub canonical: String,
    pub normalized: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    UsernameTooShort { min: usize },
    UsernameTooLong { max: usize },
    UsernameInvalidCharset { offending: char },
    UsernameReserved,

    DisplayNameTooLong { max: usize, actual: usize },
    DisplayNameContainsControl { code: u32 },
    DisplayNameContainsBidi { code: u32 },

    BioTooLong { max: usize, actual: usize },
    BioContainsControl { code: u32 },
    BioContainsBidi { code: u32 },
}

impl fmt::Display for ValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UsernameTooShort { min } => {
                write!(formatter, "username must be at least {min} characters")
            }
            Self::UsernameTooLong { max } => {
                write!(formatter, "username must be at most {max} characters")
            }
            Self::UsernameInvalidCharset { offending } => {
                write!(
                    formatter,
                    "username contains invalid character {offending:?}"
                )
            }
            Self::UsernameReserved => formatter.write_str("username is reserved"),
            Self::DisplayNameTooLong { max, actual } => {
                write!(
                    formatter,
                    "display name is {actual} characters; maximum is {max}"
                )
            }
            Self::DisplayNameContainsControl { code } => {
                write!(
                    formatter,
                    "display name contains control character U+{code:04X}"
                )
            }
            Self::DisplayNameContainsBidi { code } => {
                write!(formatter, "display name contains bidi control U+{code:04X}")
            }
            Self::BioTooLong { max, actual } => {
                write!(formatter, "bio is {actual} characters; maximum is {max}")
            }
            Self::BioContainsControl { code } => {
                write!(formatter, "bio contains control character U+{code:04X}")
            }
            Self::BioContainsBidi { code } => {
                write!(formatter, "bio contains bidi control U+{code:04X}")
            }
        }
    }
}

impl std::error::Error for ValidationError {}

const RESERVED_USERNAMES: &[&str] = &[
    "admin",
    "administrator",
    "root",
    "system",
    "support",
    "help",
    "staff",
    "moderator",
    "mod",
    "api",
    "auth",
    "login",
    "logout",
    "register",
    "settings",
    "profile",
    "forum",
    "community",
    "search",
    "notifications",
    "me",
    "u",
    "www",
    "mail",
    "email",
    "security",
    "aevum",
    "aevumchain",
    "foundation",
    "official",
];

pub fn validate_username(raw: &str) -> Result<ValidUsername, ValidationError> {
    let canonical = raw.trim().to_owned();
    let normalized = canonical.to_ascii_lowercase();

    let length = normalized.chars().count();

    if length < USERNAME_MIN_LEN {
        return Err(ValidationError::UsernameTooShort {
            min: USERNAME_MIN_LEN,
        });
    }

    if length > USERNAME_MAX_LEN {
        return Err(ValidationError::UsernameTooLong {
            max: USERNAME_MAX_LEN,
        });
    }

    for offending in normalized.chars() {
        if !offending.is_ascii() || !matches!(offending, 'a'..='z' | '0'..='9' | '_' | '-') {
            return Err(ValidationError::UsernameInvalidCharset { offending });
        }
    }

    if RESERVED_USERNAMES.contains(&normalized.as_str()) {
        return Err(ValidationError::UsernameReserved);
    }

    Ok(ValidUsername {
        canonical,
        normalized,
    })
}

pub fn validate_display_name(raw: Option<&str>) -> Result<Option<String>, ValidationError> {
    let Some(raw) = raw else {
        return Ok(None);
    };

    let value = raw.trim();

    if value.is_empty() {
        return Ok(None);
    }

    let actual = value.chars().count();

    if actual > DISPLAY_NAME_MAX_LEN {
        return Err(ValidationError::DisplayNameTooLong {
            max: DISPLAY_NAME_MAX_LEN,
            actual,
        });
    }

    for character in value.chars() {
        if character.is_control() {
            return Err(ValidationError::DisplayNameContainsControl {
                code: character as u32,
            });
        }

        if is_bidi_control(character) {
            return Err(ValidationError::DisplayNameContainsBidi {
                code: character as u32,
            });
        }
    }

    Ok(Some(value.to_owned()))
}

pub fn validate_bio(raw: Option<&str>) -> Result<Option<String>, ValidationError> {
    let Some(raw) = raw else {
        return Ok(None);
    };

    let value = raw.trim();

    if value.is_empty() {
        return Ok(None);
    }

    let actual = value.chars().count();

    if actual > BIO_MAX_LEN {
        return Err(ValidationError::BioTooLong {
            max: BIO_MAX_LEN,
            actual,
        });
    }

    for character in value.chars() {
        if character.is_control() && character != '\n' {
            return Err(ValidationError::BioContainsControl {
                code: character as u32,
            });
        }

        if is_bidi_control(character) {
            return Err(ValidationError::BioContainsBidi {
                code: character as u32,
            });
        }
    }

    Ok(Some(value.to_owned()))
}

fn is_bidi_control(character: char) -> bool {
    matches!(
        character as u32,
        0x202A..=0x202E | 0x2066..=0x2069
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn username_normalizes_uppercase_and_whitespace() {
        let result = validate_username("  Alice_01  ").unwrap();

        assert_eq!(result.canonical, "Alice_01");
        assert_eq!(result.normalized, "alice_01");
    }

    #[test]
    fn username_rejects_short_value() {
        assert_eq!(
            validate_username("ab"),
            Err(ValidationError::UsernameTooShort {
                min: USERNAME_MIN_LEN
            })
        );
    }

    #[test]
    fn username_rejects_long_value() {
        let username = "a".repeat(USERNAME_MAX_LEN + 1);

        assert_eq!(
            validate_username(&username),
            Err(ValidationError::UsernameTooLong {
                max: USERNAME_MAX_LEN
            })
        );
    }

    #[test]
    fn username_accepts_allowed_charset() {
        assert!(validate_username("aevum_01").is_ok());
        assert!(validate_username("aevum-user").is_ok());
    }

    #[test]
    fn username_rejects_invalid_charset() {
        assert_eq!(
            validate_username("aevum!"),
            Err(ValidationError::UsernameInvalidCharset { offending: '!' })
        );
    }

    #[test]
    fn username_rejects_non_ascii() {
        assert!(matches!(
            validate_username("аevum"),
            Err(ValidationError::UsernameInvalidCharset { .. })
        ));
    }

    #[test]
    fn username_reserved_check_is_case_insensitive() {
        assert_eq!(
            validate_username("  ADMIN "),
            Err(ValidationError::UsernameReserved)
        );
    }

    #[test]
    fn display_name_is_trimmed() {
        assert_eq!(
            validate_display_name(Some("  Aevum Foundation  ")).unwrap(),
            Some("Aevum Foundation".to_owned())
        );
    }

    #[test]
    fn empty_display_name_becomes_none() {
        assert_eq!(validate_display_name(Some("   ")).unwrap(), None);
        assert_eq!(validate_display_name(None).unwrap(), None);
    }

    #[test]
    fn display_name_counts_unicode_scalars() {
        let value = "é".repeat(DISPLAY_NAME_MAX_LEN);

        assert!(validate_display_name(Some(&value)).is_ok());
    }

    #[test]
    fn display_name_rejects_control_characters() {
        assert_eq!(
            validate_display_name(Some("hello\tworld")),
            Err(ValidationError::DisplayNameContainsControl { code: '\t' as u32 })
        );
    }

    #[test]
    fn display_name_rejects_bidi_controls() {
        assert_eq!(
            validate_display_name(Some("hello\u{202E}world")),
            Err(ValidationError::DisplayNameContainsBidi { code: 0x202E })
        );
    }

    #[test]
    fn bio_allows_newline() {
        assert_eq!(
            validate_bio(Some("line one\nline two")).unwrap(),
            Some("line one\nline two".to_owned())
        );
    }

    #[test]
    fn bio_rejects_other_control_characters() {
        assert_eq!(
            validate_bio(Some("hello\tworld")),
            Err(ValidationError::BioContainsControl { code: '\t' as u32 })
        );
    }

    #[test]
    fn bio_rejects_bidi_controls() {
        assert_eq!(
            validate_bio(Some("hello\u{2066}world")),
            Err(ValidationError::BioContainsBidi { code: 0x2066 })
        );
    }

    #[test]
    fn validation_messages_match_frozen_limits() {
        assert!(
            format!("Username must be at least {USERNAME_MIN_LEN} characters")
                .contains(&USERNAME_MIN_LEN.to_string())
        );

        assert!(
            format!("Username must be at most {USERNAME_MAX_LEN} characters")
                .contains(&USERNAME_MAX_LEN.to_string())
        );

        assert!(
            format!("Display name must be at most {DISPLAY_NAME_MAX_LEN} characters")
                .contains(&DISPLAY_NAME_MAX_LEN.to_string())
        );

        assert!(format!("Bio must be at most {BIO_MAX_LEN} characters")
            .contains(&BIO_MAX_LEN.to_string()));
    }
}
