# Growth Storage Design v1

Status: **Draft — for review before implementation**
Owner: Growth domain
Companion to: `storage-standard.md`, `storage-standard-v1-audit.md`
Scope: key layout, encoding, ordering, retention, deletion for
`AevumDbGrowthStorage`.

This document fixes the concrete decisions that the Growth adapter
MUST implement. It is the contract between the Growth domain and the
AevumDB storage engine.

It is written **before** the adapter code. The adapter MUST conform
to this document, not the other way around.

---

## 1. Scope and Non-goals

In scope:

- primary key layout for Source, Publication, Opportunity,
  TopicState, GrowthEvent;
- secondary index layout for the same;
- timestamp and score encoding;
- ordering contract per list method;
- retention and deletion semantics;
- idempotency / uniqueness strategy;
- batching strategy for multi-key writes and prune;
- Key Layout Contract test plan.

Out of scope:

- service logic (`service.rs`);
- HTTP layer (`api.rs`);
- ingestion (`ingestion/`);
- analysis algorithms (`analysis/`);
- retention pruning schedule;
- observability metrics beyond adapter-local counters.

---

## 2. AevumDB Capability Assumptions

Growth uses only the following AevumDB primitives:

| Capability             | Usage in Growth                                  |
| ---------------------- | ------------------------------------------------ |
| `put`                  | single-record writes                             |
| `get`                  | point lookups by primary key                     |
| `delete`               | record removal, index cleanup                    |
| `batch`                | multi-key atomic writes (primary + secondary)    |
| `prefix_scan`          | list operations, index enumeration               |
| `prefix_scan_limited`  | bounded lists (desirable, not required)          |

`prefix_scan_limited` is **desirable**, not required.
If unavailable, the adapter MAY fall back to `prefix_scan` + a
bounded `take(limit)` at the adapter level. This fallback is a
correctness-neutral implementation detail.

The following primitives are **NOT used** by Growth and MUST NOT
be assumed to exist:

- `range_scan`
- `delete_prefix`
- `put_if_absent` / CAS
- reverse iterator

DESC ordering is achieved through key layout (inverted timestamp
or inverted score), not through engine behavior.

---

## 3. Domain Root and Namespace

All Growth keys use the domain root:
growth:


No other prefix is permitted. `platform:` is not used.

Namespace rule: every key begins with `growth:` and the second
component identifies the entity class (`source`, `publication`,
`opportunity`, `topic`, `event`, `metric`).

---

## 4. Identifier Encoding

Growth uses two identifier encodings, chosen to match the semantic
nature of each entity.

| Entity          | Type              | Encoding               | Length |
| --------------- | ----------------- | ---------------------- | ------ |
| `SourceId`      | deterministic     | lowercase hex, 16 byte | 32     |
| `PublicationId` | deterministic     | lowercase hex, 16 byte | 32     |
| `OpportunityId` | non-deterministic | UUID v4, hyphenated    | 36     |
| `GrowthEventId` | non-deterministic | UUID v4, hyphenated    | 36     |

Rationale:

- Source and Publication have deterministic identities (derived
  from content). A compact hex encoding makes keys shorter and
  fully deterministic.
- Opportunity and Event are unique-per-occurrence. UUID v4 is
  appropriate and improves log readability.

Adapters MUST NOT unify these encodings without a versioned key
migration. See STORAGE STANDARD v1 section 10.

---

## 5. Timestamp and Score Encoding

Per STORAGE STANDARD v1 section 9.

Timestamp encoding for ordered keys:
ts_micros = timestamp.timestamp_micros()
asc = format!("{:020}", ts_micros)
desc = format!("{:020}", u64::MAX - ts_micros)


All timestamps use UTC, microseconds, unsigned, width 20.

Score encoding for ordered keys:
score_bp ∈ [0, 10_000]
inv_score = u64::MAX - (score_bp as u64)
desc = format!("{:020}", inv_score)


Score is always stored in basis points (`u32`), but the encoded
value is `u64` to guarantee a uniform width of 20 digits.

Every ordered key component MUST be followed by a stable
tie-breaker (entity id) to guarantee deterministic ordering when
primary ordering values collide.

