//! Aevum Platform — AUTH-27 Avatar
//!
//! User avatar metadata and payload validation.
//!
//! Storage model:
//! - metadata (mime, size, hash, blob_key) → AuthStorage
//! - encrypted payload → AevumDB KV via SecretCipher (CryptoDomain::Blob)
//!
//! Validation is strict: we never trust the client-supplied MIME type,
//! and we verify magic bytes + decode the image before accepting it.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Maximum allowed avatar payload size in bytes (2 MiB).
pub const AVATAR_MAX_BYTES: usize = 2 * 1024 * 1024;

/// Maximum allowed image dimension (width or height) in pixels.
pub const AVATAR_MAX_DIMENSION: u32 = 4096;

/// Supported avatar MIME types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AvatarMime {
    Jpeg,
    Png,
    Webp,
}

impl AvatarMime {
    /// HTTP Content-Type string for this MIME type.
    pub const fn as_str(self) -> &'static str {
        match self {
            AvatarMime::Jpeg => "image/jpeg",
            AvatarMime::Png => "image/png",
            AvatarMime::Webp => "image/webp",
        }
    }

    /// Parse from an HTTP Content-Type value.
    ///
    /// Strict: only exact matches allowed. Parameters such as
    /// `image/jpeg; charset=utf-8` are rejected.
    pub fn from_content_type(value: &str) -> Option<Self> {
        match value {
            "image/jpeg" => Some(AvatarMime::Jpeg),
            "image/png" => Some(AvatarMime::Png),
            "image/webp" => Some(AvatarMime::Webp),
            _ => None,
        }
    }

    /// Detect MIME from magic bytes.
    ///
    /// This is the authoritative check — the client-supplied
    /// Content-Type header is NOT trusted.
    pub fn detect_from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < 12 {
            return None;
        }

        // JPEG: FF D8 FF
        if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
            return Some(AvatarMime::Jpeg);
        }

        // PNG: 89 50 4E 47 0D 0A 1A 0A
        if data.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
            return Some(AvatarMime::Png);
        }

        // WebP: "RIFF" ... "WEBP"
        if data.starts_with(b"RIFF") && data.len() >= 12 && &data[8..12] == b"WEBP" {
            return Some(AvatarMime::Webp);
        }

        None
    }
}

/// Persisted avatar metadata.
///
/// `blob_key` is a domain-separated storage key derived from user_id.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Avatar {
    pub user_id: Uuid,
    pub mime_type: AvatarMime,
    pub size: u64,
    pub content_hash: [u8; 32],
    pub blob_key: String,
    pub uploaded_at: DateTime<Utc>,
}

impl Avatar {
    /// Deterministic blob storage key for a user's avatar.
    ///
    /// Domain-separated: same user always maps to the same key, so
    /// replacing an avatar overwrites the same slot atomically.
    pub fn blob_key_for(user_id: &Uuid) -> String {
        let mut hasher = Sha256::new();
        hasher.update(b"AEVUM_AUTH_AVATAR_BLOB_V1");
        hasher.update(user_id.as_bytes());
        let digest = hasher.finalize();
        format!("platform:auth:avatar_blob:{}", hex::encode(digest))
    }

    /// SHA-256 hash of the payload.
    pub fn hash_bytes(data: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hasher.finalize().into()
    }
}

/// Validation errors for avatar upload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AvatarValidationError {
    /// Payload is empty.
    EmptyPayload,
    /// Payload exceeds AVATAR_MAX_BYTES.
    TooLarge,
    /// Content-Type is not in the allow-list.
    UnsupportedContentType,
    /// Magic bytes do not match a supported format.
    UnsupportedFormat,
    /// Client Content-Type does not match the detected format.
    ContentTypeMismatch,
    /// Image decoding failed or dimensions exceeded limits.
    InvalidImage,
}

/// Validate an uploaded avatar payload.
///
/// Steps:
/// 1. Reject empty payload.
/// 2. Reject payload > AVATAR_MAX_BYTES.
/// 3. Parse declared Content-Type against allow-list.
/// 4. Detect actual format from magic bytes.
/// 5. Ensure declared and detected MIME match.
/// 6. Decode image and enforce dimension limits.
pub fn validate_avatar(
    declared_content_type: &str,
    data: &[u8],
) -> Result<AvatarMime, AvatarValidationError> {
    if data.is_empty() {
        return Err(AvatarValidationError::EmptyPayload);
    }

    if data.len() > AVATAR_MAX_BYTES {
        return Err(AvatarValidationError::TooLarge);
    }

    let declared = AvatarMime::from_content_type(declared_content_type)
        .ok_or(AvatarValidationError::UnsupportedContentType)?;

    let detected = AvatarMime::detect_from_bytes(data)
        .ok_or(AvatarValidationError::UnsupportedFormat)?;

    if declared != detected {
        return Err(AvatarValidationError::ContentTypeMismatch);
    }

    validate_dimensions(detected, data)?;

    Ok(detected)
}

