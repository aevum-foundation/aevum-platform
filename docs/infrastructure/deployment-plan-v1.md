# Deployment Plan v1

**Status:**   Draft (A3 in progress)
**Date:**     2026-09-23
**Phase:**    Stage 3 — Deployment Automation (A3)
**Based on:** D-049 (Deployment Architecture)
**Scope:**    Design only. No implementation in this document.

---

## 1. Purpose

This document specifies the design of `aevum-deploy` — the deployment tool that will replace the current manual deployment process for `aevumchain.com`.

It defines:

- Directory layout
- CLI interface
- Deployment pipeline
- Manifest and verification formats
- Activation and rollback semantics
- Failure modes

It does not contain implementation. Implementation belongs to A4.

---

## 2. Directory Layout (A3.1)

```

/var/www/
├── releases/
│   └── <full-commit-sha>/
├── active -> releases/<full-commit-sha>
└── html   -> active

```

### Invariants

1. `releases/<release-id>/` is immutable after activation.
2. `active` is the sole deployment pointer.
3. `html` preserves the existing Apache DocumentRoot.
4. Deployment switches `active`; it does not modify an active release.
5. Rollback switches `active` to an existing previous release.
6. `releases/` lives outside DocumentRoot as a physical directory.
7. No `current` symlink between `active` and release.

### Symlink chain

```

Apache -> /var/www/html -> /var/www/active -> /var/www/releases/<revision>

```

Exactly two symlink hops.

---

## 3. CLI Interface (A3.2)

### Style

Subcommand-based CLI.

```

aevum-deploy <command> [arguments] [options]

```

### Primary commands

```

aevum-deploy deploy <revision>
aevum-deploy rollback [<release-id>]
aevum-deploy list
aevum-deploy status
aevum-deploy prune
aevum-deploy verify <release-id>

```

### Global interface

```

aevum-deploy --help
aevum-deploy --version

```

### Implementation

```

/root/aevum-platform/scripts/aevum-deploy

```

Executable Python script. Python standard library only.

### Exit codes

```

0   SUCCESS
1   GENERAL_FAILURE
2   USAGE_ERROR
3   VERIFICATION_FAILED
4   ACTIVATION_FAILED
5   ROLLBACK_FAILED
10  RELEASE_NOT_FOUND
11  PERMISSION_ERROR
12  RELEASE_EXISTS
13  RETENTION_FAILURE
14  CONFLICT

```

### Boundary rule

A successful activation must not be reported as a failed deployment solely because post-deployment retention/pruning failed.

---

## 4. Pipeline Overview (A3.3)

### Deployment transaction

```

ACQUIRE LOCK

RESOLVE

STAGE

MANIFEST (initial: verification=pending, parent_release=null)

VERIFY

DETERMINE PARENT_RELEASE

FINALIZE MANIFEST

ACTIVATE

POST-ACTIVATE VERIFY

SUCCESS

RELEASE LOCK

```

No production mutation occurs before successful activation.

### Maintenance transaction

```

ACQUIRE LOCK

RETENTION POLICY

SAFETY CHECKS

REMOVE ELIGIBLE RELEASES

RELEASE LOCK

```

Prune failure does not mark deployment as failed.

### Locking

- A single deployment transaction per host.
- fcntl.flock() on /run/aevum-deploy.lock.
- Fail-fast: exit 14 CONFLICT if lock is held.
- Lock protects the entire state transition.
- Prune, rollback, and any other state-changing operation must acquire the same lock.

### Release identity

release-id is the full Git commit SHA.

```

/var/www/releases/88428e5328f7199392159df4ae3c84640b23525c/

```

- Uniqueness guaranteed by Git.
- Release maps one-to-one to source revision.
- Timestamp lives in manifest, not in directory name.

---

## 5. STAGE (A3.3.1)

### Input

revision — any Git ref:

- full SHA
- short SHA
- tag
- branch

### Resolution

```

<revision>
    |
full commit SHA
    |
/var/www/releases/<full-sha>/
```

Source

Snapshot extracted from Git revision, not from working tree.

```
Git revision
    |
exact tracked snapshot
    |
deployment whitelist
    |
release directory
```

Deployment surface (whitelist)

```
Top-level files:
    *.html

Directories:
    learn/
    css/
    js/
    img/
    components/

Files:
    robots.txt
    sitemap.xml
```

