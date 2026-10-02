# Aevum Information Architecture v2

**Status:**   Draft
**Date:**     2026-10-02
**Supersedes:** information-architecture-v1.md
**Depends on:** knowledge-architecture-v1.md
               intelligence-integration-v1.md
**Scope:**    The user-facing information architecture of the Aevum
              platform. Sections, roles, navigation model, and
              canonical URL families. No implementation.

---

## 1. Purpose

This document defines the information architecture (IA) of the Aevum
platform: what user-facing sections exist, what user intent each
section serves, how users move between them, and how the platform
remains extensible without fragmenting over time.

It exists to:

- Fix the final set of top-level sections.
- Assign a distinct user intent to each section.
- Resolve the placement of Intelligence (deferred in IA v1 and in
  intelligence-integration-v1.md).
- Define the navigation model (top-level, contextual, in-page, cross).
- Define canonical URL families per section (by reference to the
  documents that own them).
- Record the transition from IA v1 and acceptance criteria.

It does not:

- Redefine the Knowledge Layer (see knowledge-architecture-v1.md).
- Redefine the Intelligence surface (see
  intelligence-integration-v1.md).
- Prescribe navigation components, page layouts, or CSS.
- Decide storage, keyspaces, or serialization.
- Prescribe the content pipeline or the deployment model.

---

## 2. Why v2

IA v1 established five top-level sections (Learn, Build, Network,
Community, Support) and explicitly deferred the placement of the
Intelligence surface.

Since IA v1, two things changed:

1. The Knowledge Layer was defined as the single source of truth for
   structured knowledge (knowledge-architecture-v1.md). Learn and
   Intelligence are now understood as views over this layer, not as
   independent content stores.

2. The Intelligence surface was defined as a first-class platform
   surface with its own URL family and phases
   (intelligence-integration-v1.md), while its navigation placement
   was explicitly deferred to IA v2.

IA v2 resolves that deferral and completes the conceptual model. It
does not change the fundamental principles of IA v1: one system, few
entry points, sections defined by user intent, small top-level
structure, extensibility without proliferation.

---

## 3. Scope

This document covers:

- The set of top-level sections.
- The user intent and role for each section.
- The navigation model at all four levels.
- The relationship between sections and the Knowledge Layer.
- Canonical URL families, by reference to the owning document.
- Rules for adding sections, sub-sections, and cross-links.
- The transition from IA v1.
- Acceptance criteria.

This document does not cover:

- Content of any section.
- Visual design or page composition.
- HTTP endpoints.
- Storage or deployment.

---

## 4. Principles

These principles are inherited from IA v1 and reaffirmed here. They
govern the shape of the IA and every decision that extends it.

### I1. One system, few entry points, clear actions.

The platform is a single system. Users should not need to understand
internal module structure (backend services, protocol layers, Growth
Engine) to use it.

### I2. Conceptual structure precedes navigation.

Information architecture defines what sections exist. Navigation
implements how users move between them. Navigation is not the
architecture.

### I3. Sections are defined by user intent, not by backend modules.

A section exists because a user has a reason to go there, not because
a backend service exists. No section is a public face of an internal
module.

### I4. Prefer small top-level structure.

A small number of top-level sections is easier to understand than
many. Internal complexity is not exposed as navigation entries.

### I5. Structure remains extensible.

Future capabilities fit into existing sections where possible. New
top-level sections are introduced only when a distinct user intent
cannot be represented by an existing section.

### I6. One Knowledge Layer, many views.

Sections that publish structured knowledge render views over the
shared Knowledge Layer (knowledge-architecture-v1.md). No section
maintains a private content store. This is the architectural
foundation for Learn, Intelligence, and Build.

### I7. Cross-links are first-class.

Sections reference each other freely through cross-links. A Topic in
Intelligence may link to a Learning path in Learn, to a repository in
Build, and to a discussion in Community. Cross-links are not a
secondary concern; they are part of the IA.

---

## 5. Roles

Aevum recognizes five roles. Roles are ways of using the same system.
They are not separate portals, not separate sections, and not
separate content stores.

