# Notifications v1 — Architecture

Status: **B-2 implemented and tested**

Notifications v1 provides a durable, idempotent notification subsystem
owned by the Community domain. It is a foundational layer for future
Forum, Comments, and Social features.

---

## 1. Purpose and Scope

Notifications v1 owns:

- durable notification persistence;
- idempotent emit;
- read/unread state;
- unread count;
- cursor-based pagination;
- notification HTTP pull API;
- a `NotificationSink` abstraction for future realtime delivery.

Notifications v1 does **not** own:

- SSE / WebSocket delivery (only the abstraction);
- email or push notifications;
- per-kind user preferences (only the global AUTH-26 flag exists);
- notification content generation (producers supply payloads);
- Forum topic/post/comment logic (producers of notifications);
- retention/deletion policy.

---

## 2. Domain Boundaries

### Ownership

Notifications belong to a specific authenticated `user_id`.

Notification IDs are **never treated as globally readable resources**.

All authorization is performed against the authenticated `user_id`.
Lookups that do not match the authenticated user MUST NOT disclose
existence of a notification they do not own.

### Emit vs Delivery

Producers (Forum, Comments, System tooling) call `NotificationService::emit(...)`.

Storage commit happens **before** any optional sink delivery.

Delivery failure MUST NOT roll back an already committed notification.

---

## 3. Notification Kind

Exactly four kinds are supported in v1:

```rust
pub enum NotificationKind {
    ReplyToTopic,
    ReplyToComment,
    MentionedInPost,
    System,
}
```

Serde: reply_to_topic, reply_to_comment, mentioned_in_post, system.

System is the only administrative channel in v1.

New kinds MUST NOT be added speculatively. They are added when a real
producer exists.

---

## 4. Notification Model

```rust
pub struct Notification {
    pub id: Uuid,
    pub user_id: Uuid,
    pub kind: NotificationKind,
    pub payload: serde_json::Value,
    pub read_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}
```

source_id is deliberately not stored on the notification object.
It is part of the idempotency identity only and lives in the idempotency
index.

---


## 5. Idempotency

### Idempotency identity

```text
(user_id, kind, source_id)
```

Where:

· user_id — recipient;
· kind — notification kind;
· source_id — producer-supplied opaque string.

Validation of source_id

· 1..=256 Unicode scalar values;
· control characters rejected.

Keying

source_id is keyed via:

```text
SHA-256(source_id.as_bytes())
```

Result is lowercase hexadecimal, 64 characters.

The hash is used for cryptographic collision-resistant keying, not as a
mathematical proof of absence of collisions.

Idempotency storage key

```text
platform:community:notifications:idempotency:
    {user_id}:{kind}:{sha256_hex}
```

Value: notification_id.

Emit semantics

```text
lock (user_id, kind, source_id)
    ↓
check idempotency index
    ↓
if present:
    return AlreadyExists(existing notification)
    ↓
generate notification_id
    ↓
WriteBatch:
    notification
    idempotency index
    unread index
    ↓
commit
    ↓
return Created(notification)
```

The lock is owned by the storage layer. The service layer is agnostic to
concurrency mechanics.

Single-instance invariant

Consistent with B-1, idempotency in v1 is guaranteed under the platform's
single authoritative writer / single-instance invariant.

Multi-instance correctness requires AevumDB-TX-1. See
docs/backlog/aevumdb-tx-1.md.

---

## 6. Storage Layout

Canonical keys:

```text
platform:community:notifications:{user_id}:{notification_id}
platform:community:notifications:unread:{user_id}:{notification_id}
platform:community:notifications:idempotency:
    {user_id}:{kind}:{sha256_hex}
```

The unread index exists so that unread_count never requires a scan.

---

## 7. Payload

Opaque JSON supplied by the producer.

Constraints:

· maximum serialized size: 16 KiB;
· otherwise not interpreted by the notification subsystem.

Violations return a validation error.

---

## 8. Pagination

