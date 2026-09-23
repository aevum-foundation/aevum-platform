# D-050 — Deployment Migration

**Status:**       Accepted
**Date:**         2026-09-23
**Applies to:**   Production deployment, infrastructure

---

## Context

A1–A4 (Stage 3) specified and implemented a new deployment architecture (D-049), based on atomic staging + swap.

Before A5, production state was:

```

Apache DocumentRoot = /var/www/html   (real directory)
/var/www/active                        (did not exist or not connected)
/var/www/releases/                     (staged releases, not active)

```

A5 migrated production to the new architecture.

---

## Decision

Production now runs on the V3 architecture defined in D-049.

Current topology:

```

Apache

|
v
/var/www/html              (symlink)

|
v
/var/www/active            (symlink, deployment pointer)

|
v
/var/www/releases/<release-id>   (immutable release)

```

Concrete values at migration time:

```

/var/www/html   ->  /var/www/active
/var/www/active ->  /var/www/releases/b226246966e0f3fe1eab9e020cb4348e35072806

```

---

## Properties Established

- `/var/www/html` is a symlink to `/var/www/active`.
- `/var/www/active` is the sole deployment pointer.
- `/var/www/releases/` is outside DocumentRoot.
- Release directories are immutable.
- No `.git` state exists under `/var/www/active`.
- Apache configuration was not modified.
- Apache DocumentRoot remains `/var/www/html`.

---

## Backup

A tar archive of the previous `/var/www/html` was created before migration:

```

/root/archive/aevum-html-2026-09-23T13-19-43Z.tar.gz
sha256: c33c34d0a4fe8e648edd2cd8db82e7b3d9c0b78ca00493d46e8d8ff31221cf3a

```

The previous directory remains on disk as:

```

/var/www/html.old

```

---

## Rollback Model

Two levels of rollback exist.

### Normal rollback (new architecture)

```

aevum-deploy rollback [<release-id>]

```

Switches `/var/www/active` to a previous verified release.

### Emergency rollback (pre-migration)

Only if the new architecture itself is broken:

```

test -L /var/www/html && rm /var/www/html
mv /var/www/html.old /var/www/html

```

This is a manual operation. It restores the pre-migration layout.

`/var/www/html.old` must be preserved until the new architecture is deemed stable.

---

## Validation

A5 validation results:

- HTTP 200 on all 15 site pages.
- HTTP 200 on robots.txt and sitemap.xml.
- HTTP 200 on css/theme.css.
- OG and Twitter metadata served correctly.
- Apache successfully follows the two-hop symlink chain.
- `aevum-deploy status` reports the new active release.
- No `.git` under `/var/www/html` or `/var/www/active`.

---

## Consequences

- Production is now driven by `aevum-deploy`, not by manual file copying.
- Deployment is atomic: no intermediate states are visible.
- Rollback is instantaneous via symlink swap.
- Release history is preserved in `/var/www/releases/`.
- The previous layout is preserved as a single backup directory and one tarball.
- `aevum-web` repository is no longer required for production deployment.
  Its disposition is deferred to D-051.

---

## Unresolved

- `/var/www/html.old` — legacy backup, purpose to be decided in A6.
- `aevum-foundation/aevum-web` — legacy repository, purpose to be decided in A6.

Both are intentionally preserved.

---

## References

- `docs/infrastructure/deployment-audit-v1.md` (A1)
- `docs/infrastructure/decisions/D-049-deployment-architecture.md` (A2)
- `docs/infrastructure/deployment-plan-v1.md` (A3)
- `scripts/aevum-deploy` (A4)
- `docs/infrastructure/transfer-context-v1.md`