| Role        | What they do                                              |
|-------------|-----------------------------------------------------------|
| User        | Uses the network: holds assets, sends transactions, explores. |
| Provider    | Contributes resources: compute, storage, bandwidth.       |
| Researcher  | Consumes and produces knowledge: reads, experiments, publishes. |
| Developer   | Builds software and integrations around Aevum.            |
| Contributor | Improves Aevum: documentation, code, community.           |

Roles may overlap. A single account may act in multiple roles. Roles
do not map one-to-one to sections. Sections are defined by user
intent, not by role.

---

## 6. Target Sections

Aevum has six top-level sections. Each is defined by a single user
intent and a clear boundary against the others.

    AEVUM
    │
    ├── Learn          — Understand
    ├── Intelligence   — Discover what is happening
    ├── Build          — Build and contribute
    ├── Network        — Explore the network
    ├── Community      — Participate
    └── Support        — Get help and support Aevum

### 6.1 Learn

**Intent:** I want to understand.

Learn is the entry point for guided education. It answers conceptual
questions ("What is Aevum?", "How does Proof of Presence work?").
Primary entity: Article (see knowledge-architecture-v1.md).

### 6.2 Intelligence

**Intent:** I want to discover what is happening now.

Intelligence surfaces live signals, recent research, and periodic
reports about the topics Aevum observes. It answers observational
questions ("What changed in post-quantum cryptography this month?").
Primary entities: Signal, Report, Research.

The final placement of Intelligence as a top-level section is
established by this document, superseding the deferral in IA v1 and
in intelligence-integration-v1.md Section 5.2.

### 6.3 Build

**Intent:** I want to build on Aevum and contribute to it.

Build is the entry point for engineering material: documentation,
repositories, SDK, tooling, and contribution paths. It also hosts
Research that is authored by the Aevum team or the community as
engineering artifacts.

### 6.4 Network

**Intent:** I want to explore and use the network.

Network is the entry point for network state and interaction:
blockchain explorer, wallet, network status, and (in the future)
compute and storage resources.

### 6.5 Community

**Intent:** I want to participate.

Community is the entry point for participation: discussions,
contributions, and (in the future) the forum. It is anchored to
Topics and to Learn and Intelligence content.

### 6.6 Support

**Intent:** I want to get help and support Aevum.

Support is the entry point for assistance and for sustaining the
platform: help resources, funding, operations.

### 6.7 Section boundaries

Boundaries are enforced by user intent. When a page could plausibly
belong to two sections, the question is not "which section is this
content about?" but "what intent brings the user here?"

Examples:

- A conceptual introduction to post-quantum cryptography → Learn.
- A monthly report on post-quantum developments → Intelligence.
- An SDK reference for a post-quantum primitive → Build.
- A discussion thread about post-quantum adoption → Community.

The same Topic (post-quantum cryptography) appears in all four
sections. That is expected. It is the same Topic from the same
Knowledge Layer, rendered through different views.

---

## 7. Navigation Model

Navigation is the implementation of the IA. It is not the IA itself.
The IA defines what sections exist; navigation defines how users move
between them. Components (header, footer, mobile drawer, sidebar)
are implementation details and may change independently of the IA.

Navigation operates at four levels.

### 7.1 Top-level navigation

Top-level navigation exposes the six sections to users:

    Learn · Intelligence · Build · Network · Community · Support

The header may not display all six simultaneously. The mobile drawer
and the footer may show different subsets. Mapping the six sections
onto actual header slots is a UI decision, made after IA is
finalized. The IA fixes the set of sections; UI chooses how many are
visible at once.

### 7.2 Contextual navigation

Contextual navigation exposes sub-sections within a top-level section.
It may appear as a sidebar, a horizontal menu, or inline links,
depending on the section.

Examples of contextual navigation:

- Learn → Start, Paths, Roadmap, Contribute
- Intelligence → Topics, Signals, Research, Reports
- Build → Documentation, Repositories, SDK, Contribute
- Network → Explorer, Wallet, Status
- Community → Discussions, Contributors
- Support → Help, Funding

Sub-sections are not fixed by IA v2. They are the responsibility of
each section and may evolve as the section grows. IA v2 fixes only
the boundary condition: sub-sections must serve the section's user
intent and must not duplicate another section's intent.

