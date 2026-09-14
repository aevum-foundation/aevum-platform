//! Notification storage abstraction and in-memory implementation.
//!
//! B-2.2 — durable, idempotent notification storage.
//!
//! The storage layer owns:
//! - persistence;
//! - idempotency (per-key mutex + atomic batch commit);
//! - unread index maintenance.
//!
//! The storage layer does NOT generate business data (`id`, `created_at`).
//! It receives fully-formed `Notification` values from the service layer.
//!
//! Single-instance invariant: see `docs/architecture/notifications-v1.md`.

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use async_trait::async_trait;
use sha2::{Digest, Sha256};
use tokio::sync::Mutex;
use uuid::Uuid;

use super::models::{
    CreateNotificationResult, Notification, NotificationCursor, NotificationKind, NotificationPage,
};
use crate::error::ApiError;

/// Length of the lowercase-hex SHA-256 digest used for source_id keying.
pub const SOURCE_HASH_HEX_LEN: usize = 64;

/// Canonical ordering used for notification pagination:
/// `created_at DESC, id DESC`.
///
/// Kept as a single function so that InMemory and AevumDB implementations
/// (and service-level tests) share exactly the same ordering semantics.
pub(crate) fn notification_desc_cmp(a: &Notification, b: &Notification) -> Ordering {
    b.created_at
        .cmp(&a.created_at)
        .then_with(|| b.id.cmp(&a.id))
}

/// Compute the SHA-256 hex digest of a source_id.
///
/// The digest is used for cryptographic collision-resistant keying, not as
/// a mathematical proof of absence of collisions.
///
/// Output is always [`SOURCE_HASH_HEX_LEN`] lowercase hex characters.
pub(crate) fn hash_source_id(source_id: &str) -> String {
    let digest = Sha256::digest(source_id.as_bytes());
    hex::encode(digest)
}

/// Persistent storage for notifications.
///
/// Implementations:
/// - InMemoryNotificationStorage (dev/test)
/// - AevumDbNotificationStorage (production)
#[async_trait]
pub trait NotificationStorage: Send + Sync {
    /// Persist a new notification idempotently.
    ///
    /// `source_id` participates in the idempotency identity:
    /// `(notification.user_id, notification.kind, sha256(source_id))`.
    ///
    /// If a notification with the same idempotency identity already exists,
    /// returns `AlreadyExists(existing)`. Otherwise `Created(new)`.
    async fn create(
        &self,
        notification: &Notification,
        source_id: &str,
    ) -> Result<CreateNotificationResult, ApiError>;

    /// List notifications for a user, ordered by `created_at DESC, id DESC`.
    ///
    /// Returns at most `limit` items. If more items exist, the result
    /// includes a cursor pointing to the last item of the page.
    async fn list(
        &self,
        user_id: &Uuid,
        cursor: Option<NotificationCursor>,
        limit: usize,
    ) -> Result<NotificationPage, ApiError>;

    /// Count unread notifications for a user, derived from the unread index.
    async fn unread_count(&self, user_id: &Uuid) -> Result<u64, ApiError>;

    /// Mark a notification as read. Idempotent.
    ///
    /// First call sets `read_at = now`. Repeated call preserves the
    /// existing `read_at`. Both return the current notification state.
    ///
    /// Returns `NotFound` if the notification does not exist or belongs to
    /// another user.
    async fn mark_read(
        &self,
        user_id: &Uuid,
        notification_id: &Uuid,
    ) -> Result<Notification, ApiError>;
}

type IdempotencyKey = (Uuid, NotificationKind, String);
type NotificationKey = (Uuid, Uuid);

/// In-memory implementation for development and tests.
///
/// MUST NOT be treated as the production persistence layer.
///
/// ## Lock order (MUST NEVER CHANGE)
///
/// 1. `idempotency_locks` (outer map of per-key locks; acquired briefly)
/// 2. `notifications`
/// 3. `idempotency`
/// 4. `unread`
///
/// Violating this order may introduce deadlocks. The outer per-key lock is
/// always released before acquiring the inner maps.
#[derive(Clone, Debug, Default)]
pub struct InMemoryNotificationStorage {
    /// Notifications by (user_id, notification_id).
    notifications: Arc<Mutex<HashMap<NotificationKey, Notification>>>,

    /// Unread index: present iff the notification is unread.
    unread: Arc<Mutex<HashSet<NotificationKey>>>,

