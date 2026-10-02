# Aevum Knowledge Architecture v1

**Status:**   Draft
**Date:**     2026-10-02
**Depends on:** information-architecture-v1.md
**Scope:**    Conceptual model of the Aevum Knowledge Layer.
              Entities, relations, and views. No implementation.

---

## 1. Purpose

This document defines the conceptual model of the Aevum Knowledge Layer:
the single system of record for all structured knowledge published on the
Aevum platform.

It exists to:

- Establish a canonical model of knowledge entities.
- Separate knowledge entities from their user-facing views.
- Prevent fragmentation of content across multiple independent libraries.
- Provide a foundation for the Radar / Intelligence Layer, Learn, Build,
  and Community to consume the same source of truth.

It does not:

- Prescribe storage, keyspaces, or serialization formats.
- Define URL schemes or navigation (see information-architecture-v1.md).
- Define the design of any user-facing section.
- Decide implementation details of the Radar generator.

---

## 2. Scope

The Knowledge Layer is the shared substrate beneath every section of the
Aevum platform that publishes structured content:

    ┌── Learn          — understand Aevum
    ├── Intelligence   — observe what is happening now
Knowledge Layer ────┤
    ├── Build          — build on Aevum
    └── Community      — participate in Aevum

The Knowledge Layer owns the entities. Sections own the views.

No section maintains an independent content library. Any appearance of
content duplication across sections is a view-level concern, not a model
concern.

---

## 3. Principles

### K1. One Knowledge Layer, many views.

There is exactly one source of truth for structured knowledge. Sections
(Learn, Intelligence, Build, Community) are views over this layer, not
independent databases. A section must never own a private store of entities
that duplicates another section's data.

### K2. Entities precede views.

Entities are defined first. Views are projections over entities, defined
after entities exist. A section never dictates the shape of an entity.

### K3. Content is durable; signals are ephemeral; the model is stable.

An Article may live for years. A Signal may live for hours. Both are
first-class entities in the same model. The model does not distinguish
them by lifetime; only their views do.

### K4. One primary Topic per entity; many related Topics.

Every content entity (Article, Research, Signal, Report) has exactly one
primary Topic. It may reference any number of related Topics. This is a
soft relation, not a hierarchy. Topics are not trees.

### K5. Views do not create entities.

A URL, a page, a section, or a navigation entry does not create a new
entity. Entities exist independently of how they are presented.

### K6. The model is open and extensible.

New entity types, new relation types, and new attributes may be added
without restructuring existing entities. Existing entities do not need
to be migrated when a new entity type is introduced.

### K7. Canonical identity is stable.

Every entity has a stable, human-readable identity (a slug) that does not
change when its display title, description, or keywords change. URLs and
cross-references depend on identity, not on mutable presentation.

---

## 4. Entities

The Knowledge Layer defines seven entity types. Each entity has a stable
identity, a set of attributes, and defined relations to other entities.

The set is intentionally small. New entity types are added only when a
distinct concept cannot be represented by an existing entity with a
different relation or attribute (see K6).

---

### 4.1 Topic

A Topic is a stable conceptual subject. It is the organizing principle of
the Knowledge Layer: everything else is attached to a Topic.

Examples: Post-Quantum Cryptography, Proof of Presence, Distributed
Systems, Rust, GPU Compute, Privacy Networks.

A Topic is not:

- a tag (tags are free-form and many-to-many);
- a category tree (Topics are not hierarchical);
- a view (a Topic is not a page).

Attributes (conceptual):

- identity (stable slug)
- title
- description
- keywords
- related Topics

Relations:

- A Topic has many Articles, Research, Signals, Reports (via primary or
  related Topic).
- A Topic relates to other Topics (symmetric, non-hierarchical).

---

### 4.2 Article

An Article is a durable, editorial piece of knowledge. It is written to
be read once and consulted many times. It may live for years.

Examples: "What is ML-KEM?", "How Proof of Presence Works",
"Understanding Post-Quantum Signatures".

Attributes (conceptual):

- identity (stable slug)
- title
- body
- primary Topic
- related Topics
- published date
- updated date
- author or source attribution

Relations:

- An Article has exactly one primary Topic.
- An Article may reference any number of related Topics.
- An Article may reference other Articles (see: Related Articles).
- An Article may reference Research, Source, and Repositories.

---

### 4.3 Research