### 7.3 In-page navigation

In-page navigation exposes sections within a single page. It uses
anchors within the current document.

Examples: a documentation page with a sidebar of anchors; a topic
page with an internal table of contents.

### 7.4 Cross-links

Cross-links connect related content across sections. They are a
first-class part of the navigation model, not an afterthought.

Examples:

- A Learn article on post-quantum cryptography links to the
  Intelligence Topic hub for the same Topic.
- An Intelligence Topic hub links to related Learn articles, related
  Build documentation, and related Community discussions.
- A Build page on a repository links to the Topic it implements.

Cross-links are generated from the Knowledge Layer's relation graph
(see knowledge-architecture-v1.md, Section 5). They are not authored
by hand for each page.

### 7.5 Navigation precedence

When two pieces of navigation compete for the same slot, the
following precedence applies:

1. A section's primary entry point.
2. A cross-link to a related entity.
3. A contextual sub-section entry.

Precedence is a design rule, not a rigid constraint. It exists to
avoid overloading any single navigation surface.

---

## 8. Sections and the Knowledge Layer

The Knowledge Layer is the source of truth for structured knowledge
(knowledge-architecture-v1.md). Sections are views over it.

The relationship is:

    ┌─────────────────────────────────────────────────────┐
    │  Knowledge Layer                                    │
    │  Topic · Article · Research · Signal · Report       │
    │  Source · Publication                               │
    └─────────────────────────────────────────────────────┘
             ▲              ▲              ▲
             │              │              │
             │ view         │ view         │ view
             │              │              │
         ┌───────┐      ┌──────────────┐  ┌───────┐
         │ Learn │      │ Intelligence │  │ Build │
         └───────┘      └──────────────┘  └───────┘

Key rules:

- Learn reads Article entities and renders them as educational pages.
- Intelligence reads Signal, Report, and Research entities and renders
  them as observation pages.
- Build reads Research, Article, and Source entities and renders them
  as engineering references.
- Community reads any entity and renders discussions anchored to it.

No section owns a private copy of any entity. If a Topic appears in
Learn and in Intelligence, there is exactly one Topic entity and two
views. This rule is the primary defense against fragmentation.

### 8.1 Section ownership of views

While no section owns entities, each section owns the shape of its
views:

- Learn owns the definition of "a Learning page".
- Intelligence owns the definition of "an Observation view".
- Build owns the definition of "an engineering reference".
- Community owns the definition of "a discussion".

View shapes are separate from entity shapes. Changing a view shape
does not require changing any entity.

### 8.2 No section owns a Topic

Topics are canonical entities of the Knowledge Layer. A Topic is not
"a Learn topic" or "an Intelligence topic". It is a Topic. Different
sections render it differently. This rule prevents Topic lists from
multiplying across sections.

---

## 9. Canonical URL Families

Canonical URL families are defined by the documents that own the
corresponding surface. IA v2 fixes only the top-level prefixes and
the relationship between them.

| Section      | Prefix            | Owning document                          |
|--------------|-------------------|------------------------------------------|
| Learn        | `/learn/*`        | (to be defined; IA v2 fixes the prefix)  |
| Intelligence | `/intelligence/*` | intelligence-integration-v1.md, Sec. 5.3 |
| Build        | `/build/*`        | (to be defined; IA v2 fixes the prefix)  |
| Network      | `/network/*`      | (to be defined; IA v2 fixes the prefix)  |
| Community    | `/community/*`    | (to be defined; IA v2 fixes the prefix)  |
| Support      | `/support/*`      | (to be defined; IA v2 fixes the prefix)  |

Legacy URL families:

- `/growth/*` → 301 to `/intelligence/*` (see
  intelligence-integration-v1.md, Section 5.3).

Rules:

1. Each section has exactly one canonical URL prefix.
2. Entity identity is never encoded into the URL prefix. The prefix
   denotes the view, not the entity (see knowledge-architecture-v1.md,
   K7).
3. A URL belongs to exactly one section. A page cannot appear under
   two prefixes. If a page is reachable under two prefixes, one is
   canonical and the other is a redirect.
