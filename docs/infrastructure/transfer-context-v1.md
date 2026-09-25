# Aevum — Transfer Context v1

**Status:**   Active
**Audience:** New team members, future maintainers, returning contributors
**Purpose:**  Entry point — project context AND working protocol
**Date:**     2026-09-23

---

## 1. What is Aevum

**Aevum** is a user-owned protocol.

### Key ideas

- **Aevum L1** does not use the classical block model.
- Coordination is based on a **Slot → Epoch** model:
  - Slot = 30 seconds
  - Epoch = 2880 slots (24 hours)
- **Proof of Presence** consensus direction.
- **Post-Quantum** cryptography.
- **L2** for fast transactions and smart contracts.

### Positioning

> **Aevum — The Decentralized Supercomputer**
>
> *Privacy. Presence. Post-Quantum.*

### Main site

- https://aevumchain.com

---

## 2. Repository Architecture

Aevum is split across multiple repositories, each with a distinct role
and visibility.

| Repository        | Role                                              | Visibility |
|-------------------|---------------------------------------------------|------------|
| `aevum-protocol`  | Protocol core (L1). Heart of the project.         | 🔒 Private |
| `aevum-db`        | Custom database. Post-quantum. Zero deps.         | 🔒 Private |
| `aevum-platform`  | External interaction: apps, site, SDK.            | 🌐 Public  |
| `aevum-public`    | Public showcase of the protocol (partial).        | 🌐 Public  |
| `aevum-web`       | **Legacy.** First website implementation.         | ⚠️ Legacy  |

### What this means

- **`aevum-protocol` and `aevum-db` are closed.** Unique in-house
  developments. Not published. Do not attempt to fork, mirror, or
  duplicate.
- **`aevum-platform` is the working zone** for anything that talks to
  the protocol — the website, apps, SDKs.
- **`aevum-web` is legacy.** Historical first website. Scheduled for
  cleanup after deployment automation is in place.

### Vision

The protocol will be opened **gradually**, in stages. `aevum-public`
exists to show what is safe to share now. Nothing else.

---

## 3. Source of Truth

> **One source of truth. One deployment path. One active copy.**

| Role                | Path                          |
|---------------------|-------------------------------|
| Source of Truth     | `/root/aevum-platform`        |
| Deployment Target   | `/var/www/html`               |
| Live Domain         | `https://aevumchain.com`      |
| Archive             | `/root/archive/`              |

**Full details:** `docs/infrastructure/source-of-truth.md`

---

## 4. Current Project State

### Completed

| Area                            | Status |
|---------------------------------|--------|
| Frontend Architecture Audit v1  | ✅     |
| Frontend Architecture v2        | ✅     |
| Frontend Migration Plan v1      | ✅     |
| Frontend Baseline v1            | ✅     |
| Shared Runtime Extraction       | ✅     |
| SEO Rollout                     | ✅     |
| Production Hygiene              | ✅     |
| Source of Truth documentation   | ✅     |
| Decision Records infrastructure | ✅     |
| Documentation Freeze            | ✅     |
| Deployment Audit v1             | ✅     |
| A2 Target Architecture          | ✅     |
| A3 Deployment Plan              | ✅     |
| A4 Implementation               | ✅     |
| A5 Deployment Migration         | ✅     |

### Current State Summary

- **Frontend:** stable. No production defects.
- **SEO:** full contract deployed on 15/15 pages.
- **Deployment:** automated via `aevum-deploy` (V3 atomic staging + swap).
- **Stage 3:** Deployment Automation — COMPLETE (A1–A6).
- **Production topology:** Apache → /var/www/html → /var/www/active → /var/www/releases/<id>.
- **Decision Records:** active (`docs/infrastructure/decisions/`).
- **Architecture debt:** tracked in D-048 (deferred).

### Active Git HEADs

- `aevum-platform` (master): see `git log`
- `aevum-web` (main): see `git log`

---

## 5. Current Priorities

| Priority | Item                                    | Status      |
|----------|-----------------------------------------|-------------|
| P1       | Deployment Automation (Stage 3)         | DONE        |
| P2       | Protocol Core                           | Next        |
| P3       | Decision Records — expand as needed     | Active      |
| P4       | Frontend Refactor                       | Deferred    |

### Stage 3 progress