---

## 6. Effective Timestamp

Publications have two timestamp fields:

- `published_at: Option<DateTime<Utc>>` — informational, from the feed.
- `ingested_at: DateTime<Utc>` — always present, indexing time.

Growth defines:
effective_ts = published_at.unwrap_or(ingested_at)


`effective_ts` is the ONLY timestamp used in Publication indexes.
`published_at` remains available in the stored record but MUST
NOT be used for indexing.

Rationale:

- publications never drop out of time indexes;
- RSS and Atom feeds sometimes omit dates;
- every index entry has exactly one ordering key;
- adapter logic stays simple and deterministic.

---

## 7. Ordering Rule (General)

Growth applies the following architectural rule:

- User-facing list methods return **DESC** order.
- Maintenance scans (prune, rebuild, recovery) use **ASC** order.

This rule explains why inverted timestamps and inverted scores
appear in user-facing indexes, while maintenance indexes may use
the plain ascending encoding.

The rule MUST be reflected in the ordering contract table
(section 16).

---

## 8. Key Layout — Source

Primary:
growth:source:id:{source_id}


Secondary indexes:
growth:source:by_topic:{topic}:{source_id}
growth:source:by_status:{status}:{source_id}


Notes:

- `source_id` is hex-32.
- `platform` is `rss` or `atom`.
- `handle` is the source's canonical handle, normalized by the
  ingestion layer before write.
- `topic` is one of the six fixed verticals.
- `status` is one of `active`, `paused`, `error`, `disabled`.
- No timestamp or score is involved; Source lists are ordered by
  `source_id` ASC for determinism.

Uniqueness:

- There is no storage-level uniqueness constraint on Source in
  Phase 1. Uniqueness is provided by `SourceId`, which is
  deterministic over `(platform, feed_url)`: two writes for the
  same feed overwrite the same primary key.

---

## 9. Key Layout — Publication

Primary:
growth:publication:id:{publication_id}


Secondary indexes:
growth:publication:by_source:{source_id}:{inv_ts}:{publication_id}
growth:publication:by_topic:{topic}:{inv_ts}:{publication_id}
growth:publication:by_time:{inv_ts}:{publication_id}


Where:

- `inv_ts = u64::MAX - effective_ts_micros`, encoded as `{:020}`.
- `publication_id` is hex-32 and acts as the tie-breaker.
- `by_time` provides a global recent-publications feed.
- `by_source` and `by_topic` provide filtered feeds.

Deterministic lookup index:
growth:publication:by_external:{source_id}:{external_id}


Used by `publication_exists(source_id, external_id)` to answer in
O(1) without scanning publications.

`by_external` is NOT a storage-level uniqueness constraint. It is
a deterministic lookup keyed by `(source_id, external_id)`.
Uniqueness is guaranteed by `PublicationId`, which is itself
deterministic:

    PublicationId = SHA256("pub:" + source_id + ":" + external_id)[..16]

Two publications with the same `(source_id, external_id)` produce
the same `PublicationId` and therefore overwrite the same primary
key. There is no possibility of two logical publications sharing
the same external identity.

All publication indexes MUST be written in the same batch as the
primary. They MUST be deleted in the same batch as the primary
when publication removal is ever allowed (Phase 1: publications
are immutable and are not deleted).

---

## 10. Key Layout — Opportunity

Primary:
growth:opportunity:id:{opportunity_id}


Secondary indexes:
growth:opportunity:by_topic:{topic}:{inv_score}:{opportunity_id}
growth:opportunity:by_kind:{kind}:{inv_score}:{opportunity_id}
growth:opportunity:by_score:{inv_score}:{opportunity_id}


Where:

- `inv_score = u64::MAX - (score_bp as u64)`, encoded as `{:020}`.
- `opportunity_id` is UUID v4 (hyphenated), acts as tie-breaker.
- `by_score` provides a global top-opportunities feed.
- `by_topic` and `by_kind` provide filtered feeds.

All three indexes are written in the same batch as the primary
`put_opportunity`. They are deleted in the same batch when an
Opportunity is hard-deleted.

---

## 11. Key Layout — TopicState

