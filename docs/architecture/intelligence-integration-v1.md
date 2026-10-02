# Aevum Intelligence Integration v1

**Status:**   Draft
**Date:**     2026-10-02
**Depends on:** knowledge-architecture-v1.md
               information-architecture-v1.md
**Scope:**    How the Intelligence surface integrates with the Aevum
              platform. URL surface, static/dynamic split, content
              factory, deployment, phases. No implementation code.

---

## 1. Purpose

This document defines how the Intelligence surface (currently implemented
as the Growth Engine) becomes part of the Aevum platform rather than a
parallel product beside it.

It exists to:

- Establish Intelligence as a canonical, user-facing surface of
  `aevumchain.com`.
- Fix the URL surface, the static/dynamic split, and the sitemap
  ownership rules.
- Define the Content Factory pipeline that turns Radar output into
  durable, indexable pages.
- Specify the integration phases (E1-E5) with acceptance criteria.
- Record canonical decisions that must not be re-litigated later.

It does not:

- Redefine the Knowledge Layer (see knowledge-architecture-v1.md).
- Decide the final navigation placement (see IA v2).
- Prescribe storage internals (see storage-standard.md).
- Prescribe frontend implementation.

---

## 2. Scope

This document covers the integration of one surface — Intelligence —
across:

- Backend (`backend/`): HTTP surface, scheduler, deployment unit.
- Frontend (`frontend/`): HTML, CSS, JS, sitemap, robots.
- Deployment (`scripts/aevum-deploy`): release model.
- Content Factory: generator from Knowledge Layer to static pages.

It covers these, and only these, parts of the platform.

It does not cover:

- Protocol and AevumDB internals.
- Other platform surfaces (Explorer, Wallet, Community, Accounts).
- Editorial workflow (see knowledge-architecture-v1.md, governance).

---

## 3. Current State

As of 2026-10-02, the following is true.

### 3.1 What exists

- A standalone backend service (`aevum-platform-api`, Rust, actix-web)
  running on `127.0.0.1:8080`.
- A Growth Engine module (`backend/src/growth/`) with:
  - 13 live RSS/Atom sources.
  - ~2091 publications in AevumDB (`./data/growth`).
  - 6 topics with real trend data.
  - 2 active signals.
- An HTTP surface:
  - HTML: `/growth`, `/growth/topics`, `/growth/topics/{slug}`,
    `/growth/opportunities`.
  - JSON: `/api/growth/health|topics|topics/{slug}|opportunities|sources`.
- Apache reverse proxy for `/growth` and `/api/growth`, and for
  `/sitemap.xml` and `/robots.txt`.

### 3.2 What is not integrated

- The `/growth/*` HTML pages use a standalone design. They do not
  import `theme.css`, `app-shell.css`, `components.css`, or
  `components.js` from the site.
- The site has no navigation entry pointing to `/growth/*`.
- The main sitemap is not owned by the site: it is currently served by
  the backend through Apache, and the site's own static sitemap is
  shadowed.
- The backend process is not managed by systemd. It does not survive a
  reboot.
- There is no automatic ingest. The CLI is invoked manually.
- `HEAD` requests return 404 on all backend routes.

### 3.3 What is confirmed to be working

- `GET https://aevumchain.com/growth` → 200 (HTML).
- `GET https://aevumchain.com/growth/topics/post_quantum` → 200 (HTML).
- `GET https://aevumchain.com/api/growth/health` → 200 (JSON).
- `GET https://aevumchain.com/api/growth/topics` → 200 (JSON).
- Apache ProxyPass routes correctly.
- `aevum-deploy` release-based deployment is operational
  (`/var/www/releases/<sha>` → `/var/www/active` → `/var/www/html`).

### 3.4 What is broken

- `HEAD` on any backend route returns 404. Affects some crawlers.
- `/sitemap.xml` served by backend shadows the site's static sitemap.
- `/robots.txt` served by backend shadows the site's static robots.
- Backend lifecycle has no owner (no systemd, no process supervision).
- Ingest has no scheduler.
- The Growth HTML surface has no visual or navigational relationship
  to the site.

Each of these is addressed by a specific phase in Section 6.

---

## 4. Architectural Principles

These principles govern every decision in this document and every
implementation that follows from it.

### P1. Intelligence is a surface, not a product.

