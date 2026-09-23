# Decision Record Policy

**Status:** Active
**Applies to:** All Aevum decisions
**Last updated:** 2026-09-23

---

## 1. Purpose

Aevum records significant architectural, infrastructure, and process
decisions as **Decision Records**. Each decision gets a globally unique
ID and lives as a separate document in this directory.

The goal is to preserve *why* decisions were made, not only *what* was
decided. This is knowledge that cannot be reconstructed from commits
alone.

---

## 2. Format

```

D-NNN

```

- `D-` prefix
- Three-digit zero-padded number
- Starting from `D-001`
- **Globally unique. Never reused.**

---

## 3. Rules

- **IDs are never reused.** Even deprecated decisions keep their IDs.
- **Every Decision Record has a Status:** `Accepted` | `Superseded` | `Deprecated`.
- **Every Decision Record has a Date.**
- **Every Decision Record states the reason** — not only the decision.
- **Superseded decisions are not deleted** — they are annotated and kept.
- **Commits and docs reference Decision IDs** where relevant.

---

## 4. File Naming

```

D-NNN-short-slug.md

```

Example:

```

D-048-frontend-refactor-deferred.md
D-049-deployment-automation.md

```

---

## 5. Template

Each Decision Record SHOULD contain:

```markdown
# D-NNN — Title

**Status:**   Accepted | Superseded | Deprecated
**Date:**     YYYY-MM-DD
**Applies to:** <areas>

## Context

Why this decision is needed.

## Decision

What was decided.

## Reason

Why this decision was made.

## Consequences

What this decision changes or forbids.

## References

Related commits, docs, or decisions.
```

---

6. Directory Layout

```
docs/infrastructure/decisions/
├── POLICY.md
├── D-048-frontend-refactor-deferred.md
├── D-049-...
└── D-050-...
```

POLICY.md is not itself a Decision Record — it is the rulebook that
describes how Decision Records are written and managed.
