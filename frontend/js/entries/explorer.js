/**
 * Page entry point (module).
 *
 * Replaces the former inline <script type="module"> so the site can run
 * under a strict Content-Security-Policy without 'unsafe-inline'.
 * Behaviour is identical to the inline script it replaces.
 */

import { boot } from '/js/components.js';
import { initNav } from '/js/navigation.js';
import { initExplorerPage } from '/js/pages/explorer.js';

await boot();
await initNav();
await initExplorerPage();