Excluded (not copied)

```
backend/
content/
assets/
api/
infrastructure/
docs/
```

STAGE steps

1. Resolve revision to immutable full SHA.
2. Create /var/www/releases/<full-sha>/.
3. Extract ONLY deployment surface from Git snapshot.
4. Perform staging safety checks (whitelist, no symlinks, allowed file types).
5. Write initial manifest.
6. Hand off to VERIFY.

Rules

· STAGE never touches /var/www/active or /var/www/html.
· Production is unchanged until ACTIVATE.
· Whitelist validated twice: STAGE and VERIFY (defense in depth).
· Reject unexpected filesystem entries.
· Reject symlinks in staged content.

---

6. MANIFEST (A3.3.2)

File

```
/var/www/releases/<full-sha>/.deploy-manifest.json
```

Required fields (schema_version 1)

Initial manifest (written during STAGE):

```json
{
  "schema_version": 1,
  "release_id": "<full-sha>",
  "source_revision": "<full-sha>",
  "source_ref_input": "master",
  "deployed_at": "2026-09-23T10:30:00Z",
  "deployed_by": "<actor>",
  "deploy_tool": "aevum-deploy",
  "deploy_tool_version": "0.1.0",
  "parent_release": null,
  "verification": {
    "status": "pending"
  }
}
```

Final manifest (finalized immediately before ACTIVATE):

```json
{
  "schema_version": 1,
  "release_id": "<full-sha>",
  "source_revision": "<full-sha>",
  "source_ref_input": "master",
  "deployed_at": "2026-09-23T10:30:00Z",
  "deployed_by": "<actor>",
  "deploy_tool": "aevum-deploy",
  "deploy_tool_version": "0.1.0",
  "parent_release": "<previous-full-sha-or-null>",
  "verification": {
    "status": "passed"
  }
}
```

Optional fields (not used in schema_version 1)

· source_branch (metadata, not identity)
· hostname
· deployment_surface
· file_count
· total_bytes

Unknown fields

Rejected for schema_version 1. Future schema versions may add fields.

Lifecycle

```
STAGE                 -> initial manifest (verification=pending, parent_release=null)
VERIFY                -> verification updated to passed|failed
DETERMINE PARENT      -> parent_release read from current active symlink
FINALIZE MANIFEST     -> parent_release written; manifest is now FINAL
ACTIVATE              -> release becomes immutable
```

Rules

· Manifest is finalized immediately before ACTIVATE, not earlier.
· parent_release is determined under the deployment lock, just before activation, to avoid race conditions with concurrent deployment or rollback operations.
· After ACTIVATION, neither release contents nor manifest may be modified by the deployment system.
· Manifest records provenance; it does not prove content integrity.
· parent_release is a historical record, not a rollback source of truth.
· Rollback source of truth: the active symlink.
· Immutability is a semantic rule. chmod 444 may be considered as hardening in A4/A5, but is not part of the architectural contract.

---

## 6. MANIFEST (A3.3.2)

### File

```

/var/www/releases/<full-sha>/.deploy-manifest.json

```

### Required fields (schema_version 1)

Initial manifest (written during STAGE):

```json
{
  "schema_version": 1,
  "release_id": "<full-sha>",
  "source_revision": "<full-sha>",
  "source_ref_input": "master",
  "deployed_at": "2026-09-23T10:30:00Z",
  "deployed_by": "<actor>",
  "deploy_tool": "aevum-deploy",
  "deploy_tool_version": "0.1.0",
  "parent_release": null,
  "verification": {
    "status": "pending"
  }
}
```


Final manifest (finalized immediately before ACTIVATE):

```json
{
  "schema_version": 1,
  "release_id": "<full-sha>",
  "source_revision": "<full-sha>",
  "source_ref_input": "master",
  "deployed_at": "2026-09-23T10:30:00Z",
  "deployed_by": "<actor>",
  "deploy_tool": "aevum-deploy",
  "deploy_tool_version": "0.1.0",
  "parent_release": "<previous-full-sha-or-null>",
  "verification": {
    "status": "passed"
  }
}
```


### Optional fields (not used in schema_version 1)

- source_branch (metadata, not identity)
- hostname
- deployment_surface
- file_count
- total_bytes

### Unknown fields

