# Aevum Platform Storage Standard v1

Status: **Draft — for review before canonical**
Owner: Platform Architecture
Applies to: all platform services in `/root/aevum-platform/backend/src/`

Normative language follows RFC 2119: **MUST**, **MUST NOT**,
**SHOULD**, **SHOULD NOT**, **MAY**.

---

## 1. Status and Scope

This document defines how every platform service persists data.

It applies to all services that own durable state:

- Auth
- Community
- Growth
- Content (planned)
- Comments (planned)
- Forum (planned)
- Chat (planned)
- Analytics (planned)

This document does **not** define:

- L1/L2 protocol state (see `aevum-protocol` docs);
- in-memory-only caches;
- HTTP request/response shapes (see per-service API docs);
- AevumDB internal storage engine design (see `aevum-db/audit/`).

The goal is that any engineer can open a new service, read this
document, and produce storage code that is consistent with the rest
of the platform without inventing new conventions.

---

## 2. Architectural Boundary

The platform uses a strict four-layer boundary:
Service
down to
Storage trait (async, domain-typed)
down to
Service-specific adapter
down to
AevumDB (ordered KV engine)

text

Rules:

- AevumDB **MUST NOT** know anything about platform domains.
- Service code **MUST NOT** call AevumDB directly; it **MUST** go
  through a `Storage` trait owned by the service.
- Adapters **MUST** own key layout, serialization, atomicity, and
  index maintenance.
- There **MUST NOT** be cyclic dependencies between service layers.
- Protocol state and platform state **MUST** remain separate storage
  systems.

---

## 3. Storage Layers

Every service defines **one or more** storage traits, one per entity class.

Example (Notifications):

```rust
#[async_trait]
pub trait NotificationStorage: Send + Sync {
    async fn create(...) -> Result<...>;
    async fn list(...) -> Result<...>;
    async fn unread_count(...) -> Result<...>;
    async fn mark_read(...) -> Result<...>;
}
```

Each trait:

MUST be async and return Result<_, ApiError>;

MUST express "not found" as Ok(None), not as an error;

MUST NOT expose an unbounded list_all();

list methods MUST take an explicit limit: usize and (when
applicable) cursor: Option<Cursor>.

Every service SHOULD provide two implementations:

InMemory*Storage for dev/test;

AevumDb*Storage for production.

Behavioral equivalence between them is a conformance requirement
(see section 20).

---

## 4. Namespace Rules

Every persistent key **MUST** begin with a domain root:
auth:
community:
growth:
content:
forum:
comments:
chat:
analytics:

text

`platform:` is **NOT** a canonical prefix and **MUST NOT** be used in
new code. It survives only in historical adapters until they are
migrated.

Namespace rules:

- The domain root **MUST** reflect the owning domain, not the caller.
- A key **MUST NOT** be shared between domains.
- A domain **MUST NOT** read another domain's keys directly.
- Cross-domain data flows **MUST** go through service APIs, not
  through shared keys.

Rationale: clean ownership, simple auditing of retention/deletion/
security boundaries, no artificial global layer.

---

## 5. Key Layout

Every key **MUST** follow the shape:
<root>:<entity>:<component>...

text

Where:

- `<root>` — domain root from section 4;
- `<entity>` — a stable entity name (e.g. `user`, `session`, `profile`);
- `<component>...` — one or more components separated by `:`.

Components **MUST** be produced deterministically from stable inputs.

Key builders **MUST** live in one place per adapter and **MUST** be
covered by the Persistent Key Layout Contract (section 20).

---

## 6. Primary Keys

Primary keys **MUST** uniquely identify a single logical record.

Canonical shape:
<root>:<entity>:<id>

text

Example:
auth:user:id:{user_id}
community:profile:{user_id}
community:notifications:{user_id}:{notification_id}

text

Primary key components:

- **MUST** be normalized (see section 11) before concatenation;
- **MUST** use a stable encoding for UUIDs (see section 10);
- **MUST NOT** embed free-form user input.

---

## 7. Secondary Indexes

When a lookup by another field is required, the adapter **MUST** maintain
an explicit secondary index key.

Canonical shape:
<root>:<entity>:<field>:<value>:<primary_id>

text

Example:
community:profile:username:{normalized_username}:{user_id}

