# Aevum Site Migration v1

**Status:** Draft
**Depends on:** information-architecture-v1.md
**Last updated:** 2026-09-17

---

## 1. Purpose

This document tracks the migration from the current site state
(IA v1, Section 3) to the target architecture (IA v1, Section 4).

It is a working artifact derived from the reviewed IA model.
It does not modify the IA itself.

The migration process is organized by Page Classification (IA v1,
Section 6), so that each page is first understood in terms of
what it is, before deciding where it belongs.

---

## 2. Methodology

### 2.1 Scope

This document covers three distinct layers:

- **Page migration** — the information architecture of pages.
- **Navigation and link integrity** — dead links, broken references.
- **Repository hygiene** — legacy files, backups, temporary artifacts.

These layers are tracked separately. A dead link, for example,
is not automatically a page migration decision.

### 2.2 Action vocabulary

Each page is assigned exactly one action.

#### KEEP

The page remains a separate resource.

Criteria:

- Has an independent user purpose.
- Does not duplicate another page.
- Matches its Page Classification.
- Can be clearly mapped to the Target Architecture.
- Does not require structural change.

KEEP does not mean "never change". Content and UI may still evolve.

#### MOVE

The page is preserved as a concept, but its location or URL changes.

Used only if:

- The current URL or location conflicts with the Target Architecture.
- Moving improves the conceptual structure.
- The page does not need to be merged.

#### RENAME

The page's name changes without changing its informational identity.

Used only when the current name creates a misleading expectation.
RENAME is not used for aesthetics.

#### MERGE

Two or more pages are combined into a single resource.

Used only if:

- User purposes substantially overlap.
- Separate existence is not justified.
- Merging reduces duplication or cognitive load.

#### REMOVE

The resource should no longer exist as part of the site.

Basis:

- Proven duplicate.
- Obsolete or legacy artifact.
- No independent user purpose.
- Deliberately excluded after review.

A dead link alone does not imply REMOVE. If the page does not
exist, there is nothing to remove.

#### FUTURE

A conceptually needed resource that is not ready for full
implementation.

FUTURE is not a way to defer uncertain pages. A confirmed future
user purpose must exist.

#### CREATE

The resource does not currently exist but is required by the
architecture.

Used for:

- Referenced resources that are missing (dead links).
- Standard pages required for a complete site (e.g., legal pages).

CREATE applies only to Section 4 (Dead Links), not to Section 3
(Current Pages).

### 2.3 Process

Pages are processed in groups defined by Page Classification
(IA v1, Section 6):

    Landing → Documentation → Learning → Operational
           → Community → Reference → Legal

For each page, the following columns are recorded:

| CURRENT | TYPE | TARGET | ACTION | NOTES |

Each group is reviewed before the next group begins.

### 2.4 Relationship to IA v1

This document does not make architectural decisions. Those are
made in IA v1 and its review.

This document applies IA v1 to the current site state.

No page-level migration decision is made in this document until
the corresponding page has been analyzed and the analysis has
been reviewed.

### 2.5 Target classification

The TARGET column uses two kinds of values.

**User-facing sections** (from IA v1, Section 4):

- Learn
- Build
- Network
- Community
- Support

**Internal target classification** (not user-facing sections):

- **About** — used for reference artifacts that explain the
  protocol or project (e.g., Genesis, Roadmap). About is an
  internal classifier, not a top-level navigation section.
- **Platform entry point** — used for the landing page.

Internal classifications are used for architectural reasoning.
They do not imply new top-level sections.

---

## 3. Current Pages

### 3.1 Landing

| CURRENT | TYPE | TARGET | ACTION | NOTES |
| ------- | ---- | ------ | ------ | ----- |
| `index.html` | Landing | Platform entry point | KEEP | Primary site entry point. Introduces Aevum and directs users to Explorer and Documentation. |

### 3.2 Documentation

