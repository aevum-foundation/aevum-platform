# Storage Standard v1 — Adapter Audit

Status: **Initial audit**
Companion to: `storage-standard.md`
Date: v1 draft

This document records the current conformance status of the four
production storage adapters against `AEVUM PLATFORM STORAGE STANDARD v1`.

It is a factual audit only. It does not prescribe migrations.
Migrations are decided separately when a real workload requires them.

---

## Scope

Audited adapters:

- `AevumDbAuthStorage` (`auth/aevumdb_storage.rs`)
- `AevumDbSecurityEventStorage` (`auth/events/aevumdb.rs`)
- `AevumDbCommunityStorage` (`community/aevumdb_storage.rs`)
- `AevumDbNotificationStorage` (`community/notifications/aevumdb_storage.rs`)

Conformance points checked (per STANDARD v1):

1. Domain-root namespace (section 4)
2. Timestamp encoding (section 9)
3. DESC ordering strategy (section 9)
4. Atomicity of multi-key writes (section 12)
5. Idempotency via explicit index (section 13)
6. Persistent Key Layout Contract completeness (section 20)
7. InMemory / AevumDb conformance tests (section 20)

---

## Summary Table

| Adapter                  | §1 Namespace | §2 Timestamp | §3 DESC          | §4 Atomicity | §5 Idempotency | §6 Key Contract | §7 Conformance |
| ------------------------ | ------------ | ------------ | ---------------- | ------------ | -------------- | --------------- | -------------- |
| AuthStorage              | non-conform  | n/a          | n/a              | conform      | n/a            | partial         | missing        |
| SecurityEventStorage     | non-conform  | conform      | ASC only         | conform      | n/a            | partial         | missing        |
| CommunityStorage         | non-conform  | n/a          | n/a              | conform      | n/a            | partial         | missing        |
| NotificationStorage      | non-conform  | missing      | in-memory sort   | conform      | conform        | partial         | missing        |

Legend:

- conform — matches STANDARD v1
- non-conform — deviates from STANDARD v1
- partial — partially implemented
- missing — not implemented
- n/a — not applicable to this adapter

---

## Details per Adapter

### AuthStorage

Current keys:
platform:user:email:{email}
platform:user:id:{uuid}
platform:session:token:{hash}
platform:session:by_user:{uuid}:{session_id}
platform:auth:password_reset:{hash}
platform:auth:email_verification:{hash}
platform:auth:backup_code:{uuid}
platform:auth:two_factor:{uuid}
platform:auth:pre_auth:{hash}
platform:user:preferences:{uuid}

text

Findings:

- Namespace uses legacy `platform:` root (STANDARD v1 section 4).
- All multi-key writes use `db.batch()` (conform).
- No timestamps in keys (n/a for this adapter).
- No DESC ordering requirement.
- No idempotency index required.
- `key_layout_is_stable` covers exact bytes only; ordering,
  prefix boundaries, and cursor checks are absent.
- No InMemory/AevumDb conformance test.

### SecurityEventStorage

Current keys:
platform:audit:event:{event_id}
platform:audit:user:{user_id}:{ts_micros:020}:{event_id}
platform:audit:type:{kind}:{ts_micros:020}:{event_id}
platform:audit:timeline:{ts_micros:020}:{event_id}

text

Findings:

- Namespace uses legacy `platform:audit:` root.
- Timestamp encoding conforms: `format!("{:020}", ts_micros)`.
- Prefix scans per user, per type, global timeline.
- DESC is not implemented; timeline is ASC. If DESC is ever
  required, it MUST use an inverted timestamp per section 9.
- `prune_before` uses `prefix_scan` + `batch(delete ...)` (conform).
- No InMemory/AevumDb conformance test.

### CommunityStorage

Current keys:
platform:community:profile:{user_id}
platform:community:profile:username:{normalized}:{user_id}
platform:community:role:{user_id}

text

Findings:

- Namespace uses legacy `platform:community:` root.
- Secondary index (username) written in batch with primary (conform).
- No scans, no pagination, no DESC, no idempotency.
- No InMemory/AevumDb conformance test.

### NotificationStorage

Current keys:
platform:community:notifications:{user_id}:{notification_id}
platform:community:notifications:unread:{user_id}:{notification_id}
platform:community:notifications:idempotency:{user_id}:{kind}:{source_hash}

text

Findings:

- Namespace uses legacy `platform:community:` root.
- Timestamps are NOT in keys. DESC ordering is achieved by
  in-memory sort (`notification_desc_cmp`) after a full prefix scan.
  This is a latent scalability concern; it is NOT a correctness
  issue.
- Idempotency is implemented via an explicit idempotency index plus
  a per-key application lock plus a single batch (conform).
- Cursor pagination is implemented at the adapter level over
  `prefix_scan` (conform).
- No InMemory/AevumDb conformance test.

---

## Classification

Findings grouped by severity:

Systematic:

- Legacy `platform:` namespace in all four adapters.
- Conformance tests missing in all four adapters.

Latent:

- Notification DESC ordering is done in memory; it will not scale
  beyond a few thousand notifications per user without a key
  layout change.

Contract:

- Key Layout Contract tests verify exact bytes only; they do not
  verify ordering, prefix boundaries, timestamp encoding, or
  cursor boundaries.

Conforming:

- Atomicity: all multi-key writes use `db.batch()`.
- Idempotency (Notifications): explicit index, lock, batch.
- Timestamp encoding (Security Events): fixed-width micros.
- Cascade deletes: implemented via explicit `batch(delete ...)`.

No adapter has a correctness defect.

---

## Migration Plan (not scheduled)

Tier 1 — mechanical, low risk:

1. Replace legacy `platform:*` prefixes with domain roots:
   - `platform:user:*` → `auth:user:*`
   - `platform:session:*` → `auth:session:*`
   - `platform:auth:*` → `auth:*`
   - `platform:audit:*` → `auth:event:*`
   - `platform:community:*` → `community:*`
2. Update `key_layout_is_stable` tests accordingly.
3. No data migration required — platform is pre-production.

Tier 2 — workload-driven:

4. Move Notification DESC ordering into the key layout using an
   inverted timestamp:
community:notifications:{user_id}:{inv_ts}:{notification_id}

text
Requires rewrite of `list()` and cursor semantics.
5. Strengthen Key Layout Contract tests to cover ordering,
prefix boundaries, and cursor boundaries.

Tier 3 — hygiene:

6. Add InMemory/AevumDb conformance tests per adapter.

Nothing above is scheduled until a real workload requires it.

---

## Verdict

The four production adapters are functionally correct.

They are not yet fully conformant with STANDARD v1 in three
systematic areas (namespace, conformance tests, key layout contract
completeness) and one latent area (Notification DESC ordering).

None of these are blocking. New services (starting with Growth)
MUST be built conformant to STANDARD v1 from the beginning.
Existing adapters MAY be migrated when a workload justifies it.

---

## Appendix — Method

The audit was performed by reading the adapter source files and
grepping for key builders, prefix constants, and
`key_layout_is_stable` tests. No code was executed.

- `docs/architecture/storage-standard.md` — the standard being
audited against.
- `backend/src/auth/aevumdb_storage.rs`
- `backend/src/auth/events/aevumdb.rs`
- `backend/src/community/aevumdb_storage.rs`
- `backend/src/community/notifications/aevumdb_storage.rs`