text

Rules:

- Secondary indexes **MUST** be written in the same batch as the primary.
- Secondary indexes **MUST** be deleted in the same batch as the primary.
- A secondary index **MUST** be treated as derived data; it is
  rebuildable from the primary.
- Adapters **SHOULD** provide a rebuild path for every secondary index.
- Adapters **MUST NOT** read primary data through a secondary index
  without an ownership check.

---

## 8. Prefix Scans

Prefix scans are the primary mechanism for list operations.

Rules:

- A prefix **MUST** end with a `:` separator when it addresses a
  specific field value.
- Example: to scan all sessions for a user, use
  `auth:user:session:by_user:{user_id}:`, not
  `auth:user:session:by_user:{user_id}`.
- Adapters **MUST NOT** assume that `prefix_scan` returns results in
  logical domain order; logical order **MUST** be produced by the key
  layout itself (see section 9).

---

## 9. Timestamp and Ordering

Any key component that represents time **MUST** declare:

- domain (UTC),
- unit (seconds, millis, micros, nanos),
- numeric representation (unsigned, signed),
- width (fixed-width decimal),
- ordering semantics (lexicographic).

Canonical default:
unit: microseconds
format: format!("{:020}", ts_micros)

text

For descending logical order, adapters **MUST** use an inverted
timestamp:
inv_ts = u64::MAX - ts_micros
format!("{:020}", inv_ts)

text

Rules:

- A timestamp **MUST NOT** be the sole ordering key when ties are
  possible. A stable tie-breaker **MUST** be appended.
- Canonical tie-breaker: the entity's UUID (see section 10).
- Example: `...:{inv_ts}:{notification_id}`.
- Adapters **MUST NOT** rely on a reverse iterator. DESC is achieved
  through key layout, not storage-engine behavior.
- If a workload genuinely requires a different timestamp unit, the
  adapter **MUST** declare it explicitly; the default MUST NOT be
  silently changed.

---

## 10. UUID Encoding

UUIDs used as key components **MUST** use a deterministic, stable
encoding.

Canonical encoding:
lowercase hyphenated hex (uuid.to_string())

text

Rules:

- The encoding **MUST** be stable across restarts and processes.
- The encoding **MUST** be documented in the adapter's key builders.
- A future migration to a compact binary encoding is **allowed**
  provided it is declared per-domain and covered by the Persistent
  Key Layout Contract (section 20).
- Adapters **MUST NOT** switch encodings without a versioned key
  migration.

---

## 11. String Normalization

Any string used as a key component **MUST** be normalized before use.

Required normalization:

- lowercase (for case-insensitive fields);
- trim leading/trailing whitespace;
- Unicode NFC;
- reject control characters.

Normalization **MUST** happen in the adapter, before key construction.
AevumDB **MUST NOT** be expected to normalize.

Adapters **MUST** declare in code which normalization is applied and
why (e.g. `username` normalization).

---

## 12. Atomicity

Any write that affects more than one key **MUST** use a single
`db.batch().commit()`.

This applies to:

- primary + secondary indexes;
- primary + idempotency index;
- cascade deletions;
- state transitions that update both a record and an index.

Rules:

- Partial writes **MUST NOT** be possible after a successful commit.
- On batch failure, no key in the batch **MUST** be considered written.
- Adapters **MUST NOT** issue multi-key writes as separate `put`
  calls unless the semantics explicitly allow partial state.

---

## 13. Idempotency

When an operation must be safe to retry, the adapter **MUST** use an
explicit idempotency index.

Canonical shape:
<root>:<entity>:idempotency:{scope}:{hash}

text

Rules:

- The idempotency value **MUST** reference the primary record's ID.
- The idempotency index **MUST** be written in the same batch as the
  primary record.
- A read of the idempotency index **MUST** be performed under the same
  application-level lock used for the write.
- `put_if_absent` / CAS **MUST NOT** be assumed to exist in AevumDB
  until explicitly added and approved (see sections 21 and 22).
- Per-key application mutex + single-instance invariant is the
  accepted current mechanism.

---

## 14. Pagination and Cursors

List operations **MUST** be paginated.

Rules:

