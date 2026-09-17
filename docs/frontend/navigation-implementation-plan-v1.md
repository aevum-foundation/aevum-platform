# Aevum Navigation Implementation Plan v1

**Status:** Draft
**Last updated:** 2026-09-17
**Based on:** navigation-consistency-audit-v1.md (Section 5)

---

## 1. Purpose

Translate the decisions from navigation-consistency-audit-v1.md
into concrete file-level changes.

This plan does not change code. It defines what will change,
where, and how it will be verified.

---

## 2. Scope

Covers:

- `components/header.html` (desktop header + mobile drawer)
- `components/footer.html` (footer)

Does not cover:

- `docs.html` (Docs Sidebar — no decisions require changes)
- Learn navigation (no decisions require changes)
- Legal pages (privacy, terms — deferred)

---

## 3. Target Routes (verification)

All target routes must already exist before any link is added.

| Target | Route | Status |
| ------ | ----- | ------ |
| Overview | `/` | ✅ |
| Architecture | `/docs.html#architecture` | ✅ |
| Consensus | `/docs.html#consensus` | ✅ |
| Economics | `/docs.html#economics` | ✅ |
| Security | `/docs.html#security` | ✅ |
| Genesis | `/genesis.html` | ✅ |
| Documentation | `/docs.html` | ✅ |
| Repositories | `/repositories.html` | ✅ |
| Roadmap | `/roadmap.html` | ✅ |
| Learn | `/learn/` | ✅ |

---

## 4. Changes

### 4.1 Header — Community URL

| Field | Value |
| ----- | ----- |
| Source | `components/header.html` |
| Current | `/community.html?t=1` |
| Target | `/community.html` |
| Reason | Canonical internal route |
| Route exists | ✅ |
| Verification | `grep 'community.html' components/header.html` |

### 4.2 Footer — Protocol group

| Field | Value |
| ----- | ----- |
| Source | `components/footer.html` |
| Current | Architecture, Consensus, Economics, Genesis |
| Target | Overview, Architecture, Consensus, Economics, Security, Genesis |
| Reason | Unify with Mobile Protocol |
| Routes exist | ✅ |
| Verification | `grep -A10 'footer-group-title.*Protocol' components/footer.html` |

### 4.3 Footer — Development group

| Field | Value |
| ---- | ---- |
| Source | `components/footer.html` |
| Current | Documentation, Roadmap, Repositories |
| Target | Documentation, Repositories, Roadmap |
| Reason | Consistent ordering with Mobile Development |
| Routes exist | ✅ |
| Verification | `grep -A10 'footer-group-title.*Development' components/footer.html` |

### 4.4 Footer — Remove dead links

| Field | Value |
| ----- | ----- |
| Source | `components/footer.html` |
| Current | References to `/status.html`, `/privacy.html`, `/terms.html` |
| Target | Removed from navigation |
| Reason | Pages do not exist |
| Verification | `grep 'status.html\|privacy.html\|terms.html' components/footer.html` (must return nothing) |

### 4.5 Mobile — Protocol group

| Field | Value |
| ----- | ----- |
| Source | `components/header.html` (side-nav) |
| Current | Overview, Architecture, Consensus, Economics, Security |
| Target | Overview, Architecture, Consensus, Economics, Security |
| Reason | No change needed (already aligned); Genesis is Footer-only |
| Verification | `grep -A10 'nav-group-title.*Protocol' components/header.html` |

### 4.6 Mobile — Development group

| Field | Value |
| ----- | ----- |
| Source | `components/header.html` (side-nav) |
| Current | Documentation, Repositories, Roadmap, Learn |
| Target | Documentation, Repositories, Roadmap |
| Reason | Learn is a separate top-level area, not a sub-item of Development |
| Route exists | ✅ |
| Verification | `grep -A10 'nav-group-title.*Development' components/header.html` |

### 4.7 Mobile — Remove dead link

| Field | Value |
| ----- | ----- |
| Source | `components/header.html` (side-nav) |
| Current | Reference to `/status.html` |
| Target | Removed from navigation |
| Reason | Page does not exist |
| Verification | `grep 'status.html' components/header.html` (must return nothing) |

### 4.8 Mobile — Learn

| Field | Value |
| ----- | ----- |
| Source | `components/header.html` (side-nav) |
| Current | Learn in Development group |
| Target | Learn remains accessible from Header; not required in Mobile Development |
| Reason | Header already exposes Learn |
| Verification | Confirmed after 4.6 |

---

## 5. Verification Plan

After changes:

- `grep` for dead links returns nothing.
- `grep` for `?t=1` returns nothing.
- Protocol group in Footer and Mobile use the same link set
  (except Genesis, which is Footer-only).
- Development group in Footer and Mobile use the same link set.
- All target routes resolve to existing files.
- No HTML/CSS/JS errors introduced.
