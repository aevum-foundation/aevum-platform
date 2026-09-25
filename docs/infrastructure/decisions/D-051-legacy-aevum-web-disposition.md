# D-051 — Legacy aevum-web Disposition

**Status:**       Accepted
**Date:**         2026-09-25
**Applies to:**   Legacy repositories, production artifacts

---

## Context

Stage 3 (Deployment Automation) completed with A5 migration.
Production now runs on the V3 architecture (D-049, D-050):

```

Apache

|
v
/var/www/html              (symlink)

|
v
/var/www/active            (symlink, deployment pointer)

|
v
/var/www/releases/<release-id>   (immutable release)

```

Two legacy artifacts remain to be addressed:

1. /var/www/html.old — the pre-migration production directory.
2. aevum-foundation/aevum-web — the historical website repository.

A6 audit established:

### /var/www/html.old

- Complete duplicate of /root/aevum-platform/frontend.
- No unique files.
- Contains .git (the aevum-web repository).
- Last commit: c9cd8fd (synchronized with origin).

### aevum-foundation/aevum-web

- 35 unique commits, not present in aevum-platform.
- Includes f8d5941 — "Aevum Web Platform v1 — Foundation Freeze".
- Current content identical to source-of-truth.
- GitHub repository is private.
- No longer required for active deployment.

---

## Decision

### 1. /var/www/html.old

**Status:**  Temporary rollback asset.

**Action:**  Keep temporarily. Remove after operational validation.

Removal is event-based, not calendar-based. Conditions:

1. A6 complete.
2. At least one successful deployment performed via aevum-deploy.
3. No rollback incidents during the validation period.

Purpose: provide a pre-migration emergency fallback while the new architecture proves itself in production.

### 2. aevum-foundation/aevum-web

**Status:**  Historical repository.

**Action:**  Archive (read-only).

Rationale:

- Not deleted: 35 unique commits represent the early history of the website, including the Foundation Freeze.
- Not kept active: active development must live only in aevum-platform. Keeping aevum-web as an active repository would reintroduce the risk of divergent sources.
- Archive: history preserved, no future development, single source of truth remains intact.

### 3. Active Source of Truth

```

/root/aevum-platform

```

Remains the sole active source of truth for the website and platform code.

---

## Guiding Principle

> **Historical value is not operational value.**

aevum-web is preserved because it contains the historical evolution of the project. It is not preserved because it is required to operate the system.

---

## Consequences

- /var/www/html.old remains on disk as an emergency asset until validation completes.
- aevum-web will be archived on GitHub and receive no further commits.
- No second active source of development exists.
- The pre-migration state is preserved in two forms:
  - /var/www/html.old (directory)
  - /root/archive/aevum-html-2026-09-23T13-19-43Z.tar.gz (tarball)

---

## References

- D-048 — Frontend Refactor Deferred
- D-049 — Deployment Architecture
- D-050 — Deployment Migration
- docs/infrastructure/deployment-audit-v1.md (A1)
- docs/infrastructure/deployment-plan-v1.md (A3)
- docs/infrastructure/transfer-context-v1.md
