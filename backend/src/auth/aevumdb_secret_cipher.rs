//! Aevum Platform — AUTH-25 AevumDB Secret Cipher
//!
//! Production SecretCipher backed by AevumDB's Envelope primitive.
//!
//! Uses CryptoDomain::Blob — a generic encrypted payload domain —
//! and does NOT introduce new CryptoDomain variants into AevumDB.
//!
//! Storage format: SealedEnvelope serialized as JSON.

use std::sync::Arc;

use async_trait::async_trait;
use zeroize::Zeroizing;

use aevum_db::crypto::envelope::SealedEnvelope;
use aevum_db::crypto::keys::KeyManager;
use aevum_db::{CryptoDomain, DbRuntime, Envelope};

use crate::auth::secret_cipher::SecretCipher;
use crate::error::ApiError;

/// Production SecretCipher using AevumDB Envelope.
pub struct AevumDbSecretCipher {
    envelope: Arc<Envelope>,
    key_manager: Arc<KeyManager>,
}

impl AevumDbSecretCipher {
    /// Build cipher from a DbRuntime.
    ///
    /// Fails if the runtime does not carry an envelope —
    /// production must not silently fall back to plaintext.
    pub fn from_runtime(runtime: &DbRuntime) -> Result<Self, ApiError> {
        let envelope = runtime
            .envelope()
            .ok_or_else(|| {
                tracing::error!("AevumDbSecretCipher: runtime has no envelope");
                ApiError::Internal
            })?
            .clone();

        let key_manager = runtime
            .key_manager()
            .ok_or_else(|| {
                tracing::error!("AevumDbSecretCipher: runtime has no key manager");
                ApiError::Internal
            })?
            .clone();

        Ok(Self {
            envelope,
            key_manager,
        })
    }
}

#[async_trait]
impl SecretCipher for AevumDbSecretCipher {
    async fn encrypt(&self, object_id: u64, plaintext: &[u8]) -> Result<Vec<u8>, ApiError> {
        let key_epoch = self.key_manager.current_epoch();

        let sealed = self
            .envelope
            .seal(plaintext, CryptoDomain::Blob, object_id, key_epoch)
            .map_err(|error| {
                tracing::error!("AevumDB seal failed: {}", error);
                ApiError::Internal
            })?;

        Ok(sealed.to_bytes())
    }

    async fn decrypt(&self, ciphertext: &[u8]) -> Result<Zeroizing<Vec<u8>>, ApiError> {
        let sealed = SealedEnvelope::from_bytes(ciphertext).map_err(|error| {
            tracing::error!("SealedEnvelope deserialize failed: {}", error);
            ApiError::Internal
        })?;

        self.envelope.open(&sealed).map_err(|error| {
            tracing::error!("AevumDB open failed: {}", error);
            ApiError::Internal
        })
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use aevum_db::crypto::context::CryptoContext;
    use aevum_db::crypto::envelope::Envelope;
    use aevum_db::crypto::kdf::HkdfSha256Kdf;
    use aevum_db::crypto::suite::CryptoSuiteId;
    use aevum_db::config::{DbConfig, DbRuntime};
    use zeroize::Zeroizing;

    fn test_runtime() -> DbRuntime {
        let master_key = Zeroizing::new([0x42u8; 32]);
        let envelope = Envelope::new(
            0x0001,
            &master_key,
            &HkdfSha256Kdf,
        )
        .unwrap();
        let crypto_context = CryptoContext::new(
            CryptoSuiteId::V1,
            Zeroizing::new([0x42u8; 32]),
        )
        .unwrap();

        DbRuntime::encrypted(
            envelope,
            crypto_context,
            CryptoSuiteId::V1,
            1,
        )
    }

    #[tokio::test]
    async fn roundtrip_through_real_envelope() {
        let runtime = test_runtime();
        let cipher = AevumDbSecretCipher::from_runtime(&runtime).unwrap();

        let secret = b"JBSWY3DPEHPK3PXP";
        let object_id = 42u64;

        let ciphertext = cipher.encrypt(object_id, secret).await.unwrap();

        // Ciphertext must not contain plaintext
        assert!(!ciphertext.windows(secret.len()).any(|w| w == secret));
        assert_ne!(ciphertext.as_slice(), secret.as_slice());

        let decrypted = cipher.decrypt(&ciphertext).await.unwrap();
        assert_eq!(&decrypted[..], secret);
    }

    #[tokio::test]
    async fn different_object_ids_produce_different_ciphertexts() {
        let runtime = test_runtime();
        let cipher = AevumDbSecretCipher::from_runtime(&runtime).unwrap();

        let secret = b"same-secret";
        let ct_a = cipher.encrypt(1, secret).await.unwrap();
        let ct_b = cipher.encrypt(2, secret).await.unwrap();

        // Both decrypt correctly, but ciphertexts differ (AEAD context binds object_id)
        assert_ne!(ct_a, ct_b);
        assert_eq!(&cipher.decrypt(&ct_a).await.unwrap()[..], secret);
        assert_eq!(&cipher.decrypt(&ct_b).await.unwrap()[..], secret);
    }

    #[test]
    fn from_plaintext_runtime_fails() {
        let runtime = DbRuntime::plaintext();
        assert!(AevumDbSecretCipher::from_runtime(&runtime).is_err());
    }
}
