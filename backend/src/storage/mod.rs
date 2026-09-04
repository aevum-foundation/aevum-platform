//! Aevum Platform API — storage abstraction layer.
//!
//! The platform layer depends only on this abstraction.
//! Concrete storage engines such as AevumDB must remain behind
//! this boundary.
//!
//! Rules:
//! - Platform code must not depend on AevumDB implementation details.
//! - Storage failures must not leak internal implementation details.
//! - The abstraction must remain object-safe for use with `dyn Storage`.
//! - Health/readiness are intentionally separate concepts.
//! - MockStorage is deterministic and safe for development/testing.

use std::fmt::Debug;

/// Current storage health state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageStatus {
    /// Storage is available and ready for normal operations.
    Healthy,

    /// Storage exists but is not currently ready.
    NotReady,

    /// Storage is unavailable.
    Unavailable,
}

impl StorageStatus {
    /// Stable string representation for API responses.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Healthy => "healthy",
            Self::NotReady => "not_ready",
            Self::Unavailable => "unavailable",
        }
    }

    /// Returns true when storage can serve normal requests.
    pub const fn is_healthy(self) -> bool {
        matches!(self, Self::Healthy)
    }

    /// Returns true when storage is temporarily unavailable/not ready.
    pub const fn is_retryable(self) -> bool {
        matches!(self, Self::NotReady | Self::Unavailable)
    }
}

/// Abstract storage backend used by the Platform API.
///
/// This trait intentionally contains only platform-level guarantees.
/// Concrete database APIs, transactions, WALs, SSTables, encryption,
/// compaction, and other implementation details belong behind this
/// boundary.
///
/// The trait is object-safe and can therefore be stored as:
///
/// `Arc<dyn Storage>`
pub trait Storage: Send + Sync + Debug {
    /// Return the current health state of the backend.
    fn health(&self) -> StorageStatus;

    /// Return a stable backend identifier.
    ///
    /// This value is intended for internal diagnostics and metrics,
    /// not for exposing implementation details to untrusted clients.
    fn name(&self) -> &'static str;

    /// Check whether the backend is ready to serve requests.
    fn is_ready(&self) -> bool {
        self.health().is_healthy()
    }
}

/// Mock storage backend for development and testing.
///
/// MockStorage deliberately contains no external state and performs
/// no I/O. This makes it deterministic and safe for local development.
#[derive(Debug, Clone, Copy, Default)]
pub struct MockStorage;

impl MockStorage {
    /// Create a new mock storage backend.
    pub const fn new() -> Self {
        Self
    }
}

impl Storage for MockStorage {
    fn health(&self) -> StorageStatus {
        StorageStatus::Healthy
    }

    fn name(&self) -> &'static str {
        "mock"
    }
}

/// Shared storage handle used by application state.
///
/// Keeping the alias here gives the rest of the platform a single
/// dependency type and makes the concrete backend replaceable.
pub type SharedStorage = std::sync::Arc<dyn Storage>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_storage_is_healthy() {
        let storage = MockStorage::new();

        assert_eq!(storage.health(), StorageStatus::Healthy);
        assert!(storage.is_ready());
        assert_eq!(storage.name(), "mock");
    }

    #[test]
    fn healthy_status_is_not_retryable() {
        assert!(!StorageStatus::Healthy.is_retryable());
    }

    #[test]
    fn unavailable_status_is_retryable() {
        assert!(StorageStatus::Unavailable.is_retryable());
    }

    #[test]
    fn not_ready_status_is_retryable() {
        assert!(StorageStatus::NotReady.is_retryable());
    }

    #[test]
    #[test]
    fn storage_status_has_stable_names() {
        assert_eq!(StorageStatus::Healthy.as_str(), "healthy");
        assert_eq!(StorageStatus::NotReady.as_str(), "not_ready");
        assert_eq!(StorageStatus::Unavailable.as_str(), "unavailable");
    }

    fn shared_storage_is_object_safe() {
        let storage: SharedStorage = std::sync::Arc::new(MockStorage::new());

        assert!(storage.is_ready());
        assert_eq!(storage.name(), "mock");
    }
}