A Research entity is a deep, structured investigation. It is more formal
than an Article and typically longer, with references and methodology.

Examples: "Survey of Post-Quantum Schemes 2026",
"Comparative Analysis of Consensus Protocols".

Attributes (conceptual):

- identity
- title
- summary
- body
- primary Topic
- related Topics
- publication date
- author(s)
- references

Relations:

- Same shape as Article, with additional reference structure.
- Research may supersede an earlier Research on the same Topic.

---

### 4.4 Signal

A Signal is a time-bound observation derived from an external event or
publication. Signals are ephemeral by design: they are relevant for a
limited window and are typically aggregated, not archived indefinitely.

Examples: "NIST published final ML-KEM parameters",
"New Rust release includes async improvements relevant to Aevum".

Attributes (conceptual):

- identity
- title
- summary
- primary Topic
- related Topics
- observed_at
- source (see 4.6)
- originating Publication (see 4.7, optional)
- confidence or relevance marker

Relations:

- A Signal has exactly one primary Topic.
- A Signal may reference a Source.
- A Signal may derive from a Publication.

---

### 4.5 Report

A Report is a periodic aggregation across many Signals for a Topic or
set of Topics. It is a synthesis, not a single observation.

Examples: "Post-Quantum Trends, October 2026",
"Weekly Intelligence Summary, Week 40".

Attributes (conceptual):

- identity
- title
- period (start, end)
- scope (Topic or set of Topics)
- body (synthesis)
- supporting Signals (by reference)

Relations:

- A Report references many Signals.
- A Report has one primary Topic or an explicit multi-Topic scope.

---

### 4.6 Source

A Source is an external origin of Publications and Signals. It is a
first-class entity, not a string attribute, because Sources have their
own lifecycle, reliability profile, and feed configuration.

Examples: eprint.iacr.org, NIST, Rust Blog, Hugging Face.

Attributes (conceptual):

- identity
- name
- url (canonical home)
- feed_url (RSS / Atom / other)
- kind (RSS, Atom, API, manual)
- reliability or trust marker (editorial)
- topics_of_interest (advisory)

Relations:

- A Source has many Publications.
- A Source may be associated with one or more Topics (advisory).

---

### 4.7 Publication

A Publication is a raw, external item ingested from a Source. It is the
lowest-level entity in the model and exists primarily to be classified
into Signals and Topics.

Examples: a single RSS entry from eprint.iacr.org, a single blog post
from the Rust Blog.

Attributes (conceptual):

- identity
- source (see 4.6)
- title
- url
- published_at
- raw summary or content excerpt
- classification (Topic assignment, with confidence)

Relations:

- A Publication belongs to exactly one Source.
- A Publication may produce zero or more Signals.
- A Publication has a classification result (primary Topic + related).

---

## 5. Relations Overview

The model is not a tree. It is a graph with clear ownership edges.

Ownership (strong) edges:

    Source ────owns────▶ Publication
    Publication ─classifies─▶ Topic (primary)
    Topic ◀───primary──── Article, Research, Signal, Report

Reference (soft) edges:

    Article  ──related──▶ Topic (many)
    Research ──related──▶ Topic (many)
    Signal   ──related──▶ Topic (many)
    Topic    ◀─related──▶ Topic (symmetric)
    Article  ──refs─────▶ Source, Article, Research
    Report   ──refs─────▶ Signal (many)

Derivation edges:

    Publication ─produces─▶ Signal (zero or more)
    Signal      ─aggregated into─▶ Report

The rule of thumb:

- Ownership edges define what something is.
- Reference edges define what something is connected to.
- Derivation edges define how one entity is produced from another.

The Radar / Intelligence generator operates on derivation edges. It does
not own entities directly; it produces Signals and Reports from
Publications, and it classifies Publications into Topics.

---

## 6. Views

A View is a projection of the Knowledge Layer for a specific user intent.
Views do not own entities. Views do not create entities. Views read the
Knowledge Layer and present selected subsets.

The following views are recognized at the conceptual level. Their URLs,
navigation, and layout are out of scope for this document (see
information-architecture-v1.md).

### 6.1 Learning view

Intent: "I want to understand."

Primary entities: Article.
Primary Topic: one.
Examples of presentation: a tutorial page, a conceptual overview,
a guided explanation.

A Learning view reads Articles. It does not maintain its own copy of any
Article. Two Learning pages for the same Topic are two renderings of the
same Article.

