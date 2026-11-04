/**
 * Page entry point (module).
 *
 * Replaces the former inline <script type="module"> so the site can run
 * under a strict Content-Security-Policy without 'unsafe-inline'.
 * Behaviour is identical to the inline script it replaces.
 */

import { boot } from '/js/components.js';
import { initNav } from '/js/navigation.js';
import { initSupportPage } from '/js/pages/support.js';

await boot();
await initNav();
initSupportPage();
