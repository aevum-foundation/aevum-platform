/**
 * Aevum Components Loader
 *
 * Canonical component loader for all pages.
 * Extracted from docs.html during Phase 1.2.
 *
 * Contract:
 *   EVENTS               — lifecycle event names
 *   fetchWithTimeout()   — fetch with timeout
 *   preloadComponent()   — preload into cache
 *   loadComponent()      — load into container
 *   boot()               — idempotent boot
 *
 * Guarantees:
 *   - boot() is idempotent.
 *   - boot() can be called multiple times.
 *   - header is loaded before footer.
 *   - components-ready is always last.
 *
 * Does not:
 *   - Handle navigation (js/navigation.js).
 *   - Handle page-specific behavior (js/pages/*.js).
 */

const LOG_PREFIX = "[Aevum Components]";
const COMPONENT_TIMEOUT_MS = 10000;
const componentCache = new Map();

let bootPromise = null;

export const EVENTS = {
    COMPONENT_LOADED: "aevum:component-loaded",
    HEADER_LOADED: "aevum:header-loaded",
    COMPONENTS_READY: "aevum:components-ready"
};

function emit(name, detail = {}) {
    document.dispatchEvent(
        new CustomEvent(name, { detail })
    );
}

export async function fetchWithTimeout(url) {
    const controller = new AbortController();

    const timeoutId = window.setTimeout(() => {
        controller.abort();
    }, COMPONENT_TIMEOUT_MS);

    try {
        const response = await fetch(url, {
            cache: "default",
            credentials: "same-origin",
            signal: controller.signal
        });

        if (!response.ok) {
            throw new Error(
                `Component request failed: ${response.status}`
            );
        }

        const contentType =
            response.headers.get("content-type") ?? "";

        if (!contentType.includes("text/html")) {
            throw new Error(
                `Component response was not HTML: ${url}`
            );
        }

        return await response.text();

    } finally {
        window.clearTimeout(timeoutId);
    }
}

export async function preloadComponent(url) {
    if (componentCache.has(url)) {
        return componentCache.get(url);
    }

    const html = await fetchWithTimeout(url);

    if (!html.trim()) {
        throw new Error(
            `Component response was empty: ${url}`
        );
    }

    componentCache.set(url, html);

    return html;
}

export async function loadComponent(id, url) {
    const element = document.getElementById(id);

    if (!element) {
        return false;
    }

    try {
        const html = await preloadComponent(url);

        element.innerHTML = html;
        element.setAttribute("aria-busy", "false");

        emit(EVENTS.COMPONENT_LOADED, { id, url });

        return true;

    } catch (error) {
        console.error(
            `${LOG_PREFIX} load failed:`,
            url,
            error
        );

        element.innerHTML = "";
        element.setAttribute("aria-busy", "false");

        return false;
    }
}

async function internalBoot() {
    const headerOk = await loadComponent(
        "header-container",
        "/components/header.html"
    );

    if (headerOk) {
        emit(EVENTS.HEADER_LOADED);
    }

    await loadComponent(
        "footer-container",
        "/components/footer.html"
    );

    emit(EVENTS.COMPONENTS_READY);
}

export function boot() {
    if (bootPromise) {
        return bootPromise;
    }

    bootPromise = internalBoot();

    return bootPromise;
}