### 6.2 Observation view

Intent: "What is happening now?"

Primary entities: Signal, Report.
Primary Topic: one or many.
Examples of presentation: a topic hub, a signals feed,
a periodic report page.

An Observation view reads Signals and Reports. It does not own them.
The lifetime of the underlying entities does not change the fact that
the view is a projection.

### 6.3 Building view

Intent: "I want to build on Aevum."

Primary entities: Research, Article, Source (for tooling and references).
Examples: documentation references, SDK-oriented material, related
repository links.

### 6.4 Community view

Intent: "I want to participate."

Primary entities: any, filtered by community relevance.
Examples: discussion topics anchored to a Topic, contributor notes.

### 6.5 Cross-view rule

Any entity (Article, Research, Signal, Report) may appear in more than
one view. This is expected and correct. It is not duplication. Duplication
would be a second entity describing the same thing. Referencing the same
entity from multiple views is the intended design.

---

## 7. Source of Truth

There is exactly one source of truth for each entity type.

- Topics are defined in the Knowledge Layer.
- Articles, Research, Reports are authored into the Knowledge Layer.
- Publications are ingested into the Knowledge Layer from Sources.
- Signals are derived inside the Knowledge Layer from Publications.
- Sources are registered in the Knowledge Layer.

Views are rendered from the Knowledge Layer. A view that appears to
disagree with another view means the Knowledge Layer is inconsistent;
it does not mean the views are independent.

No section (Learn, Intelligence, Build, Community) may maintain a private
copy of any Knowledge Layer entity. When a section needs to publish an
entity, it asks the Knowledge Layer, and renders the result.

This rule is the primary defense against long-term fragmentation. It is
enforced conceptually here and will be enforced technically at the layer
of the storage adapter when implementation begins.

---

## 8. Worked Example: Post-Quantum Cryptography

This example shows how multiple entities and views compose around one
Topic without any content duplication.

Topic (canonical):

    slug:        post-quantum-cryptography
    title:       Post-Quantum Cryptography
    keywords:    pqc, ml-kem, ml-dsa, quantum resistance
    related:     Cryptography, Distributed Systems, Privacy Networks

Articles (durable, editorial):

    /what-is-ml-kem
        primary topic:  post-quantum-cryptography
        related:        Cryptography

    /understanding-post-quantum-signatures
        primary topic:  post-quantum-cryptography
        related:        Cryptography, Distributed Systems

Research (deep):

    /survey-of-pq-schemes-2026
        primary topic:  post-quantum-cryptography
        references:     eprint.iacr.org, NIST

Signals (ephemeral):

    "NIST finalized ML-KEM parameters"
        primary topic:  post-quantum-cryptography
        source:         NIST
        derived from:   Publication (NIST feed entry)

    "New Rust release adds PQ-safe primitives to std"
        primary topic:  post-quantum-cryptography
        related:        Rust
        source:         Rust Blog

Reports (periodic):

    "Post-Quantum Trends, October 2026"
        primary topic:  post-quantum-cryptography
        references:     all Signals observed in October 2026

Views:

    Learn:        /learn/post-quantum-cryptography
                  → renders the Articles for the Topic

    Intelligence: /intelligence/topics/post-quantum-cryptography
                  → renders Signals, Reports, related Articles

                  /intelligence/signals/post-quantum-cryptography
                  → renders only Signals for the Topic

                  /intelligence/research/post-quantum-cryptography
                  → renders only Research for the Topic

All of the above reference the same underlying entities. There is
exactly one Article titled "What is ML-KEM?". There is exactly one Topic
with slug "post-quantum-cryptography". The number of views is unbounded;
the number of entities is not.

---

## 9. Relation to the Growth Engine

The existing Growth Engine (module path: `growth::*`) defines a closed
`Topic` enum used by its classifier:

    enum Topic {
        PostQuantum,
        Rust,
        GpuCompute,
        BlockchainArchitecture,
        DistributedSystems,
        StorageSystems,
    }

This enum is not the same thing as a Knowledge Topic. It is a
classifier-level representation, used by the ingestion pipeline to
assign an incoming Publication to a coarse bucket.

The conceptual relation is:

    growth::Topic (enum)         classifier-level, temporary
            │
            │  seed / classifier
            ▼
    Knowledge::Topic             canonical entity
            │
            ├── stable identity (slug)
            ├── title
            ├── description
            ├── keywords
            ├── related Topics
            └── content relationships

