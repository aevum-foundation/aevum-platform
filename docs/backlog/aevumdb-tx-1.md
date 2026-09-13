
AevumDB-TX-1 — Conditional Write Primitive

Status: Backlog

Priority: Medium

Phase: Core Infrastructure

Blocks: Multi-instance Community

Problem

AevumDB currently provides atomic WriteBatch commits but does not expose
a conditional write primitive such as:

Compare-And-Swap
PutIfAbsent

Community v1 therefore performs uniqueness checks at the application layer
under a per-key mutex.

This is safe under the current platform invariant:

single authoritative application writer

It is not sufficient for independent application instances/processes that
can concurrently claim the same unique key.

Goal

Add a conditional write operation to AevumDB, for example:

WriteOp::PutIfAbsent {
    key,
    value,
}

Required semantic:

if key does not exist:
    write value

if key already exists:
    abort the batch

The conditional operation must participate in AevumDB's existing atomic
batch/recovery model.

The exact failure representation and WAL/recovery semantics must be
designed as part of the AevumDB transaction contract before implementation.

Primary Use Case

Community username uniqueness:

platform:community:profile:username:{normalized}

The operation must prevent two independent writers from successfully
claiming the same normalized username.

Other Future Use Cases

The same primitive can support future unique-index claims, including:

platform:user:email:{email}

and other platform-level uniqueness constraints.

Current Workaround

Community v1 currently uses:

per-key async mutex
    +
get
    +
check
    +
atomic WriteBatch

This is intentionally bounded by the single-instance authoritative-writer
invariant.

The workaround must not be described as distributed transactional
uniqueness.

Acceptance Criteria

AevumDB-TX-1 is complete when:

A conditional write primitive exists in the public transaction API.
Successful first claim is atomic.
A second claim for the same key fails atomically.
A failed conditional batch leaves no partial writes.
WAL recovery preserves the same semantics.
Concurrent writers are covered by deterministic tests.
The primitive is usable by Community without application-level
cross-process coordination.
Existing AevumDB tests remain green.