Rejected for schema_version 1. Future schema versions may add fields.

### Lifecycle

```

STAGE                 -> initial manifest (verification=pending, parent_release=null)
VERIFY                -> verification updated to passed|failed
DETERMINE PARENT      -> parent_release read from current active symlink
FINALIZE MANIFEST     -> parent_release written; manifest is now FINAL
ACTIVATE              -> release becomes immutable

```

### Rules

- Manifest is finalized immediately before ACTIVATE, not earlier.
- parent_release is determined under the deployment lock, just before activation, to avoid race conditions with concurrent deployment or rollback operations.
- After ACTIVATION, neither release contents nor manifest may be modified by the deployment system.
- Manifest records provenance; it does not prove content integrity.
- parent_release is a historical record, not a rollback source of truth.
- Rollback source of truth: the active symlink.
- Immutability is a semantic rule. chmod 444 may be considered as hardening in A4/A5, but is not part of the architectural contract.

---

## 7. VERIFY (A3.3.3)

### Purpose

VERIFY is the production gate. A release may only be activated if VERIFY passes. It is the single barrier between staged and activated.

VERIFY produces two artifacts inside the release directory:

- .deploy-manifest.json (updated: verification.status = passed|failed)
- .deploy-verify.json (detailed check results)

### Severity model

Checks are divided into two severities:

- BLOCKING — a single failure makes VERIFY FAILED. ACTIVATE is forbidden.
- ADVISORY — a failure produces a warning but does not fail VERIFY.

Rule:

```

0 blocking failures  -> VERIFY PASSED
1+ blocking failures -> VERIFY FAILED

```

Advisory warnings never make a deployment fail.

### Structural checks (BLOCKING)

- Release directory exists.
- Release directory is non-empty.
- All filesystem entries are regular files or directories.
- No symlinks.
- No sockets, pipes, or devices.
- No files outside the deployment whitelist.
- Manifest exists and is valid JSON.
- Manifest schema_version is supported.
- All required manifest fields present.
- Manifest contains no unknown fields (schema_version 1).

### Content checks (BLOCKING)

Required files exist:

- index.html
- robots.txt
- sitemap.xml
- components/header.html
- components/footer.html

Every URL declared in sitemap.xml resolves to a local file within the release, using the sitemap-to-file mapping defined in A3.6.

Every local asset reference in HTML resolves to a file within the release:

- link href
- script src
- img src

Every local asset reference in CSS url(...) resolves.

### Semantic core (BLOCKING)

- HTML is parseable.
- Every HTML page has a <title>.
- No obvious production placeholders in served content: TODO, FIXME, LOREM, XXX.
  Placeholder scan respects an allowlist for legitimate uses.
- No absolute paths from the build environment appear in served content (e.g. /root/...).

### Semantic extended (ADVISORY)

- Open Graph tags present on all pages.
- Twitter Card tags present on all pages.
- Additional SEO consistency checks.

### Provenance (BLOCKING)

- manifest.release_id equals the release directory name.
- manifest.release_id equals manifest.source_revision.
- manifest.schema_version is supported.

### Scope limitations

- JavaScript is not parsed for AST-level analysis in this version.
- HTML semantic linting is intentionally shallow.

### Verification report

Written to:

```

/var/www/releases/<full-sha>/.deploy-verify.json

```

Format:

```json
{
  "schema_version": 1,
  "release_id": "<full-sha>",
  "verified_at": "2026-09-23T10:31:12Z",
  "status": "passed",
  "checks": [
    {
      "name": "manifest_valid",
      "severity": "blocking",
      "status": "passed"
    },
    {
      "name": "required_files",
      "severity": "blocking",
      "status": "passed"
    },
    {
      "name": "html_titles",
      "severity": "blocking",
      "status": "passed"
    },
    {
      "name": "og_tags",
      "severity": "advisory",
      "status": "passed"
    }
  ]
}
```

On failure, status is failed and each failed check entry includes a reason.

Lifecycle

```
STAGE                 -> manifest: verification=pending
VERIFY PASSED         -> verify report written
                      -> manifest: verification=passed
VERIFY FAILED         -> verify report written
                      -> manifest: verification=failed
                      -> ACTIVATE FORBIDDEN
                      -> active unchanged
```

Rules