Intelligence is a user-facing surface of `aevumchain.com`. It is not a
separate site, not a subdomain, not a parallel product. It shares the
domain, the design system, the navigation model, the SEO contour, and
the release pipeline with the rest of the platform.

### P2. Intelligence does not own a content system.

Intelligence does not maintain its own content library. It is a
publishing and application surface over the unified Knowledge Layer
(see knowledge-architecture-v1.md). It reads entities and renders views.
It does not duplicate entities.

This principle is the primary defense against a future split into
"Learn DB" and "Intelligence DB" and "Growth DB". There is one
Knowledge Layer. There are many views.

### P3. Static first, dynamic second.

Durable content is published as static HTML and deployed through the
existing `aevum-deploy` release pipeline. Dynamic endpoints are used
only where freshness or interactivity require them.

Rationale: if the backend is unavailable, the indexable content of the
platform must still be served. Search engines must see a document, not
an application.

### P4. URLs are a view-level concern.

URLs are not identity. Identity lives in the Knowledge Layer as stable
slugs. URLs are projections of identity and may change with redirects
preserving identity. See knowledge-architecture-v1.md, K7.

### P5. The site owns SEO.

The main `sitemap.xml` and `robots.txt` are owned by the site
(`frontend/`) and published through `aevum-deploy`. The backend
provides data; the site publishes it. The backend does not own the
main sitemap.

### P6. Legacy is preserved, not broken.

Existing `/growth/*` URLs continue to resolve. They redirect (301) to
the canonical `/intelligence/*` surface. Existing API consumers of
`/api/growth/*` continue to work. Renaming is additive, not breaking.

### P7. Evolutionary transition, not rewriting.

The Growth Engine's existing modules (`growth::*`), its enum
(`growth::Topic`), and its CLI remain operational during the
transition. They are superseded gradually, not deleted or rewritten
in a single step.

### P8. Canonical decisions are frozen at this version.

Decisions in Section 5 are canonical for v1 of this document. Changing
them requires a new version of this document and an explicit migration
note. This prevents re-litigation of settled points during
implementation.

---

## 5. Canonical Decisions

The following decisions are canonical for Intelligence Integration v1.
They follow from the principles in Section 4 and the knowledge model in
knowledge-architecture-v1.md. They are not open for reinterpretation
within this version.

### 5.1 Knowledge Layer relationship

Intelligence is a view over the Knowledge Layer. It owns no entities.
Its inputs are:

- Topics (canonical entities, from the Knowledge Layer).
- Signals (ephemeral observations).
- Reports (periodic syntheses).
- Articles and Research (durable editorial content), referenced but
  not owned.

The Growth Engine is a **producer** into the Knowledge Layer. It
classifies Publications, derives Signals, and associates them with
Topics. It does not own Topics; it references them.

The mapping from `growth::Topic` enum variants to canonical Knowledge
Topic slugs is data, not code, and is defined in Section 5.6.

### 5.2 Intelligence as a user-facing section/view

Intelligence is a proposed top-level information-architecture section.
Its final navigation placement (top-level section, sub-section of an
existing section, or a hybrid arrangement) is resolved in IA v2, not
in this document.

However, the following are canonical for v1 regardless of IA v2's
outcome:

- Intelligence is a user-facing surface of `aevumchain.com`.
- Intelligence is reachable from the site's primary navigation.
- Intelligence uses the site's design system (`theme.css`,
  `app-shell.css`, `components.css`, `components.js`).
- Intelligence appears in the site's sitemap.

The specific placement in the navigation tree is deferred to IA v2.

### 5.3 URL canonicalization

The canonical URL surface for Intelligence is:

    /intelligence                                    — index
    /intelligence/topics                             — topics list
    /intelligence/topics/{topic-slug}                — topic hub
    /intelligence/signals                            — signals feed
    /intelligence/signals/{topic-slug}               — signals by topic
    /intelligence/research                           — research index
    /intelligence/research/{slug}                    — research item
    /intelligence/reports                            — reports index
    /intelligence/reports/{period-or-slug}           — report item

The canonical topic slug is human-readable, kebab-case, and stable. It
is not the snake_case value used internally by the Growth classifier.
Example:

    internal (growth::Topic):   PostQuantum
    canonical (Knowledge Topic): post-quantum-cryptography

The mapping is explicit data (see 5.6), not a mechanical transform.

