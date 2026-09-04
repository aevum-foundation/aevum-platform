use std::fmt::Debug;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageStatus {
    Healthy,
    NotReady,
    Unavailable,
}

impl StorageStatus {
    pub const fn is_healthy(self) -> bool {
        matches!(self, Self::Healthy)
    }

    pub const fn is_retryable(self) -> bool {
        matches!(self, Self::NotReady | Self::Unavailable)
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Healthy => "healthy",
            Self::NotReady => "not_ready",
            Self::Unavailable => "unavailable",
        }
    }
}

pub trait Storage: Send + Sync + Debug {
    fn health(&self) -> StorageStatus;
    fn name(&self) -> &'static str;
    fn is_ready(&self) -> bool {
        self.health().is_healthy()
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct MockStorage;

impl MockStorage {
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
    fn shared_storage_is_object_safe() {
        let storage: SharedStorage = std::sync::Arc::new(MockStorage::new());
        assert!(storage.is_ready());
        assert_eq!(storage.name(), "mock");
    }

    #[test]
    fn storage_status_has_stable_names() {
        assert_eq!(StorageStatus::Healthy.as_str(), "healthy");
        assert_eq!(StorageStatus::NotReady.as_str(), "not_ready");
        assert_eq!(StorageStatus::Unavailable.as_str(), "unavailable");
    }
}
