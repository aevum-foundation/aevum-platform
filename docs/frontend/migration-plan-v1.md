# Aevum Frontend Migration Plan v1
## Architecture v2 Implementation Roadmap

**Status:** Draft
**Last updated:** 2026-09-18
**Based on:** architecture-v2.md, frontend-architecture-audit-v1.md

---

## 1. Purpose

This document defines the execution plan for migrating the
Aevum frontend from its current state (Audit v1) to the
target state (Architecture v2).

The plan is organized by phases and risk.

Each phase defines:

- Scope
- Risk profile
- Deliverables
- Verification criteria
- Rollback procedure
- Exit criteria

---

## 2. Scope

In scope:

- Shared runtime extraction (component loader, navigation)
- Page JavaScript extraction (js/pages/*.js)
- Page CSS extraction (css/pages/*.css)

Out of scope (deferred):

- js/logo-3d.js integration
- Build pipeline
- Advanced SEO (structured data)
- Component lazy loading
- Framework adoption

---

## 3. Risk Classification

| Phase   | Risk   | Reason |
| ------- | ------ | ------ |
| Phase 0 | Low    | Read-only inventory |
| Phase 1 | Medium | Shared runtime affects all pages |
| Phase 2 | Medium | Page-specific JS extraction |
| Phase 3 | High   | CSS extraction may cause visual regressions |

Risk levels:

- Low = no user-facing behavior changes.
- Medium = behavior changes possible.
- High = visual or functional regressions possible.

---

## 4. Success Criteria

A phase is complete only if:

- All pages return HTTP 200.
- Header and footer load on all pages.
- Theme toggle works.
- Mobile drawer works.
- Keyboard navigation works.
- No layout shifts larger than baseline.
- No broken navigation links.
- No accessibility regression from baseline.
- No JavaScript runtime errors.
- No increase in page load failures.
- Source ↔ production checksums match after deployment.

---

## 5. Phase 0 — Baseline Protection

Goal:

Capture the current state before any change.

Risk: Low.

### Actions

1. Record URL inventory.
2. Record HTTP status for every URL.
3. Record Lighthouse baseline (performance, accessibility, SEO).
4. Record accessibility baseline (skip link, landmarks, focus).
5. Record current source ↔ production checksum map.
6. Record dependency inventory (JS imports, external scripts).
7. Record shared runtime inventory (component loader, navigation).
8. Record CSS inventory (global, inline, page-specific).
9. Record JavaScript inventory (shared, inline, page-specific).

### Deliverables

A baseline snapshot in:

    docs/frontend/baseline-v1.md

### Verification

- Baseline snapshot is committed.
- Baseline URLs match current site.

### Rollback

None required. Phase 0 is read-only.

### Exit Criteria

- Baseline snapshot exists and is committed.
- All measured values are recorded.

---

## 6. Phase 1 — Shared Runtime Extraction

Goal:

Establish a single canonical runtime for all pages.

Risk: Medium.

### Scope

Extract into shared modules:

- componentCache
- loadComponent()
- boot()
- initNav()

These four elements are present in the audited implementations
on multiple pages.

### Deliverables

    js/components.js
    js/navigation.js

### Actions

1. Extract a canonical component loader into js/components.js.
   The audited component loader is duplicated across 15 pages.
   At least 3 variants exist. The canonical source will be
   selected during Phase 1.

2. Extract a canonical navigation runtime into js/navigation.js.
   The audited navigation runtime is duplicated across 8
   top-level pages. At least 2 variants exist. The canonical
   source will be selected during Phase 1.

3. Replace inline loaders in all 15 pages with:

       <script src="/js/components.js"></script>

4. Replace inline navigation code in 8 top-level pages with:

       <script src="/js/navigation.js"></script>

5. Remove duplicated inline code.

### Verification

- Header loads on all pages.
- Footer loads on all pages.
- Theme toggle works.
- Mobile navigation works.
- Keyboard navigation (Escape, Tab) works.
- HTTP 200 for all pages.
- No console errors.

### Rollback

    git revert <phase-1-commit>

### Exit Criteria

- All 15 pages use the shared runtime.
- No inline loader or nav logic remains.
- Verification passes in production.

---

## 7. Phase 2 — Page JavaScript Extraction

Goal:

Move page-specific JavaScript into js/pages/*.js.

Risk: Medium.

### Scope

Page-specific JavaScript exists in:

- index.html — Three.js bee animation
- explorer.html — Store integration, search, detail
- docs.html — hash navigation (scrollToHash, getHeaderOffset)
- support.html — initCopy()

All other pages contain only shared runtime logic and require
no page-specific JS file.

### Deliverables

    js/pages/home.js
    js/pages/explorer.js
    js/pages/docs.js
    js/pages/support.js

Pages without unique behavior do not receive a page-specific
JS file.

### Order

TBD. Migration order will be determined during execution
planning based on complexity and risk.

### Verification

- Each page behaves as before.
- No inline page-specific JS remains.
- No console errors.

### Rollback

    git revert <phase-2-commit>

### Exit Criteria

- All page-specific JS is external.
- Verification passes in production.

---

## 8. Phase 3 — Page CSS Extraction

Goal:

Move inline `<style>` blocks into css/pages/*.css.

Risk: High.

### Scope

Inline CSS exists on 9 top-level pages:

| Page | Inline CSS lines |
| ---- | ---------------- |
| community.html | 695 |
| genesis.html | 527 |
| docs.html | 489 |
| explorer.html | 455 |
| support.html | 372 |
| wallet.html | 359 |
| repositories.html | 354 |
| index.html | 299 |
| roadmap.html | 175 |

Total: 3725 lines.

Learn pages already use css/pages/learn.css and require no
change.

### Deliverables

    css/pages/home.css
    css/pages/docs.css
    css/pages/explorer.css
    css/pages/genesis.css
    css/pages/community.css
    css/pages/repositories.css
    css/pages/roadmap.css
    css/pages/support.css
    css/pages/wallet.css

### Order

TBD. Migration order will be determined during execution
planning based on complexity and risk.

### Verification

- Visual output unchanged.
- CSS files loaded once.
- No unused selectors introduced.
- No duplicated rules introduced.

### Rollback

    git revert <phase-3-commit>

### Exit Criteria

- No inline `<style>` blocks remain in top-level pages.
- All page CSS is external and cacheable.
- Verification passes in production.

---

## 9. Architecture Guardrails

The migration must not:

- Introduce frameworks.
- Introduce build tooling.
- Break static hosting.
- Introduce duplicate runtimes.
- Introduce page-specific copies of shared logic.

These guardrails apply to all phases.

---

## 10. Verification Strategy

Each phase is verified against:

- Loaded assets (HTTP 200).
- Console errors (must be none).
- Visual output (unchanged from baseline).
- Keyboard navigation (still works).
- Mobile drawer (still works).
- Source ↔ production checksum (must match).

Cross-page verification is mandatory after Phase 1, because
the shared runtime affects every page.

---

## 11. Rollback Strategy

Each phase is reversible via a single commit revert.

    git revert <phase-N-commit>

After rollback:

- Verify HTTP 200 for all pages.
- Verify header and footer load.
- Verify navigation works.
- Verify no console errors.

---

## 12. Evidence Mapping

Every decision in this plan is based on audited facts.

### Phase 1 — Shared Runtime Extraction

Evidence:

- Frontend Architecture Audit v1.

Findings:

- loadComponent() duplicated across 15 pages.
- initNav() duplicated across 8 top-level pages.
- At least 3 component loader variants detected.
- At least 2 navigation runtime variants detected.

### Phase 2 — Page JavaScript Extraction

Evidence:

- Frontend Architecture Audit v1.

Findings:

- Page-specific JS exists in:
    - index.html (Three.js)
    - explorer.html (Store integration)
    - docs.html (hash navigation)
    - support.html (initCopy)

### Phase 3 — Page CSS Extraction

Evidence:

- Frontend Architecture Audit v1.

Findings:

- 3725 lines of inline CSS across 9 top-level pages.
- Learn pages already use css/pages/learn.css.
- No duplicated rules identified during the audit.

---

## 13. Deferred Work

Not part of this migration.

Items are tracked here so they are not lost.

- js/logo-3d.js — dormant feature, revisit during homepage
  redesign.
- Homepage animation framework — not required for v1.
- Advanced SEO (structured data, rich results) — not required
  for v1.
- Component-level lazy loading — out of scope.
- Build pipeline (bundling, minification) — out of scope.

---

## 14. Status

Aevum Frontend Migration Plan v1

State: DRAFT
Blockers: NONE
Next step: Phase 0 — Baseline Protection.

---

