# Aevum — Source of Truth

**Status:** Active
**Applies to:** Aevum platform, website, deployment
**Last updated:** 2026-09-22

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
GitHub (aevum-platform)
↓
/root/aevum-platform (source of truth)
↓
verify
↓
deploy
↓
/var/www/html (deployment target)
↓
Apache (80/443)
↓
aevumchain.com

text

**Rules:**

- Never edit `/var/www/html` manually.
- Never create parallel frontend copies under `/var/www/`.
- Never deploy from anywhere except `/root/aevum-platform`.

---

## 5. Prohibited

The following are **not allowed**:
/var/www/aevum-platform (removed 2026-09-22)
/var/www/aevumchain (removed 2026-09-22)
frontend-copy
frontend-backup
site-final
site-final-final

text

Backups live only in `/root/archive/` with a date suffix and a README.

---

## 6. Archive Policy
/root/archive/
├── aevumchain-YYYY-MM-DD/ — legacy directories
├── aevum-platform-web-copy-.../ — removed old copies
├── web-legacy-YYYY-MM-DD/ — removed files from production
└── aevum-platform-audit/ — audit manifests

text

Each archive directory must contain a `README.md` explaining:
- what it contains,
- why it was archived,
- when it can be deleted.

---

## 7. Change Log

| Date       | Change                                                    |
|------------|-----------------------------------------------------------|
| 2026-09-22 | Initial document. Archive of `/var/www/aevumchain` and `/var/www/aevum-platform`. Legacy file cleanup in `/var/www/html`. |

