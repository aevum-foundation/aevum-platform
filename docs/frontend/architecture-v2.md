# Aevum Frontend Architecture v2

**Status:** Draft
**Last updated:** 2026-09-18
**Based on:** frontend-architecture-audit-v1.md

---

## 1. Purpose

This document defines the target architecture for the Aevum
frontend and records the decisions required to reach it.

It is a decisions document, not an implementation plan.

Audit v1 describes the current state.
Architecture v2 describes the target state.
Migration Plan describes how to move between them.

---

## 2. Design Principles

### P1. Single Source of Truth

Every shared concern has exactly one canonical implementation.

Examples:

- one component loader
- one navigation runtime
- one theme system

### P2. Shared Before Duplicate

If two pages need the same behavior, that behavior is shared,
not copied.

Page-specific code is only created when behavior is genuinely
unique to a page.

### P3. Production Mirrors Source

Production (`/var/www/html`) mirrors source (`frontend/`)
byte-for-byte, except for explicitly documented production-only
artifacts.

### P4. Accessibility by Default

New pages and components include:

- semantic landmarks
- keyboard support
- focus visibility
- ARIA where required
- skip-to-content where relevant

### P5. Static First

Prefer static HTML and CSS.

Introduce JavaScript only when behavior requires it.

### P6. Performance Before Complexity

Prefer smaller, cacheable assets over clever abstractions.

External assets are cached and reusable.

---

## 3. Current State Summary

From frontend-architecture-audit-v1.md:

- Global CSS architecture is clean.
- Top-level pages use inline CSS (3725 lines total).
- Learn pages use external `css/pages/learn.css`.
- Component loader is duplicated across all 15 pages with
  multiple variants.
- Navigation runtime is duplicated across 8 top-level pages
  with two variants.
- `js/logo-3d.js` is not loaded anywhere (dormant).
- Skip links are missing.
- 9 pages lack meta description.
- Canonical links, Open Graph, robots.txt, sitemap.xml are
  missing.
- Source and production are aligned (commit d8d6d6b).

---

## 4. Target Architecture

### 4.1 CSS Architecture

    css/
    ├── theme.css          Design tokens (colors, spacing)
    ├── app-shell.css      Layout: header, footer, drawer
    ├── components.css     Shared components (.btn, .card)
    └── pages/
        ├── home.css
        ├── docs.css
        ├── explorer.css
        ├── genesis.css
        ├── community.css
        ├── repositories.css
        ├── roadmap.css
        ├── support.css
        ├── wallet.css
        └── learn.css

Rules:

- Page-specific CSS lives in `css/pages/*.css`.
- Inline `<style>` is not used for page layout.
- Global tokens are defined only in `theme.css`.
- Shared component styles are defined only in `components.css`.
- Page CSS does not redefine global classes.

### 4.2 JavaScript Architecture

    js/
    ├── theme.js            Theme toggle (shared)
    ├── components.js       Component loader (shared)
    ├── navigation.js       Navigation runtime (shared)
    ├── api/                API layer (explorer only)
    │   ├── client.js
    │   ├── contract.js
    │   ├── mock.js
    │   └── store.js
    └── pages/
        ├── home.js         Three.js animation
        ├── explorer.js     Explorer logic
        ├── docs.js         Hash navigation
        └── support.js      Copy address

Rules:

- Shared runtime lives in `js/*.js`.
- Page-specific behavior lives in `js/pages/*.js`.
- Every page loads:

    <script src="/js/theme.js"></script>
    <script src="/js/components.js"></script>
    <script src="/js/navigation.js"></script>
    <script src="/js/pages/<page>.js"></script>  (only if needed)

- No page defines its own component loader.
- No page defines its own navigation runtime.

### 4.3 Component Architecture

Components are loaded from `/components/`:

- `header.html` — header and mobile drawer
- `footer.html` — footer

Component loading is handled by `js/components.js`.

Components do not contain page-specific logic.

### 4.4 Page Architecture

Each page:

- has `<main class="...-page" id="main-content">`
- has one `<h1>`
- loads shared runtime from `js/*.js`
- loads page CSS from `css/pages/*.css`
- loads page JS from `js/pages/*.js` (only if needed)

Pages do not contain inline `<style>` or inline component loader
logic after migration.

---

## 5. Decisions

### D-001 Canonical Component Loader

Decision:

A single component loader is implemented in `js/components.js`.

Current state:

- Component loader is duplicated across 15 pages.
- At least 3 variants exist.

Target state:

- One canonical implementation.
- Loaded via `<script src="/js/components.js"></script>`.

Consequences:

- Removes duplication.
- Fixes subtle behavior drift between pages.

Status: PLANNED.

---

### D-002 Centralized Navigation Runtime

Decision:

Navigation logic (mobile drawer, focus trap, escape handling)
is implemented in `js/navigation.js`.

Current state:

- `initNav()` duplicated across 8 top-level pages.
- Two versions exist.

Target state:

- One canonical implementation.
- Loaded via `<script src="/js/navigation.js"></script>`.

Consequences:

- Single source of accessibility behavior.
- Removes ~500 duplicated lines.