Cursor-based only. Offset pagination is not supported.

Ordering: ORDER BY created_at DESC, id DESC.

Cursor contains:

```json
{
  "created_at": "<RFC3339>",
  "id": "<UUID>"
}
```

Encoded as: UTF-8 JSON → Base64.

Cursor is opaque to HTTP clients.

Cursor denotes: return items strictly after (created_at, id) in the DESC
order.

Limits:

· default: 20;
· maximum: 100;
· minimum: 1.

limit = 0 is rejected. Limit validation belongs to the service.

An invalid cursor (bad base64, bad JSON, bad timestamp, bad UUID) returns
400 INVALID_CURSOR.

---


## 9. Read Semantics

`mark_read(user_id, notification_id)`:

- if the notification belongs to another user → `404`;
- if the notification does not exist → `404`;
- first call: sets `read_at = now`;
- repeated call: preserves the existing `read_at`;
- both calls return `200` with the current notification state.

`mark_read` never mutates `read_at` after the first transition.

---

## 10. Unread Count

`unread_count(user_id)` is served from the unread index.

The unread index entry:

- is written on emit (same WriteBatch);
- is deleted on first successful `mark_read`.

No scan is performed.

---

## 11. Storage Layer

```rust
#[async_trait]
pub trait NotificationStorage: Send + Sync {
    async fn create(
        &self,
        notification: &Notification,
        source_id: &str,
    ) -> Result<CreateNotificationResult, ApiError>;

    async fn list(
        &self,
        user_id: &Uuid,
        cursor: Option<NotificationCursor>,
        limit: usize,
    ) -> Result<NotificationPage, ApiError>;

    async fn unread_count(
        &self,
        user_id: &Uuid,
    ) -> Result<u64, ApiError>;

    async fn mark_read(
        &self,
        user_id: &Uuid,
        notification_id: &Uuid,
    ) -> Result<Notification, ApiError>;
}
```

Where:

```rust
pub enum CreateNotificationResult {
    Created(Notification),
    AlreadyExists(Notification),
}

pub struct NotificationPage {
    pub items: Vec<Notification>,
    pub next_cursor: Option<NotificationCursor>,
}
```

next_cursor = None means there is no further page.

Repeat emit does not return 409. It returns AlreadyExists and the
existing notification.

---

## 12. Service Layer

```rust
#[async_trait]
pub trait NotificationApi: Send + Sync {
    async fn emit(
        &self,
        user_id: &Uuid,
        kind: NotificationKind,
        source_id: &str,
        payload: serde_json::Value,
    ) -> Result<Notification, ApiError>;

    async fn list(
        &self,
        user_id: &Uuid,
        cursor: Option<String>,
        limit: usize,
    ) -> Result<NotificationPageResponse, ApiError>;

    async fn unread_count(
        &self,
        user_id: &Uuid,
    ) -> Result<UnreadCountResponse, ApiError>;

    async fn mark_read(
        &self,
        user_id: &Uuid,
        notification_id: &Uuid,
    ) -> Result<NotificationResponse, ApiError>;
}
```

The service owns:

· validation of source_id;
· validation of payload size;
· validation of pagination cursor and limit;
· UUID generation;
· idempotency orchestration;
· authorization by user_id.

The service does not own:

· concurrency primitives (they live in storage);
· realtime delivery (see §13).

---


## 13. Delivery Sink

```rust
#[async_trait]
pub trait NotificationSink: Send + Sync {
    async fn deliver(
        &self,
        notification: &Notification,
    ) -> Result<(), NotificationSinkError>;
}
```

Rules:

· storage commit happens before sink delivery;
· sink failures do not roll back storage;
· sink failures are logged, not surfaced to the emit caller;
· v1 may ship a NoopNotificationSink;
· SSE/WebSocket implementations may be added later without changing the
  storage or service contracts.

---

## 14. HTTP API

All endpoints require authentication.

```text
GET  /api/v1/community/notifications
GET  /api/v1/community/notifications/unread-count
POST /api/v1/community/notifications/{id}/read
```

