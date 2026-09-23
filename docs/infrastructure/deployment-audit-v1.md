# Deployment Audit v1

**Status:** Complete  
**Date:** 2026-09-23  
**Phase:** Stage 3 — Deployment Automation (A1)

---

## 1. Purpose

This document is a **read-only audit** of the current deployment
process for `aevumchain.com`.

It does **not** propose a target solution. Architectural decisions
belong to **A2 — Target Deployment Architecture**.

The goal is to record the current state factually, including
previously observed operational issues, so that future decisions
are based on evidence rather than assumption.

---

## 2. Scope

### In scope

- Source of truth
- Deployment target
- Current deployment mechanism
- Existing deployment automation
- Observed source/production drift
- Operational risks
- Relevant environment constraints

### Out of scope

- Target deployment architecture → A2
- Deployment implementation → A4
- Legacy `aevum-web` future status → A6 / D-049
- Removal of historical production artifacts → A6

---

## 3. Source and Target

### Source of Truth

| Property | Value |
|---|---|
| Path | `/root/aevum-platform` |
| Git remote | `aevum-foundation/aevum-platform` |
| Branch | `master` |
| Relevant contents | `frontend/`, `backend/`, `docs/` |

### Deployment Target

| Property | Value |
|---|---|
| Path | `/var/www/html` |
| Git remote | `aevum-foundation/aevum-web` |
| Branch | `main` |
| Apache vhost | `aevumchain.com` |
| DocumentRoot | `/var/www/html` |
| Ownership | `root:root` |
| Directory permissions | `755` |
| File permissions | `644` |
| Approximate size | ~2.9 MB |

### Server Environment

| Component | Version / Status |
|---|---|
| Apache | Active |
| rsync | 3.2.7 |
| git | 2.43.0 |
| tar | 1.35 |
| systemd | Available (`systemd-run`) |

---

## 4. Current Deployment Process

**Current deployment is fully manual.**

The audit found no dedicated deployment automation in the
examined locations.

There is currently no:

- Deploy script
- Build pipeline
- systemd deployment unit
- systemd deployment timer
- Cron deployment job
- Repository deployment hook
- CI/CD deployment workflow

### Actual process

The deployment process currently consists of:

1. Edit files under `/root/aevum-platform/frontend/`.
2. Commit changes in `aevum-platform` on `master`.
3. Push the source repository to GitHub.
4. Manually copy the required files into `/var/www/html/`.
5. Commit the resulting production tree in `aevum-web` on `main`.
6. Push the production repository to GitHub.

Steps 4–6 are operator-controlled and are not enforced by
deployment tooling.

There is no automated mechanism that verifies that the production
tree corresponds to the intended source revision after deployment.

### Audit evidence

The following checks were performed:

- `/root/aevum-platform/scripts/` — does not exist.
- No deploy/build files were found at the source repository root.
- No deployment scripts were found under `docs/`.
- No Aevum-related systemd deployment units were found.
- No Aevum-related deployment cron jobs were found.
- Git hooks in the examined repositories contain only sample files.
- No dedicated deployment workflow was identified.

---

## 5. Observed Operational Issues

### 5.1 Source / Production Drift

Manual deployment has **already resulted in source/production
drift** that required explicit audit and synchronization.

Confirmed instances:

- `css/components.css` differed between source and production.
- `learn/index.html` differed between source and production.

Both differences were discovered during explicit source/production
auditing and subsequently synchronized.

The audit does **not** establish that each discrepancy was caused by
a specific operator error. It establishes that the current manual
process does not provide an automated guarantee of source/production
consistency.

### 5.2 Historical Artifacts in Production

`/var/www/html/` contains historical directories and files that are
not part of the currently identified static frontend deployment
surface, including:

```text
api/
assets/
backend/
content/
docs/
infrastructure/
```

Their continued presence is a separate production-cleanup and
repository-architecture concern.

No removal is performed or proposed by A1.

Evaluation of these artifacts belongs to A6 — Legacy aevum-web
Decision / D-049.

---

6. Risks

Risk Assessment
Source / production drift Observed
Missing or incomplete manual synchronization Possible
Forgotten commit or push in one of two repositories Possible
Divergent history between source and production repositories Possible
Deployment without automated post-deploy verification Confirmed
Standardized automated rollback Not present
Standardized deployment audit trail Not present

These are properties of the current process, not conclusions about
future architecture.

---

7. Environment Constraints

The following constraints are relevant to A2:

· Production files are owned by root:root.
· The production directory uses 755 permissions.
· Production files use 644 permissions.
· Apache is active and serves /var/www/html.
· No zero-downtime requirement has been established.
· No CDN or external deployment layer has been identified.
· The current frontend deployment does not require a build step;
  it consists of static HTML, CSS, JavaScript, images, and related
  frontend assets.
· rsync, git, tar, and systemd-run are available on the
  server.

These facts describe the current environment and do not constitute
a target deployment design.

---

8. Summary

Current deployment is fully manual, with source and production
represented by separate Git repositories. No dedicated
deployment automation, build pipeline, systemd deployment unit,
cron deployment job, or repository deployment hook was identified
during the audit. Manual synchronization has already resulted in
source/production drift that required explicit audit and
synchronization.

This document establishes the A1 baseline.

Target architecture and remediation decisions are deferred to A2.

---

9. Open Questions for A2

The following questions are intentionally unresolved:

1. Should production be generated as a clean deployment tree, or
   should the existing production layout be preserved?
2. Should aevum-web remain part of the production workflow, or
   should production become an artifact derived from
   aevum-platform?
3. Should deployment use atomic staging and swap, incremental
   synchronization, or another mechanism?
4. What deployment metadata should be retained for auditability?
5. What rollback guarantees are required?
6. What files and directories constitute the authoritative web
   deployment surface?
7. How should source revision and deployed revision be associated
   and verified?

No answers to these questions are made by A1.

---

10. References

· docs/infrastructure/source-of-truth.md
· docs/infrastructure/transfer-context-v1.md
· docs/infrastructure/decisions/POLICY.md
· docs/infrastructure/decisions/D-048-frontend-refactor-deferred.md
· Related future decision: D-049 — aevum-web Future Status (A6)
