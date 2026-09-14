//! AevumDB-backed notification storage.
//!
//! B-2.2b — production storage layer for Notifications.
//!
//! Idempotency is guaranteed by a per-key application mutex combined with
//! the platform's single-instance invariant. AevumDB does not currently
//! expose conditional writes / CAS; see `docs/architecture/notifications-v1.md`
//! and the AevumDB-TX-1 backlog item.

use std::collections::HashMap;
use std::sync::Arc;

use aevum_db::{AevumDb, DbConfig, DbError, DbRuntime};
use serde::de::DeserializeOwned;
use serde::Serialize;
use tokio::sync::Mutex;
use uuid::Uuid;

use super::models::{
    CreateNotificationResult, Notification, NotificationCursor, NotificationKind, NotificationPage,
};
use super::storage::{hash_source_id, notification_desc_cmp, NotificationStorage};
use crate::error::ApiError;

const NOTIFICATION_PREFIX: &str = "platform:community:notifications:";
const UNREAD_PREFIX: &str = "platform:community:notifications:unread:";
const IDEMPOTENCY_PREFIX: &str = "platform:community:notifications:idempotency:";

type IdempotencyKey = (Uuid, NotificationKind, String);

#[derive(Clone)]
pub struct AevumDbNotificationStorage {
    db: Arc<AevumDb>,
    /// Per-key locks for serializing atomic idempotent emit.
    /// Policy: never hold two different idempotency locks at the same time.
    idempotency_locks: Arc<Mutex<HashMap<IdempotencyKey, Arc<Mutex<()>>>>>,
}

impl std::fmt::Debug for AevumDbNotificationStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AevumDbNotificationStorage")
            .field("db", &"<redacted>")
            .field("idempotency_locks", &"<redacted>")
            .finish()
    }
}