- Cursor **MUST** be an exclusive start key, never an offset.
- Cursor **MUST** be composed of the same fields as the ordering key.
- The page result **MUST** include `next_cursor: Option<Cursor>`.
- The adapter **MUST** determine `has_more` by reading one extra item,
  not by counting the full result set.
- List results **MUST** be deterministically ordered. The ordering
  function **MUST** be shared between in-memory and AevumDB
  implementations.
- Adapters **SHOULD** expose the cursor as a domain type, not as raw
  bytes.

---

## 15. Retention

Each entity **MUST** declare a retention policy. Canonical classes:
TTL
explicit deletion
legal / business retention
immutable
indefinite

text

Rules:

- Retention class **MUST** be declared in the service design doc.
- The concrete mechanism (prune-before-timestamp, TTL sweep, legal
  hold) is workload-specific and **MUST NOT** be assumed to be
  universal.
- When pruning is required, the adapter **MUST** implement it as
  `prefix_scan` + `batch(delete primary + secondary)`.
- Adapters **MUST NOT** rely on a `delete_prefix` primitive; there is
  no such primitive in AevumDB v1 (see section 21).

---

## 16. Deletion Semantics

Every entity **MUST** declare its deletion semantics:

- soft delete (tombstone in the record);
- hard delete (primary + all indexes removed);
- cascade (dependent records explicitly removed).

Rules:

- Cascade **MUST** be implemented explicitly in the adapter.
- Secondary indexes **MUST NOT** be left dangling after primary
  deletion.
- If soft delete is chosen, index updates **MUST** be atomic with the
  primary update.

---

## 17. Concurrency

AevumDB is a single-writer-per-instance storage engine.

Rules:

- Multi-process writers **MUST NOT** be assumed to be safe.
- Cross-process coordination **MUST** live above AevumDB (leader
  election, sharding, or explicit single-instance invariants).
- Within a process, adapters **SHOULD** use per-key locks for
  operations that require read-then-write atomicity.
- Adapters **MUST** document their lock ordering to prevent deadlocks.

---

## 18. Failure Semantics

Every adapter **MUST** declare behavior for:

- WAL append failure — operation fails; no partial state.
- Batch failure — all-or-nothing; no partial state.
- Process crash — WAL replay on `open`.
- Storage unavailable — error propagated to caller as `ApiError`.
- External failure (network, upstream service) — service-specific;
  MUST NOT corrupt storage state.

Best-effort operations (e.g. event recording for audit purposes)
**MAY** ignore failures, but **MUST** log them and **MUST NOT** silently
swallow them.

---

## 19. Observability

Each adapter **SHOULD** emit at least:

- operation count (per method);
- error count (per method);
- latency p50 / p95 / p99 (per method);
- scan size (items returned per list call);
- batch size (keys per batch).

These metrics **MUST NOT** be AevumDB-specific; they describe the
adapter.

AevumDB-level metrics (WAL bytes, SST count, memtable size, cache hit
ratio) are owned by AevumDB and consumed separately.

---

## 20. Testing and Conformance

Every adapter **MUST** provide:

- unit tests for happy path, rejections, and boundary cases;
- a Persistent Key Layout Contract test;
- a conformance test that the InMemory and AevumDb implementations
  produce equivalent observable behavior.

### Persistent Key Layout Contract

The key layout contract test **MUST** verify:

- exact bytes of every primary key builder output;
- exact bytes of every secondary index key builder output;
- exact bytes of every prefix builder output;
- lexicographic ordering of composite keys;
- prefix boundaries (a prefix MUST NOT match a sibling entity);
- timestamp encoding (width, unit, lexicographic order);
- cursor boundary behavior, where applicable.

Changing any of these **MUST** be treated as a breaking change
(see section 24).

---

## 21. AevumDB Capability Boundary

Platform adapters **MAY** use only the following AevumDB primitives
unless and until this document is amended:

| Capability             | Status                        |
| ---------------------- | ----------------------------- |
| `put`                  | production                    |
| `get`                  | production                    |
| `delete`               | production                    |
| `batch`                | production                    |
| `prefix_scan`          | production                    |
| `flush`                | operational                   |
| `stats`                | operational                   |
| `prefix_scan_limited`  | available, workload-driven    |
| `range_scan`           | not yet justified/implemented |
| `delete_prefix`        | not required currently        |
| `put_if_absent` / CAS  | not required currently        |
| reverse iterator       | not required                  |