| Step | Item                                    | Status |
|------|-----------------------------------------|--------|
| A1   | Deployment Audit v1                     | DONE   |
| A2   | Target Deployment Architecture          | DONE   |
| A3   | Deployment Plan                         | DONE   |
| A4   | Implementation                          | DONE   |
| A5   | Validation                              | DONE   |
| A6   | Legacy `aevum-web` Decision (D-051)     | DONE   |

See: `docs/infrastructure/decisions/D-048-frontend-refactor-deferred.md`

### A2 Outcome (Deployment Architecture)

**Decision:** D-049 — `atomic staging + swap` (V3) with deployment manifest.

**Target architecture:**

```
/root/aevum-platform              (source of truth)
        ↓
/var/www/releases/<revision>/     (immutable snapshots)
        ↓
/var/www/active -> releases/...   (symlink)
        ↓
Apache DocumentRoot               (serves active release)
```

**Pipeline:** stage → manifest → verify → activate → retain → prune

**Rollback:** `ln -sfn releases/<previous> active` (instant)

**Key properties:**
- No Git state on the production host.
- Each release is an immutable snapshot.
- Deployment manifest records source revision, timestamp, actor.
- `aevum-web` is no longer needed for deployment.

See: `docs/infrastructure/decisions/D-049-deployment-architecture.md`

---

## 6. Working Protocol

All changes to the frontend and infrastructure follow this order:

```

Audit
↓
Architecture
↓
Migration Plan
↓
Implementation
↓
Verification
↓
Commit
↓
Push
↓
Deploy

```

### Rules

- **Never refactor because "it looks nicer".** Every refactor needs a
  confirmed defect OR a documented architectural justification.
- **One command — one step.** No batch operations on live systems.
- **Verify before commit.** `git diff` and integrity checks.
- **Commit in source first.** Production receives updates via deploy.

---

## 7. Constraints and Invariants

### Do NOT touch

- **`js/logo-3d.js`** — Dormant feature. Kept for future use. Not dead
  code.
- **Aevum branding colors:**
```

#020914  #061426  #0B1B30
#F5C542  #FFD76A  #B88916
#20D9FF

```
- **`371,000,000 AEV`** — Fixed project constant. Never change.

### Do NOT publish

- Unique in-house developments from `aevum-protocol` / `aevum-db`.
- Anything not already present in `aevum-public`.

### Do NOT create

- Parallel frontend copies under `/var/www/`.
- Duplicate deployment trees.
- Alternative "final" or "backup" directories.

Full list: `docs/infrastructure/source-of-truth.md` § 5 Prohibited.

---

## 8. Decision Records

All significant decisions are recorded as **Decision Records**.

- **Policy:** `docs/infrastructure/decisions/POLICY.md`
- **Directory:** `docs/infrastructure/decisions/`

**Format:** `D-NNN-title.md`

**IDs are globally unique and never reused.**

### Current decisions

- `D-048` — Frontend Refactor Deferred

Referenced from commits and architecture documents.

---

## 9. Documentation Map

```

docs/
├── architecture/         — protocol architecture docs
├── backlog/              — future work items
├── community/            — community-facing docs
├── frontend/             — frontend audit, plan, contracts
└── infrastructure/
├── source-of-truth.md
├── transfer-context-v1.md      ← this file
    ├── deployment-audit-v1.md      (Stage 3 — A1)
└── decisions/
├── POLICY.md
├── D-048-frontend-refactor-deferred.md
        ├── D-049-deployment-architecture.md
        ├── D-050-deployment-migration.md
        └── D-051-legacy-aevum-web-disposition.md
```

---

## 10. Next Planned Work

### Current focus — Protocol Core

Stage 3 (Deployment Automation) is COMPLETE (A1–A6).

Next step: **Protocol Core Audit.**

Before any protocol changes, a fresh audit of the core is required:

- Consensus
- Presence
- Emission
- Settlement
- Epoch Snapshot
- Networking
- Storage
- Crypto
- L2 Interfaces
- Legacy Modules

Reason: the architecture has evolved significantly (block model
removed, slot/epoch model established, emission v15, EpochSnapshot v2,
legacy modules still present).

### After Protocol Core

1. **Platform Backend** — stabilize auth, sessions, permissions.
2. **Unified Account** — single Aevum account across services.
3. **Waitlist / Early Access** — begin collecting future users.
4. **Community Hub** — lightweight community inside the platform.
5. **Forum** — full community platform.
6. **Mainnet Integration.**

### Deferred

- Frontend Refactor (D-048)
- `html.old` removal (D-051, event-based)

---

**End of transfer context.**
