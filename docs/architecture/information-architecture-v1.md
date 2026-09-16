# Aevum Information Architecture v1

**Status:** Draft
**Last updated:** 2026-09-17

---

## 3. Current State

The Aevum web platform currently consists of 15 user-facing HTML pages,
organized as a flat page structure with one nested subsection (Learn).

Key observations:

- 15 pages total: 9 top-level + 6 in `learn/`
- Three independent navigation systems (header, side-nav, footer)
  reflect different models of the site
- Learn (v1.0) exists as a self-contained subsection
- Legacy artifacts remain: `css/style.css`, `docs.html.bak-*`, `.tmp` files
- Dead links exist in navigation: `/status.html`, `/privacy.html`, `/terms.html`
- Documentation uses a sidebar-based architecture (inline in `docs.html`)
- No unified IA document has existed until now

Detailed page inventory will be maintained separately in the
`CURRENT → FUTURE → ACTION` table (see Section 10).

---

## 4. Target Architecture

The proposed target architecture organizes Aevum into five top-level
sections based on user intent:

    AEVUM
    ├── Learn       — understand Aevum
    ├── Build       — build on Aevum
    ├── Network     — use Aevum
    ├── Community   — participate in Aevum
    └── Support     — sustain Aevum

This model is intentionally small. It avoids exposing internal
module structure (backend services, protocol layers) as separate
navigation entries.

Each section is defined by:

- A single purpose
- A primary user intent
- A clear boundary against other sections

The five sections represent the conceptual organization of the
platform.

Navigation components (header, footer, mobile navigation, sidebar)
are implementation details and may change independently of this
conceptual model. They are described separately in Section 7.

---

## 1. Purpose

This document proposes the conceptual structure of the Aevum
web platform.

It exists to:

- Establish a working model for the site's information architecture.
- Separate conceptual structure from navigation implementation.
- Provide a foundation for future decisions (CURRENT → FUTURE → ACTION).

This document does not:

- Prescribe UI implementation.
- Decide which pages to delete, merge, or rename.
- Fix the current header, footer, or mobile navigation.

Those decisions belong to later stages.

---

## 2. Principles

### P1. One system, few entry points, clear actions.

The platform is a single system. Users should not need to
understand internal module structure (backend services,
protocol layers) to use it.

### P2. Conceptual structure precedes navigation.

Information architecture defines what sections exist.
Navigation implements how users move between them.
Navigation is not the architecture.

### P3. Sections are defined by user intent, not by backend modules.

A section exists because a user has a reason to go there,
not because a backend service exists.

### P4. Prefer small top-level structure.

Five top-level sections are easier to understand than fifteen.
Internal complexity should not be exposed as navigation entries.

### P5. Structure should remain extensible without adding unnecessary top-level sections.

Future capabilities should fit into the existing conceptual
structure where appropriate. New top-level sections should be
introduced only when a distinct user intent cannot be clearly
represented by an existing section.

---

## 5. Roles

Aevum recognizes five roles. These are not separate portals;
they are ways of using the same system.

### User
Uses the network: holds assets, sends transactions, explores.

### Provider
Contributes resources: compute, storage, bandwidth.

### Researcher
Consumes and produces knowledge: reads docs, runs experiments,
publishes findings.

### Developer
Builds software and integrations around Aevum: writes code,
develops applications, integrates APIs, and works with protocol
interfaces.

### Contributor
Improves Aevum: writes documentation, fixes bugs, contributes
to open source.

Roles may overlap. A single account can act in multiple roles.

Roles are not information-architecture sections or separate portals.

---

## 6. Page Classification

Every user-facing page belongs to one of seven types.

| Type          | Purpose                              | Examples                                      |
| ------------- | ------------------------------------ | --------------------------------------------- |
| Landing       | Entry point, orientation             | index.html                                    |
| Documentation | Reference material                   | docs.html                                     |
| Learning      | Guided education                     | learn/*                                       |
| Operational   | Interact with the network            | explorer.html, wallet.html                    |
| Community     | Participate in discussion            | community.html                                |
| Reference     | Stable artifacts                     | genesis.html, roadmap.html, repositories.html |
| Legal         | Required legal pages                 | privacy.html, terms.html                      |

This classification is descriptive, not prescriptive.
It helps identify which patterns apply to which pages.

Notes:

- `roadmap.html` — product/project reference
- `learn/roadmap.html` — Learning
- `repositories.html` — Reference (classification, not a decision
  about future placement)

---

## 7. Navigation Model

Navigation is an implementation of the information architecture,
not the information architecture itself.

The conceptual structure (Section 4) defines what sections exist.
Navigation defines how users move between them. Implementation
(header, footer, mobile drawer, sidebar) is a separate concern.

Navigation operates at four levels:

### 7.1 Top-level navigation

Top-level navigation exposes the five conceptual sections
(Learn, Build, Network, Community, Support) to users.

It does not necessarily map one-to-one to current header items.
Mapping is a UI decision, made after IA is finalized.

### 7.2 Contextual navigation

Contextual navigation exposes sub-sections within a top-level
section (for example, Learn → Start, Paths, Roadmap, Contribute,
Community).

It may appear as a sidebar, a menu, or inline links.

### 7.3 In-page navigation

In-page navigation exposes sections within a single page
(for example, Docs sidebar, Learn section anchors).

It uses anchors within the current document.

### 7.4 Cross-links

Cross-links connect related content across sections
(for example, a Learn page linking to Docs, or a Community page
linking to Repositories).

They are not part of the primary navigation tree.

---

## 8. Future Components

The following components are part of the platform's planned
evolution. They are documented here to ensure the conceptual
structure can accommodate them without requiring new top-level
sections.

| Component     | Section        | Purpose                                      |
| ------------- | -------------- | -------------------------------------------- |
| Forum         | Community      | Structured discussion for participants        |
| Status        | Network        | Live network status and health                |
| Compute       | Network        | Compute and storage resources                 |
| Dashboard     | (cross-section)| Unified account and activity view             |

New top-level sections should be introduced only when a distinct
user intent cannot be clearly represented by an existing section
(see P5).

---

## 9. Content Ownership

Each top-level section has a primary content responsibility.

Responsibility is conceptual, not organizational.

| Section   | Primary responsibility | Focus                                  |
| --------- | ---------------------- | -------------------------------------- |
| Learn     | education              | Guided learning, curriculum, paths     |
| Build     | engineering            | Documentation, repositories, tooling   |
| Network   | protocol               | Protocol behavior, network operation   |
| Community | community              | Participation, discussion, moderation  |
| Support   | operations             | Sustainability, funding, operations    |

Responsibility does not imply exclusive authorship. It defines
responsibility for the section's coherence.

---

## 10. Migration Notes

This section will track the transition from the current state
(Section 3) to the target architecture (Section 4).

Detailed migration is maintained separately in the
`CURRENT → FUTURE → ACTION` table.

The migration table will:

- List every existing page.
- Classify each page by type (Section 6).
- Map each page to a target section (Section 4).
- Define the action required (KEEP / MERGE / MOVE / RENAME / REMOVE / FUTURE).

No page-level migration decisions are made in this draft until
the architecture has been reviewed.

The migration table is a separate working artifact derived from
the reviewed IA model.
