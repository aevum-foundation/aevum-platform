# Aevum Web Layout Contract v1

Status: frozen
Last updated: 2026-09-16

This document is the single source of truth for the layout
architecture of the Aevum web platform. All pages MUST follow it.

---

## 1. Global Foundation

Three CSS files form the base layout system:

| File | Purpose |
|---|---|
| `theme.css` | Design tokens (Dark / Light) |
| `app-shell.css` | Header, drawer, footer, container |
| `components.css` | Reusable UI components (btn, card) |

Every page MUST include all three CSS files in this order:

```html
<link rel="stylesheet" href="/css/theme.css">
<link rel="stylesheet" href="/css/app-shell.css">
<link rel="stylesheet" href="/css/components.css">
2. Theme
Applied via data-theme attribute on <html>.

Allowed values: dark, light.

Default: dark.

Persisted in localStorage under key aevum-theme.

Theme toggle MUST use aria-pressed and MUST NOT overwrite
button textContent (SVG-safe).

No emojis in UI — SVG icons only.

Example toggle:

js
function toggleTheme() {
    const html = document.documentElement;
    const current = html.getAttribute('data-theme');
    const next = current === 'dark' ? 'light' : 'dark';
    html.setAttribute('data-theme', next);
    localStorage.setItem('aevum-theme', next);

    document.querySelectorAll('.theme-toggle').forEach(function (btn) {
        btn.setAttribute('aria-pressed', next === 'light' ? 'true' : 'false');
    });
}
3. Container
Canonical container values live in theme.css:

css
:root {
    --container-max: 1280px;
    --container-padding: 24px;
}
Rules:

.container in legacy style.css is NOT part of the active system.

New pages MUST use max-width: var(--container-max) for content width.

Horizontal centering MUST use margin: 0 auto (or margin: X auto Y).

Page wrappers MUST NOT hardcode pixel max-widths — always use the token.

4. Page Wrappers
Each page defines its own wrapper class:

.roadmap-page

.community-page

.learn-page

.repositories-page

etc.

Wrapper rules:

Desktop: padding: 56px 0 88px

Mobile (≤ 768px): padding: 48px 0 72px

Inner content: max-width: var(--container-max), centered via margin: X auto.

5. Components
Canonical components live in components.css:

.btn, .btn-primary, .btn-secondary, .btn-sm

.card, .card-gold

.status-card, .repository-card

New components MUST be added to components.css — never invented per page.

6. Header and Footer
Header is loaded via fetch('/components/header.html').

Footer is loaded via fetch('/components/footer.html').

Both containers MUST have aria-busy="true" initially and set to
"false" after load.

Burger menu logic (initNav) is defined per-page.

Standard loading pattern:

js
async function loadComponent(id, url) {
    const el = document.getElementById(id);
    if (!el) return false;
    const response = await fetch(url, { cache: 'default' });
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    el.innerHTML = await response.text();
    el.setAttribute('aria-busy', 'false');
    return true;
}
7. Rules for New Pages
Include theme.css, app-shell.css, components.css (in this order).

Include theme.js.

Use <div id="header-container"> and <div id="footer-container">.

Use a page-specific wrapper class with max-width: var(--container-max).

Do NOT create new layout systems.

Do NOT hardcode colors — use tokens from theme.css.

No emojis in UI — SVG only.

Use Cache-Control: no-store for identity/personal endpoints.

External links: target="_blank" rel="noopener noreferrer".

Internal links: no target, no rel.

8. Legacy
frontend/css/style.css is legacy and NOT loaded by active pages.

Do not use its rules as reference (including .container { max-width: 1100px }).

.bak-* files in css/ are technical noise and MUST NOT be included
in production.

9. Accessibility
Burger menu MUST trap focus while open.

Escape closes the burger menu.

aria-expanded on burger toggle.

aria-hidden on sideNav when closed.

All interactive elements MUST be keyboard-accessible.

10. Frozen Status
This contract is frozen as of 2026-09-16.

Any change to layout architecture MUST update this document in the same
commit as the change.
