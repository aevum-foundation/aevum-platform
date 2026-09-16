# Aevum Learn — Design Contract

Status: **frozen**

This document describes the design rules that every Learn page must follow.
It exists so that Learn remains visually consistent with the rest of the
Aevum site as the number of pages grows.

Learn is not a separate site. It is part of `aevumchain.com`. It uses the
same design system, the same tokens, and the same rhythm as the existing
pages (`index.html`, `roadmap.html`, `docs.html`, `explorer.html`).

---

## 1. Golden Rule

> **Do not create a new component if an existing one fits.**

Before adding a new CSS class, ask:

- Can this be done with an existing class?
- Is this a new entity, or a variation of an existing entity?

New entities get new classes. Variations of existing entities do not.

**Good — new entity:**

- `.learn-level`
- `.learn-level-header`
- `.learn-level-topics`

**Bad — variation of an existing component:**

- `.learn-blue-card`
- `.learn-yellow-card`
- `.learn-big-card`

---

## 2. Typography

Aligned with the existing Aevum site.

| Element | Size |
|---------|------|
| H1 (hero) | `clamp(38px, 5vw, 58px)` |
| H2 (section / level header) | `clamp(26px, 3.5vw, 32px)` |
| H3 (card / subtopic) | `17px` – `19px` |
| Body | `16px` (secondary), `14px` (dense) |
| Small (labels, meta) | `11px` – `13px` |

Font family:

- Sans: `var(--font-family-sans)`
- Mono: `var(--font-family-mono)` (used for labels)

---

## 3. Spacing

| Element | Value |
|---------|-------|
| Page wrapper (`padding`) | `56px 0 88px` |
| Section margin-top | `72px` (desktop), `56px` (mobile) |
| Hero margin-bottom | `72px` |
| Grid gap | `20px` (cards), `24px` (dense), `40px` (two-column body) |

Section rhythm must match `roadmap.html`.

---

## 4. Colors

All colors come from `theme.css` tokens.

- **Gold** (`--color-accent-gold`) — brand, primary CTA
- **Cyan** (`--color-accent-cyan`) — technical, links, status, kicker
- **Red** (`--color-status-offline`) — warning, error, offline only
- **Green** — **not used** as a UI color

No hardcoded colors in any Learn CSS file.

---

## 5. Components

### Buttons

Use `.btn`, `.btn-primary`, `.btn-secondary`, `.btn-cyan`, `.btn-sm`, `.btn-lg`.

Do not create `.learn-btn` or similar. If a Learn-specific variant is
required, propose it explicitly.

### Cards

Learn-specific cards:

- `.learn-path-card` — path or step card
- `.learn-cta-card` — navigation CTA card
- `.learn-level` — curriculum or path level section

All cards use the same tokens:

- `background: var(--color-bg-surface)`
- `border: 1px solid var(--color-border)`
- `border-radius: var(--radius-md)`

### Labels

- `.learn-label` — tag (cyan pill, mono font)
- `.learn-path-label` — numeric identifier for a card (01, 02, ...)

### Section metadata

- `.learn-kicker` — section marker (uppercase, cyan dot)

### Layout wrappers

- `.learn-page` — page wrapper
- `.learn-hero` — page header
- `.learn-section` — content section
- `.learn-section-header` — section title block

---

## 6. Hover States

All clickable cards must have consistent hover behavior.

- `.learn-path-card` — border color + slight translateY
- `.learn-cta-card` — border color + slight translateY
- `.learn-label` (as link) — border color + background

There must not be:

- one card that lifts and another that does nothing
- one card that glows and another that only changes border color

---

## 7. Focus States

`focus-visible` is defined globally in `theme.css` (line ~282).

Every interactive element must:

- remain focusable
- show the focus ring
- preserve contrast in both Dark and Light themes

Special attention: card links, button links, anchor chips.

---

---

## 8. Dark / Light Themes

Every Learn page must be tested in:

- Dark theme (default)
- Light theme
- Mobile (≤ 768px)
- Desktop (> 960px)

That is four combinations per page. A page that looks correct only in one
combination is not complete.

---

## 9. Page Structure

Every Learn page follows the same structure:

```

<header-container>
    (loaded via loadComponent)

<main class="learn-page" id="main-content">
    <section class="learn-hero"> ... </section>
    <section class="learn-section"> ... </section>
    ...

<footer-container>
    (loaded via loadComponent)

<script>loadComponent(...) for header and footer</script>

```

No page uses a different loading mechanism. No page loads components
inline.

---

## 10. CSS Loading Order

Every Learn page loads exactly these stylesheets, in this order:

```html
<link rel="stylesheet" href="/css/theme.css">
<link rel="stylesheet" href="/css/app-shell.css">
<link rel="stylesheet" href="/css/components.css">
<link rel="stylesheet" href="/css/pages/learn.css">
```

No additional CSS files. No inline <style> blocks.

---

## 11. Content Rules

- No emoji in the user interface.
- Icons are SVG only, defined inline or via CSS pseudo-elements.
- No marketing language ("revolutionary", "next-generation").
- No promises of financial outcome.
- No progress bars, badges, certificates, or gamification elements.
- No tracking widgets or third-party embeds on Learn pages.

---

## 12. Checklist for Every New Learn Page

Before considering a Learn page complete:

- [ ] Uses the four stylesheets in the correct order
- [ ] Uses <header-container> and <footer-container> with loadComponent
- [ ] H1 matches clamp(38px, 5vw, 58px)
- [ ] H2 matches clamp(26px, 3.5vw, 32px)
- [ ] Page wrapper uses padding: 56px 0 88px
- [ ] All colors use var(--...) tokens
- [ ] No new component class unless it is a new entity
- [ ] Hover states consistent with other cards
- [ ] Focus states visible
- [ ] Tested in Dark + Light + Mobile + Desktop
- [ ] No emoji, no marketing language, no gamification

---

## 13. When to Update This Contract

This contract is updated when:

- A new entity is introduced (new section type, new card role).
- A design token is changed globally.

It is not updated for cosmetic variations within existing entities.

The goal is consistency, not rigidity. Learn and the rest of aevumchain.com
should look like one product built by one team.