Rules:

- `prefix_scan_limited` **MAY** be used by an adapter when bounded
  traversal materially improves the workload and the adapter contract
  remains unchanged. It **MUST NOT** be treated as a correctness
  requirement.
- `range_scan` **MUST NOT** be assumed to exist. If a workload
  genuinely requires it, the request follows section 22.
- `delete_prefix` **MUST NOT** be assumed to exist. Cascade deletions
  are implemented via `prefix_scan` + `batch`.
- `put_if_absent` / CAS **MUST NOT** be assumed to exist. Idempotency
  is implemented via explicit idempotency indexes (section 13). The
  AevumDB team maintains a separate backlog item (`AevumDB-TX-1`) for
  future consideration.
- A reverse iterator **MUST NOT** be required. DESC ordering is
  achieved through key layout (section 9).

This boundary is **not a permanent freeze**. It is a statement of the
current state and the process by which it changes.

---

## 22. Capability Extension Process

AevumDB capabilities **MUST NOT** be added for convenience.

Any new capability **MUST** follow this pipeline:
Requirement
down to
Architecture
down to
Contract
down to
Workload (real, measured)
down to
Invariant
down to
Failing Test (RED)
down to
Implementation
down to
Regression
down to
Load
down to
Measurement
down to
Targeted Optimization

text

Rules:

- Each step **MUST** produce an artifact (doc, test, measurement).
- A capability **MUST NOT** be added without a concrete consumer.
- A capability **MUST NOT** be added because "it might be useful".
- Once added, a capability **MUST** be documented in section 21 with
  its status and its justification.

This process preserves the property that AevumDB is capability-driven,
not feature-driven.

---

## 23. Real Production Examples

The following adapters currently conform to this standard (modulo
namespace migration, see section 4):

### Auth
auth:user:email:{email}
auth:user:id:{uuid}
auth:user:session:token:{hash}
auth:user:session:by_user:{uuid}:{session_id}
auth:user:backup_code:{uuid}
auth:user:2fa:{uuid}
auth:user:preferences:{uuid}
auth:user:avatar:{uuid}

text

Notes: point lookups + one prefix scan (sessions per user,
backup codes per user). No range scans. No pagination.

### Security Events
auth:event:id:{event_id}
auth:event:user:{user_id}:{ts_micros:020}:{event_id}
auth:event:type:{kind}:{ts_micros:020}:{event_id}
auth:event:timeline:{ts_micros:020}:{event_id}

text

Notes: prefix scans per user, per type, and global timeline.
Timestamp already fixed-width. DESC is achieved via inverted ts
when required (next revision).

### Community
community:profile:{user_id}
community:profile:username:{normalized}:{user_id}
community:role:{user_id}

text

Notes: point lookups + one secondary index. No scans.

### Notifications
community:notifications:{user_id}:{notification_id}
community:notifications:unread:{user_id}:{notification_id}
community:notifications:idempotency:{user_id}:{kind}:{source_hash}

text

Notes: primary + unread index + idempotency index. Cursor
pagination is implemented at the adapter level over
`prefix_scan`. DESC is currently achieved by in-memory sort; the
next revision will move DESC into the key layout per section 9.

### Growth (planned)

To be filled in when Growth storage is finalized.

---

## 24. Versioning and Breaking Changes

This document is versioned: `storage-standard-vN.md`.

Rules:

- Adding a new MUST / MUST NOT **MUST** produce a new version.
- Changing an existing key layout **MUST** be treated as a breaking
  change and **MUST** be accompanied by a migration plan.
- Removing a capability from section 21 **MUST** be justified by the
  same process as adding one (section 22).
- Adapters **MUST** declare which version of this standard they
  conform to.

---

## 25. Non-goals

This standard explicitly does **NOT** attempt to:

- define a universal query language;
- define SQL or an ORM;
- provide cross-domain joins;
- provide cross-domain transactions;
- embed business logic in AevumDB;
- replace per-service API design;
- replace per-service security design.

Storage is a low-level capability. Domains own their semantics.

---

## Appendix A — Changelog

- v1 (draft) — initial standard, derived from existing Auth, Security
  Events, Community, and Notifications adapters, plus the AevumDB
  capability audit.