/// Decode image header and enforce dimension limits.
fn validate_dimensions(
    mime: AvatarMime,
    data: &[u8],
) -> Result<(), AvatarValidationError> {
    let (width, height) = match mime {
        AvatarMime::Jpeg => jpeg_dimensions(data),
        AvatarMime::Png => png_dimensions(data),
        AvatarMime::Webp => webp_dimensions(data),
    }
    .ok_or(AvatarValidationError::InvalidImage)?;

    if width == 0 || height == 0 {
        return Err(AvatarValidationError::InvalidImage);
    }

    if width > AVATAR_MAX_DIMENSION || height > AVATAR_MAX_DIMENSION {
        return Err(AvatarValidationError::InvalidImage);
    }

    Ok(())
}

fn png_dimensions(data: &[u8]) -> Option<(u32, u32)> {
    // PNG IHDR: bytes 16..20 = width (big-endian), 20..24 = height
    if data.len() < 24 {
        return None;
    }
    if &data[12..16] != b"IHDR" {
        return None;
    }
    let width = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
    let height = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);
    Some((width, height))
}

fn jpeg_dimensions(data: &[u8]) -> Option<(u32, u32)> {
    // Parse JPEG SOF markers to extract dimensions.
    let mut i = 2usize;

    while i + 9 < data.len() {
        if data[i] != 0xFF {
            i += 1;
            continue;
        }

        let marker = data[i + 1];

        // SOF0..SOF15 (excluding DHT=0xC4, JPG=0xC8, DAC=0xCC)
        let is_sof = (0xC0..=0xCF).contains(&marker)
            && marker != 0xC4
            && marker != 0xC8
            && marker != 0xCC;

        if is_sof {
            if i + 9 >= data.len() {
                return None;
            }
            let height = u16::from_be_bytes([data[i + 5], data[i + 6]]) as u32;
            let width = u16::from_be_bytes([data[i + 7], data[i + 8]]) as u32;
            return Some((width, height));
        }

        // Skip this segment
        if i + 4 >= data.len() {
            return None;
        }
        let segment_length = u16::from_be_bytes([data[i + 2], data[i + 3]]) as usize;
        if segment_length < 2 {
            return None;
        }
        i += 2 + segment_length;
    }

    None
}