List

```text
GET /api/v1/community/notifications?limit=20&cursor=...
```

Response:

```json
{
  "items": [],
  "next_cursor": null
}
```

Unread count

Response:

```json
{
  "count": 3
}
```

Mark read

```text
POST /api/v1/community/notifications/{id}/read
```

Response: the current notification state (200).

404 is returned both when the notification does not exist and when it
belongs to another user.

CSRF applies because POST is mutating.

---

## 15. Security Invariants

· All authorization is performed against the authenticated user_id.
· Notification IDs are never treated as globally readable resources.
· Public responses MUST NOT leak notifications that belong to another
  user, and MUST NOT reveal existence through status-code differentiation.
· Payload size is bounded.
· source_id length and character set are bounded.
· Pagination cursor is validated and rejected on corruption.
· No scan is used for unread count.

---


## 16. Known Limitations and Backlog

### AevumDB-TX-1

Notification idempotency in v1 relies on the same single-instance
invariant as Community usernames. Multi-instance correctness requires
AevumDB-TX-1. See `docs/backlog/aevumdb-tx-1.md`.

### Retention

Notifications are stored indefinitely in B-2. A retention policy will be
designed separately once operational requirements are established.

### Realtime

SSE / WebSocket delivery is deferred. The `NotificationSink` trait is the
extension point.

### Forum producers

Forum, Comments, and other producers call `NotificationService::emit`
starting from their respective phases (Forum begins in B-3).

---

## 17. Architectural Rules for Future Phases

- New notification kinds are added only when a producer exists.
- Producers must not write notification storage keys directly.
- Producers must go through the service layer.
- Realtime delivery must not mutate storage state.
- Retention must not change idempotency semantics.
- AevumDB-TX-1 must not change the service or HTTP contracts, only the
  storage concurrency guarantees.

---

## 18. Test Coverage

B-2 covers:

- source_id validation (length, control chars);
- payload size validation;
- idempotent emit (Created + AlreadyExists);
- concurrent idempotent emit;
- list pagination ordering and cursor semantics;
- invalid cursor rejection;
- limit validation;
- unread count derived from index;
- mark_read first call;
- mark_read repeat call (idempotent, preserves read_at);
- cross-user 404 semantics;
- authentication;
- CSRF enforcement;
- HTTP endpoints behaviour.

---

## 19. Implementation Status

Notifications v1 is implemented, tested, and committed as of commit
`9bfb7dc` (B-2.4). The B-2.5 integration suite is committed on top.

### Completed phases

| Phase | Scope | Tests |
|-------|-------|-------|
| B-2.0 | Architecture contract | — (document) |
| B-2.1 | Models, validation, contracts | +30 unit |
| B-2.2a | NotificationStorage trait + InMemory | +21 unit |
| B-2.2b | AevumDbNotificationStorage | +19 unit |
| B-2.3a | Sink abstraction + NotificationApi trait | +6 unit |
| B-2.3b | NotificationService | +19 unit |
| B-2.4 | HTTP handlers + wiring | +3 unit |
| B-2.5b | Integration tests | +13 integration |

### Delivered surface

- Three HTTP endpoints (list / unread-count / mark-read)
- Cursor-based pagination with UUID tie-breaker
- Idempotent emit keyed by `(user_id, kind, sha256(source_id))`
- Durable storage with unread index
- Post-commit best-effort sink delivery
- Public DTO whitelist without `user_id`

### Test coverage

All B-2 tests are green with zero regressions in AUTH-10..28 and B-1.

### Post-B-2 follow-ups

- `AevumDB-TX-1` — conditional write primitive for multi-instance
  uniqueness (see `docs/backlog/aevumdb-tx-1.md`).
- Retention policy — deferred until operational requirements exist.
- Realtime delivery — `NotificationSink` trait is the extension point.
- Producer integration — Forum (B-3) will call `NotificationService::emit`.