· Whitelist is validated again in VERIFY (defense in depth).
· Sitemap-to-file mapping is defined in A3.6 and not heuristic.
· Advisory warnings are recorded but do not block activation.
· On VERIFY failure, the release remains on disk for diagnosis but cannot be activated.

---

## 8. ACTIVATE (A3.3.4)

### Purpose

ACTIVATE is the only step that mutates production. All prior steps operate in isolation within the release directory.

### Preconditions

- Deployment lock is held (acquired at start of deployment transaction).
- VERIFY status is passed.
- Release directory exists and is valid.
- /var/www/active exists (or will be created).
- Temporary symlink and active symlink are on the same filesystem.

### Atomic swap

Activation uses atomic rename semantics, implemented via Python:

```

os.symlink(target, /var/www/.active.<pid>.tmp)
os.replace(/var/www/.active.<pid>.tmp, /var/www/active)

```

os.replace() uses the OS's atomic rename semantics on Unix.

This guarantees that /var/www/active transitions from one valid state to another; never intermediate.

### Deterministic parent_release

Immediately before the atomic swap:

1. Read current active symlink.
2. Validate current active (target exists, points to a valid release).
3. parent_release = current active release id.
4. Create temporary symlink.
5. Atomic replace.

### Active state matrix

| State of /var/www/active       | Behavior                              |
|--------------------------------|---------------------------------------|
| -> target release              | success, idempotent (exit 0)          |
| missing                        | create (exit 0)                       |
| -> different release           | atomic swap (exit 0)                  |
| regular directory              | CONFLICT (exit 14)                    |
| broken symlink                 | CONFLICT (exit 14)                    |
| os.replace failed              | ACTIVATION_FAILED (exit 4)            |

Broken active symlink is treated as CONFLICT, not silently repaired. The deployment tool must not silently heal a potentially corrupted production state.

### Post-activate verification

After successful os.replace():

Technical check:

```

readlink(/var/www/active) == /var/www/releases/<full-sha>

```

Semantic check:

Read /var/www/active/.deploy-manifest.json and confirm:

- manifest.release_id == expected release_id
- manifest.verification.status == passed

If both checks pass: ACTIVATION = SUCCESS.

### Post-activate failure semantics

If os.replace() succeeded but post-activate verification failed:

- ACTIVATION = COMPLETED
- POST-ACTIVATE VERIFICATION = FAILED
- No automatic rollback.

Production is already switched. The deployment system does not rewrite history. Rollback is a separate command.

### Apache

Apache is not restarted, reloaded, or reconfigured during activation. The symlink swap is transparent to Apache.

HTTP acceptance / health check belongs to A5.

### Idempotency

If active already points to target release:

```

exit 0 (already active)

```

This is distinct from RELEASE_EXISTS (exit 12), which applies to STAGE.

### Invariant

No production mutation occurs before the atomic swap. After the atomic swap, the deployment state is considered changed and is not automatically rolled back.

---

## 9. PRUNE (A3.3.5)

### Purpose

PRUNE removes old releases according to the retention policy. It is a separate command; not part of the deployment pipeline.

PRUNE failure does not affect deployment success.

### Protected releases (never pruned)

- The active release (target of /var/www/active symlink).
- The N most recent releases by deployed_at (retention window).
- Any release marked with a .do-not-prune marker file.
- Any release with an invalid or missing manifest (flagged for manual review).
- Any path outside /var/www/releases/ (realpath protection).

### Retention policy

Default: keep N releases total (including active).

Default N = 5.

Override:

```

aevum-deploy prune --keep <n>

```

Semantics: keep = total number of releases allowed on disk (including active).

Example: with keep=5 and 10 releases, 5 are removed, 5 remain (including active).

### Safety checks (per release before removal)

1. Release exists.
2. Release is not active (via readlink).
3. Release is not in protected window.
4. Release has no .do-not-prune marker.
5. Release path resolves inside /var/www/releases/ (realpath).
6. Release is a real directory, not a symlink.

Any failure = release is skipped, not removed.

### Removal

Removal uses shutil.rmtree() (Python standard library).

Additional safety: do not cross filesystem boundaries.

### Prune report

Written to:

```

/var/log/aevum-deploy/prune-<timestamp>.json

```

Format:

```json
{
  "pruned_at": "2026-09-23T10:40:00Z",
  "active_release": "<full-sha>",
  "policy": {"keep": 5},
  "candidates": ["<sha1>", "<sha2>"],
  "deleted": ["<sha1>", "<sha2>"],
  "failed": [],
  "skipped": [
    {"release": "<sha3>", "reason": "active"},
    {"release": "<sha4>", "reason": "protected_marker"},
    {"release": "<sha5>", "reason": "within_retention"}
  ]
}
```

Rules

· Prune acquires the same lock as deploy and rollback.
· Prune never removes the active release.
· Prune never removes a protected release.
· Prune failure produces exit code 13 (RETENTION_FAILURE).
· Prune failure does not affect the deployment success of a prior deploy.
· Idempotency: prune with no candidates returns exit 0 with "nothing to prune".

---

## 10. Rollback (A3.7)

### Purpose

Rollback re-points /var/www/active to a previously deployed and verified release. It does not recreate files or re-run verify.

Rollback is a separate command. It is never triggered automatically by the deployment system.

### Command

```

aevum-deploy rollback [<release-id>]

```

- Without argument: rollback to parent_release recorded in current active manifest (if it exists and is valid).
- With argument: rollback to the specified release-id.

### Preconditions

- Deployment lock is held.
- Target release exists in /var/www/releases/.
- Target release is a valid immutable release (has .deploy-manifest.json and .deploy-verify.json).
- Target release manifest verification.status == passed.
- Target release is not the currently active release.

### Behavior

Rollback uses the same atomic swap mechanism as ACTIVATE:

```

os.symlink(target, /var/www/.active.<pid>.tmp)
os.replace(/var/www/.active.<pid>.tmp, /var/www/active)

```

No files are copied. No verification is re-run. The target release is already validated.

### Determining parent_release

If invoked without an explicit release-id:

1. Read current /var/www/active.
2. Read current active manifest.
3. parent_release = manifest.parent_release.
4. Validate parent_release exists and is a valid release.
5. If parent_release is null or invalid: exit 10 (RELEASE_NOT_FOUND).

### Rollback result

- Success: active symlink re-pointed, exit 0.
- Target release invalid: exit 10 (RELEASE_NOT_FOUND).
- Target release = current active: exit 0 (no-op, already active).
- Atomic swap failed: exit 5 (ROLLBACK_FAILED).
- Lock conflict: exit 14 (CONFLICT).

### After rollback

The rolled-back-from release remains on disk. It is not deleted.

The rolled-back-to release becomes active. Its manifest is unchanged.

A new manifest entry is not created for the rollback operation itself. Rollback state is derivable from active symlink + release manifests.

### Rules

- Rollback never modifies release contents.
- Rollback never re-runs verification.
- Rollback does not create a new release.
- Rollback is atomic via os.replace().
- Rollback does not automatically prune.
- Rollback respects the same lock as deploy and prune.

### Distinction from ACTIVATE

ACTIVATE brings a newly staged release into production.

Rollback re-activates a previously deployed release.

Both use the same atomic swap mechanism and the same lock, but they have distinct exit codes and distinct preconditions.

---

## 11. Apache Integration (A3.8)

### Purpose

Define how Apache serves the active release without config changes.

### DocumentRoot

Apache DocumentRoot remains /var/www/html (unchanged).

Apache config is not modified by deployment. No reload, no restart.

### Symlink chain

```

Apache -> /var/www/html -> /var/www/active -> /var/www/releases/<release-id>

```

Exactly two symlink hops.

### Requirements

Apache must be allowed to follow symlinks in DocumentRoot:

- Options FollowSymLinks (or Options Indexes FollowSymLinks)
- AllowOverride matching current production configuration

### Verification of requirement

Actual verification of Apache symlink following belongs to A5 (validation).

During A4 (implementation), the chain is tested on a staging path.

### Migration to new layout

Transition from current state (/var/www/html as real directory) to new layout (/var/www/html as symlink) is a one-time migration.

Migration procedure belongs to A4.

One-time migration steps (summary):

1. Create /var/www/releases/.
2. Move current /var/www/html contents into /var/www/releases/<current-sha>/ (matching an appropriate revision).
3. Create /var/www/active -> releases/<current-sha>.
4. Create /var/www/html -> active (symlink).
5. Verify Apache serves content correctly.

### Rules