    /// Idempotency index: (user_id, kind, source_hash) → notification_id.
    idempotency: Arc<Mutex<HashMap<IdempotencyKey, Uuid>>>,

    /// Per-key locks for atomic idempotent emit.
    ///
    /// Policy: never hold two different idempotency locks at the same time.
    idempotency_locks: Arc<Mutex<HashMap<IdempotencyKey, Arc<Mutex<()>>>>>,
}

impl InMemoryNotificationStorage {
    pub fn new() -> Self {
        Self::default()
    }

    async fn lock_for_idempotency(&self, key: IdempotencyKey) -> Arc<Mutex<()>> {
        let mut locks = self.idempotency_locks.lock().await;
        locks
            .entry(key)
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    /// Test helper: number of stored notifications for a user.
    pub async fn notification_count(&self, user_id: &Uuid) -> usize {
        self.notifications
            .lock()
            .await
            .keys()
            .filter(|(uid, _)| uid == user_id)
            .count()
    }
}

#[async_trait]
impl NotificationStorage for InMemoryNotificationStorage {
    async fn create(
        &self,
        notification: &Notification,
        source_id: &str,
    ) -> Result<CreateNotificationResult, ApiError> {
        let source_hash = hash_source_id(source_id);
        let key: IdempotencyKey = (notification.user_id, notification.kind, source_hash);

        let lock = self.lock_for_idempotency(key.clone()).await;
        let _guard = lock.lock().await;

        // Idempotency check.
        let existing_id = self.idempotency.lock().await.get(&key).copied();

        if let Some(existing_id) = existing_id {
            // Invariant:
            // if the idempotency index contains a notification_id,
            // the corresponding notification MUST exist.
            //
            // A miss here would indicate a bug in the storage implementation,
            // hence `Internal`.
            let existing = self
                .notifications
                .lock()
                .await
                .get(&(notification.user_id, existing_id))
                .cloned()
                .ok_or(ApiError::Internal)?;

            return Ok(CreateNotificationResult::AlreadyExists(existing));
        }

        // Commit: notification + idempotency + unread.
        //
        // Under the per-key idempotency lock, no other writer for the same
        // (user_id, kind, source_hash) can interleave. All three inserts
        // therefore become visible atomically from the perspective of that
        // idempotency key.
        let nk: NotificationKey = (notification.user_id, notification.id);

        self.notifications
            .lock()
            .await
            .insert(nk, notification.clone());

        self.idempotency.lock().await.insert(key, notification.id);

        self.unread.lock().await.insert(nk);

        Ok(CreateNotificationResult::Created(notification.clone()))
    }

    async fn list(
        &self,
        user_id: &Uuid,
        cursor: Option<NotificationCursor>,
        limit: usize,
    ) -> Result<NotificationPage, ApiError> {
        // Collect and sort by canonical DESC order.
        let mut items: Vec<Notification> = self
            .notifications
            .lock()
            .await
            .values()
            .filter(|n| &n.user_id == user_id)
            .cloned()
            .collect();

        items.sort_by(notification_desc_cmp);

        // Apply cursor: keep items strictly "after" (created_at, id) in
        // DESC order — i.e. strictly older, or same timestamp with smaller id.
        if let Some(c) = cursor {
            items.retain(|n| {
                n.created_at < c.created_at || (n.created_at == c.created_at && n.id < c.id)
            });
        }

        // Fetch limit + 1 to determine if there is a next page.
        //
        // This mirrors the AevumDB prefix-scan approach, where the full set
        // is not known in advance and "peek one more" is the natural way to
        // detect a next page without a separate count.
        let mut page: Vec<Notification> = items.into_iter().take(limit + 1).collect();

        let has_more = page.len() > limit;
        if has_more {
            page.pop();
        }

        let next_cursor = if has_more {
            page.last().map(|n| NotificationCursor {
                created_at: n.created_at,
                id: n.id,
            })
        } else {
            None
        };

        Ok(NotificationPage {
            items: page,
            next_cursor,
        })
    }

    async fn unread_count(&self, user_id: &Uuid) -> Result<u64, ApiError> {
        let count = self
            .unread
            .lock()
            .await
            .iter()
            .filter(|(uid, _)| uid == user_id)
            .count();

        Ok(count as u64)
    }