Legacy URL surface:

    /growth                  →  /intelligence                (301)
    /growth/topics           →  /intelligence/topics         (301)
    /growth/topics/{slug}    →  /intelligence/topics/{slug}  (301)
    /growth/opportunities    →  /intelligence/signals        (301)

During E2, legacy `/growth/*` redirects to `/intelligence` as a
section. During E3, once topic pages exist, `/growth/topics/{slug}`
redirects to the corresponding `/intelligence/topics/{canonical-slug}`.

The exact redirect target for `/growth/topics/{slug}` is deferred until
E3, when the canonical slug mapping is finalized.

### 5.4 Static vs Dynamic

Durable content is static; live data is dynamic. The split is:

Static (published through `aevum-deploy`, served from `/var/www/html`):

- `/intelligence` — index page
- `/intelligence/topics` — topics list
- `/intelligence/topics/{slug}` — topic hub
- `/intelligence/research` — research index
- `/intelligence/research/{slug}` — research item
- `/intelligence/reports` — reports index
- `/intelligence/reports/{slug}` — report item

Dynamic (served by backend, Apache proxy):

- `/api/intelligence/topics`
- `/api/intelligence/topics/{slug}`
- `/api/intelligence/signals`
- `/api/intelligence/signals/{slug}`
- `/api/intelligence/sources`
- `/api/intelligence/health`

Rationale: indexable pages must survive backend unavailability. Live
widgets and freshness endpoints may depend on backend availability.

Exception: `/intelligence/signals` and `/intelligence/signals/{slug}`
are dynamic-first in E2 (backend renders them), and may be migrated to
static in a later version if Signals stabilise into a stable view
pattern.

### 5.5 Sitemap ownership

The main `sitemap.xml` is owned by the site. Source of truth:

    frontend/sitemap.xml

Published by `aevum-deploy` into `/var/www/html/sitemap.xml`.

The backend does not own `/sitemap.xml`. The Apache `ProxyPass` for
`/sitemap.xml` and `/robots.txt` is removed during E1.

In later phases (E4), the main sitemap becomes a sitemap index that
references sub-sitemaps:

    /sitemap.xml                    (index)
    /sitemaps/intelligence.xml
    /sitemaps/topics.xml
    /sitemaps/research.xml
    /sitemaps/reports.xml

The sub-sitemaps are generated by the Content Factory and published
through `aevum-deploy`. The index refers to them. Ownership remains
with the site.

### 5.6 Canonical topic slug mapping

The mapping from `growth::Topic` enum variants to canonical Knowledge
Topic slugs is data, not code. For v1, the initial mapping is:

    PostQuantum              →  post-quantum-cryptography
    Rust                     →  rust
    GpuCompute               →  gpu-compute
    BlockchainArchitecture   →  blockchain-architecture
    DistributedSystems       →  distributed-systems
    StorageSystems           →  storage-systems

The mapping may be extended as new Topics are added. The location of
the mapping (config file, Knowledge Layer registry, or code constant)
is deferred (see Section 8).

Rule: internal enum values are never exposed in URLs. Only canonical
Knowledge Topic slugs appear in URLs.

### 5.7 Legacy API compatibility

