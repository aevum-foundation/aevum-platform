//! CSRF protection primitives.
//!
//! AUTH-13 — CSRF Protection
//!
//! The CSRF token is a cryptographically random value issued to the browser
//! after successful authentication. The browser must send it back in the
//! `X-CSRF-Token` header for every state-changing request.
//!
//! The token is stored in a non-HttpOnly cookie so the frontend can read it.
//! The cookie is bound to the current host and session lifetime.
//!
//! Full validation requires:
//! 1. cookie token == header token (constant-time)
//! 2. hash(cookie token) == session.csrf_token_hash (if session exists)

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use sha2::{Digest, Sha256};
use std::fmt;
use subtle::ConstantTimeEq;

use actix_web::cookie::{Cookie, SameSite};

/// CSRF cookie name.
pub const CSRF_COOKIE_NAME: &str = "__Host-aevum_csrf";

/// CSRF header name.
pub const CSRF_HEADER_NAME: &str = "X-CSRF-Token";

/// CSRF token size in random bytes.
pub const CSRF_TOKEN_BYTES: usize = 32;

/// CSRF cookie configuration.
#[derive(Debug, Clone)]
pub struct CsrfConfig {
    pub secure: bool,
}

impl Default for CsrfConfig {
    fn default() -> Self {
        Self { secure: true }
    }
}

/// Opaque cryptographically random CSRF token.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct CsrfToken(String);

impl CsrfToken {
    pub fn generate() -> Self {
        let mut bytes = [0u8; CSRF_TOKEN_BYTES];
        use argon2::password_hash::rand_core::RngCore;
        argon2::password_hash::rand_core::OsRng.fill_bytes(&mut bytes);
        Self(URL_SAFE_NO_PAD.encode(bytes))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for CsrfToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("CsrfToken(REDACTED)")
    }
}

/// SHA-256 hash of a CSRF token.
#[derive(Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct CsrfTokenHash(String);

impl CsrfTokenHash {
    pub fn from_token(token: &CsrfToken) -> Self {
        let digest = Sha256::digest(token.as_str().as_bytes());
        Self(URL_SAFE_NO_PAD.encode(digest))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for CsrfTokenHash {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("CsrfTokenHash(REDACTED)")
    }
}

/// Build a CSRF cookie that the browser can read (not HttpOnly).
pub fn build_csrf_cookie(token: &CsrfToken, config: &CsrfConfig) -> Cookie<'static> {
    Cookie::build(CSRF_COOKIE_NAME, token.as_str().to_owned())
        .http_only(false)
        .secure(config.secure)
        .same_site(SameSite::Lax)
        .path("/")
        .finish()
}

/// Build a removal cookie for the CSRF token.
pub fn build_csrf_removal_cookie(config: &CsrfConfig) -> Cookie<'static> {
    let mut cookie = Cookie::build(CSRF_COOKIE_NAME, "")
        .http_only(false)
        .secure(config.secure)
        .same_site(SameSite::Lax)
        .path("/")
        .finish();

    cookie.make_removal();
    cookie
}

/// Constant-time comparison of two CSRF tokens.
pub fn validate_csrf_tokens(cookie_token: &str, header_token: &str) -> bool {
    cookie_token
        .as_bytes()
        .ct_eq(header_token.as_bytes())
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csrf_tokens_are_random() {
        let token1 = CsrfToken::generate();
        let token2 = CsrfToken::generate();
        assert_ne!(token1.as_str(), token2.as_str());
        assert_eq!(token1.as_str().len(), 43);
    }

    #[test]
    fn csrf_token_debug_is_redacted() {
        let token = CsrfToken::generate();
        let debug = format!("{token:?}");
        assert!(!debug.contains(token.as_str()));
        assert!(debug.contains("REDACTED"));
    }

    #[test]
    fn csrf_hash_is_sha256_urlsafe() {
        let token = CsrfToken::generate();
        let hash = CsrfTokenHash::from_token(&token);
        assert_eq!(hash.as_str().len(), 43);
        assert_ne!(hash.as_str(), token.as_str());
    }

    #[test]
    fn constant_time_comparison_works() {
        assert!(validate_csrf_tokens("same-token", "same-token"));
        assert!(!validate_csrf_tokens("token-a", "token-b"));
        assert!(!validate_csrf_tokens("", "token"));
        assert!(!validate_csrf_tokens("token", ""));
    }
}