Primary (and only) key:
growth:topic:state:{topic}


Notes:

- There is exactly one TopicState per Topic.
- `Topic::ALL` is a fixed set of six entries; therefore
  `list_topic_states()` has no `limit` parameter (see
  STORAGE STANDARD v1 section 3 for the justification).
- No secondary indexes.
- No timestamps in the key; ordering is by `topic` ASC when the
  fixed set is returned.

Topic itself is a compile-time enum. There is no persisted
`growth:topic:{topic}` record in Phase 1. If a Topic record is
introduced later, it MUST use:
growth:topic:{topic}


---

## 12. Key Layout — GrowthEvent

Primary:
growth:event:event:{event_id}


Secondary indexes:
growth:event:timeline:{inv_ts}:{event_id}
growth:event:timeline_asc:{ts}:{event_id}
growth:event:by_kind:{kind}:{inv_ts}:{event_id}
growth:event:by_topic:{topic}:{inv_ts}:{event_id}


Where:

- `inv_ts = u64::MAX - occurred_at_micros`, encoded as `{:020}`.
- `ts = occurred_at_micros`, encoded as `{:020}`.
- `event_id` is UUID v4 (hyphenated), acts as tie-breaker.

Two timeline indexes are maintained because the two read
operations require opposite orders:

- `timeline` (inverted timestamp) provides DESC order for
  `get_recent_events` and for the `prune_events_before` scan.
- `timeline_asc` (ascending timestamp) provides ASC order for
  `get_events_since`.

`by_kind` and `by_topic` use inverted timestamp and provide DESC
order, matching `get_recent_events` semantics.

Events are append-only. There is no `delete_event` primitive.
Events are removed only via `prune_events_before(ts)`.

For `get_events_since(ts, limit)` the adapter currently performs a
full scan of `timeline_asc`, filters `occurred_at >= ts` in the
adapter, and truncates to `limit`. This is correctness-neutral but
not optimal. A future AevumDB `range_scan(start, end, limit)`
primitive will replace the full scan without changing the public
contract.

`prune_events_before(ts)` currently scans the full `timeline` and
filters `occurred_at < ts`. It does not use the ordering as an
early-exit optimization. A future `range_scan` will allow the
scan to stop at the cutoff.

All event indexes are written in the same batch as the primary.

---

## 13. Uniqueness and Idempotency

Growth applies the following uniqueness rules:

- **Source**: no storage-level uniqueness constraint in Phase 1.
  `SourceId` is deterministic (see section 4), so two writes for
  the same `(platform, feed_url)` produce the same primary key
  and overwrite the same record. A `by_handle` index will be
  introduced when the `Source` model gains an explicit `handle`
  field (see section 23).
- **Publication**: no storage-level uniqueness constraint.
  `PublicationId` is deterministic (see section 4), so two writes
  for the same `(source_id, external_id)` produce the same
  primary key and overwrite the same record. The `by_external`
  index (section 9) is a deterministic lookup for
  `publication_exists`, not a constraint.
- **Opportunity**: no uniqueness constraint. Each detection is a
  new record.
- **GrowthEvent**: no uniqueness constraint. Each emission is a
  new record.

Concurrency:

- Growth storage is a single-writer-per-instance subsystem in
  Phase 1 (single manual ingestion trigger). No cross-process
  coordination is attempted.
- Per-key application locks are **not** used in Phase 1 because
  the ingestion flow is single-threaded per Source.
- If concurrent ingestion is introduced later, the adapter MUST
  add per-key locks before serializing writes. This is a
  documented future extension, not a Phase 1 requirement.

No `put_if_absent` or CAS primitive is required. Uniqueness is
achieved via explicit secondary indexes and read-then-write under
a single-instance invariant.

---

## 14. Retention Policy

Declared per STORAGE STANDARD v1 section 15.

Phase 1 does not implement any pruning. Retention classes are
declared so that future pruning logic has an explicit contract.

| Entity        | Retention class | Notes                                    |
| ------------- | --------------- | ---------------------------------------- |
| Source        | indefinite      | registry, removed only by manual action  |
| Publication   | indefinite      | immutable once stored                    |
| TopicState    | indefinite      | one record per topic, replaced on update |
| Opportunity   | indefinite      | no pruning in Phase 1                    |
| GrowthEvent   | 30 days (soft)  | pruning supported via `prune_events_before` |

