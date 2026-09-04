//! Aevum Platform — password and session-token cryptographic primitives.
//!
//! Security model:
//! - Passwords are hashed with Argon2id.
//! - Every password receives a fresh random salt.
//! - Password hashes use the standard PHC string format.
//! - Raw session tokens are cryptographically random opaque values.
//! - Secret-bearing types redact their Debug representation.
//! - No authentication/business logic lives in this module.

use argon2::{
    password_hash::{
        rand_core::OsRng, PasswordHash, PasswordHasher as ArgonPasswordHasher, PasswordVerifier,
        SaltString,
    },
    Algorithm, Argon2, Params, Version,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use std::fmt;
use thiserror::Error;

/// Maximum password length accepted by this cryptographic layer.
pub const MAX_PASSWORD_BYTES: usize = 128;

/// Default Argon2id memory cost in KiB.
pub const ARGON2_MEMORY_COST_KIB: u32 = 19_456;

/// Default Argon2id time cost.
pub const ARGON2_TIME_COST: u32 = 2;

/// Default Argon2id parallelism.
pub const ARGON2_PARALLELISM: u32 = 1;

/// Session token size in random bytes.
pub const SESSION_TOKEN_BYTES: usize = 32;

/// Password cryptographic errors.
#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum PasswordError {
    #[error("Password is empty")]
    EmptyPassword,

    #[error("Password is too long")]
    PasswordTooLong,

    #[error("Invalid password hash")]
    InvalidHash,

    #[error("Password hashing failed")]
    HashFailed,

    #[error("Password verification failed")]
    VerificationFailed,

    #[error("Invalid Argon2 parameters")]
    InvalidParameters,
}

pub type PasswordResult<T> = Result<T, PasswordError>;

/// Argon2id password hashing service.
#[derive(Clone)]
pub struct PasswordHasher {
    argon2: Argon2<'static>,
}

impl fmt::Debug for PasswordHasher {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("PasswordHasher(REDACTED)")
    }
}

impl Default for PasswordHasher {
    fn default() -> Self {
        Self::new()
    }
}

impl PasswordHasher {
    pub fn new() -> Self {
        Self::from_params(ARGON2_MEMORY_COST_KIB, ARGON2_TIME_COST, ARGON2_PARALLELISM)
            .expect("canonical Argon2 parameters must be valid")
    }

    pub fn with_params(memory_cost: u32, time_cost: u32, parallelism: u32) -> PasswordResult<Self> {
        Self::from_params(memory_cost, time_cost, parallelism)
    }

    fn from_params(memory_cost: u32, time_cost: u32, parallelism: u32) -> PasswordResult<Self> {
        let params = Params::new(memory_cost, time_cost, parallelism, None)
            .map_err(|_| PasswordError::InvalidParameters)?;

        let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);

        Ok(Self { argon2 })
    }

    pub fn hash(&self, password: &str) -> PasswordResult<String> {
        self.validate_password(password)?;

        let salt = SaltString::generate(&mut OsRng);

        let hash = self
            .argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|_| PasswordError::HashFailed)?;

        Ok(hash.to_string())
    }

    pub fn verify(&self, password: &str, encoded_hash: &str) -> PasswordResult<bool> {
        if password.is_empty() || password.len() > MAX_PASSWORD_BYTES {
            return Ok(false);
        }

        let parsed_hash =
            PasswordHash::new(encoded_hash).map_err(|_| PasswordError::InvalidHash)?;

        match self
            .argon2
            .verify_password(password.as_bytes(), &parsed_hash)
        {
            Ok(()) => Ok(true),
            Err(argon2::password_hash::Error::Password) => Ok(false),
            Err(_) => Err(PasswordError::VerificationFailed),
        }
    }

    fn validate_password(&self, password: &str) -> PasswordResult<()> {
        if password.is_empty() {
            return Err(PasswordError::EmptyPassword);
        }

        if password.len() > MAX_PASSWORD_BYTES {
            return Err(PasswordError::PasswordTooLong);
        }

        Ok(())
    }
}

/// Opaque cryptographically random session token.
#[derive(Clone)]
pub struct SessionToken(String);

impl SessionToken {
    pub fn generate() -> Self {
        let mut bytes = [0u8; SESSION_TOKEN_BYTES];
        use argon2::password_hash::rand_core::RngCore;

        OsRng.fill_bytes(&mut bytes);

        Self(URL_SAFE_NO_PAD.encode(bytes))
    }

    pub(crate) fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SessionToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SessionToken(REDACTED)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_hash_verify_works() {
        let hasher = PasswordHasher::new();

        let password = "correct-horse-battery-staple";
        let hash = hasher.hash(password).unwrap();

        assert!(hash.starts_with("$argon2id$"));
        assert!(hasher.verify(password, &hash).unwrap());
    }

    #[test]
    fn wrong_password_fails() {
        let hasher = PasswordHasher::new();

        let hash = hasher.hash("correct-horse-battery-staple").unwrap();

        assert!(!hasher.verify("wrong-password", &hash).unwrap());
    }

    #[test]
    fn different_salts_produce_different_hashes() {
        let hasher = PasswordHasher::new();
        let password = "correct-horse-battery-staple";

        let hash1 = hasher.hash(password).unwrap();
        let hash2 = hasher.hash(password).unwrap();

        assert_ne!(hash1, hash2);
        assert!(hasher.verify(password, &hash1).unwrap());
        assert!(hasher.verify(password, &hash2).unwrap());
    }

    #[test]
    fn empty_password_is_rejected_for_hashing() {
        let hasher = PasswordHasher::new();

        assert_eq!(hasher.hash(""), Err(PasswordError::EmptyPassword));
    }

    #[test]
    fn empty_password_fails_verification() {
        let hasher = PasswordHasher::new();

        let hash = hasher.hash("valid-password").unwrap();

        assert!(!hasher.verify("", &hash).unwrap());
    }

    #[test]
    fn oversized_password_is_rejected() {
        let hasher = PasswordHasher::new();
        let password = "a".repeat(MAX_PASSWORD_BYTES + 1);

        assert_eq!(hasher.hash(&password), Err(PasswordError::PasswordTooLong));
    }

    #[test]
    fn invalid_hash_is_rejected() {
        let hasher = PasswordHasher::new();

        assert_eq!(
            hasher.verify("password", "not-a-valid-phc-hash"),
            Err(PasswordError::InvalidHash)
        );
    }

    #[test]
    fn custom_parameters_work() {
        let hasher = PasswordHasher::with_params(4_096, 1, 1).unwrap();

        let hash = hasher.hash("test-password").unwrap();

        assert!(hasher.verify("test-password", &hash).unwrap());
    }

    #[test]
    fn session_tokens_are_random() {
        let token1 = SessionToken::generate();
        let token2 = SessionToken::generate();

        assert_ne!(token1.expose(), token2.expose());
        assert_eq!(token1.expose().len(), 43);
    }

    #[test]
    fn secret_debug_output_is_redacted() {
        let token = SessionToken::generate();

        let token_debug = format!("{token:?}");

        assert!(!token_debug.contains(token.expose()));
        assert!(token_debug.contains("REDACTED"));
    }

    #[test]
    fn password_hasher_debug_is_redacted() {
        let hasher = PasswordHasher::new();
        let debug = format!("{hasher:?}");

        assert_eq!(debug, "PasswordHasher(REDACTED)");
    }
}

impl SessionToken {
    pub(crate) fn from_secret(secret: String) -> Self {
        Self(secret)
    }
}
