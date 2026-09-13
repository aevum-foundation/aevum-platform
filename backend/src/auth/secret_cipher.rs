//! Aevum Platform — AUTH-25 Secret Cipher
//!
//! Provides encryption for application secrets (TOTP secrets).
//!
//! The cipher is a standalone dependency of AuthService, like
//! PasswordHasher and EmailProvider. It does NOT live inside AuthStorage.
//!
//! Implementations:
//! - TestSecretCipher (dev/test — passthrough, NEVER in production)
//! - AevumDbSecretCipher (production — via AevumDB Envelope + Blob domain)

use async_trait::async_trait;
use zeroize::Zeroizing;

use crate::error::ApiError;

/// Symmetric encryption for application secrets.
///
/// The cipher does not know about AUTH domain types (Uuid, User).
/// The caller provides a domain-separated `object_id` for each secret.
#[async_trait]
pub trait SecretCipher: Send + Sync {
    /// Encrypt plaintext for a given storage object.
    ///
    /// `object_id` must be deterministic and domain-separated:
    /// same logical secret must always map to the same object_id.
    async fn encrypt(&self, object_id: u64, plaintext: &[u8]) -> Result<Vec<u8>, ApiError>;

    /// Decrypt ciphertext previously produced by `encrypt`.
    ///
    /// The returned plaintext is zeroized on drop.
    async fn decrypt(&self, ciphertext: &[u8]) -> Result<Zeroizing<Vec<u8>>, ApiError>;
}

/// In-memory passthrough cipher for tests and local development.
///
/// SECURITY: This cipher does NOT provide confidentiality.
/// It MUST NEVER be used in production.
#[derive(Default)]
pub struct TestSecretCipher;

impl TestSecretCipher {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl SecretCipher for TestSecretCipher {
    async fn encrypt(&self, _object_id: u64, plaintext: &[u8]) -> Result<Vec<u8>, ApiError> {
        Ok(plaintext.to_vec())
    }

    async fn decrypt(&self, ciphertext: &[u8]) -> Result<Zeroizing<Vec<u8>>, ApiError> {
        Ok(Zeroizing::new(ciphertext.to_vec()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cipher_roundtrip() {
        let cipher = TestSecretCipher::new();
        let plaintext = b"my-secret-value";

        let ciphertext = cipher.encrypt(42, plaintext).await.unwrap();
        let decrypted = cipher.decrypt(&ciphertext).await.unwrap();

        assert_eq!(&decrypted[..], plaintext);
    }

    #[tokio::test]
    async fn test_cipher_is_passthrough() {
        let cipher = TestSecretCipher::new();
        let plaintext = b"visible-plaintext";

        let ciphertext = cipher.encrypt(0, plaintext).await.unwrap();
        assert_eq!(ciphertext, plaintext);
    }
}