4. The same entity rendered in two sections produces two URLs (one
   per section) both referring to the same canonical entity. This is
   not duplication; it is two views over one entity.

IA v2 does not define the sub-paths inside each prefix beyond what
the owning document specifies. For Intelligence, sub-paths are defined
in intelligence-integration-v1.md Section 5.3. For other sections,
sub-paths will be defined as those sections are built out.

---

## 10. Extensibility Rules

The IA is designed to grow without fragmenting. The following rules
govern every extension.

### 10.1 Adding a sub-section

A sub-section is added within an existing top-level section. It must:

- Serve the section's user intent.
- Not duplicate an existing sub-section in the same section.
- Not serve an intent that clearly belongs to another section.

Adding a sub-section does not require changing this document.

### 10.2 Adding a top-level section

A new top-level section is introduced only when:

- A distinct user intent exists that cannot be represented by an
  existing section without violating that section's boundary.
- The intent is durable (expected to persist for years), not a
  transient capability.
- The intent cannot be expressed as a sub-section of an existing
  section without harming the section's coherence.

Adding a top-level section requires a new version of this document
(v3) with explicit justification and a migration note describing the
impact on navigation and cross-links.

### 10.3 Adding a cross-link

Cross-links are generated from the Knowledge Layer's relation graph.
Adding a cross-link between two entities does not require changing
this document. The rule is: cross-links follow relations, and
relations follow the knowledge model.

### 10.4 Adding a role

Roles are not sections. Adding a role does not require changing this
document unless the role needs a dedicated view, in which case a
sub-section is added within the appropriate section.

### 10.5 Adding a view

A new view of existing entities may be added within any section
without changing this document, provided it serves the section's
intent. If a new view would serve a different intent, it belongs to a
different section, and the boundary rules of Sections 6.7 and 10.2
apply.

### 10.6 Removing a section

Removing a top-level section requires a new version of this document
with a migration plan: where existing content moves, how URLs
redirect, and how navigation is updated. Silent removal is not
acceptable.

---

## 11. Transition from IA v1

IA v1 established five top-level sections and deferred the placement
of the Intelligence surface. IA v2 resolves the deferral and
completes the model.

### 11.1 What changed

| Aspect                        | IA v1              | IA v2                       |
|-------------------------------|--------------------|-----------------------------|
| Top-level sections            | 5                  | 6                           |
| Intelligence placement        | Deferred           | Top-level section           |
| Knowledge Layer relationship  | Not specified      | Sections are views over the Knowledge Layer |
| Canonical URL families        | Implicit           | Explicit prefixes per section |
| Cross-links                   | Mentioned          | First-class, relation-driven |

### 11.2 What did not change

- The five original sections remain: Learn, Build, Network,
  Community, Support.
- Principles I1-I5 remain in force.
- Roles remain: User, Provider, Researcher, Developer, Contributor.
- Sections remain defined by user intent, not by backend modules.
- Small top-level structure remains preferred.

### 11.3 Deferred items from IA v1

The following were deferred in IA v1 and are resolved in IA v2 or in
subordinate documents:

- **Intelligence placement** → resolved in Section 6.2 of this
  document.
- **Sub-section definitions** (Start, Paths, etc.) → deferred to
  per-section documents.
- **Migration of specific pages** → tracked in
  `site-migration-v1.md` and future updates to it.

### 11.4 Compatibility

IA v2 is backward-compatible with IA v1 for existing sections. No
existing page or navigation entry must be removed because of IA v2.
The only additive change is the Intelligence section.

### 11.5 IA v1 status

After IA v2 is accepted, IA v1 is marked as superseded. It remains in
the repository for historical reference. Its principles remain part
of the canon; its section set is replaced by the six-section model in
Section 6.

---

## 12. Navigation Implementation Constraints

IA v2 does not prescribe how navigation is implemented. However, it
imposes the following constraints on any implementation.

### 12.1 All sections are reachable

Every top-level section must be reachable from the site's primary
navigation (header, mobile drawer, or a combination). No section is
hidden from the primary navigation tree.

### 12.2 Active section is indicated

When a user is inside a section, the navigation indicates which
section is active. The indication mechanism (highlight, styling,
aria-current) is a UI concern; the requirement is that it exists.