Rules:

- The Growth Engine may continue to use its enum internally. No
  immediate rewrite is required.
- Knowledge Topics are the canonical entities that appear in views,
  URLs, and cross-references.
- A mapping exists between growth::Topic variants and Knowledge Topic
  slugs. This mapping is data, not code.
- When the Knowledge Layer is implemented, the Growth Engine becomes a
  producer of entities (Publications, Signals) tagged with a Knowledge
  Topic identity, not a holder of the Topic list itself.
- The enum is not deleted, not renamed, and not promoted. It is
  gradually superseded by the canonical model as implementation
  progresses.

This is an evolutionary transition, not a breaking rewrite.

---

## 10. Extensibility

New entity types, new relations, and new attributes are added under the
following constraints:

1. A new entity type is introduced only when no existing entity can
   represent the concept with a different attribute or relation.
2. A new relation type is introduced only when ownership, reference, and
   derivation edges do not cover the case.
3. Adding an attribute to an entity does not require migrating existing
   instances of that entity if the attribute is optional.
4. Adding a new entity type does not require restructuring existing
   entities.
5. Adding a new view does not require any change to the entity model.

Future entity types that may be introduced without violating the model:

- Collection (a curated set of entities across Topics)
- Curated path (an ordered learning sequence of Articles)
- Repository reference (a link to an external code repository,
  treated as a Source-adjacent entity)
- Person or Contributor (an author identity, if attribution needs to be
  first-class)

These are examples, not commitments. They are listed to demonstrate that
the model is open by design.

---

## 11. Anti-Patterns

The following patterns are explicitly rejected by this architecture.
Each is a real failure mode observed in content platforms at scale.

### 11.1 Section-private content stores

Rejected: a section (Learn, Intelligence, Build) owns its own database
of Articles and Topics, independent of other sections.

Why rejected: leads to duplicated entities, divergent metadata, and a
proliferation of parallel search indexes. Eventually the two stores
must be merged, and the merge is expensive and lossy.

Correct model: sections render views over the shared Knowledge Layer.

### 11.2 Topic as tag

Rejected: Topics are treated as free-form tags attached to entities.

Why rejected: tags multiply without bound, lose meaning, and cannot
carry a canonical slug, description, or related Topics. Tags are for
informal grouping; Topics are for structure.

Correct model: Topics are canonical entities with stable identity.

### 11.3 Topic as category tree

Rejected: Topics form a hierarchy (parent / child) with strict levels.

Why rejected: real knowledge does not fit a single tree. Post-Quantum
Cryptography relates to Cryptography, to Distributed Systems, and to
Privacy Networks without a unique parent. Enforcing a tree forces
arbitrary placement decisions that age badly.

Correct model: Topics are a graph with symmetric related edges.

### 11.4 One entity, one view

Rejected: an Article exists in exactly one place on the site, and any
need to surface it elsewhere is solved by duplicating the entity.

Why rejected: duplication creates drift. Views must be allowed to
reference the same entity.

Correct model: many views, one entity.

### 11.5 URL as identity

Rejected: an entity's identity is its current URL. Changing the URL
creates a new entity.

Why rejected: URLs change (navigational restructuring, redirects,
SEO-driven renaming). Identity must be independent of presentation.

Correct model: identity is a stable slug owned by the entity. URLs are
a view-level concern that may change with redirects preserving identity.

### 11.6 Signal as content

Rejected: Signals are treated as durable editorial content, retained
indefinitely, and referenced as long-form material.

Why rejected: Signals are time-bound. Treating them as durable content
inflates the model, degrades search quality, and hides the fact that
the relevant durable knowledge belongs in an Article or Research.

Correct model: Signals are ephemeral; durable knowledge is promoted
into Articles or Research explicitly.

---

## 12. Governance

Any change to this document follows the same governance model as the
rest of the architecture documentation under `docs/architecture/`.

### 12.1 Change categories

- **Editorial** — clarifications, examples, wording. No architectural
  impact. May be applied without a new version.
- **Additive** — new entity types, new views, new attributes described.
  Requires a version increment and explicit note in the changelog.
- **Structural** — changes to the entity set, relation model, or
  principles. Requires a new major version (v2) and a migration note
  describing the impact on existing sections and modules.

### 12.2 Authority