impl AevumDbNotificationStorage {
    pub fn open(config: DbConfig, runtime: DbRuntime) -> Result<Self, ApiError> {
        let db = AevumDb::open(config, runtime).map_err(|error| {
            log::error!("AevumDB open failed: {}", error);
            ApiError::Internal
        })?;
        Ok(Self {
            db: Arc::new(db),
            idempotency_locks: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    async fn lock_for_idempotency(&self, key: IdempotencyKey) -> Arc<Mutex<()>> {
        let mut locks = self.idempotency_locks.lock().await;
        locks
            .entry(key)
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    // ─── Key builders ──────────────────────────────────

    pub(crate) fn notification_key(user_id: &Uuid, notification_id: &Uuid) -> String {
        format!("{}{}:{}", NOTIFICATION_PREFIX, user_id, notification_id)
    }

    pub(crate) fn unread_key(user_id: &Uuid, notification_id: &Uuid) -> String {
        format!("{}{}:{}", UNREAD_PREFIX, user_id, notification_id)
    }

    pub(crate) fn idempotency_key(
        user_id: &Uuid,
        kind: NotificationKind,
        source_hash: &str,
    ) -> String {
        format!(
            "{}{}:{}:{}",
            IDEMPOTENCY_PREFIX,
            user_id,
            kind.as_str(),
            source_hash
        )
    }

    pub(crate) fn notification_prefix(user_id: &Uuid) -> String {
        format!("{}{}:", NOTIFICATION_PREFIX, user_id)
    }

    pub(crate) fn unread_prefix(user_id: &Uuid) -> String {
        format!("{}{}:", UNREAD_PREFIX, user_id)
    }

    // ─── Serialization helpers ─────────────────────────

    fn serialize<T: Serialize>(value: &T) -> Result<Vec<u8>, ApiError> {
        serde_json::to_vec(value).map_err(|error| {
            log::error!("notification serialize failed: {}", error);
            ApiError::Internal
        })
    }

    fn deserialize<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, ApiError> {
        serde_json::from_slice(bytes).map_err(|error| {
            log::error!("notification deserialize failed: {}", error);
            ApiError::Internal
        })
    }

    fn map_db_error(error: DbError) -> ApiError {
        log::error!("AevumDB error: {}", error);
        ApiError::Internal
    }
}

#[async_trait::async_trait]
impl NotificationStorage for AevumDbNotificationStorage {
    async fn create(
        &self,
        notification: &Notification,
        source_id: &str,
    ) -> Result<CreateNotificationResult, ApiError> {
        let source_hash = hash_source_id(source_id);
        let key: IdempotencyKey = (notification.user_id, notification.kind, source_hash.clone());

        let lock = self.lock_for_idempotency(key.clone()).await;
        let _guard = lock.lock().await;

        let idem_key =
            Self::idempotency_key(&notification.user_id, notification.kind, &source_hash);

        // Idempotency check.
        if let Some(existing_id_bytes) = self
            .db
            .get(idem_key.as_bytes())
            .map_err(Self::map_db_error)?
        {
            let existing_id_str = std::str::from_utf8(&existing_id_bytes).map_err(|e| {
                log::error!("idempotency index utf8 decode failed: {}", e);
                ApiError::Internal
            })?;

            let existing_id = Uuid::parse_str(existing_id_str).map_err(|e| {
                log::error!("idempotency index uuid parse failed: {}", e);
                ApiError::Internal
            })?;

            // Invariant: idempotency index implies the notification exists.
            let existing_key = Self::notification_key(&notification.user_id, &existing_id);
            let existing_bytes = self
                .db
                .get(existing_key.as_bytes())
                .map_err(Self::map_db_error)?
                .ok_or(ApiError::Internal)?;

            let existing: Notification = Self::deserialize(&existing_bytes)?;
            return Ok(CreateNotificationResult::AlreadyExists(existing));
        }

        // Commit: notification + idempotency + unread.
        let n_key = Self::notification_key(&notification.user_id, &notification.id);
        let u_key = Self::unread_key(&notification.user_id, &notification.id);

        let notif_bytes = Self::serialize(notification)?;
        let user_id_str = notification.user_id.to_string();

        let mut batch = self.db.batch();
        batch.put(n_key.as_bytes(), &notif_bytes);
        batch.put(idem_key.as_bytes(), notification.id.to_string().as_bytes());
        batch.put(u_key.as_bytes(), user_id_str.as_bytes());
        batch.commit().map_err(Self::map_db_error)?;

        Ok(CreateNotificationResult::Created(notification.clone()))
    }

    async fn list(
        &self,
        user_id: &Uuid,
        cursor: Option<NotificationCursor>,
        limit: usize,
    ) -> Result<NotificationPage, ApiError> {
        let prefix = Self::notification_prefix(user_id);
        let raw = self
            .db
            .prefix_scan(prefix.as_bytes())
            .map_err(Self::map_db_error)?;

        let mut items: Vec<Notification> = raw
            .into_iter()
            .map(|(_k, v)| Self::deserialize::<Notification>(&v))
            .collect::<Result<_, _>>()?;

        // Canonical DESC ordering — same comparator as InMemory.
        items.sort_by(notification_desc_cmp);

        // Apply cursor.
        if let Some(c) = cursor {
            items.retain(|n| {
                n.created_at < c.created_at || (n.created_at == c.created_at && n.id < c.id)
            });
        }

        // Peek one extra to detect next page.
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
        let prefix = Self::unread_prefix(user_id);
        let raw = self
            .db
            .prefix_scan(prefix.as_bytes())
            .map_err(Self::map_db_error)?;

        Ok(raw.len() as u64)
    }

    async fn mark_read(
        &self,
        user_id: &Uuid,
        notification_id: &Uuid,
    ) -> Result<Notification, ApiError> {
        let n_key = Self::notification_key(user_id, notification_id);
        let existing_bytes = self
            .db
            .get(n_key.as_bytes())
            .map_err(Self::map_db_error)?
            .ok_or(ApiError::NotFound)?;

        let mut notification: Notification = Self::deserialize(&existing_bytes)?;

        // Idempotent: preserve the original read_at on repeat calls.
        if notification.read_at.is_none() {
            notification.read_at = Some(chrono::Utc::now());

            let updated_bytes = Self::serialize(&notification)?;
            let u_key = Self::unread_key(user_id, notification_id);

            let mut batch = self.db.batch();
            batch.put(n_key.as_bytes(), &updated_bytes);
            batch.delete(u_key.as_bytes());
            batch.commit().map_err(Self::map_db_error)?;
        }

        Ok(notification)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aevum_db::config::SyncMode;
    use chrono::{Duration, Utc};
    use tempfile::TempDir;

    fn test_db() -> (AevumDbNotificationStorage, TempDir) {
        let temp = TempDir::new().unwrap();
        let config = DbConfig {
            path: temp.path().to_path_buf(),
            memtable_max_bytes: 64 * 1024 * 1024,
            wal_segment_bytes: 64 * 1024 * 1024,
            sync_mode: SyncMode::Always,
            max_open_files: 1000,
            block_size: 4096,
            storage_mode: aevum_db::config::StorageMode::default(),
        };
        let runtime = DbRuntime::plaintext();
        let storage = AevumDbNotificationStorage::open(config, runtime).unwrap();
        (storage, temp)
    }

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

    // ─── Key layout ────────────────────────────────────

    #[test]
    fn key_layout_is_stable() {
        let uid = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let nid = Uuid::parse_str("660e8400-e29b-41d4-a716-446655440001").unwrap();

        assert_eq!(
            AevumDbNotificationStorage::notification_key(&uid, &nid),
            "platform:community:notifications:550e8400-e29b-41d4-a716-446655440000:660e8400-e29b-41d4-a716-446655440001"
        );
        assert_eq!(
            AevumDbNotificationStorage::unread_key(&uid, &nid),
            "platform:community:notifications:unread:550e8400-e29b-41d4-a716-446655440000:660e8400-e29b-41d4-a716-446655440001"
        );
        assert_eq!(
            AevumDbNotificationStorage::idempotency_key(&uid, NotificationKind::System, "abcd"),
            "platform:community:notifications:idempotency:550e8400-e29b-41d4-a716-446655440000:system:abcd"
        );
        assert_eq!(
            AevumDbNotificationStorage::notification_prefix(&uid),
            "platform:community:notifications:550e8400-e29b-41d4-a716-446655440000:"
        );
        assert_eq!(
            AevumDbNotificationStorage::unread_prefix(&uid),
            "platform:community:notifications:unread:550e8400-e29b-41d4-a716-446655440000:"
        );
    }

    // ─── create / list ─────────────────────────────────

    #[tokio::test]
    async fn create_then_list_returns_notification() {
        let (storage, _t) = test_db();
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
        let (storage, _t) = test_db();
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

        let page = storage.list(&user_id, None, 10).await.unwrap();
        assert_eq!(page.items.len(), 1);
    }

    #[tokio::test]
    async fn create_different_source_id_creates_two() {
        let (storage, _t) = test_db();
        let user_id = Uuid::new_v4();
        let n1 = make_notification(user_id, NotificationKind::System, 0);
        let n2 = make_notification(user_id, NotificationKind::System, 0);

        storage.create(&n1, "src-1").await.unwrap();
        storage.create(&n2, "src-2").await.unwrap();

        let page = storage.list(&user_id, None, 10).await.unwrap();
        assert_eq!(page.items.len(), 2);
    }

    #[tokio::test]
    async fn create_different_kind_same_source_is_distinct() {
        let (storage, _t) = test_db();
        let user_id = Uuid::new_v4();
        let n1 = make_notification(user_id, NotificationKind::ReplyToTopic, 0);
        let n2 = make_notification(user_id, NotificationKind::MentionedInPost, 0);

        storage.create(&n1, "src-1").await.unwrap();
        storage.create(&n2, "src-1").await.unwrap();

        let page = storage.list(&user_id, None, 10).await.unwrap();
        assert_eq!(page.items.len(), 2);
    }

    #[tokio::test]
    async fn create_different_user_same_source_is_distinct() {
        let (storage, _t) = test_db();
        let user_a = Uuid::new_v4();
        let user_b = Uuid::new_v4();
        let n1 = make_notification(user_a, NotificationKind::System, 0);
        let n2 = make_notification(user_b, NotificationKind::System, 0);

        storage.create(&n1, "src-1").await.unwrap();
        storage.create(&n2, "src-1").await.unwrap();

        let page_a = storage.list(&user_a, None, 10).await.unwrap();
        let page_b = storage.list(&user_b, None, 10).await.unwrap();
        assert_eq!(page_a.items.len(), 1);
        assert_eq!(page_b.items.len(), 1);
    }

    // ─── list / pagination ─────────────────────────────

    #[tokio::test]
    async fn list_empty_returns_empty_page() {
        let (storage, _t) = test_db();
        let page = storage.list(&Uuid::new_v4(), None, 10).await.unwrap();
        assert!(page.items.is_empty());
        assert!(page.next_cursor.is_none());
    }

    #[tokio::test]
    async fn list_pagination_uses_cursor() {
        let (storage, _t) = test_db();
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

        let mut ids: Vec<Uuid> = page1
            .items
            .iter()
            .chain(page2.items.iter())
            .chain(page3.items.iter())
            .map(|n| n.id)
            .collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 5);
    }

    #[tokio::test]
    async fn list_orders_by_created_at_desc() {
        let (storage, _t) = test_db();
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
        let (storage, _t) = test_db();
        let user_id = Uuid::new_v4();

        for i in 0..3 {
            let n = make_notification(user_id, NotificationKind::System, i);
            storage.create(&n, &format!("s{i}")).await.unwrap();
        }

        let page = storage.list(&user_id, None, 3).await.unwrap();
        assert_eq!(page.items.len(), 3);
        assert!(page.next_cursor.is_none());
    }

    #[tokio::test]
    async fn list_one_over_limit_has_next_cursor() {
        let (storage, _t) = test_db();
        let user_id = Uuid::new_v4();

        for i in 0..4 {
            let n = make_notification(user_id, NotificationKind::System, i);
            storage.create(&n, &format!("s{i}")).await.unwrap();
        }

        let page = storage.list(&user_id, None, 3).await.unwrap();
        assert_eq!(page.items.len(), 3);
        assert!(page.next_cursor.is_some());
    }

    #[tokio::test]
    async fn same_created_at_uses_uuid_tiebreaker() {
        let (storage, _t) = test_db();
        let user_id = Uuid::new_v4();

        let t = Utc::now();
        let n1 = make_notification_at(user_id, NotificationKind::System, t);
        let n2 = make_notification_at(user_id, NotificationKind::System, t);
        let n3 = make_notification_at(user_id, NotificationKind::System, t);

        storage.create(&n1, "s1").await.unwrap();
        storage.create(&n2, "s2").await.unwrap();
        storage.create(&n3, "s3").await.unwrap();

        let mut expected_ids = [n1.id, n2.id, n3.id];
        expected_ids.sort_by(|a, b| b.cmp(a));

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

    // ─── unread / mark_read ────────────────────────────

    #[tokio::test]
    async fn unread_count_tracks_unread_state() {
        let (storage, _t) = test_db();
        let user_id = Uuid::new_v4();

        for i in 0..3 {
            let n = make_notification(user_id, NotificationKind::System, i);
            storage.create(&n, &format!("s{i}")).await.unwrap();
        }

        assert_eq!(storage.unread_count(&user_id).await.unwrap(), 3);
    }

    #[tokio::test]
    async fn mark_read_decrements_unread_count() {
        let (storage, _t) = test_db();
        let user_id = Uuid::new_v4();
        let n = make_notification(user_id, NotificationKind::System, 0);

        storage.create(&n, "s1").await.unwrap();
        assert_eq!(storage.unread_count(&user_id).await.unwrap(), 1);

        storage.mark_read(&user_id, &n.id).await.unwrap();
        assert_eq!(storage.unread_count(&user_id).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn mark_read_first_call_sets_read_at() {
        let (storage, _t) = test_db();
        let user_id = Uuid::new_v4();
        let n = make_notification(user_id, NotificationKind::System, 0);

        storage.create(&n, "s1").await.unwrap();
        let marked = storage.mark_read(&user_id, &n.id).await.unwrap();

        assert!(marked.read_at.is_some());
    }

    #[tokio::test]
    async fn mark_read_repeat_is_idempotent() {
        let (storage, _t) = test_db();
        let user_id = Uuid::new_v4();
        let n = make_notification(user_id, NotificationKind::System, 0);

        storage.create(&n, "s1").await.unwrap();
        let first = storage.mark_read(&user_id, &n.id).await.unwrap();
        let second = storage.mark_read(&user_id, &n.id).await.unwrap();

        assert_eq!(first.read_at, second.read_at);
    }

    #[tokio::test]
    async fn mark_read_missing_returns_not_found() {
        let (storage, _t) = test_db();
        let result = storage.mark_read(&Uuid::new_v4(), &Uuid::new_v4()).await;
        assert!(matches!(result, Err(ApiError::NotFound)));
    }

    #[tokio::test]
    async fn mark_read_wrong_user_returns_not_found() {
        let (storage, _t) = test_db();
        let owner = Uuid::new_v4();
        let other = Uuid::new_v4();
        let n = make_notification(owner, NotificationKind::System, 0);

        storage.create(&n, "s1").await.unwrap();

        let result = storage.mark_read(&other, &n.id).await;
        assert!(matches!(result, Err(ApiError::NotFound)));
    }

    // ─── concurrency ───────────────────────────────────

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn concurrent_create_same_source_one_wins() {
        let (storage, _t) = test_db();
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
    }
}