fn webp_dimensions(data: &[u8]) -> Option<(u32, u32)> {
    if data.len() < 30 {
        return None;
    }

    // VP8 (lossy)
    if &data[12..16] == b"VP8 " {
        // Frame header starts at 20, dimensions at 26..30
        if data.len() < 30 {
            return None;
        }
        let w = u16::from_le_bytes([data[26], data[27]]) as u32 & 0x3FFF;
        let h = u16::from_le_bytes([data[28], data[29]]) as u32 & 0x3FFF;
        return Some((w, h));
    }

    // VP8L (lossless)
    if &data[12..16] == b"VP8L" {
        if data.len() < 25 {
            return None;
        }
        let b = &data[21..25];
        let bits = u32::from_le_bytes([b[0], b[1], b[2], b[3]]);
        let w = (bits & 0x3FFF) + 1;
        let h = ((bits >> 14) & 0x3FFF) + 1;
        return Some((w, h));
    }

    // VP8X (extended)
    if &data[12..16] == b"VP8X" {
        if data.len() < 30 {
            return None;
        }
        let w = u32::from_le_bytes([data[24], data[25], data[26], 0]) + 1;
        let h = u32::from_le_bytes([data[27], data[28], data[29], 0]) + 1;
        return Some((w, h));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_png(width: u32, height: u32) -> Vec<u8> {
        let mut data = vec![
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
            0x00, 0x00, 0x00, 0x0D, // IHDR length
            b'I', b'H', b'D', b'R',
        ];
        data.extend_from_slice(&width.to_be_bytes());
        data.extend_from_slice(&height.to_be_bytes());
        data.extend_from_slice(&[0x08, 0x02, 0x00, 0x00, 0x00]);
        data
    }

    fn minimal_jpeg(width: u16, height: u16) -> Vec<u8> {
        vec![
            0xFF, 0xD8, // SOI
            0xFF, 0xC0, // SOF0
            0x00, 0x11, // length
            0x08, // precision
            (height >> 8) as u8, (height & 0xFF) as u8,
            (width >> 8) as u8, (width & 0xFF) as u8,
            0x03,
            0x01, 0x11, 0x00,
            0x02, 0x11, 0x01,
            0x03, 0x11, 0x01,
        ]
    }

    fn minimal_webp_vp8(width: u16, height: u16) -> Vec<u8> {
        let mut data = vec![
            b'R', b'I', b'F', b'F',
            0x00, 0x00, 0x00, 0x00, // size (ignored)
            b'W', b'E', b'B', b'P',
            b'V', b'P', b'8', b' ',
            0x00, 0x00, 0x00, 0x00, // chunk size
            0x00, 0x00, 0x00, // frame tag
            0x9D, 0x01, 0x2A, // start code
            (width & 0xFF) as u8, (width >> 8) as u8,
            (height & 0xFF) as u8, (height >> 8) as u8,
        ];
        data.resize(30, 0);
        data
    }

    #[test]
    fn detects_png_from_magic() {
        let data = minimal_png(100, 100);
        assert_eq!(AvatarMime::detect_from_bytes(&data), Some(AvatarMime::Png));
    }

    #[test]
    fn detects_jpeg_from_magic() {
        let data = minimal_jpeg(100, 100);
        assert_eq!(AvatarMime::detect_from_bytes(&data), Some(AvatarMime::Jpeg));
    }

    #[test]
    fn detects_webp_from_magic() {
        let data = minimal_webp_vp8(100, 100);
        assert_eq!(AvatarMime::detect_from_bytes(&data), Some(AvatarMime::Webp));
    }

    #[test]
    fn rejects_empty_payload() {
        let result = validate_avatar("image/png", &[]);
        assert_eq!(result, Err(AvatarValidationError::EmptyPayload));
    }

    #[test]
    fn rejects_oversized_payload() {
        let data = vec![0u8; AVATAR_MAX_BYTES + 1];
        let result = validate_avatar("image/png", &data);
        assert_eq!(result, Err(AvatarValidationError::TooLarge));
    }

    #[test]
    fn rejects_svg_content_type() {
        let result = validate_avatar("image/svg+xml", b"<svg/>");
        assert_eq!(result, Err(AvatarValidationError::UnsupportedContentType));
    }

    #[test]
    fn rejects_content_type_mismatch() {
        let png_data = minimal_png(100, 100);
        let result = validate_avatar("image/jpeg", &png_data);
        assert_eq!(result, Err(AvatarValidationError::ContentTypeMismatch));
    }

    #[test]
    fn rejects_unknown_magic_bytes() {
        let result = validate_avatar("image/png", b"not an image at all");
        assert_eq!(result, Err(AvatarValidationError::UnsupportedFormat));
    }

    #[test]
    fn accepts_valid_png() {
        let data = minimal_png(256, 256);
        let result = validate_avatar("image/png", &data);
        assert_eq!(result, Ok(AvatarMime::Png));
    }

    #[test]
    fn accepts_valid_jpeg() {
        let data = minimal_jpeg(256, 256);
        let result = validate_avatar("image/jpeg", &data);
        assert_eq!(result, Ok(AvatarMime::Jpeg));
    }

    #[test]
    fn accepts_valid_webp() {
        let data = minimal_webp_vp8(256, 256);
        let result = validate_avatar("image/webp", &data);
        assert_eq!(result, Ok(AvatarMime::Webp));
    }

    #[test]
    fn rejects_png_with_excessive_dimensions() {
        let data = minimal_png(5000, 100);
        let result = validate_avatar("image/png", &data);
        assert_eq!(result, Err(AvatarValidationError::InvalidImage));
    }

    #[test]
    fn blob_key_is_deterministic() {
        let user_id = Uuid::new_v4();
        let k1 = Avatar::blob_key_for(&user_id);
        let k2 = Avatar::blob_key_for(&user_id);
        assert_eq!(k1, k2);
        assert!(k1.starts_with("platform:auth:avatar_blob:"));
    }

    #[test]
    fn blob_key_differs_per_user() {
        let u1 = Uuid::new_v4();
        let u2 = Uuid::new_v4();
        assert_ne!(Avatar::blob_key_for(&u1), Avatar::blob_key_for(&u2));
    }
}