GrowthEvent is the only entity with a non-indefinite retention
class in Phase 1. `prune_events_before(ts)` is available but is
NOT scheduled automatically. The Phase 1 operator triggers it
manually.

---

## 15. Deletion Semantics

Declared per STORAGE STANDARD v1 section 16.

| Entity        | Semantics     | Notes                                         |
| ------------- | ------------- | --------------------------------------------- |
| Source        | soft delete   | sets `status = disabled`; record is kept      |
| Publication   | immutable     | never deleted in Phase 1                      |
| TopicState    | replace       | single record, overwritten on update          |
| Opportunity   | hard delete   | primary + all three secondary indexes removed |
| GrowthEvent   | prune only    | removed only via `prune_events_before`        |

Cascade rules:

- **Publication is immutable after write.** Its `published_at`,
  `ingested_at`, `topics`, `source_id`, and `external_id` MUST NOT
  change once stored. `put_publication` is therefore an idempotent
  upsert keyed by deterministic `PublicationId`; it does not
  remove stale index entries.
- Source soft delete MUST NOT remove its Publications. Publications
  remain part of the historical record.
- If a Source is ever hard-deleted (future), the adapter MUST
  cascade-delete its Publications and all their indexes, in
  batches. This is a future extension, not Phase 1.
- Opportunity hard delete MUST remove all three secondary indexes
  in the same batch as the primary delete.
- GrowthEvent prune MUST remove the primary and all three
  secondary indexes for each removed event.

The adapter MUST NOT leave any orphan secondary index entries
under any deletion path.

---

## 16. Ordering Contract (per method)

This table is normative. Adapters MUST return results in the
specified order. Changing any row is a breaking change to the
Growth API.

| Method                          | Order               |
| ------------------------------- | ------------------- |
| `list_sources`                  | source_id ASC       |
| `list_sources_by_topic`         | source_id ASC       |
| `list_sources_by_status`        | source_id ASC       |
| `list_publications_by_source`   | effective_ts DESC   |
| `list_publications_by_topic`    | effective_ts DESC   |
| `list_recent_publications`      | effective_ts DESC   |
| `list_opportunities_by_topic`   | score_bp DESC       |
| `list_opportunities_by_kind`    | score_bp DESC       |
| `list_top_opportunities`        | score_bp DESC       |
| `list_topic_states`             | topic ASC           |
| `get_events_by_kind`            | occurred_at DESC    |
| `get_events_by_topic`           | occurred_at DESC    |
| `get_recent_events`             | occurred_at DESC    |
| `get_events_since`              | occurred_at ASC     |

Tie-breaker in all cases: the entity id (hex-32 for Source /
Publication, UUID for Opportunity / GrowthEvent).

The ASC ordering of `get_events_since` is deliberate: a "since"
query reads forward in time. All other user-facing lists are DESC
per the general rule in section 7.

---

## 17. Batching Strategy

Multi-key writes MUST use a single `db.batch()` per logical
operation. This applies to:

- Source insert: primary + `by_topic` + `by_status`
- Source update: primary + affected indexes
- Publication insert: primary + `by_source` + `by_topic` +
  `by_time` + `by_external`
- Opportunity insert: primary + `by_topic` + `by_kind` + `by_score`
- Opportunity hard delete: primary + all three indexes
- TopicState update: primary only
- GrowthEvent record: primary + `timeline` + `by_kind` +
  `by_topic` (when topic is present)

Batch size for `prune_events_before` is capped by a private
implementation constant:
const PRUNE_BATCH_SIZE: usize = 500;


This constant is an implementation detail and MUST NOT be part
of the public contract. It MAY be tuned without a version bump.

`prune_events_before` iterates in batches, deleting one batch at
a time, until no more events match the cutoff. It returns the
total number of primary event records removed.

---

## 18. Failure Semantics

Per STORAGE STANDARD v1 section 18.