### 12.3 No orphan sections

No section exists in the IA without a corresponding navigation entry.
A section defined in this document but missing from navigation is a
bug, not a design choice.

### 12.4 No orphan navigation entries

No navigation entry exists without a corresponding section or
sub-section in the IA. Navigation entries that do not correspond to
IA entities are removed.

### 12.5 Cross-links do not replace primary navigation

Cross-links enrich navigation. They do not substitute for the primary
navigation tree. A user must always be able to reach any section
through the primary navigation, independent of any cross-link.

### 12.6 Mobile and desktop parity

The IA is navigation-agnostic. Any section reachable on desktop must
be reachable on mobile, through whatever combination of header,
drawer, and inline navigation the design uses. No section is
desktop-only or mobile-only.

---

## 13. Acceptance Criteria

IA v2 is accepted when the following criteria are met. These are
conceptual criteria for the IA document itself, not for the platform
implementation. Implementation criteria are defined in
intelligence-integration-v1.md and in per-section documents.

### 13.1 Section model

- The set of top-level sections is fixed at six: Learn,
  Intelligence, Build, Network, Community, Support.
- Each section has exactly one user intent, stated explicitly.
- Each section has a stated boundary against other sections.
- No section duplicates another section's intent.

### 13.2 Knowledge Layer relationship

- Sections that publish structured knowledge render views over the
  Knowledge Layer.
- No section owns a private content store.
- The relationship between sections and the Knowledge Layer is
  documented in this document and in knowledge-architecture-v1.md,
  without contradiction.

### 13.3 Navigation model

- The four levels of navigation are defined: top-level, contextual,
  in-page, cross-links.
- Every top-level section is reachable from primary navigation.
- Cross-links are relation-driven, not hand-authored per page.
- No section is desktop-only or mobile-only.

### 13.4 Canonical URLs

- Each section has a canonical URL prefix.
- A URL belongs to exactly one section.
- Legacy URL families are documented with their redirect targets.
- The same entity rendered in two sections produces two URLs that
  both refer to the same canonical entity, and this is documented as
  not-a-duplication.

### 13.5 Extensibility

- Rules for adding sub-sections, top-level sections, cross-links,
  roles, and views are documented.
- Rules for removing a section are documented.
- Adding a sub-section does not require changing this document.

### 13.6 Transition from IA v1

- What changed between IA v1 and IA v2 is explicit.
- What did not change is explicit.
- IA v1 is marked as superseded, not deleted.
- The transition is backward-compatible for existing sections.

---

## 14. Deferred Decisions

The following decisions are intentionally deferred. They are not
blockers for accepting IA v2. They will be resolved as the platform
is built out.

### 14.1 Sub-sections per section

The exact sub-sections inside each top-level section (other than
Intelligence, which is defined in
intelligence-integration-v1.md) are deferred to per-section documents.
IA v2 fixes only the boundary condition that sub-sections must serve
the section's intent.

### 14.2 Header composition

How many of the six sections appear in the desktop header, versus the
mobile drawer, versus the footer, is a UI decision. Deferred to the
design phase.

### 14.3 Order of sections in navigation

The order in which the six sections appear in navigation is a UI
decision. IA v2 lists them in a canonical order (Learn, Intelligence,
Build, Network, Community, Support) for reference, but does not
require this order in the UI.

### 14.4 Roles and accounts

How roles are represented in accounts (badges, capabilities,
permissions) is deferred to the account system documents (auth-v1.md
and future updates).

### 14.5 Cross-section workflows

Some workflows span multiple sections (for example, a Researcher
moves from an Intelligence Signal to a Learn Article to a Build
repository). The precise UX of these workflows is deferred to the
design phase.

### 14.6 Localization

Whether the IA is localized into languages other than English is
deferred. It is compatible with the model but not required by it.

### 14.7 Landing page redesign

The current `index.html` was written under IA v1 assumptions. Whether
and how it changes to reflect the six-section model is deferred to
the design phase. IA v2 does not require changes to the landing page.

### 14.8 Legacy IA v1 artifacts

The existing `information-architecture-v1.md` and
`site-migration-v1.md` remain in the repository. Whether they are
merged, archived, or referenced from IA v2 is deferred.