Status: PLANNED.

---

### D-003 Page CSS Strategy

Decision:

Page-specific CSS is moved from inline `<style>` blocks to
`css/pages/*.css`.

Current state:

- 3725 lines of inline CSS across 9 top-level pages.
- Learn pages already use `css/pages/learn.css`.

Target state:

- Every page has one corresponding `css/pages/<page>.css`.
- No inline `<style>` for layout.

Consequences:

- CSS becomes cacheable.
- Consistent with Learn.
- Easier to maintain.

Status: PLANNED.

---

### D-004 Shared Runtime Architecture

Decision:

All shared runtime is loaded from `js/*.js`.

Target:

- `js/theme.js`
- `js/components.js`
- `js/navigation.js`

Page-specific JS goes to `js/pages/*.js`.

Page-specific JS is created only when needed:

- `js/pages/home.js`
- `js/pages/explorer.js`
- `js/pages/docs.js`
- `js/pages/support.js`

Consequences:

- Shared logic is initialized once.
- Page modules receive a ready shell.

Status: PLANNED.

---

### D-005 Source ↔ Production Workflow

Decision:

Production (`/var/www/html`) mirrors source (`frontend/`).

Rules:

- Source is the canonical location for all changes.
- Production is updated by copying verified files from source.
- Checksums are compared after synchronization.
- Backup artifacts and unrelated files are not committed.

Status: ACTIVE (partially implemented).

---

### D-006 Dormant Feature Policy

Decision:

`js/logo-3d.js` is treated as a dormant feature, not dead code.

Rules:

- Not removed until confirmed unused by roadmap.
- Not integrated until homepage redesign.
- Reviewed again during homepage redesign.

Status: DEFERRED.

---

### D-007 Accessibility Baseline

Decision:

All pages must meet the following baseline:

- Skip-to-content link.
- Semantic landmarks (header, nav, main, footer).
- Correct heading hierarchy.
- Focus visibility.
- Keyboard-operable interactive elements.
- Explicit `type` on all `<button>`.

Status: PLANNED.

---

### D-008 SEO Baseline

Decision:

All pages must include:

- Unique `<title>`
- Meta description
- Canonical link
- `lang` attribute
- Viewport

Site-wide:

- `robots.txt`
- `sitemap.xml`
- Open Graph metadata

Status: PLANNED.

---

### D-009 Performance Baseline

Decision:

Performance guidelines:

- No inline CSS for page layout.
- No inline JS for shared behavior.
- External assets cached.
- Three.js loaded only on pages that require it.
- Page weight reviewed before adding new scripts.

Status: PLANNED.

---

## 6. Migration Plan

The migration is staged. Each phase is self-contained and
verifiable.

### Phase 1 — Shared Runtime Extraction

Scope:

- Introduce `js/components.js`
- Introduce `js/navigation.js`

Actions:

- Extract the most complete component loader from
  `docs.html` into `js/components.js`.
- Extract the most complete navigation runtime from
  `docs.html` and `explorer.html` into `js/navigation.js`.
- Replace inline loaders in all 15 pages with
  `<script src="/js/components.js"></script>`.
- Replace inline nav code in 8 top-level pages with
  `<script src="/js/navigation.js"></script>`.

Verification:

- All pages load header and footer correctly.
- Mobile drawer works on all top-level pages.
- Keyboard navigation (Escape, Tab) works.
- No console errors.

### Phase 2 — Page CSS Extraction

Scope:

- Move inline `<style>` blocks to `css/pages/*.css`.

Order:

1. roadmap.html (smallest)
2. index.html
3. repositories.html
4. wallet.html
5. support.html
6. explorer.html
7. genesis.html
8. community.html
9. docs.html (largest)

Verification:

- Visual output unchanged.
- CSS files loaded once.
- No unused selectors introduced.

### Phase 3 — Page JS Extraction

Scope:

- Move inline page-specific JS to `js/pages/*.js`.

Order:

1. support.html → js/pages/support.js (initCopy)
2. docs.html → js/pages/docs.js (hash navigation)
3. index.html → js/pages/home.js (Three.js)
4. explorer.html → js/pages/explorer.js (Store integration)

Verification:

- Each page behaves as before.
- No inline module code remains.

---

## 7. Verification Strategy

Each phase is verified against:

- Loaded assets (HTTP 200).
- Console errors (must be none).
- Visual output (unchanged).
- Keyboard navigation (still works).
- Mobile drawer (still works).
- Source ↔ production checksum (must match).

Cross-page verification is mandatory after Phase 1, because
the shared runtime affects every page.

---

## 8. Deferred Work

Not part of Architecture v2.

Items are tracked here so they are not lost.

- `js/logo-3d.js` — dormant feature, revisit during homepage
  redesign.
- Homepage animation framework — not required for v2.
- Advanced SEO (structured data, rich results) — not required
  for v2.
- Component-level lazy loading — out of scope for v2.
- Build pipeline (bundling, minification) — out of scope for
  v2.

---

## 9. Status

Aevum Frontend Architecture v2

State: DRAFT
Blockers: NONE
Next step: Migration Plan (execution-level).

---

