# Aevum — Source of Truth

**Status:** Active
**Applies to:** Aevum platform, website, deployment
**Last updated:** 2026-09-23

---

## 1. Principle

> **One source of truth. One deployment path. One active copy.**

Aevum follows a strict single-source model. Parallel copies of the same
frontend, deployment tree, or website artifact are prohibited.

---

## 2. Canonical Paths

| Role                | Path                          |
|---------------------|-------------------------------|
| Source of Truth     | `/root/aevum-platform`        |
| Deployment Target   | `/var/www/html`               |
| Live Domain         | `https://aevumchain.com`      |
| Archive             | `/root/archive/`              |

---

## 3. Repository Roles

| Repository                | Role                                        | Visibility |
|---------------------------|---------------------------------------------|------------|
| `aevum-protocol`          | Protocol core (L1). Heart of the project.   | Private    |
| `aevum-db`                | Custom database. Post-quantum.              | Private    |
| `aevum-platform`          | External interaction: apps, site, SDK.      | Public     |
| `aevum-public`            | Public showcase of the protocol (partial).  | Public     |
| `aevum-web`               | **Legacy.** First website implementation.   | Legacy     |

---

## 4. Deployment Flow

```

GitHub (aevum-platform)
↓
/root/aevum-platform     (source of truth)
↓
verify
↓
deploy
↓
/var/www/html           (deployment target)
↓
Apache (80/443)
↓
aevumchain.com

```

**Rules:**

- Never edit `/var/www/html` manually.
- Never create parallel frontend copies under `/var/www/`.
- Never deploy from anywhere except `/root/aevum-platform`.

---

## 5. Prohibited

The following are **not allowed**:

```

/var/www/aevum-platform        (removed 2026-09-22)
/var/www/aevumchain            (removed 2026-09-22)
frontend-copy
frontend-backup
site-final
site-final-final

```

Backups live only in `/root/archive/` with a date suffix and a README.

---

## 6. Archive Policy

```

/root/archive/
├── aevumchain-YYYY-MM-DD/         — legacy directories
├── aevum-platform-web-copy-.../   — removed old copies
├── web-legacy-YYYY-MM-DD/         — removed files from production
└── aevum-platform-audit/          — audit manifests

```

Each archive directory must contain a `README.md` explaining:

- what it contains,
- why it was archived,
- when it can be deleted.

---

## 7. Frontend Status

### Completed

| Area                            | Status |
|---------------------------------|--------|
| Frontend Architecture Audit v1  | ✅     |
| Frontend Architecture v2        | ✅     |
| Frontend Migration Plan v1      | ✅     |
| Frontend Baseline v1            | ✅     |
| Shared Runtime Extraction       | ✅     |
| SEO Rollout                     | ✅     |
| Production Hygiene              | ✅     |

### Current State

**Shared Runtime:**

```

js/components.js      — canonical component loader
js/navigation.js      — canonical navigation runtime
js/theme.js           — theme switcher
js/logo-3d.js         — dormant (kept for future use)
js/pages/*.js         — page-specific modules (4 of 15)

```

**SEO Contract (15/15 pages):**

```

<title>              — unique per page
<meta description>   — unique per page
<link canonical>     — unique per page

<!-- Open Graph -->

og:type              — website
og:site_name         — Aevum
og:title             — = <title>
og:description       — = <meta description>
og:image             — /img/aevum-og.png (1200×630)
og:image:width       — 1200
og:image:height      — 630
og:url               — = canonical

<!-- Twitter Cards -->

twitter:card         — summary_large_image
twitter:title        — = <title>
twitter:description  — = <meta description>
twitter:image        — /img/aevum-og.png

```

**Infrastructure:**

```

robots.txt           — deployed
sitemap.xml          — deployed

```

### Deferred

See `docs/infrastructure/decisions/D-048-frontend-refactor-deferred.md`.

---

## 8. Change Log

| Date       | Change                                                    |
|------------|-----------------------------------------------------------|
| 2026-09-22 | Initial document. Archive of `/var/www/aevumchain` and `/var/www/aevum-platform`. Legacy file cleanup in `/var/www/html`. |
| 2026-09-23 | Add Frontend Status. Document SEO contract (OG + Twitter Cards, 15/15 pages). |
