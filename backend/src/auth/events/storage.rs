//! Security event storage abstractions.
//!
//! AUTH-16 — Security Events
//!
//! Security events are recorded fire-and-forget. Authentication flows
//! must never fail because audit logging failed.

use std::collections::HashMap;
use std::sync::RwLock;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::error::ApiError;

use super::{SecurityEvent, SecurityEventKind, SecurityEventRecord};

/// Persistent storage for security events.
///
/// Implementations:
/// - InMemorySecurityEventStorage (dev/test)
/// - AevumDbSecurityEventStorage (production)
#[async_trait]
pub trait SecurityEventStorage: Send + Sync {
    async fn record_event(&self, event: SecurityEvent) -> Result<(), ApiError>;

    async fn get_events_for_user(
        &self,
        user_id: Uuid,
        limit: usize,
    ) -> Result<Vec<SecurityEvent>, ApiError>;

    async fn get_events_by_type(
        &self,
        event_kind: SecurityEventKind,
        limit: usize,
    ) -> Result<Vec<SecurityEvent>, ApiError>;

    async fn get_recent_events(&self, limit: usize) -> Result<Vec<SecurityEvent>, ApiError>;

    async fn get_events_since(
        &self,
        timestamp: DateTime<Utc>,
        limit: usize,
    ) -> Result<Vec<SecurityEvent>, ApiError>;

    async fn prune_before(&self, timestamp: DateTime<Utc>) -> Result<usize, ApiError>;

    async fn flush(&self) -> Result<(), ApiError>;
}

/// Internal index container for atomic read/write.
#[derive(Default)]
struct EventIndex {
    events: Vec<SecurityEventRecord>,
    by_user: HashMap<Uuid, Vec<usize>>,
    by_type: HashMap<SecurityEventKind, Vec<usize>>,
    timeline: Vec<(DateTime<Utc>, usize)>,
}

/// In-memory security event storage for development and tests.
#[derive(Default)]
pub struct InMemorySecurityEventStorage {
    inner: RwLock<EventIndex>,
}

impl InMemorySecurityEventStorage {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(EventIndex::default()),
        }
    }

    pub fn event_count(&self) -> usize {
        self.inner.read().unwrap().events.len()
    }
}

#[async_trait]
impl SecurityEventStorage for InMemorySecurityEventStorage {
    async fn record_event(&self, event: SecurityEvent) -> Result<(), ApiError> {
        let record = SecurityEventRecord::new(event);

        let mut inner = self.inner.write().unwrap();
        let index = inner.events.len();
        inner.events.push(record.clone());

        if let Some(user_id) = record.event.user_id() {
            inner.by_user.entry(user_id).or_default().push(index);
        }

        inner
            .by_type
            .entry(record.event.kind())
            .or_default()
            .push(index);

        inner.timeline.push((record.timestamp, index));

        Ok(())
    }

    async fn get_events_for_user(
        &self,
        user_id: Uuid,
        limit: usize,
    ) -> Result<Vec<SecurityEvent>, ApiError> {
        let inner = self.inner.read().unwrap();
        let events: Vec<SecurityEvent> = inner
            .by_user
            .get(&user_id)
            .map(|indices| {
                indices
                    .iter()
                    .rev()
                    .take(limit)
                    .filter_map(|&idx| inner.events.get(idx))
                    .map(|record| record.event.clone())
                    .collect()
            })
            .unwrap_or_default();
        Ok(events)
    }

    async fn get_events_by_type(
        &self,
        event_kind: SecurityEventKind,
        limit: usize,
    ) -> Result<Vec<SecurityEvent>, ApiError> {
        let inner = self.inner.read().unwrap();
        let events: Vec<SecurityEvent> = inner
            .by_type
            .get(&event_kind)
            .map(|indices| {
                indices
                    .iter()
                    .rev()
                    .take(limit)
                    .filter_map(|&idx| inner.events.get(idx))
                    .map(|record| record.event.clone())
                    .collect()
            })
            .unwrap_or_default();
        Ok(events)
    }

    async fn get_recent_events(&self, limit: usize) -> Result<Vec<SecurityEvent>, ApiError> {
        let inner = self.inner.read().unwrap();
        let recent: Vec<SecurityEvent> = inner
            .events
            .iter()
            .rev()
            .take(limit)
            .map(|record| record.event.clone())
            .collect();
        Ok(recent)
    }

    async fn get_events_since(
        &self,
        timestamp: DateTime<Utc>,
        limit: usize,
    ) -> Result<Vec<SecurityEvent>, ApiError> {
        let inner = self.inner.read().unwrap();
        let filtered: Vec<SecurityEvent> = inner
            .events
            .iter()
            .rev()
            .filter(|record| record.timestamp >= timestamp)
            .take(limit)
            .map(|record| record.event.clone())
            .collect();
        Ok(filtered)
    }

    async fn flush(&self) -> Result<(), ApiError> {
        Ok(())
    }

    async fn prune_before(&self, timestamp: DateTime<Utc>) -> Result<usize, ApiError> {
        let mut inner = self.inner.write().unwrap();
        let original_len = inner.events.len();

        let retained: Vec<SecurityEventRecord> = inner
            .events
            .iter()
            .filter(|record| record.timestamp >= timestamp)
            .cloned()
            .collect();

        let pruned = original_len - retained.len();
        inner.events = retained;

        let mut new_by_user: HashMap<Uuid, Vec<usize>> = HashMap::new();
        let mut new_by_type: HashMap<SecurityEventKind, Vec<usize>> = HashMap::new();
        let mut new_timeline: Vec<(DateTime<Utc>, usize)> = Vec::new();

        for (idx, record) in inner.events.iter().enumerate() {
            if let Some(user_id) = record.event.user_id() {
                new_by_user.entry(user_id).or_default().push(idx);
            }
            new_by_type
                .entry(record.event.kind())
                .or_default()
                .push(idx);
            new_timeline.push((record.timestamp, idx));
        }

        inner.by_user = new_by_user;
        inner.by_type = new_by_type;
        inner.timeline = new_timeline;

        Ok(pruned)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::events::SecurityMetadata;

    #[tokio::test]
    async fn event_recording_and_retrieval() {
        let storage = InMemorySecurityEventStorage::new();
        let user_id = Uuid::new_v4();

        let event = SecurityEvent::LoginSuccess {
            metadata: SecurityMetadata::new(None, None),
            user_id,
            email: "test@example.com".to_string(),
        };

        storage.record_event(event.clone()).await.unwrap();
        assert_eq!(storage.event_count(), 1);

        let user_events = storage.get_events_for_user(user_id, 10).await.unwrap();
        assert_eq!(user_events.len(), 1);

        let recent = storage.get_recent_events(10).await.unwrap();
        assert_eq!(recent.len(), 1);

        let by_type = storage
            .get_events_by_type(SecurityEventKind::LoginSuccess, 10)
            .await
            .unwrap();
        assert_eq!(by_type.len(), 1);

        let since = storage
            .get_events_since(Utc::now() - chrono::Duration::seconds(60), 10)
            .await
            .unwrap();
        assert_eq!(since.len(), 1);

        let pruned = storage
            .prune_before(Utc::now() + chrono::Duration::seconds(1))
            .await
            .unwrap();
        assert_eq!(pruned, 1);
        assert_eq!(storage.event_count(), 0);
    }
}
