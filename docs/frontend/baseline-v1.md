# Aevum Frontend Baseline v1

**Status:** Draft
**Last updated:** 2026-09-18
**Phase:** 0 — Baseline Protection

---

## 1. Purpose

This document records the frontend baseline before any
migration work begins.

It is a snapshot of the current state, derived from the
completed audits.

It is used to:

- Measure progress during migration.
- Detect regressions.
- Provide rollback reference.

This document does not introduce changes. It records state.

---

## 2. Sources

Baseline is derived from:

- Frontend Architecture Audit v1
- Link Integrity Audit v1
- Source ↔ Production Audit v1
- Migration Plan v1

Where a value is captured live during Phase 0, it is marked
as (live).

---

## 3. URL Inventory

All production URLs were checked live.

### 3.1 Site pages

| URL | Status |
| --- | ------ |
| / | 200 |
| /explorer.html | 200 |
| /docs.html | 200 |
| /wallet.html | 200 |
| /community.html | 200 |
| /learn/ | 200 |
| /support.html | 200 |
| /repositories.html | 200 |
| /roadmap.html | 200 |
| /genesis.html | 200 |
| /learn/index.html | 200 |
| /learn/start.html | 200 |
| /learn/roadmap.html | 200 |
| /learn/paths.html | 200 |
| /learn/contribute.html | 200 |
| /learn/community-link.html | 200 |

### 3.2 Expected 404 (not in navigation)

| URL | Status |
| --- | ------ |
| /status.html | 404 |
| /privacy.html | 404 |
| /terms.html | 404 |

---

## 4. Baseline Metrics

| Metric | Value |
| ------ | ----- |
| Total pages | 15 |
| Top-level pages | 9 |
| Learn pages | 6 |
| Global CSS files | 4 (theme, app-shell, components, learn) |
| CSS total size | ~52 KB |
| Inline CSS lines | 3725 |
| Inline CSS pages | 9 |
| Shared JS files | 6 (theme, logo-3d, api/*) |
| Component loader variants | 3 (simple, medium, advanced) |
| Navigation runtime variants | 2 |
| Pages with component loader | 15 |
| Pages with initNav | 8 |
| Dead links in navigation | 0 (removed) |

---

## 5. CSS Baseline

### 5.1 Global CSS

| File | Size |
| ---- | ---- |
| css/theme.css | 8 KB |
| css/app-shell.css | 25 KB |
| css/components.css | 8 KB |
| css/pages/learn.css | 11 KB |

Total: ~52 KB.

### 5.2 Inline CSS

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

### 5.3 Duplication

No duplicated rules identified during Audit v1 between inline
CSS and global CSS.

---

## 6. JS Baseline

### 6.1 Shared JS

| File | Size | Loaded |
| ---- | ---- | ------ |
| js/theme.js | 0.7 KB | all pages |
| js/logo-3d.js | 7 KB | none (dormant) |
| js/api/client.js | 5 KB | explorer only |
| js/api/contract.js | 7 KB | explorer only |
| js/api/mock.js | 17 KB | explorer only |
| js/api/store.js | 15 KB | explorer only |

### 6.2 Inline JS

| Element | Present on |
| ------- | ---------- |
| component loader | 15 pages |
| boot() | 15 pages |
| initNav() | 8 top-level pages |
| page-specific JS | 4 pages |

### 6.3 Runtime Variants

- Component loader: 3 variants (simple, medium, advanced).
- Navigation runtime: 2 variants.

### 6.4 Page-specific JS

| Page | Purpose |
| ---- | ------- |
| index.html | Three.js bee animation |
| explorer.html | Store integration |
| docs.html | Hash navigation |
| support.html | initCopy() |

---

## 7. Accessibility Baseline

| Item | Status |
| ---- | ------ |
| H1 per page | 1 (all pages) |
| Landmarks (header, nav, footer) | via components |
| Skip link | missing |
| Focus states | present |
| Keyboard navigation | present |
| Header buttons with type="button" | 0 of 4 |

---

## 8. SEO Baseline

| Item | Status |
| ---- | ------ |
| Unique title | all 15 pages |
| Meta description | 6 of 15 pages |
| Canonical | 0 of 15 |
| lang attribute | all pages |
| Open Graph | 0 of 15 |
| robots.txt | missing |
| sitemap.xml | missing |
| Viewport | all pages |

---

## 9. Source ↔ Production Baseline

Source: /root/aevum-platform/frontend/
Production: /var/www/html/

Method: SHA-256 checksum comparison.

Status: ALIGNED (after commit d8d6d6b).

Production-only artifacts:

- index.nginx-debian.html (external Nginx placeholder).
- css/components.css.bak-sync-* (rollback copy).
- learn/index.html.bak-sync-* (rollback copy).

Source-only artifact:

- frontend/Дать (unrelated untracked file).

None of these artifacts are part of the site.

---

## 10. Lighthouse Baseline

Status: Not captured.

Reason:
Lighthouse tooling unavailable during Phase 0.

Action:
Capture before Phase 1 implementation begins.

Status: Deferred.

---

## 11. Deferred Measurements

The following measurements are deferred to later phases:

- Lighthouse performance score.
- Lighthouse accessibility score.
- Lighthouse SEO score.
- Core Web Vitals.
- Network waterfall analysis.
- Cache header verification.

---

## 12. Status

Aevum Frontend Baseline v1

State: DRAFT
Blockers: NONE
Next step: Phase 1 — Shared Runtime Extraction.

---

