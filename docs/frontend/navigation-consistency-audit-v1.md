# Aevum Navigation Consistency Audit v1

**Status:** Draft
**Last updated:** 2026-09-17
**Based on:** site-migration-v1.md (Section 6)

---

## 1. Purpose

Document the current state of navigation systems and identify
inconsistencies.

This is a read-only audit. No code changes are made here.

---

## 2. Scope

Covers:

- Header Navigation
- Footer Navigation
- Mobile Drawer / Side-nav
- Docs Sidebar
- Learn Navigation
- Internal link integrity
- Dead links

---

## 3. Current State

### 3.1 Header Navigation

    Explorer | Docs | Wallet | Community | Learn | Support

Six items. Flat. No sub-menus.

- `Community` → `/community.html?t=1`

### 3.2 Footer Navigation

    Protocol:      Architecture, Consensus, Economics, Genesis
    Network:       Explorer, Network Status
    Ecosystem:     Wallet, Community
    Development:   Documentation, Roadmap, Repositories
    Support:       Support Aevum
    Legal:         Privacy, Terms

### 3.3 Mobile Drawer / Side-nav

    Protocol:      Overview, Architecture, Consensus, Economics, Security
    Network:       Explorer, Network Status
    Ecosystem:     Wallet, Community
    Development:   Documentation, Repositories, Roadmap, Learn
    Support:       Support Aevum

### 3.4 Docs Sidebar

Eight groups:

    Introduction:      What is Aevum, Why Aevum, Architecture Overview
    L1 Architecture:   Slots, Epochs, Presence, EpochSnapshot, Consensus, Finality
    Monetary System:   Genesis Supply, Emission, Halvings, Maximum Supply, Post-Emission Economy
    Cryptography:      Addresses, Transactions, State Roots, Wallet Security
    Network:           Nodes, P2P, Synchronization, Recovery
    L2:                Overview, Transactions, Smart Contracts, L1 Finality
    Ecosystem:         Overview, Nexa, Nova, Vault
    Development:       Repository, Local Development, Testing, Roadmap

### 3.5 Learn Navigation

Six pages:

    /learn/index.html
    /learn/start.html
    /learn/roadmap.html
    /learn/paths.html
    /learn/contribute.html
    /learn/community-link.html

Each page has its own in-page anchors.

### 3.6 Internal Link Integrity

Document internal navigation targets and identify:

- valid routes
- dead routes
- inconsistent route variants
- query-string variants
- links to future pages

Current observations:

- `/community.html?t=1` and `/community.html` both resolve to the
  same page. The query string is not known to have functional
  effect.
- `/status.html` — referenced but does not exist.
- `/privacy.html` — referenced but does not exist.
- `/terms.html` — referenced but does not exist.

---

## 4. Inconsistencies

| # | Inconsistency | Where | Details | Type |
| - | ------------- | ----- | ------- | ---- |
| 1 | Community URL | Header / Mobile vs Footer | `?t=1` vs without | URL |
| 2 | Protocol group | Footer vs Mobile | Different links | STRUCTURE |
| 3 | Development group | Footer vs Mobile | Different links | STRUCTURE |
| 4 | Genesis | Footer only | Not in mobile | COVERAGE |
| 5 | Learn | Header + Mobile | Not in footer | COVERAGE |
| 6 | Dead links | Footer + Mobile | `/status.html`, `/privacy.html`, `/terms.html` | DEAD LINK |
| 7 | Header ≠ Footer | Header vs Footer | Different structure | STRUCTURE |

---

## 5. Decisions

| # | Item | Decision | Rationale |
| - | ---- | -------- | --------- |
| 1 | Community URL | TBD | — |
| 2 | Protocol group | TBD | — |
| 3 | Development group | TBD | — |
| 4 | Genesis | TBD | — |
| 5 | Learn | TBD | — |
| 6 | /status.html | TBD | — |
| 7 | /privacy.html | TBD | — |
| 8 | /terms.html | TBD | — |

Decisions use the vocabulary:

- KEEP — no change.
- ALIGN — make consistent with another navigation.
- REMOVE — remove the item.
- FUTURE — keep the reference, but mark as not yet implemented.

The vocabulary is intentionally separate from the migration
document's action vocabulary (which includes CREATE).