- **WAL append failure** → operation fails; no partial state.
- **Batch failure** → all-or-nothing; no partial state.
- **Process crash** → WAL replay on `open`.
- **Storage unavailable** → error propagated as `ApiError`.
- **Event recording failure** → best-effort at call site; the
  Growth operation MUST NOT fail because event recording failed.
  The adapter returns a `Result` so callers can observe failures
  when they want to.

The adapter MUST NOT swallow errors silently. Every failure path
returns a typed `ApiError` and is logged at `error` level.

---

## 19. prefix_scan_limited — Desirable, Not Required

Growth benefits from `prefix_scan_limited(prefix, limit)` for all
list methods with a `limit` parameter. Without it, the adapter
would read all matching keys and slice in memory.

This document treats `prefix_scan_limited` as **desirable**.

The adapter MUST be implementable today even if
`prefix_scan_limited` is not available. In that case it MAY fall
back to:
prefix_scan(prefix) + take(limit) in memory


The fallback is correctness-neutral. It is less efficient for
large prefixes. Growth MUST NOT block on the availability of
`prefix_scan_limited`.

If and when `prefix_scan_limited` is confirmed in production,
the adapter MAY switch to it without changing its public contract.

The same pattern applies to `range_scan(start, end, limit)`.
Growth currently performs full scans for `get_events_since` and
`prune_events_before` and filters in the adapter. When
`range_scan` becomes available, the adapter MAY switch to it
without changing the public contract.

The list methods most likely to benefit:

- `list_recent_publications`
- `list_publications_by_source`
- `list_publications_by_topic`
- `list_top_opportunities`
- `list_opportunities_by_topic`
- `list_opportunities_by_kind`
- `get_recent_events`
- `get_events_by_kind`
- `get_events_by_topic`
- `list_sources` / `list_sources_by_topic` / `list_sources_by_status`

---

## 20. Key Layout Contract Tests (Mandatory)

Per STORAGE STANDARD v1 section 20. These tests MUST exist
before the adapter is considered complete.

Required tests:

1. **Exact bytes** — every key builder is asserted against a
   literal expected string for a fixed set of inputs.
2. **Lexicographic ordering** — for a fixed pair of values,
   `key_a < key_b` must match the intended logical order (e.g.
   newer event sorts before older event in a DESC index).
3. **Prefix boundaries** — a prefix MUST NOT match a sibling
   entity. For example,
   `growth:source:by_topic:rust:` MUST NOT match
   `growth:source:by_topic:post_quantum:`.
4. **Timestamp encoding** — `format!("{:020}", ts)` MUST be
   monotone: for `ts_a < ts_b`, the encoded strings satisfy
   `enc_a < enc_b`.
5. **Inverted timestamp encoding** — for `ts_a < ts_b`,
   `inv(ts_a) > inv(ts_b)`.
6. **Score encoding** — the same monotonicity test for score.
7. **Inverted score encoding** — the same inversion test.
8. **Tie-breaker placement** — for equal primary ordering
   values, the id component decides; a test MUST cover this.

These tests are the contract. If any of them changes, the
adapter's key layout has changed and a migration plan is
required.

---

## 21. Non-goals

This document explicitly does NOT define:

- ingestion parsing rules;
- classification logic;
- trend computation;
- opportunity detection thresholds;
- HTTP request/response shapes;
- authentication or authorization;
- retention scheduling;
- analytics or metrics beyond adapter-local counters.

These belong to their respective modules.

---

## 22. Open Items

Documented here so that they are not lost:

- **Source handle normalization** — the exact normalization rules
  (lowercase, scheme stripping, trailing slash removal) are owned
  by the ingestion layer and MUST be documented there before
  Phase 1 ingestion begins.
- **Publication external_id normalization** — same as above.
- **Source hard delete cascade** — not implemented in Phase 1;
  documented in section 15.
- **Per-key locking for concurrent ingestion** — not implemented
  in Phase 1; documented in section 13.
- **AevumDB-TX-1** — CAS / put_if_absent is a separate AevumDB
  backlog item. Growth does not depend on it.

---

## Appendix A — Changelog

- v1 (draft) — initial design document. Derived from
  STORAGE STANDARD v1, the adapter audit, and the Growth Phase 1
  plan.
