# Aevum Repository Hygiene v1

**Status:** Draft
**Depends on:** site-migration-v1.md
**Last updated:** 2026-09-17

---

## 1. Purpose

This document tracks repository-level cleanup: files that are
tracked by Git but no longer serve any active purpose.

It is a working artifact derived from site-migration-v1.md
(Section 5, Legacy Artifacts).

It does not modify the site's information architecture.

---

## 2. Scope

This document covers:

- Backup artifacts (`.bak-*`)
- Temporary artifacts (`.tmp`)
- Legacy stylesheets (`css/style.css`)
- `.gitignore` additions

It does not cover:

- Untracked unrelated files (e.g., `frontend/Дать`)
- Active files in use

---

## 3. Inventory

| FILE | STATUS | ACTION | REASON |
| ---- | ------ | ------ | ------ |
| `css/app-shell.css.bak-header-actions-20260902-030442` | tracked | REMOVE | Backup artifact. Not referenced. |
| `css/app-shell.css.bak-mobile-drawer-20260902-025356` | tracked | REMOVE | Backup artifact. Not referenced. |
| `docs.html.bak-20260903-052928` | tracked | REMOVE | Backup artifact. Not referenced. |
| `js/api/store.js.tmp` | tracked | REMOVE | Older version (v1) of store.js (v3). Not referenced. |
| `css/style.css` | tracked | REMOVE | Legacy stylesheet. Zero runtime references. No tooling dependencies. Unchanged since initial commit. Superseded by theme.css, app-shell.css, components.css. |

---

## 4. Decisions

All five currently identified artifacts are confirmed as removable:

- No runtime references (HTML / CSS / JS).
- No tooling references (no build system).
- No unique selectors or tokens still in use.
- Superseded by current stylesheets.

---

## 5. .gitignore

Add patterns to prevent recurrence:

    # Backups and temporary files
    *.bak
    *.bak-*
    *.tmp

---

## 6. Verification

After removal:

- `git status` contains no unexpected changes.
- The five intended removals and `.gitignore` change are the only expected changes.
- `grep` for removed filenames returns no results.
- Site functionality is unchanged because no active references existed.