This document is the canonical reference for the Knowledge Layer. Any
document that defines URLs, navigation, storage, or views is
subordinate to it. When a subordinate document conflicts with this
one, this one governs, and the subordinate document is corrected.

### 12.3 Relationship to other architecture documents

- `information-architecture-v1.md` — defines user intent sections.
  Views described here will be assigned to those sections.
- `growth-storage-design-v1.md` — defines storage of Growth entities
  (Publications, Signals, Sources) prior to this document. Its
  entities correspond to a subset of the Knowledge Layer and remain
  compatible.
- `storage-standard.md` — defines storage layer invariants. The
  Knowledge Layer's entities will be persisted through the storage
  layer described there.
- Future documents (`intelligence-integration-v1.md`, IA v2) will be
  subordinate to this one.

---

## 13. Open Questions

The following questions are intentionally left open. They are not
blockers for the conceptual model. They will be resolved before or
during implementation.

1. **Slug canonicalization.** Are Knowledge Topic slugs generated from
   a fixed map, from a naming rule, or from an explicit registry? The
   model requires stable identity; the mechanism for producing the
   initial slug for a new Topic is deferred.

2. **Relation cardinality for Reports.** May a Report span multiple
   primary Topics, or must every Report have exactly one primary Topic
   and several related? Deferred.

3. **Author identity.** Should authorship be a string attribute on
   Article / Research, or a first-class Person entity? Deferred.

4. **Signal retention.** Are Signals retained forever with a time-bound
   relevance flag, or physically removed after a window? Deferred. Both
   options are compatible with the model.

5. **Source reliability.** Is reliability a fixed enum, a numeric score,
   or an editorial flag? Deferred.

6. **Mapping ownership.** Where does the mapping between growth::Topic
   variants and Knowledge Topic slugs live? In code, in a configuration
   file, or in the Knowledge Layer itself? Deferred.

7. **Multi-language.** Are Topics and Articles ever localized into
   other languages? If yes, does the model support localization, or is
   localization handled at the view layer? Deferred.

8. **Editorial workflow.** Who may create, update, and retire entities?
   The model does not prescribe workflow. Deferred.

These questions are recorded so that future decisions are made
explicitly, not by accident.

---

## 14. Summary

The Aevum Knowledge Layer is a single source of truth for structured
knowledge published on the platform. It defines seven entity types:

    Topic, Article, Research, Signal, Report, Source, Publication

These entities are connected by three kinds of edges:

    Ownership, Reference, Derivation

The Knowledge Layer is consumed by views. Views are projections over
entities, defined by user intent:

    Learn          — I want to understand.
    Intelligence   — What is happening now?
    Build          — I want to build on Aevum.
    Community      — I want to participate.

Views do not own entities. Sections do not maintain private content
stores. There is exactly one Article titled "What is ML-KEM?" and exactly
one Topic with slug "post-quantum-cryptography". Everything else is a
presentation.

The existing Growth Engine remains a producer of entities, not an owner
of Topics. Its classifier enum is a temporary representation, gradually
superseded by the canonical Knowledge Topic as implementation proceeds.

This model is designed to survive years of growth without fragmenting
into parallel libraries, parallel sitemaps, or parallel search indexes.
It is the primary defense against the most common long-term failure mode
of content platforms.

---

## 15. One-Sentence Statement

> The Knowledge Layer is the single system of record for Aevum's
> structured knowledge; Learn, Intelligence, Build, and Community are
> views over it, not independent stores, and the Radar / Growth Engine
> is a producer of entities into it, not a separate product beside it.

---

## 16. Changelog

### v1 — 2026-10-02

- Initial draft.
- Defines seven entity types: Topic, Article, Research, Signal, Report,
  Source, Publication.
- Defines three relation kinds: ownership, reference, derivation.
- Defines four views: Learn, Intelligence, Build, Community.
- Establishes the Knowledge Layer as the single source of truth.
- Establishes the relation between `growth::Topic` (enum) and
  Knowledge Topic (canonical entity) as an evolutionary transition.
- Establishes anti-patterns and governance.
- Records open questions for future resolution.

---

## 17. Closing

This document is conceptual. It does not prescribe implementation.

Its purpose is to make the underlying model explicit, so that every
subsequent decision — URL scheme, navigation, storage, generator design,
editorial workflow — is made with the model in view.

When the model and the presentation disagree, the model is corrected
deliberately, or the presentation is brought into line. The model is
never silently contradicted.

End of document.
