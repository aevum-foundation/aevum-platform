/**
 * Aevum Navigation Runtime
 *
 * Canonical navigation runtime for all pages.
 * Extracted from docs.html during Phase 1.3.
 *
 * Contract:
 *   initNav()    — idempotent; waits for header
 *   openNav()    — open drawer (programmatic)
 *   closeNav()   — close drawer (programmatic)
 *   isNavOpen()  — state
 *
 * Guarantees:
 *   - initNav() is idempotent.
 *   - initNav() waits for aevum:header-loaded.
 *   - No auto-start on import.
 *   - initPromise is reset on failure.
 *
 * TODO:
 *   - destroyNav() for SPA / hot reload use.
 *
 * Does not:
 *   - Load header or footer (js/components.js).
 */

const LOG_PREFIX = "[Aevum Navigation]";

let initPromise = null;

const navState = {
    burger: null,
    overlay: null,
    sideNav: null,
    closeButton: null,
    previousFocus: null,
    previousOverflow: ""
};

function waitForHeader() {
    return new Promise((resolve) => {
        if (document.getElementById("header-container")) {
            resolve();
            return;
        }

        document.addEventListener(
            "aevum:header-loaded",
            resolve,
            { once: true }
        );
    });
}

function getFocusableElements() {
    const { sideNav } = navState;

    if (!sideNav) {
        return [];
    }

    return [
        ...sideNav.querySelectorAll(
            [
                'a[href]',
                'button:not([disabled])',
                'input:not([disabled])',
                'select:not([disabled])',
                'textarea:not([disabled])',
                '[tabindex]:not([tabindex="-1"])'
            ].join(",")
        )
    ].filter((element) => {
        return (
            element.offsetParent !== null &&
            !element.hasAttribute("aria-hidden")
        );
    });
}

export function openNav() {
    const { burger, overlay, sideNav, closeButton } = navState;

    if (!burger || !overlay || !sideNav || !closeButton) {
        return false;
    }

    if (sideNav.classList.contains("active")) {
        return true;
    }

    navState.previousFocus = document.activeElement;
    navState.previousOverflow = document.body.style.overflow;

    sideNav.classList.add("active");
    overlay.classList.add("active");
    burger.classList.add("active");

    burger.setAttribute("aria-expanded", "true");
    sideNav.setAttribute("aria-hidden", "false");

    document.body.style.overflow = "hidden";

    window.requestAnimationFrame(() => {
        closeButton.focus();
    });

    return true;
}

export function closeNav(restoreFocus = true) {
    const { burger, overlay, sideNav } = navState;

    if (!burger || !overlay || !sideNav) {
        return false;
    }

    if (!sideNav.classList.contains("active")) {
        return true;
    }

    sideNav.classList.remove("active");
    overlay.classList.remove("active");
    burger.classList.remove("active");

    burger.setAttribute("aria-expanded", "false");
    sideNav.setAttribute("aria-hidden", "true");

    document.body.style.overflow = navState.previousOverflow;

    if (
        restoreFocus &&
        navState.previousFocus &&
        typeof navState.previousFocus.focus === "function"
    ) {
        const target = navState.previousFocus;

        window.requestAnimationFrame(() => {
            target.focus();
        });
    }

    navState.previousFocus = null;

    return true;
}

export function isNavOpen() {
    const { sideNav } = navState;

    if (!sideNav) {
        return false;
    }

    return sideNav.classList.contains("active");
}

async function internalInit() {
    await waitForHeader();

    const burger = document.getElementById("burger");
    const overlay = document.getElementById("overlay");
    const sideNav = document.getElementById("sideNav");
    const closeButton = document.getElementById("closeNav");

    if (!burger || !overlay || !sideNav || !closeButton) {
        console.warn(
            `${LOG_PREFIX} required elements not found`
        );
        return false;
    }

    navState.burger = burger;
    navState.overlay = overlay;
    navState.sideNav = sideNav;
    navState.closeButton = closeButton;

    burger.addEventListener("click", () => {
        if (isNavOpen()) {
            closeNav();
        } else {
            openNav();
        }
    });

    overlay.addEventListener("click", () => {
        closeNav();
    });

    closeButton.addEventListener("click", () => {
        closeNav();
    });

    sideNav.addEventListener("click", (event) => {
        const link = event.target.closest("a");

        if (!link) {
            return;
        }

        closeNav(false);
    });

    // registered exactly once (guarded by initPromise)
    document.addEventListener("keydown", (event) => {
        if (!isNavOpen()) {
            return;
        }

        if (event.key === "Escape") {
            event.preventDefault();
            closeNav();
            return;
        }

        if (event.key !== "Tab") {
            return;
        }

        const focusable = getFocusableElements();

        if (!focusable.length) {
            event.preventDefault();
            closeButton.focus();
            return;
        }

        const first = focusable[0];
        const last = focusable[focusable.length - 1];

        if (
            event.shiftKey &&
            document.activeElement === first
        ) {
            event.preventDefault();
            last.focus();

        } else if (
            !event.shiftKey &&
            document.activeElement === last
        ) {
            event.preventDefault();
            first.focus();
        }
    });

    sideNav.setAttribute("aria-hidden", "true");
    burger.setAttribute("aria-expanded", "false");

    return true;
}

export function initNav() {
    if (initPromise) {
        return initPromise;
    }

    initPromise = internalInit().catch((error) => {
        initPromise = null;
        throw error;
    });

    return initPromise;
}