---

## 15. Summary

IA v2 completes the information architecture of the Aevum platform.
It resolves the deferral from IA v1 by establishing Intelligence as a
top-level section, and it formalizes the relationship between every
section and the shared Knowledge Layer.

The model:

    AEVUM
    │
    ├── Learn          — Understand
    ├── Intelligence   — Discover what is happening
    ├── Build          — Build and contribute
    ├── Network        — Explore the network
    ├── Community      — Participate
    └── Support        — Get help and support Aevum

Every section is defined by user intent. Every section that publishes
structured knowledge renders views over the Knowledge Layer. No
section owns a private content store. Cross-links between sections
are generated from the Knowledge Layer's relation graph and are a
first-class part of navigation.

Extensibility is governed by explicit rules: sub-sections are added
freely; top-level sections are added only when a distinct user intent
cannot be represented otherwise; the model is designed to remain
small and coherent over years.

IA v2 is backward-compatible with IA v1 for existing sections. Its
only additive change is the Intelligence section. IA v1 is superseded
but preserved.

---

## 16. One-Sentence Statement

> The Aevum platform has six user-facing sections — Learn,
> Intelligence, Build, Network, Community, Support — each defined by
> a single user intent, each rendering views over one shared
> Knowledge Layer, connected by relation-driven cross-links, and
> extensible by explicit rules that prevent fragmentation over time.

---

## 17. Changelog

### v2 — 2026-10-02

- Supersedes information-architecture-v1.md.
- Establishes six top-level sections: Learn, Intelligence, Build,
  Network, Community, Support.
- Resolves the deferral of Intelligence placement from IA v1 and from
  intelligence-integration-v1.md Section 5.2.
- Formalizes the relationship between sections and the Knowledge
  Layer: sections are views, not content stores.
- Defines the four-level navigation model: top-level, contextual,
  in-page, cross-links.
- Defines canonical URL prefixes per section.
- Defines extensibility rules for adding sub-sections, top-level
  sections, cross-links, roles, and views.
- Defines navigation implementation constraints.
- Documents the transition from IA v1.
- Records deferred decisions for future resolution.

### v1 — 2026-09-17

- Initial IA document.
- Five top-level sections: Learn, Build, Network, Community, Support.
- Five roles: User, Provider, Researcher, Developer, Contributor.
- Seven page types: Landing, Documentation, Learning, Operational,
  Community, Reference, Legal.
- Four navigation levels.
- Intelligence placement deferred.

---

## 18. Relationship to Other Documents

This document is subordinate to knowledge-architecture-v1.md on
matters of the knowledge model. When a conflict arises about entities
or relations, knowledge-architecture-v1.md governs.

This document is coordinated with intelligence-integration-v1.md on
matters of the Intelligence surface. IA v2 resolves the navigation
placement of Intelligence; intelligence-integration-v1.md defines
its URL surface, content pipeline, and phases. There is no conflict
between them: IA v2 supersedes only Section 5.2 of
intelligence-integration-v1.md, which explicitly deferred the
placement decision to IA v2.

Related documents:

- `knowledge-architecture-v1.md` — Knowledge Layer model.
- `intelligence-integration-v1.md` — Intelligence surface.
- `information-architecture-v1.md` — superseded by this document.
- `site-migration-v1.md` — tracks migration of specific pages.
- `deployment-plan-v1.md` — release pipeline.
- `storage-standard.md` — storage layer invariants.
- `auth-v1.md`, `community-v1.md` — account and community models
  (used in later phases).

---

## 19. Closing

This document defines the conceptual structure of the Aevum platform.
It is the canonical reference for user-facing sections and for how
those sections relate to each other and to the shared Knowledge
Layer.

Its purpose is to make the architecture explicit, so that every
subsequent decision — navigation, URL, content, design — is made
with the model in view. When a decision and this document disagree,
either the decision is corrected or this document is revised
deliberately. Silent divergence is not acceptable.

The platform is one system with six entry points, one Knowledge
Layer, and explicit rules for growth. This is the difference between
a coherent platform and a collection of parallel projects under one
domain.

End of document.
