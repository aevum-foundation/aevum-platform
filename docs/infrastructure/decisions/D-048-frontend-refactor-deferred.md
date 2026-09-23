# D-048 — Frontend Refactor Deferred

**Status:**       Accepted
**Date:**         2026-09-23
**Applies to:**   Frontend, navigation, page modules, CSS

---

## Context

The frontend has reached a stable state after completing:

- Frontend Architecture Audit v1
- Frontend Architecture v2
- Frontend Migration Plan v1
- Frontend Baseline v1
- Shared Runtime Extraction (components.js, navigation.js)
- SEO Rollout (canonical, meta description, Open Graph, Twitter Cards)
- Production Hygiene (archive legacy copies, cleanup)
- Source of Truth documentation

The following areas remain as architectural debt, but **no production
defects have been identified**:

1. **Navigation Consistency** — `roadmap.html` and 6 `learn/*` pages
   do not call `initNav()`. All other pages do. No functional bug has
   been confirmed.

2. **Page Modules** — only 4 of 15 pages have extracted page-specific
   JS modules (`docs.js`, `explorer.js`, `home.js`, `support.js`).

3. **CSS Extraction** — page-specific CSS exists for 9 root pages.
   Inline CSS remains on some pages. No duplicates, no conflicts, no
   bugs detected.

These are **architectural debt**, not production incidents.

---

## Decision

The following work is **deferred**:

- Navigation Consistency refactor
- Page Modules extraction for remaining pages
- CSS Extraction for remaining pages

All three are postponed until completion of the
**Documentation Freeze / Knowledge Transfer** stage.

---

## Reason

Current frontend is stable:

- No production defects.
- No broken navigation.
- No broken pages.
- No 404s.
- No JS exceptions.

SEO rollout completed successfully (15/15 pages).

The immediate priority is to **freeze knowledge** — move decisions from
chat history and memory into the repository. Refactoring without
documented decisions creates risk of losing context.

Per project policy: **do not refactor because "it looks nicer".**
Every refactor must have:

1. Confirmed defect or architectural justification.
2. Documented decision.
3. Migration plan.

---

## Consequences

- `roadmap.html` and `learn/*` remain as-is (no `initNav`).
- 11 of 15 pages remain without dedicated page modules.
- Inline CSS remains on some pages.
- Frontend is **frozen** until documentation is complete and reviewed.

Any future frontend change MUST:

- reference this decision, and
- either supersede it via a new Decision Record, or
- explicitly confirm the refactor trigger has fired.

---

## Review Trigger

This decision will be reviewed **after**:

- Source of Truth documentation is complete.
- Decision Records infrastructure is in place.
- Transfer/context document is updated.

At that point, a new cycle (Phase C — Frontend Refactor) may begin.

---

## References

- `docs/infrastructure/source-of-truth.md` — Sections 7, 8
- `docs/infrastructure/decisions/POLICY.md` — Decision Record Policy
- Related commits:
  - `5342318` — Open Graph rollout
  - `731f055`, `84209c7` — Twitter Cards rollout
  - `c9cd8fd` — Deploy sync (aevum-web)