Both API surfaces exist during and after the transition:

    /api/growth/*          — legacy, preserved
    /api/intelligence/*    — canonical, introduced in E2

The backend may internally route both to the same handler set. The
legacy prefix is not removed in v1 of this document. Removing it
requires a new version with a deprecation plan.

Internal module names remain `growth::*` in v1. Renaming to
`intelligence::*` is a separate, later decision.

### 5.8 HEAD request handling

All backend GET routes respond to HEAD with the same status and headers
as GET, without a body. Implemented explicitly per route, not via
middleware. Rationale: visible in code, testable, no hidden behavior.

Applies to all routes under `/api/growth/*` and `/api/intelligence/*`.

### 5.9 Process management

The backend is managed by a systemd unit:

    /etc/systemd/system/aevum-platform-api.service

Characteristics:

- User: root (for v1).
- Restart: on-failure.
- Environment: `AEVUM_GROWTH_DB`, `AEVUM_INTELLIGENCE_REFRESH_INTERVAL_SECS`.
- No `nohup`, no manual shell startup.

Migrating to a dedicated `aevum` user is deferred to a later phase
(see Section 8), when the account system and permission model are
introduced.

### 5.10 Ingest scheduling

The backend runs an in-process scheduler:

- Interval: 3600 seconds (hourly) by default.
- Override: `AEVUM_INTELLIGENCE_REFRESH_INTERVAL_SECS`.
- Mechanism: `tokio::spawn` + `tokio::time::interval` inside the HTTP
  server process.
- Rationale: AevumDB's single-writer advisory lock requires one process
  per database directory. A separate scheduler process would violate
  this invariant. The HTTP server is the sole owner of the database.

No external cron. No separate scheduler binary.

---

## 6. Target Architecture

This section describes the target shape of the Intelligence surface.
It is normative for what the surface must be at the end of the
integration. It is not a prescription of internal code.

### 6.1 Intelligence surface

The Intelligence surface comprises two families of endpoints on the
same domain:

**HTML (indexable):**

- `/intelligence` — landing: what Intelligence is, current topics,
  current signals, entry points into Topics.
- `/intelligence/topics` — list of all Topics with a short summary.
- `/intelligence/topics/{slug}` — Topic hub: overview, latest Signals,
  related Research, related Articles, related Sources.
- `/intelligence/research` — list of Research items.
- `/intelligence/research/{slug}` — a Research item.
- `/intelligence/reports` — list of periodic Reports.
- `/intelligence/reports/{slug}` — a Report item.
- `/intelligence/signals` — signals feed (dynamic-first).
- `/intelligence/signals/{slug}` — signals by Topic (dynamic-first).

**JSON (consumable):**

- `/api/intelligence/topics`
- `/api/intelligence/topics/{slug}`
- `/api/intelligence/signals`
- `/api/intelligence/signals/{slug}`
- `/api/intelligence/sources`
- `/api/intelligence/health`

Each HTML page is a view over the Knowledge Layer. Each JSON endpoint
exposes the same underlying entities in a machine-readable form.

The HTML surface must:

- Use `theme.css`, `app-shell.css`, `components.css` from the site.
- Load `components.js` and `navigation.js` from the site.
- Provide `<div id="header-container">` and `<div id="footer-container">`.
- Include `<link rel="canonical">` with the canonical URL.
- Include Open Graph and Twitter Card metadata.
- Include JSON-LD structured data where appropriate (Article,
  BreadcrumbList, WebSite).
- Be renderable without JavaScript for the primary content (the
  entity data appears in the HTML).

### 6.2 Content Factory

The Content Factory is the pipeline that turns Knowledge Layer
entities into static HTML pages. It is not a web application. It is a
generator, run periodically, that produces files and hands them to
`aevum-deploy`.

Pipeline:

    Radar (ingest)
        ↓
    AevumDB (Publications, Signals, Topics)
        ↓
    Knowledge Layer (canonical entities)
        ↓
    Generator (Rust binary or CLI subcommand)
        ↓
    Static HTML (frontend/intelligence/**)
        ↓
    Git (commit generated files)
        ↓
    aevum-deploy (publish release)
        ↓
    /var/www/html (served)

Properties:

- Deterministic: same input entities produce the same output HTML.
- Idempotent: running the generator twice produces no spurious diff.
- Incremental: unchanged entities are not re-rendered.
- Reviewable: generated files are readable and diffable in git.
- Reversible: `aevum-deploy rollback` restores the previous release.

The generator owns:

- HTML rendering of entity views.
- Sitemap fragment generation.
- Cross-link generation between related entities.

The generator does not own:

- Ingestion of Publications (Radar does).
- Classification of Publications (Radar does).
- Signal derivation (Radar does).
- Deployment (aevum-deploy does).

The generator is a consumer of the Knowledge Layer, not a producer.

### 6.3 API

The JSON API remains served by the backend, dynamically. It is a
first-class surface for:

- Interactive widgets on HTML pages.
- External consumers (future SDK, dashboards, integrations).
- Freshness endpoints.

The API is not a substitute for the HTML surface. It complements it.

Canonical prefix: `/api/intelligence/*`.

Legacy prefix preserved: `/api/growth/*`.

Both prefixes may be routed to the same handlers in the backend.

### 6.4 Frontend

The frontend integration is layered on the existing design system:

- Location: `frontend/intelligence/` for generated pages.
- Styles: reuse `theme.css`, `app-shell.css`, `components.css`.
  Additional styles, if any, belong in `css/pages/intelligence.css`.
- Scripts: reuse `components.js`, `navigation.js`. Page-specific
  logic belongs in `js/pages/intelligence/*.js`.
- Components: reuse `components/header.html` and
  `components/footer.html`.
- Navigation: a new navigation entry (see 5.2) pointing to
  `/intelligence`. The final placement is resolved in IA v2.

The frontend never renders entities server-side at request time for
static pages. Static pages are pre-rendered by the generator (6.2).

### 6.5 Deployment

Deployment reuses the existing release model. No new deployment
mechanism is introduced.

Release flow for Intelligence content:

1. Generator runs and produces updated HTML in `frontend/intelligence/`.
2. Operator or automation commits generated files.
3. `aevum-deploy deploy <sha>` publishes a new release.
4. `/var/www/active` switches to the new release.
5. `aevum-deploy rollback` reverts if needed.

Release flow for backend changes (API, scheduler, HEAD handlers):

1. Code change is committed and built (`cargo build --release`).
2. Service is restarted (`systemctl restart aevum-platform-api`).
3. Health check confirms `/api/intelligence/health` returns 200.

Deployment of frontend and backend are independent. Neither blocks the
other.

### 6.6 Relationship diagram

    ┌──────────────────────────────────────────────────────┐
    │  Knowledge Layer (single source of truth)            │
    │  Topic · Article · Research · Signal · Report        │
    │  Source · Publication                                │
    └──────────────────────────────────────────────────────┘
             ▲                              │
             │ writes                       │ reads
             │                              ▼
    ┌────────────────────┐        ┌────────────────────────┐
    │  Radar (Growth)    │        │  Content Factory       │
    │  ingest + classify │        │  static HTML generator │
    │  derive Signals    │        │  sitemap fragments     │
    └────────────────────┘        └────────────────────────┘
             │                              │
             │                              ▼
             │                    ┌────────────────────────┐
             │                    │  frontend/intelligence/│
             │                    │  → git → aevum-deploy  │
             │                    │  → /var/www/html       │
             │                    └────────────────────────┘
             │
             ▼
    ┌────────────────────┐
    │  Backend HTTP      │
    │  /api/intelligence │
    │  /api/growth (lg)  │
    │  systemd-managed   │
    └────────────────────┘

---

## 7. Integration Phases

The integration is organized into five phases. Each phase has an
explicit scope, deliverables, and acceptance criteria. A phase is
considered complete only when all of its acceptance criteria are met.

The phases are ordered. No phase begins before the previous phase is
accepted.

### 7.1 Phase E1 — Stabilization

**Goal:** Close technical debt that would otherwise leak into every
subsequent phase.

**Scope:**

- HEAD request handling on all backend routes (see 5.8).
- systemd unit for the backend (see 5.9).
- In-process scheduler for periodic ingest (see 5.10).
- Removal of `/sitemap.xml` and `/robots.txt` ProxyPass entries,
  restoring the site as owner (see 5.5).
- Structured logging for the backend (JSON or key-value, not ad-hoc).

**Deliverables:**

- `backend/src/api/growth.rs` — HEAD handlers on every GET route.
- `/etc/systemd/system/aevum-platform-api.service` — service unit.
- `backend/src/main.rs` — `tokio::spawn` scheduler initialization.
- `/etc/apache2/sites-available/aevumchain-le-ssl.conf` — ProxyPass
  entries for `/sitemap.xml` and `/robots.txt` removed.
- Tests: HEAD returns 200 with empty body for each route.
- Tests: scheduler interval override via environment variable.

**Acceptance criteria:**

- `HEAD /api/growth/health` returns 200.
- `HEAD /api/growth/topics` returns 200.
- `GET https://aevumchain.com/sitemap.xml` returns the site's static
  sitemap (with `/` and top-level pages), not the backend's.
- `GET https://aevumchain.com/robots.txt` returns the site's static
  robots, not the backend's.
- `systemctl status aevum-platform-api` shows active.
- Reboot the VM; backend is up automatically.
- Scheduler runs at the configured interval; logs show periodic
  ingest events.

**Out of scope:** Renaming, URL redirects, design integration.

---

### 7.2 Phase E2 — Integration

**Goal:** Make Intelligence visually and navigationally a part of the
site. The user must not be able to tell where `/docs.html` ends and
`/intelligence` begins.

**Scope:**

- Backend HTML renderer uses the site's design system.
- Backend HTML includes `header-container` and `footer-container`
  and loads `components.js`, `navigation.js`.
- Backend HTML includes canonical, Open Graph, Twitter Card, and
  JSON-LD metadata.
- Apache adds ProxyPass entries for `/intelligence` (backend during
  E2, replaced by static in E3).
- Apache adds 301 redirects: `/growth/*` → `/intelligence`.
- Apache adds `/api/intelligence/*` proxy entry.
- Site navigation gains an Intelligence entry (placement resolved in
  IA v2, but a working entry is required in E2).

**Deliverables:**

- `backend/src/api/public.rs` — renderer emits full site shell.
- `frontend/components/header.html` — navigation entry added.
- `frontend/components/footer.html` — optional footer entry.
- `frontend/css/pages/intelligence.css` — any additional styles.
- Apache config updated.
- Manual visual review: `/intelligence` and `/intelligence/topics`
  look indistinguishable from the site's other pages.

**Acceptance criteria:**

- `GET https://aevumchain.com/intelligence` returns HTML that loads
  `theme.css`, `components.js`, `navigation.js` and renders header
  and footer from the site.
- A user clicking from `/docs.html` to `/intelligence` sees a
  consistent design.
- `GET https://aevumchain.com/growth` returns 301 to
  `/intelligence`.
- `GET https://aevumchain.com/api/intelligence/health` returns 200.
- `GET https://aevumchain.com/api/growth/health` still returns 200
  (legacy).
- Navigation entry to `/intelligence` is visible on the site.

**Out of scope:** Static generator, topic pages, sitemap fragments.

---

### 7.3 Phase E3 — Content Factory

**Goal:** Turn Intelligence from a dynamic surface into a durable,
indexable content surface.

**Scope:**

- Generator binary (Rust) that reads the Knowledge Layer and produces
  static HTML for Topic pages, Research pages, Reports pages, and
  the Intelligence index.
- Generator writes into `frontend/intelligence/**`.
- Generator produces fragment sitemaps into `frontend/sitemaps/`.
- Generator is deterministic, idempotent, and incremental.
- Canonical slug mapping finalized and applied to all generated URLs
  (see 5.6).
- `/intelligence/topics/{slug}` is served from static files, not
  from the backend.

**Deliverables:**

- `backend/src/bin/intelligence-generate.rs` (or similar) — generator.
- `frontend/intelligence/index.html` — generated.
- `frontend/intelligence/topics/index.html` — generated.
- `frontend/intelligence/topics/{slug}.html` — generated per Topic.
- `frontend/intelligence/research/**` — generated.
- `frontend/intelligence/reports/**` — generated.
- `frontend/sitemaps/topics.xml`, `research.xml`, `reports.xml` —
  generated fragments.
- Tests: golden-output comparison for the generator.
- Tests: idempotency (two runs produce no diff).
- Tests: slug mapping correctness.

**Acceptance criteria:**

- `GET https://aevumchain.com/intelligence/topics/post-quantum-cryptography`
  returns a static HTML page, served from `/var/www/html`.
- Stopping the backend does not affect the availability of that page.
- Two consecutive generator runs produce no git diff.
- The page includes canonical URL, Open Graph, JSON-LD, and internal
  links to related Topics, Research, and Signals.
- Legacy redirect
  `/growth/topics/post_quantum` → `/intelligence/topics/post-quantum-cryptography`
  returns 301.

**Out of scope:** Sitemap index, robots.txt changes, full SEO review.

---

### 7.4 Phase E4 — SEO

**Goal:** Make the generated content fully discoverable through search
engines and AI systems.

**Scope:**

- Main sitemap becomes a sitemap index (see 5.5).
- Fragment sitemaps are referenced from the index.
- `robots.txt` allows all crawlers and points to the sitemap index.
- Cross-linking between Topics, Research, and Articles is generated
  systematically.
- JSON-LD structured data is added to all generated pages: WebSite,
  BreadcrumbList, Article (or TechArticle), and where applicable,
  Dataset for research with published data.
- Page titles and descriptions are generated from Knowledge Layer
  metadata.

**Deliverables:**

- `frontend/sitemap.xml` — sitemap index.
- `frontend/sitemaps/*.xml` — fragment sitemaps.
- `frontend/robots.txt` — updated policy.
- JSON-LD blocks in every generated page.
- Cross-link generation in the generator.

**Acceptance criteria:**

- `GET https://aevumchain.com/sitemap.xml` returns a valid sitemap
  index.
- Each fragment sitemap is valid per the sitemaps.org schema.
- All generated pages pass Google Rich Results Test with valid
  Article or TechArticle markup.
- Cross-links between related Topics appear on every Topic page.
- The site passes a Lighthouse SEO audit at 95+ for all generated
  pages.

**Out of scope:** Submission to Google Search Console, Bing Webmaster
Tools, IndexNow.

---

### 7.5 Phase E5 — Accounts and Community

**Goal:** Convert Intelligence readers into participants. This is the
first phase that touches the account system.

**Scope:**

- Email waitlist endpoint and storage.
- Unified account (see auth-v1.md and community-v1.md).
- Community Hub anchored to Topics.
- Forum deferred to a later phase (after traffic exists).

**Deliverables:**

- Waitlist API and storage.
- Account activation flow.
- Community Hub page.
- Anchoring: each Topic has a link to its Community view.

**Acceptance criteria:**

- A visitor can join the waitlist from `/intelligence`.
- A signed-up visitor receives a confirmation and can activate an
  account.
- Each Topic hub links to a Community discussion anchored to that
  Topic.

**Out of scope:** Forum, moderation tooling, notifications at scale.

---

## 8. Acceptance Criteria — Overall

The following criteria describe the end state of Intelligence
Integration v1, after all phases (E1-E5) are complete.

### 8.1 Functional

- Intelligence is a user-facing surface of `aevumchain.com`
  reachable from the site's primary navigation.
- Every canonical URL in Section 5.3 resolves to a working page.
- Every legacy URL in Section 5.3 resolves with a 301 redirect to
  the canonical URL.
- Both API prefixes (`/api/growth/*` and `/api/intelligence/*`)
  return the same responses for equivalent requests.
- `HEAD` requests succeed on all backend routes.

### 8.2 Architectural

- Intelligence owns no content store. All entities come from the
  Knowledge Layer.
- No entity is duplicated between Learn, Intelligence, Build, or
  Community.
- The generator is deterministic and idempotent.
- The site's main sitemap is owned by the site, not the backend.
- The backend is managed by systemd and survives reboot.
- The scheduler runs automatically at the configured interval.

### 8.3 Operational

- `systemctl restart aevum-platform-api` succeeds without manual
  intervention.
- `aevum-deploy rollback` restores the previous release if a
  generated page is broken.
- Backend logs are structured and searchable.
- A failed generator run does not affect serving of the current
  release.

### 8.4 SEO

- The main sitemap is a valid sitemap index that references all
  fragment sitemaps.
- All generated pages include canonical, Open Graph, Twitter Card,
  and JSON-LD metadata.
- Cross-links between related Topics are present on every Topic page.
- The site is not blocked from any major crawler in `robots.txt`.

### 8.5 Experience

- A user cannot distinguish between a native site page and an
  Intelligence page based on design or navigation.
- A user arriving from a search engine to a Topic page can navigate
  to Research, Signals, Articles, and Community from that page
  without returning to the site home.

---

## 9. Deferred Decisions

The following decisions are intentionally deferred to later versions
or to separate documents. Each has a designated owner artifact.

### 9.1 Final IA placement of Intelligence

The final navigation placement (top-level section, sub-section of an
existing section, hybrid) is resolved in IA v2.

### 9.2 Location of slug mapping

Whether the mapping from `growth::Topic` enum variants to canonical
Knowledge Topic slugs lives in a config file, in the Knowledge Layer
registry, or in a code constant. Resolved during E3.

### 9.3 Author identity

Whether authorship of Articles and Research is a string attribute or
a first-class entity (Person, Contributor). Resolved in
knowledge-architecture-v1.md, Open Question 3.

### 9.4 Signal retention

Whether Signals are retained indefinitely with a relevance flag, or
removed after a window. Resolved in knowledge-architecture-v1.md,
Open Question 4.

### 9.5 Multi-language

Whether Topics and Articles are ever localized. Resolved in
knowledge-architecture-v1.md, Open Question 7.

### 9.6 Editorial workflow

Who may create, update, and retire entities. Resolved in a future
editorial workflow document.

### 9.7 Migration to `aevum` user

When and how the backend moves from running as `root` to running as a
dedicated `aevum` user. Resolved when the account system is
introduced.

### 9.8 Renaming of internal modules

Whether internal modules (`growth::*`) are renamed to
`intelligence::*`. Deferred indefinitely. The public surface uses
`/intelligence/*` regardless of internal naming.

### 9.9 Removal of legacy API prefix

When, if ever, `/api/growth/*` is deprecated and removed. Requires a
new version of this document and a public deprecation notice.

### 9.10 Deployment automation for the generator

Whether the generator runs manually, via cron, or via a CI pipeline.
Deferred. The generator itself is required by E3; how it is invoked
is not prescribed by this document.

---

## 10. Summary

Intelligence Integration v1 turns the Growth Engine from a standalone
service into a canonical surface of the Aevum platform.

The integration rests on four commitments:

1. **One Knowledge Layer.** Intelligence owns no content store. It is
   a view over the Knowledge Layer defined in
   knowledge-architecture-v1.md. No entity is duplicated.

2. **Static first, dynamic second.** Durable content is published as
   static HTML through `aevum-deploy`. Dynamic endpoints serve only
   freshness and interactivity. The indexable content survives the
   backend.

3. **Site owns SEO.** The main sitemap and `robots.txt` are owned by
   the site. The backend provides data; the site publishes it.

4. **Evolution, not rewrite.** The Growth Engine keeps its internal
   modules and enum. The public surface moves to `/intelligence/*`.
   Legacy `/growth/*` URLs and `/api/growth/*` continue to work with
   redirects and parallel routing.

The integration proceeds in five phases (E1 Stabilization,
E2 Integration, E3 Content Factory, E4 SEO, E5 Accounts/Community).
Each phase has explicit acceptance criteria. No phase begins before
the previous is accepted.

The result is a single platform: one domain, one design system, one
navigation model, one SEO contour, one release pipeline. Intelligence
is not a second product beside the site. It is a surface of the
platform.

---

## 11. One-Sentence Statement

> Intelligence is a user-facing surface of `aevumchain.com`, built as
> a view over the unified Knowledge Layer, published as static HTML
> through `aevum-deploy`, and integrated into the site's design,
> navigation, and SEO; the Growth Engine remains a producer of
> entities into the Knowledge Layer, and legacy URLs continue to work
> through redirects.

---

## 12. Changelog

### v1 — 2026-10-02

- Initial draft.
- Establishes Intelligence as a user-facing surface, not a separate
  product.
- Establishes the Knowledge Layer relationship: Intelligence owns no
  content store.
- Establishes URL canonicalization: `/intelligence/*` canonical,
  `/growth/*` legacy with 301 redirects.
- Establishes static/dynamic split: durable content static via
  `aevum-deploy`, live data dynamic via backend.
- Establishes sitemap ownership by the site.
- Establishes legacy API compatibility: both `/api/growth/*` and
  `/api/intelligence/*` remain active.
- Establishes HEAD request handling on all backend routes.
- Establishes systemd unit for the backend.
- Establishes in-process scheduler at hourly interval.
- Defines five integration phases (E1-E5) with acceptance criteria.
- Records deferred decisions for future resolution.

---

## 13. Relationship to Other Documents

This document is subordinate to knowledge-architecture-v1.md. When a
conflict arises, knowledge-architecture-v1.md governs.

This document is subordinate to information-architecture-v1.md on
matters of navigation placement. The final placement of Intelligence
is resolved in IA v2, which will be subordinate to this document on
matters of canonical URLs and surface behavior.

Related documents:

- `knowledge-architecture-v1.md` — Knowledge Layer model.
- `information-architecture-v1.md` — user-facing section model.
- `growth-storage-design-v1.md` — storage of Growth entities.
- `storage-standard.md` — storage layer invariants.
- `auth-v1.md` — account system (used in E5).
- `community-v1.md` — community model (used in E5).
- `deployment-plan-v1.md` — `aevum-deploy` design.

---

## 14. Closing

This document defines what Intelligence Integration v1 is. It is the
canonical reference for the Intelligence surface until superseded by
a future version.

Its purpose is to make every decision explicit and auditable before
implementation begins. Where implementation and this document
disagree, either the implementation is corrected or this document is
revised deliberately. Silent divergence is not acceptable.

The Intelligence surface is not a second product. It is a view over
the Knowledge Layer, presented through the site. This single
commitment is the difference between a coherent platform and a
collection of parallel projects under one domain.

End of document.