    async fn mark_read(
        &self,
        user_id: &Uuid,
        notification_id: &Uuid,
    ) -> Result<Notification, ApiError> {
        let nk: NotificationKey = (*user_id, *notification_id);

        let mut notifications = self.notifications.lock().await;
        let notification = notifications.get_mut(&nk).ok_or(ApiError::NotFound)?;

        // Idempotent: preserve the original read_at on repeat calls.
        if notification.read_at.is_none() {
            notification.read_at = Some(chrono::Utc::now());
            self.unread.lock().await.remove(&nk);
        }

        Ok(notification.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

    fn make_notification(user_id: Uuid, kind: NotificationKind, age_secs: i64) -> Notification {
        Notification {
            id: Uuid::new_v4(),
            user_id,
            kind,
            payload: serde_json::json!({"text": "hello"}),
            read_at: None,
            created_at: Utc::now() - Duration::seconds(age_secs),
        }
    }

    fn make_notification_at(
        user_id: Uuid,
        kind: NotificationKind,
        created_at: chrono::DateTime<Utc>,
    ) -> Notification {
        Notification {
            id: Uuid::new_v4(),
            user_id,
            kind,
            payload: serde_json::json!({"text": "hello"}),
            read_at: None,
            created_at,
        }
    }

    // ─── hash_source_id ────────────────────────────────────

    #[test]
    fn hash_source_id_is_deterministic_and_distinct() {
        assert_eq!(hash_source_id("abc"), hash_source_id("abc"));
        assert_ne!(hash_source_id("abc"), hash_source_id("abcd"));
    }

    #[test]
    fn hash_source_id_has_expected_length() {
        assert_eq!(hash_source_id("abc").len(), SOURCE_HASH_HEX_LEN);
        assert_eq!(hash_source_id("").len(), SOURCE_HASH_HEX_LEN);
    }

    #[test]
    fn hash_source_id_is_lowercase_hex() {
        let h = hash_source_id("hello");
        assert!(h
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
    }

    // ─── create / list ─────────────────────────────────────

    #[tokio::test]
    async fn create_then_list_returns_notification() {
        let storage = InMemoryNotificationStorage::new();
        let user_id = Uuid::new_v4();
        let n = make_notification(user_id, NotificationKind::System, 0);

        let result = storage.create(&n, "src-1").await.unwrap();
        assert!(matches!(result, CreateNotificationResult::Created(_)));

        let page = storage.list(&user_id, None, 10).await.unwrap();
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].id, n.id);
        assert!(page.next_cursor.is_none());
    }

    #[tokio::test]
    async fn create_duplicate_source_id_is_already_exists() {
        let storage = InMemoryNotificationStorage::new();
        let user_id = Uuid::new_v4();
        let n1 = make_notification(user_id, NotificationKind::System, 0);
        let n2 = make_notification(user_id, NotificationKind::System, 0);

        storage.create(&n1, "src-1").await.unwrap();

        let result = storage.create(&n2, "src-1").await.unwrap();
        match result {
            CreateNotificationResult::AlreadyExists(existing) => {
                assert_eq!(existing.id, n1.id);
            }
            CreateNotificationResult::Created(_) => panic!("expected AlreadyExists"),
        }

        assert_eq!(storage.notification_count(&user_id).await, 1);
    }

    #[tokio::test]
    async fn create_different_source_id_creates_two() {
        let storage = InMemoryNotificationStorage::new();
        let user_id = Uuid::new_v4();
        let n1 = make_notification(user_id, NotificationKind::System, 0);
        let n2 = make_notification(user_id, NotificationKind::System, 0);

        storage.create(&n1, "src-1").await.unwrap();
        storage.create(&n2, "src-2").await.unwrap();

        assert_eq!(storage.notification_count(&user_id).await, 2);
    }

    #[tokio::test]
    async fn create_different_kind_same_source_is_distinct() {
        let storage = InMemoryNotificationStorage::new();
        let user_id = Uuid::new_v4();
        let n1 = make_notification(user_id, NotificationKind::ReplyToTopic, 0);
        let n2 = make_notification(user_id, NotificationKind::MentionedInPost, 0);

        storage.create(&n1, "src-1").await.unwrap();
        storage.create(&n2, "src-1").await.unwrap();

        assert_eq!(storage.notification_count(&user_id).await, 2);
    }

    #[tokio::test]
    async fn create_different_user_same_source_is_distinct() {
        let storage = InMemoryNotificationStorage::new();
        let user_a = Uuid::new_v4();
        let user_b = Uuid::new_v4();
        let n1 = make_notification(user_a, NotificationKind::System, 0);
        let n2 = make_notification(user_b, NotificationKind::System, 0);

        storage.create(&n1, "src-1").await.unwrap();
        storage.create(&n2, "src-1").await.unwrap();

        assert_eq!(storage.notification_count(&user_a).await, 1);
        assert_eq!(storage.notification_count(&user_b).await, 1);
    }

    // ─── list / pagination ─────────────────────────────────

    #[tokio::test]
    async fn list_empty_returns_empty_page() {
        let storage = InMemoryNotificationStorage::new();
        let page = storage.list(&Uuid::new_v4(), None, 10).await.unwrap();
        assert!(page.items.is_empty());
        assert!(page.next_cursor.is_none());
    }

    #[tokio::test]
    async fn list_pagination_uses_cursor() {
        let storage = InMemoryNotificationStorage::new();
        let user_id = Uuid::new_v4();

        for i in 0..5 {
            let n = make_notification(user_id, NotificationKind::System, i as i64);
            storage.create(&n, &format!("src-{i}")).await.unwrap();
        }

        let page1 = storage.list(&user_id, None, 2).await.unwrap();
        assert_eq!(page1.items.len(), 2);
        assert!(page1.next_cursor.is_some());

        let page2 = storage.list(&user_id, page1.next_cursor, 2).await.unwrap();
        assert_eq!(page2.items.len(), 2);
        assert!(page2.next_cursor.is_some());

        let page3 = storage.list(&user_id, page2.next_cursor, 2).await.unwrap();
        assert_eq!(page3.items.len(), 1);
        assert!(page3.next_cursor.is_none());

        let ids: Vec<Uuid> = page1
            .items
            .iter()
            .chain(page2.items.iter())
            .chain(page3.items.iter())
            .map(|n| n.id)
            .collect();
        let mut sorted = ids.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), 5);
    }