| CURRENT | TYPE | TARGET | ACTION | NOTES |
| ------- | ---- | ------ | ------ | ----- |
| `docs.html` | Documentation | Build | KEEP | Protocol reference with deep structured content and sidebar navigation. |

### 3.3 Learning

| CURRENT | TYPE | TARGET | ACTION | NOTES |
| ------- | ---- | ------ | ------ | ----- |
| `learn/index.html`          | Learning | Learn | KEEP | Learn entry point. Introduces the learning model and routes users to Start, Paths, Roadmap, Contribute, and Community. |
| `learn/start.html`          | Learning | Learn | KEEP | Learning entry point explaining levels, paths, curriculum, and how to begin. |
| `learn/roadmap.html`        | Learning | Learn | KEEP | Learning curriculum covering six levels from Computer Science to Contributor. |
| `learn/paths.html`          | Learning | Learn | KEEP | Learning paths covering Beginner, Rust/Systems, Cryptography, Protocol/Blockchain, Compute/GPU, and Contributor. |
| `learn/contribute.html`     | Learning | Learn | KEEP | Learning-oriented contributor pathway covering contribution scope, expectations, and ways to get help. |
| `learn/community-link.html` | Learning | Learn | KEEP | Learn-to-Community bridge explaining where participants communicate and how to ask questions. |

### 3.4 Operational

| CURRENT | TYPE | TARGET | ACTION | NOTES |
| ------- | ---- | ------ | ------ | ----- |
| `explorer.html` | Operational | Network | KEEP | Network explorer providing an operational interface for epochs, network activity, transactions, and addresses. |
| `wallet.html` | Operational | Network | FUTURE | Wallet resource reserved for the planned wallet experience. Current page describes intended functionality but does not provide wallet operations. |

### 3.5 Community

| CURRENT | TYPE | TARGET | ACTION | NOTES |
| ------- | ---- | ------ | ------ | ----- |
| `community.html` | Community | Community | KEEP | Community hub. Describes the protocol community, participation paths, rules, and links to Telegram and repositories. References the planned Aevum Forum. |

### 3.6 Reference

| CURRENT | TYPE | TARGET | ACTION | NOTES |
| ------- | ---- | ------ | ------ | ----- |
| `genesis.html` | Reference | About | KEEP | Protocol reference artifact covering Genesis, principles, monetary design, L1 flow, and timeline. Not a guided learning page. |
| `roadmap.html` | Reference | About | KEEP | Public project roadmap covering protocol and platform development. Distinct from the Learn curriculum roadmap. |
| `repositories.html` | Reference | Build | KEEP | Official open-source entry point for Aevum repositories, capabilities, security, and contribution access. |

### 3.7 Support

| CURRENT | TYPE | TARGET | ACTION | NOTES |
| ------- | ---- | ------ | ------ | ----- |
| `support.html` | Support | Support | KEEP | Voluntary protocol support. Explains the support model, contribution principles, and participant guidance. |

---

## 4. Dead Links

Current navigation contains references to resources that do not yet exist.

| CURRENT | TYPE | TARGET | ACTION | NOTES |
| ------- | ---- | ------ | ------ | ----- |
| `/privacy.html` | Legal | Support | CREATE | Referenced in footer, file does not exist. Standard legal page. |
| `/terms.html` | Legal | Support | CREATE | Referenced in footer, file does not exist. Standard legal page. |
| `/status.html` | Operational | Network | FUTURE | Referenced in navigation but not yet implemented. Reserved for future network status and health reporting. |

---

## 5. Legacy Artifacts

The following files are tracked by Git but are not referenced
by any active page, stylesheet, or script.

