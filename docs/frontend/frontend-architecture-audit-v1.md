# Aevum Frontend Architecture Audit v1

**Status:** Draft
**Last updated:** 2026-09-18
**Audit date:** 2026-09-17 / 2026-09-18

---

## 1. Scope

This audit covers the current state of the Aevum frontend as
observed during the architecture review.

It is a read-only audit. No code changes are made here.

Pages covered:

- index.html
- docs.html
- explorer.html
- genesis.html
- community.html
- repositories.html
- roadmap.html
- wallet.html
- support.html
- learn/* (6 pages)

Components covered:

- components/header.html
- components/footer.html

Assets covered:

- css/* (theme.css, app-shell.css, components.css)
- css/pages/* (learn.css)
- js/* (theme.js, logo-3d.js)
- js/api/* (client.js, contract.js, mock.js, store.js)

---

## 2. Source ↔ Production Audit

Method: SHA-256 checksum comparison between:

    /root/aevum-platform/frontend/   (source)
    /var/www/html/                    (production)

Result:

- 26 of 28 files identical.
- 2 files differed at the time of the audit:
    - css/components.css (production was missing 107 lines
      of Repositories page styles)
    - learn/index.html (production had an older `?t=1` link)

Both were synchronized to source and committed in production
repository aevum-web (commit d8d6d6b).

Production-only artifacts observed:

- index.nginx-debian.html (external Nginx placeholder, not Aevum)
- css/components.css.bak-sync-* (temporary rollback copy)
- learn/index.html.bak-sync-* (temporary rollback copy)

Source-only artifact observed:

- frontend/Дать (unrelated untracked file)

These artifacts are not part of the audit scope and are not
modified.

---

## 3. CSS Audit

### 3.1 Global CSS

| File | Size | Purpose |
| ---- | ---- | ------- |
| css/theme.css | 8 KB | Design tokens, dark/light |
| css/app-shell.css | 25 KB | Layout, header, footer, drawer |
| css/components.css | 8 KB | Shared components (.btn, .card) |
| css/pages/learn.css | 11 KB | Learn-specific |

Total global CSS: ~52 KB.

### 3.2 Inline CSS

Nine top-level pages contain inline `<style>` blocks:

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

Total inline CSS: ~3725 lines.

### 3.3 Duplication

A class-name comparison between inline CSS and global CSS
(components.css, app-shell.css) found:

- No duplicated rules identified during this audit.

All inline classes use page-specific prefixes:

- .community-* (community.html)
- .docs-* (docs.html)
- .explorer-* (explorer.html)
- .genesis-* (genesis.html)
- .home, .hero (index.html)
- .repos-* (repositories.html)
- .roadmap-* (roadmap.html)
- .support-* (support.html)
- .wallet-* (wallet.html)

### 3.4 Findings

- Global CSS architecture is clean and consistent.
- Learn already uses the external `css/pages/learn.css` pattern.
- Top-level pages use inline CSS, which is a deviation from the
  Learn pattern.
- Inline CSS is page-specific, not duplicated.

### 3.5 Technical Debt

- MEDIUM: 3725 lines of inline page CSS on 9 pages.
- No production blocker.
- Refactor deferred until Architecture v2.

---

## 4. JS Audit

### 4.1 Shared JS files

| File | Size | Loaded | Notes |
| ---- | ---- | ------ | ----- |
| js/theme.js | 0.7 KB | Yes (all pages) | Theme toggle |
| js/logo-3d.js | 7 KB | No | Dormant |
| js/api/client.js | 5 KB | via store.js | API client |
| js/api/contract.js | 7 KB | via mock.js | API contract |
| js/api/mock.js | 17 KB | via client.js | Mock provider |
| js/api/store.js | 15 KB | explorer.html | Store layer |

### 4.2 Inline JS

Every page has inline JS for:

- component loading (header, footer)
- navigation initialization (top-level pages only)

Component loader duplication:

| Version | Pages | Features |
| ------- | ----- | -------- |
| Simple | community, support, wallet, repositories, genesis, roadmap, learn/* | componentCache, loadComponent |
| Medium | explorer | + initialized |
| Advanced | docs | + fetchWithTimeout, COMPONENT_TIMEOUT_MS |
| Homepage | index | + Three.js |

Page-specific JS:

| Page | Type | Purpose |
| ---- | ---- | ------- |
| index.html | Three.js | Bee animation (dormant feature path) |
| explorer.html | Module | Store integration, search, detail |
| docs.html | Hash navigation | scrollToHash, getHeaderOffset |
| support.html | initCopy | Copy wallet address |
| All others | — | Component loader only |

`initNav()` duplication:

- Present in 8 top-level pages (all except roadmap.html).
- Two versions: simple (community, support, wallet, repositories,
  genesis, index) and advanced (docs, explorer).
- Learn pages do not use initNav.

### 4.3 Findings

- Multiple implementations of the same shared logic.
- Component loader has at least 4 variants.
- Navigation logic has at least 2 variants.
- `js/logo-3d.js` is not loaded anywhere (dormant).

### 4.4 Technical Debt

- MEDIUM: multiple component loader implementations.
- MEDIUM: multiple initNav implementations.
- LOW: dormant `js/logo-3d.js`.

---

## 5. Dependency Audit

### 5.1 JS Dependency Chain

    explorer.html
        │ (type=module)
        ▼
    js/api/store.js
        │ import
        ▼
    js/api/client.js
        │ import
        ▼
    js/api/mock.js
        │ import
        ▼
    js/api/contract.js
        │ (no imports)

This chain is only used by explorer.html.

### 5.2 External Dependencies

| Resource | Where | Type |
| -------- | ----- | ---- |
| cdnjs.cloudflare.com | index.html | CDN (preconnect) |
| three.js r128 | index.html | External script (defer) |
| github.com/aevum-foundation | docs.html, repositories.html, learn/* | External links |
| t.me/aevumchain | community.html | External link |

Three.js is loaded from cdnjs with `defer`. It is used only
for the homepage bee animation.

### 5.3 Findings

- The API layer (store, client, mock, contract) is used only
  by explorer.html.
- js/logo-3d.js is not part of any import chain.
- Three.js is the only external JS dependency.

---

## 6. HTML Audit

### 6.1 Page Structure

Every page has:

- `<main>` with a page-specific class and `id="main-content"`.
- `<section class="...">` or `<section id="...">` for content.
- Unique class prefixes per page.

### 6.2 Main Classes

| Page | main class |
| ---- | ---------- |
| index.html | home |
| docs.html | docs-page |
| explorer.html | explorer-page |
| genesis.html | genesis-page |
| community.html | community-page |
| repositories.html | repos-page |
| roadmap.html | roadmap-page |
| support.html | support-page |
| wallet.html | wallet-page |
| learn/* | learn-page |

### 6.3 Components

- components/header.html: `<header>` + `<nav>` (desktop and
  mobile drawer).
- components/footer.html: `<footer>` with role="contentinfo".

Loaded via inline component loader on every page.

### 6.4 Findings

- HTML structure is consistent.
- Class prefixes are page-specific and non-overlapping.
- No structural anomalies detected.

---

## 7. Accessibility Audit

### 7.1 Landmarks

- Every page has `<main id="main-content">`.
- `<header>`, `<nav>`, `<footer>` are provided by components
  loaded at runtime.
- No `<nav>` element in page source (it lives in header.html).

### 7.2 Headings

- Every page has exactly one `<h1>`. ✅

### 7.3 ARIA

| Page | aria-label / labelledby count |
| ---- | ----------------------------- |
| community.html | 13 |
| learn/paths.html | 12 |
| learn/roadmap.html | 10 |
| repositories.html | 8 |
| learn/contribute.html | 8 |
| learn/start.html | 7 |
| learn/index.html | 6 |
| genesis.html | 5 |
| index.html | 5 |
| learn/community-link.html | 5 |
| support.html | 4 |
| wallet.html | 4 |
| explorer.html | 3 |
| docs.html | 2 |
| roadmap.html | 0 |

### 7.4 Focus States

- Global `:focus-visible` in theme.css.
- Component-level focus styles in app-shell.css for brand,
  header nav, theme toggle, burger, side-nav, footer links.

### 7.5 Keyboard Navigation

- `keydown` handlers present in top-level pages.
- `Escape` and `Tab` handled in navigation code.

### 7.6 Findings

- No skip-to-content link on any page. ⚠️
- roadmap.html has no ARIA labels. ⚠️
- Header buttons lack explicit `type="button"` (4 buttons). ⚠️
- Documentation and other long pages would benefit from
  skip link.

### 7.7 Technical Debt

- MEDIUM: missing skip links.
- LOW: missing button types in header component.
- LOW: roadmap.html ARIA coverage.

---

## 8. SEO Audit

### 8.1 Title

All 15 pages have a unique `<title>`. ✅

### 8.2 Meta Description

Present:
- community.html
- explorer.html
- repositories.html
- roadmap.html
- support.html
- wallet.html

Missing:
- docs.html
- genesis.html
- index.html
- learn/community-link.html
- learn/contribute.html
- learn/index.html
- learn/paths.html
- learn/roadmap.html
- learn/start.html

9 of 15 pages lack meta description. ⚠️

### 8.3 Canonical

No page declares `<link rel="canonical">`. ❌

### 8.4 Language

All pages declare `<html lang="en">`. ✅

### 8.5 Open Graph

No page declares Open Graph metadata. ❌

### 8.6 Robots

No robots.txt file exists. ⚠️

### 8.7 Sitemap

No sitemap.xml file exists. ⚠️

### 8.8 Viewport

All pages declare a responsive viewport. ✅

### 8.9 Technical Debt

- MEDIUM: 9 missing meta descriptions.
- MEDIUM: no canonical links.
- LOW: no Open Graph.
- LOW: no robots.txt.
- LOW: no sitemap.xml.

---

## 9. Mobile Audit

### 9.1 Viewport

All 15 pages declare:

    <meta name="viewport" content="width=device-width, initial-scale=1.0">

### 9.2 Breakpoints

Global CSS (app-shell.css):

    1200px, 1024px, 768px, 480px, 360px, 320px
    prefers-reduced-motion

Learn CSS:

    960px, 768px, prefers-reduced-motion

Theme CSS:

    prefers-reduced-motion

### 9.3 Inline Media Queries

| Page | Media queries |
| ---- | ------------- |
| community.html | 6 |
| repositories.html | 5 |
| wallet.html | 5 |
| docs.html | 4 |
| support.html | 4 |
| explorer.html | 3 |
| genesis.html | 3 |
| index.html | 3 |
| roadmap.html | 1 |

Total inline media queries: 34.

### 9.4 Findings

- Viewport is consistent across pages. ✅
- Breakpoints are consistent with the design system.
- Inline media queries duplicate some layout logic from the
  global breakpoints.
- reduced-motion is respected in global CSS.

### 9.5 Technical Debt

- MEDIUM: inline media queries add maintenance cost.
- No production blocker.

---

## 10. Performance Audit

### 10.1 Page sizes

| Page | Size |
| ---- | ---- |
| docs.html | 85 KB |
| index.html | 52 KB |
| explorer.html | 45 KB |
| community.html | 39 KB |
| genesis.html | 37 KB |
| learn/paths.html | 26 KB |
| repositories.html | 23 KB |
| support.html | 23 KB |
| wallet.html | 23 KB |
| learn/roadmap.html | 20 KB |
| learn/start.html | 18 KB |
| learn/contribute.html | 16 KB |
| roadmap.html | 16 KB |
| learn/index.html | 12 KB |
| learn/community-link.html | 10 KB |

### 10.2 CSS sizes

| File | Size |
| ---- | ---- |
| css/app-shell.css | 25 KB |
| css/pages/learn.css | 11 KB |
| css/theme.css | 8 KB |
| css/components.css | 8 KB |

Total CSS: ~52 KB.

Plus inline CSS: ~130 KB (not separately cached).

### 10.3 JS sizes

| File | Size | Loaded |
| ---- | ---- | ------ |
| js/api/mock.js | 17 KB | explorer only |
| js/api/store.js | 15 KB | explorer only |
| js/api/contract.js | 7 KB | explorer only |
| js/logo-3d.js | 7 KB | never |
| js/api/client.js | 5 KB | explorer only |
| js/theme.js | 0.7 KB | all pages |

### 10.4 External resources

| Resource | Page | Notes |
| -------- | ---- | ----- |
| cdnjs.cloudflare.com | index.html | preconnect |
| three.js r128 | index.html | ~600 KB, defer |
| github.com | docs.html, repositories.html, learn/* | links |
| t.me | community.html | link |

### 10.5 Findings

- docs.html is the largest page (85 KB).
- index.html loads Three.js (~600 KB) from CDN.
- Inline CSS cannot be cached separately.
- API JS is scoped to explorer.html only.
- `js/logo-3d.js` is never loaded.

### 10.6 Technical Debt

- MEDIUM: inline CSS adds non-cacheable page weight.
- MEDIUM: Three.js is a heavy homepage dependency.
- LOW: unused logo-3d.js.

---

## 11. Content Consistency Audit

### 11.1 Key facts

| Fact | Where | Status |
| ---- | ----- | ------ |
| Max supply: 371,000,000 AEV | docs.html, genesis.html | Consistent |
| Proof of Presence | docs.html | Documented |
| L1 coordination | docs.html | Documented |
| L2 bridge | docs.html | Documented |
| Decentralized compute infrastructure | learn/* | Consistent |

### 11.2 Findings

- No factual contradictions identified.
- Key protocol facts (supply, presence, L1, L2) are contained
  in docs.html.
- Learning material uses "decentralized compute infrastructure"
  consistently.
- Information is segmented by audience: docs for reference,
  learn for education.

### 11.3 Status

CONSISTENT.

Information is intentionally segmented by audience.

---

## 12. Findings

Summary of the current state.

### Architecture

- Global CSS architecture is clean and consistent.
- Component loader logic is duplicated across all pages with
  multiple variants.
- Navigation logic is duplicated across 8 top-level pages.
- Learn pages use a cleaner pattern (external CSS, simple
  loader).

### Synchronization

- Source and production are aligned after commit d8d6d6b.
- Two backup artifacts remain in production.
- One unrelated untracked file exists in source.

### Content

- Key protocol facts are consistent.
- Information is segmented by audience (docs, learn).

### Accessibility

- H1 hierarchy is correct.
- Focus states are defined globally.
- Keyboard navigation exists.
- Skip links are missing.
- Header buttons lack explicit `type="button"`.

### SEO

- Titles are unique.
- 9 pages lack meta description.
- Canonical links are missing.
- Open Graph is missing.
- No robots.txt or sitemap.xml.

### Performance

- docs.html is 85 KB.
- index.html loads Three.js from CDN.
- Inline CSS is not separately cacheable.

---

## 13. Technical Debt

### MEDIUM

- Multiple component loader implementations (3+ variants).
- Multiple initNav implementations (2 variants).
- 3725 lines of inline page CSS on 9 top-level pages.
- 9 pages without meta description.
- No canonical links.
- Missing skip links.
- Three.js homepage dependency.

### LOW

- Dormant js/logo-3d.js (unused, not integrated).
- Header buttons missing explicit `type="button"`.
- roadmap.html has no ARIA labels.
- No Open Graph.
- No robots.txt.
- No sitemap.xml.
- Production backup artifacts (`.bak-sync-*`).
- Unrelated untracked file (frontend/Дать).

### NONE

- No production blockers identified.
- No structural HTML anomalies.
- No content contradictions.

---

## 14. Recommendations

These recommendations are informational. They will be turned
into decisions in Architecture v2.

### CSS

- Consider moving inline page CSS to `css/pages/*.css`.
- Keep `learn.css` as the reference pattern.

### JS

- Introduce a single canonical component loader.
- Introduce a single navigation runtime.
- Consider `js/pages/*.js` for page-specific behavior.
- Decide the status of `js/logo-3d.js` (keep dormant or remove).

### HTML

- No structural change recommended.

### Accessibility

- Add a skip-to-content link.
- Add `type="button"` to header buttons.
- Review roadmap.html ARIA coverage.

### SEO

- Add missing meta descriptions.
- Add canonical links.
- Add Open Graph metadata.
- Add robots.txt.
- Add sitemap.xml.

### Performance

- Cache inline CSS externally.
- Review Three.js loading strategy.
- Verify cache headers for CSS and JS.

---

## 15. Status

Frontend Architecture Audit v1

State: COMPLETE
Blockers: NONE
Recommended next step: Architecture v2 (decisions document).

---