    #[tokio::test]
    async fn list_orders_by_created_at_desc() {
        let storage = InMemoryNotificationStorage::new();
        let user_id = Uuid::new_v4();

        let n_oldest = make_notification(user_id, NotificationKind::System, 100);
        let n_middle = make_notification(user_id, NotificationKind::System, 50);
        let n_newest = make_notification(user_id, NotificationKind::System, 1);

        storage.create(&n_oldest, "s1").await.unwrap();
        storage.create(&n_middle, "s2").await.unwrap();
        storage.create(&n_newest, "s3").await.unwrap();

        let page = storage.list(&user_id, None, 10).await.unwrap();
        assert_eq!(page.items[0].id, n_newest.id);
        assert_eq!(page.items[1].id, n_middle.id);
        assert_eq!(page.items[2].id, n_oldest.id);
    }

    #[tokio::test]
    async fn list_exact_limit_has_no_next_cursor() {
        let storage = InMemoryNotificationStorage::new();
        let user_id = Uuid::new_v4();

        for i in 0..3 {
            let n = make_notification(user_id, NotificationKind::System, i);
            storage.create(&n, &format!("s{i}")).await.unwrap();
        }

        // Exactly limit items — no more pages.
        let page = storage.list(&user_id, None, 3).await.unwrap();
        assert_eq!(page.items.len(), 3);
        assert!(page.next_cursor.is_none());
    }

    #[tokio::test]
    async fn list_one_over_limit_has_next_cursor() {
        let storage = InMemoryNotificationStorage::new();
        let user_id = Uuid::new_v4();

        for i in 0..4 {
            let n = make_notification(user_id, NotificationKind::System, i);
            storage.create(&n, &format!("s{i}")).await.unwrap();
        }

        // One over the limit — next_cursor must be present.
        let page = storage.list(&user_id, None, 3).await.unwrap();
        assert_eq!(page.items.len(), 3);
        assert!(page.next_cursor.is_some());
    }

    // ─── deterministic tie-breaker ────────────────────────