| FILE | ACTION | NOTES |
| ---- | ------ | ----- |
| `css/app-shell.css.bak-header-actions-20260902-030442` | REMOVE | Backup artifact. Not referenced. |
| `css/app-shell.css.bak-mobile-drawer-20260902-025356` | REMOVE | Backup artifact. Not referenced. |
| `docs.html.bak-20260903-052928` | REMOVE | Backup artifact. Not referenced. |
| `js/api/store.js.tmp` | REMOVE | Older version (v1) of store.js (v3). Not referenced. |
| `css/style.css` | REVIEW | Legacy stylesheet. Zero references in active HTML/CSS/JS. Contains legacy tokens and classes that differ from current theme. Requires content audit before removal. |

---

## 6. Navigation Models

The current site uses five independent navigation systems. They
are documented here as-is, without proposed changes.

Current navigation systems are not fully synchronized.
Differences are documented here as current-state observations.
Resolution is deferred to the implementation phase.

### 6.1 Header Navigation

**Structure (desktop):**

    Explorer | Docs | Wallet | Community | Learn | Support

**Purpose:** primary top-level navigation for desktop users.

**Scope:** six items. Flat. No sub-menus.

**Notes:**

- Includes `Learn` → `/learn/`
- Includes `Community` → `/community.html?t=1`
- Does not expose `Repositories`, `Roadmap`, or `Genesis` as direct
  desktop header items.

### 6.2 Footer Navigation

**Structure (5 groups):**

    Protocol:      Architecture, Consensus, Economics, Genesis
    Network:       Explorer, Network Status
    Ecosystem:     Wallet, Community
    Development:   Documentation, Roadmap, Repositories
    Support:       Support Aevum

**Plus legal:**

    Privacy | Terms

**Purpose:** secondary navigation, deep links.

**Scope:** five groups + legal.

**Notes:**

- References `/status.html` (does not exist)
- References `/privacy.html` (does not exist)
- References `/terms.html` (does not exist)
- `Community` link without `?t=1` (inconsistent with header)

### 6.3 Side-nav / Mobile Drawer

**Structure (5 groups):**

    Protocol:      Overview, Architecture, Consensus, Economics, Security
    Network:       Explorer, Network Status
    Ecosystem:     Wallet, Community
    Development:   Documentation, Repositories, Roadmap, Learn
    Support:       Support Aevum

**Purpose:** primary mobile navigation.

**Scope:** five groups.

**Notes:**

- Same five group names as footer, but different contents
- `Protocol` group includes `Overview`, `Security` (not in footer)
- `Development` group includes `Learn` (not in footer)
- References `/status.html` (does not exist)

### 6.4 Docs Sidebar

**Structure (8 groups):**

    Introduction:      What is Aevum, Why Aevum, Architecture Overview
    L1 Architecture:   Slots, Epochs, Presence, EpochSnapshot, Consensus, Finality
    Monetary System:   Genesis Supply, Emission, Halvings, Maximum Supply, Post-Emission Economy
    Cryptography:      Addresses, Transactions, State Roots, Wallet Security
    Network:           Nodes, P2P, Synchronization, Recovery
    L2:                Overview, Transactions, Smart Contracts, L1 Finality
    Ecosystem:         Overview, Nexa, Nova, Vault
    Development:       Repository, Local Development, Testing, Roadmap

**Purpose:** in-page navigation within `docs.html`.

**Scope:** eight groups, anchor links only.

**Notes:**

- Uses active-state styling and ARIA state for sidebar navigation.
- Styles are inline in `docs.html` (not in an external stylesheet).

### 6.5 Learn Navigation

**Structure (6 pages):**

    /learn/index.html
    /learn/start.html
    /learn/roadmap.html
    /learn/paths.html
    /learn/contribute.html
    /learn/community-link.html

**In-page navigation:** each page has its own in-page anchors
(for example, `#beginner`, `#rust-systems`, `#cryptography`).

**Purpose:** guided learning navigation.

**Scope:** six pages + in-page anchors.

**Notes:**

- No shared sidebar
- Sequential section navigation is currently used on `paths.html`
  and `roadmap.html`.
- Other Learn pages rely on cross-links and in-page anchors rather
  than the sequential section-navigation pattern.