- Apache config is never modified by aevum-deploy.
- Apache is never reloaded or restarted by aevum-deploy.
- DocumentRoot /var/www/html remains stable.
- Only the /var/www/active symlink is switched.
- The atomic swap is invisible to Apache at the request level.

### Fallback if symlink following is disabled

If Apache cannot follow symlinks, alternative approaches (in order of preference):

1. Enable FollowSymLinks for DocumentRoot (minimal change).
2. Change DocumentRoot to /var/www/active directly.
3. Replace symlink-based activation with bind-mount-based activation.

The choice is an A4/A5 implementation detail, informed by the actual Apache configuration on the production host.

---

## 12. Failure Modes (A3.9)

### Purpose

Define how aevum-deploy behaves under failure. Ensures that failures are classified, contained, and never silently corrupt production.

### Failure categories

A. Input failures (before STAGE)
B. Staging failures (during STAGE / MANIFEST)
C. Verification failures (during VERIFY)
D. Activation failures (during ACTIVATE)
E. Post-activation failures (after ACTIVATE)
F. Maintenance failures (during PRUNE / ROLLBACK)
G. Concurrency failures (lock conflicts)

### A. Input failures

- Invalid revision (unknown ref, typo): exit 10 (RELEASE_NOT_FOUND).
- Unknown command: exit 2 (USAGE_ERROR).
- Missing argument: exit 2 (USAGE_ERROR).
- Permission denied on source repository: exit 11 (PERMISSION_ERROR).

Production is unchanged. No partial state is created.

### B. Staging failures

- Git extraction failed: exit 1 (GENERAL_FAILURE). Cleanup: partial release directory is removed.
- Release directory already exists: exit 12 (RELEASE_EXISTS). No data modified.
- Filesystem write failed: exit 11 (PERMISSION_ERROR). Cleanup: partial release directory is removed.
- Staging safety checks failed (whitelist, symlinks): exit 1 (GENERAL_FAILURE). Cleanup: release directory is removed.

Production unchanged.

### C. Verification failures

- Blocking check failed: exit 3 (VERIFICATION_FAILED).
- Manifest invalid or missing required fields: exit 3.
- Required files missing: exit 3.
- HTML parse error: exit 3.
- Placeholder detected: exit 3.

Release remains on disk for diagnosis but cannot be activated.

Active release unchanged.

### D. Activation failures

- Precondition violated (VERIFY not passed): exit 4 (ACTIVATION_FAILED).
- Broken active symlink detected: exit 14 (CONFLICT).
- Active is a regular directory: exit 14 (CONFLICT).
- os.replace() raised: exit 4 (ACTIVATION_FAILED). Production unchanged (atomic operation).
- Temp symlink could not be created: exit 4 (ACTIVATION_FAILED). Production unchanged.

Production unchanged until os.replace() succeeds.

### E. Post-activation failures

- Post-activate readlink mismatch: ACTIVATION = COMPLETED, POST-VERIFY = FAILED. No automatic rollback.
- Post-activate manifest check failed: same as above.

Production is on new release. Operator must decide: investigate or rollback manually.

### F. Maintenance failures

- Prune: safety check failed for a release: that release is skipped, others proceed.
- Prune: shutil.rmtree() raised: exit 13 (RETENTION_FAILURE). Partial cleanup possible.
- Rollback: target release invalid: exit 10 (RELEASE_NOT_FOUND).
- Rollback: atomic swap failed: exit 5 (ROLLBACK_FAILED).

### G. Concurrency failures

- Lock acquisition failed (another deployment in progress): exit 14 (CONFLICT).
- No waiting; fail-fast is intentional.

### General rule

Every failure mode is:

- Classified (a specific exit code).
- Contained (production is not corrupted).
- Recorded (in stdout and, where applicable, in a report file).
- Recoverable (via retry, rollback, or manual intervention).

---

## 13. References

- docs/infrastructure/deployment-audit-v1.md (A1)
- docs/infrastructure/decisions/D-049-deployment-architecture.md (A2)
- docs/infrastructure/source-of-truth.md
- docs/infrastructure/transfer-context-v1.md
- docs/infrastructure/decisions/POLICY.md

### Status

This document is complete for A3 (Deployment Plan).

Implementation belongs to A4.

Validation belongs to A5.

Legacy aevum-web disposition belongs to A6.