    #[tokio::test]
    async fn same_created_at_uses_uuid_tiebreaker() {
        let storage = InMemoryNotificationStorage::new();
        let user_id = Uuid::new_v4();

        // Same timestamp — ordering must fall back to id DESC.
        let t = Utc::now();
        let n1 = make_notification_at(user_id, NotificationKind::System, t);
        let n2 = make_notification_at(user_id, NotificationKind::System, t);
        let n3 = make_notification_at(user_id, NotificationKind::System, t);

        storage.create(&n1, "s1").await.unwrap();
        storage.create(&n2, "s2").await.unwrap();
        storage.create(&n3, "s3").await.unwrap();

        // Expected order: highest id first (DESC), then middle, then lowest.
        let mut expected_ids = [n1.id, n2.id, n3.id];
        expected_ids.sort_by(|a, b| b.cmp(a)); // DESC

        // Page through 1 item at a time to exercise cursor on ties.
        let mut collected = Vec::new();
        let mut cursor = None;
        for _ in 0..3 {
            let page = storage.list(&user_id, cursor, 1).await.unwrap();
            assert_eq!(page.items.len(), 1);
            collected.push(page.items[0].id);
            cursor = page.next_cursor;
        }

        assert_eq!(collected, expected_ids.to_vec());
        assert!(cursor.is_none());
    }

    // ─── unread / mark_read ────────────────────────────────

    #[tokio::test]
    async fn unread_count_tracks_unread_state() {
        let storage = InMemoryNotificationStorage::new();
        let user_id = Uuid::new_v4();

        for i in 0..3 {
            let n = make_notification(user_id, NotificationKind::System, i);
            storage.create(&n, &format!("s{i}")).await.unwrap();
        }

        assert_eq!(storage.unread_count(&user_id).await.unwrap(), 3);
    }

    #[tokio::test]
    async fn mark_read_decrements_unread_count() {
        let storage = InMemoryNotificationStorage::new();
        let user_id = Uuid::new_v4();
        let n = make_notification(user_id, NotificationKind::System, 0);

        storage.create(&n, "s1").await.unwrap();
        assert_eq!(storage.unread_count(&user_id).await.unwrap(), 1);

        storage.mark_read(&user_id, &n.id).await.unwrap();
        assert_eq!(storage.unread_count(&user_id).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn mark_read_first_call_sets_read_at() {
        let storage = InMemoryNotificationStorage::new();
        let user_id = Uuid::new_v4();
        let n = make_notification(user_id, NotificationKind::System, 0);

        storage.create(&n, "s1").await.unwrap();
        let marked = storage.mark_read(&user_id, &n.id).await.unwrap();

        assert!(marked.read_at.is_some());
    }

    #[tokio::test]
    async fn mark_read_repeat_is_idempotent() {
        let storage = InMemoryNotificationStorage::new();
        let user_id = Uuid::new_v4();
        let n = make_notification(user_id, NotificationKind::System, 0);

        storage.create(&n, "s1").await.unwrap();
        let first = storage.mark_read(&user_id, &n.id).await.unwrap();
        let second = storage.mark_read(&user_id, &n.id).await.unwrap();

        assert_eq!(first.read_at, second.read_at);
    }

    #[tokio::test]
    async fn mark_read_missing_returns_not_found() {
        let storage = InMemoryNotificationStorage::new();
        let result = storage.mark_read(&Uuid::new_v4(), &Uuid::new_v4()).await;
        assert!(matches!(result, Err(ApiError::NotFound)));
    }

    #[tokio::test]
    async fn mark_read_wrong_user_returns_not_found() {
        let storage = InMemoryNotificationStorage::new();
        let owner = Uuid::new_v4();
        let other = Uuid::new_v4();
        let n = make_notification(owner, NotificationKind::System, 0);

        storage.create(&n, "s1").await.unwrap();

        let result = storage.mark_read(&other, &n.id).await;
        assert!(matches!(result, Err(ApiError::NotFound)));
    }

    // ─── concurrency ───────────────────────────────────────

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn concurrent_create_same_source_one_wins() {
        let storage = InMemoryNotificationStorage::new();
        let user_id = Uuid::new_v4();
        let n1 = make_notification(user_id, NotificationKind::System, 0);
        let n2 = make_notification(user_id, NotificationKind::System, 0);

        let (r1, r2) = tokio::join!(
            storage.create(&n1, "same-src"),
            storage.create(&n2, "same-src"),
        );

        let mut created = 0;
        let mut exists = 0;
        for result in [r1, r2] {
            match result.unwrap() {
                CreateNotificationResult::Created(_) => created += 1,
                CreateNotificationResult::AlreadyExists(_) => exists += 1,
            }
        }

        assert_eq!(created, 1, "exactly one create must succeed");
        assert_eq!(exists, 1, "exactly one must report AlreadyExists");
        assert_eq!(storage.notification_count(&user_id).await, 1);
    }
}
