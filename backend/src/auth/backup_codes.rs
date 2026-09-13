//! Aevum Platform — AUTH-24 Backup Codes
//!
//! Secure backup-code generation and normalization.
//!
//! Format:
//!     ABCDE-FGHJK
//!
//! Properties:
//! - 10 payload characters
//! - 5 + 5 grouping
//! - uppercase
//! - human-readable alphabet
//! - excludes ambiguous characters: O, 0, I, 1, L
//! - generated exclusively from OS CSPRNG
//! - no plaintext persistence or logging
//!
//! Hashing is intentionally NOT performed in this module.
//! Argon2id hashing belongs to the auth service/password-hasher layer.

use rand::{rngs::OsRng, Rng};

use crate::auth::contracts::{
    BACKUP_CODE_GROUP_SIZE,
    BACKUP_CODE_GROUPS,
    BACKUP_CODES_PER_SET,
};

/// Human-readable backup-code alphabet.
///
/// Excluded ambiguous characters:
/// - O
/// - 0
/// - I
/// - 1
/// - L
///
/// This currently contains 31 symbols.
const ALPHABET: &[u8] = b"ABCDEFGHJKMNPQRSTUVWXYZ23456789";

/// Number of characters in the normalized backup code.
pub const BACKUP_CODE_PAYLOAD_LENGTH: usize =
    BACKUP_CODE_GROUP_SIZE * BACKUP_CODE_GROUPS;

/// Number of separators in the formatted representation.
pub const BACKUP_CODE_SEPARATOR_COUNT: usize =
    BACKUP_CODE_GROUPS.saturating_sub(1);

/// Length of the human-readable representation.
pub const BACKUP_CODE_FORMATTED_LENGTH: usize =
    BACKUP_CODE_PAYLOAD_LENGTH + BACKUP_CODE_SEPARATOR_COUNT;

/// Generate one cryptographically secure backup code.
pub fn generate_code() -> String {
    let mut rng = OsRng;

    let mut code = String::with_capacity(BACKUP_CODE_FORMATTED_LENGTH);

    for position in 0..BACKUP_CODE_PAYLOAD_LENGTH {
        if position > 0 && position % BACKUP_CODE_GROUP_SIZE == 0 {
            code.push('-');
        }

        let index = rng.gen_range(0..ALPHABET.len());
        code.push(ALPHABET[index] as char);
    }

    code
}

/// Generate a complete backup-code set.
pub fn generate_set() -> Vec<String> {
    (0..BACKUP_CODES_PER_SET)
        .map(|_| generate_code())
        .collect()
}

/// Normalize a user-provided backup code.
pub fn normalize_code(code: &str) -> String {
    code.chars()
        .filter(|c| !c.is_ascii_whitespace() && *c != '-')
        .map(|c| {
            if c.is_ascii_lowercase() {
                c.to_ascii_uppercase()
            } else {
                c
            }
        })
        .collect()
}

/// Validate a normalized backup code.
pub fn is_valid_normalized_code(code: &str) -> bool {
    if code.len() != BACKUP_CODE_PAYLOAD_LENGTH {
        return false;
    }

    code.bytes().all(|byte| ALPHABET.contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_code_has_correct_length() {
        let code = generate_code();
        assert_eq!(code.len(), BACKUP_CODE_FORMATTED_LENGTH);
    }

    #[test]
    fn generated_code_has_correct_separator() {
        let code = generate_code();
        assert_eq!(
            code.chars().filter(|c| *c == '-').count(),
            BACKUP_CODE_SEPARATOR_COUNT
        );
        assert_eq!(code.chars().nth(BACKUP_CODE_GROUP_SIZE), Some('-'));
    }

    #[test]
    fn generated_code_uses_only_safe_alphabet() {
        let code = generate_code();
        for byte in code.bytes() {
            if byte == b'-' {
                continue;
            }
            assert!(ALPHABET.contains(&byte));
            assert!(!matches!(byte, b'O' | b'0' | b'I' | b'1' | b'L'));
        }
    }

    #[test]
    fn generated_code_is_valid_after_normalization() {
        let code = generate_code();
        let normalized = normalize_code(&code);
        assert_eq!(normalized.len(), BACKUP_CODE_PAYLOAD_LENGTH);
        assert!(is_valid_normalized_code(&normalized));
    }

    #[test]
    fn generated_set_has_correct_count() {
        let set = generate_set();
        assert_eq!(set.len(), BACKUP_CODES_PER_SET);
    }

    #[test]
    fn generated_set_has_unique_codes() {
        let set = generate_set();
        let unique_count = set.iter().collect::<std::collections::HashSet<_>>().len();
        assert_eq!(unique_count, set.len());
    }

    #[test]
    fn normalize_lowercase() {
        assert_eq!(normalize_code("abcde-fghjk"), "ABCDEFGHJK");
    }

    #[test]
    fn normalize_removes_separator() {
        assert_eq!(normalize_code("ABCDE-FGHJK"), "ABCDEFGHJK");
    }

    #[test]
    fn normalize_removes_ascii_whitespace() {
        assert_eq!(normalize_code(" ABCDE - FGHJK "), "ABCDEFGHJK");
    }

    #[test]
    fn normalize_does_not_hide_invalid_punctuation() {
        assert_eq!(normalize_code("ABCDE!FGHJK"), "ABCDE!FGHJK");
        assert!(!is_valid_normalized_code(&normalize_code("ABCDE!FGHJK")));
    }

    #[test]
    fn normalize_does_not_hide_invalid_unicode() {
        // Dash is removed by normalization; non-ASCII remains and is rejected.
        assert_eq!(normalize_code("ABCDE-ФGHJK"), "ABCDEФGHJK");
        assert!(!is_valid_normalized_code(&normalize_code("ABCDE-ФGHJK")));
    }

    #[test]
    fn invalid_length_is_rejected() {
        // 10 chars is the correct payload length — this MUST pass.
        assert!(is_valid_normalized_code("ABCDEFGHJK"));
        // 9 chars is too short.
        assert!(!is_valid_normalized_code("ABCDEFGHJ"));
        // 11 chars is too long.
        assert!(!is_valid_normalized_code("ABCDEFGHJKM"));
        // Empty is rejected.
        assert!(!is_valid_normalized_code(""));
    }

    #[test]
    fn ambiguous_characters_are_rejected() {
        assert!(!is_valid_normalized_code("ABCDE0FGHJK"));
        assert!(!is_valid_normalized_code("ABCDEIFGHJK"));
        assert!(!is_valid_normalized_code("ABCDE1FGHJK"));
        assert!(!is_valid_normalized_code("ABCDELGHJK"));
        assert!(!is_valid_normalized_code("ABCDEOFGHJK"));
    }
}
