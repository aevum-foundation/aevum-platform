# D-049 — Deployment Architecture

**Status:**       Accepted
**Date:**         2026-09-23
**Applies to:**   Deployment, infrastructure, aevum-web

---

## Context

Stage 3 (Deployment Automation) began with A1 — Deployment Audit v1,
which established that:

- Current deployment is fully manual.
- Source and production are represented by separate Git repositories.
- No automation exists (no script, systemd, cron, or git hook).
- Manual synchronization has already produced source/production drift.

A2 — Target Deployment Architecture evaluated three candidate
architectures across 12 criteria (6 Must-have, 6 Nice-to-have), with
qualitative ratings Strong / Moderate / Weak.

Candidates:

- **V1** — rsync-based
- **V2** — git-based (production as Git checkout of `aevum-platform`)
- **V3** — atomic staging + swap

---

## A2 Evaluation Summary

Must-have criteria results:

| # | Criterion | V1 | V2 | V3 |
|---|---|---|---|---|
| 1 | Source-of-truth integrity | Moderate | Strong | Moderate |
| 2 | Atomicity | Weak | Moderate | Strong |
| 3 | Rollback | Weak | Moderate | Strong |
| 5 | Security / Blast Radius | Moderate | Moderate | Moderate |
| 7 | Drift Prevention | Weak | Moderate | Strong |
| 11 | Recovery Time | Weak | Moderate | Strong |

Full evaluation matrix: see A2 working notes
(12 criteria, qualitative ratings, no numeric scoring).

### Interpretation

- **V1** fails 5 of 6 Must-have criteria. Eliminated.
- **V2** satisfies Must-have but does not lead on any of them.
  Its single structural advantage is traceability through Git state.
- **V3** leads on 4 of 6 Must-have criteria and on 3 Nice-to-have
  criteria (Setup Complexity is its only structural weakness).

---

## Decision

**Adopt V3 — atomic staging + swap**, extended with a deployment
manifest.

### Architecture

```

/root/aevum-platform              (source of truth)

/var/www/releases/
├── 2026-09-23-abc1234/       (immutable release snapshot)
│   ├── index.html
│   ├── css/
│   ├── js/
│   ├── img/
│   └── .deploy-manifest.json
├── 2026-09-22-def5678/
└── ...

/var/www/active  →  releases/2026-09-23-abc1234   (symlink)

/var/www/html    →  configured Apache DocumentRoot
(either symlinked to active, or active is DocumentRoot)

```

### Deployment pipeline

1. **Stage** — copy source revision into a new release directory.
2. **Write manifest** — record source revision, timestamp, actor.
3. **Verify** — run static checks against the staged release
   (file presence, HTML validity, integrity of expected files).
4. **Activate** — atomically re-point `active` symlink to the new release.
5. **Retain** — keep N previous releases (configurable, default 5).
6. **Prune** — remove releases beyond the retention window.

### Rollback

```

ln -sfn releases/<previous> active

```

Instant. No file reconstruction. No re-deployment.

### Deployment manifest

Each release contains `.deploy-manifest.json`:

```json
{
  "source_revision": "<git commit sha of aevum-platform>",
  "source_branch": "master",
  "deployed_at": "<ISO timestamp>",
  "deployed_by": "<actor>",
  "deploy_tool": "aevum-deploy",
  "deploy_tool_version": "<semver>"
}
```

This closes the traceability gap that V3 does not natively provide
(compared with V2's built-in Git state), without introducing Git
state onto the production host.

---

Reason

The architecture must serve a static frontend without a build step,
on a single production host, with a long-term goal of single source
of truth and minimal operational risk.

V3 was selected because:

1. Deployment correctness is structural, not bolted on.
   Atomicity, rollback, and drift prevention are properties of the
   architecture itself, not operator discipline.
2. Recovery is instantaneous. Rollback is a symlink change.
3. Production stays clean. Each release is a fresh snapshot;
   historical artifacts cannot accumulate by construction.
4. Automation is natural. Staging, verification, activation, and
   retention are already separated steps — CI/CD and automated
   verification fit without rewriting.
5. No Git state on the production host. The serving surface is
   not a repository. This matches the long-term model established
   in source-of-truth.md.

The deployment manifest extends V3 to match V2's traceability without
importing V2's structural overhead.

---

Consequences

· /var/www/html is no longer a Git repository.
· aevum-web becomes redundant for deployment purposes. Its future
  disposition is governed by A6 (see D-048).
· A deploy tool (aevum-deploy) must be built. Design belongs to A3.
· Apache configuration may need adjustment (symlink following,
  DocumentRoot target) — implementation detail belongs to A4.
· Retention policy must be defined. Implementation detail belongs
  to A3/A4.
· Verification step must be defined. Implementation detail belongs
  to A3/A4.

---

References

· docs/infrastructure/deployment-audit-v1.md (A1)
· docs/infrastructure/source-of-truth.md
· docs/infrastructure/transfer-context-v1.md
· docs/infrastructure/decisions/D-048-frontend-refactor-deferred.md
· A2 working notes (evaluation matrix, 12 criteria)

---

Next

· A3 — Deployment Plan (design of aevum-deploy, retention,
  verification, apache integration)
· A4 — Implementation
· A5 — Validation
· A6 — Legacy aevum-web Decision (covered by D-048 review)
