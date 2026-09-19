/**
 * Aevum Docs Page Module
 *
 * Page-specific logic for docs.html.
 *
 * Contract:
 *   initDocsPage() — idempotent; safe to call multiple times.
 *
 * Responsibilities:
 *   - Hash navigation for the docs sidebar.
 *   - IntersectionObserver for active-link tracking.
 *   - Scroll offset for the fixed header.
 *
 * Does not:
 *   - Load header or footer (js/components.js).
 *   - Handle mobile drawer (js/navigation.js).
 */

const LOG_PREFIX = "[Aevum Docs]";

let initialized = false;

function getHeaderOffset() {
    const rootStyle = getComputedStyle(document.documentElement);
    const raw = rootStyle.getPropertyValue("--header-height");
    const value = parseInt(raw, 10);

    return Number.isFinite(value) ? value : 72;
}

function scrollToHash(hash, behavior = "auto") {
    if (!hash || !hash.startsWith("#")) {
        return;
    }

    const target = document.getElementById(hash.slice(1));

    if (!target) {
        return;
    }

    const reduceMotion = window.matchMedia(
        "(prefers-reduced-motion: reduce)"
    ).matches;

    const finalBehavior = reduceMotion ? "auto" : behavior;

    window.setTimeout(() => {
        target.scrollIntoView({
            behavior: finalBehavior,
            block: "start"
        });
    }, 0);
}

export function initDocsPage() {
    if (initialized) {
        return;
    }

    const navigation = document.getElementById("docs-navigation");

    if (!navigation) {
        return;
    }

    const links = [...navigation.querySelectorAll("a[href^='#']")];

    if (!links.length) {
        return;
    }

    const linkById = new Map();

    links.forEach((link) => {
        const href = link.getAttribute("href");

        if (href) {
            linkById.set(href, link);
        }
    });

    const targets = links
        .map((link) => {
            const href = link.getAttribute("href");

            if (!href) {
                return null;
            }

            return document.getElementById(href.slice(1));
        })
        .filter(Boolean);

    if (!targets.length) {
        return;
    }

    initialized = true;

    let activeId =
        window.location.hash && linkById.has(window.location.hash)
            ? window.location.hash
            : "#intro";

    function setActive(id) {
        if (!linkById.has(id)) {
            return;
        }

        activeId = id;

        links.forEach((link) => {
            const isActive = link.getAttribute("href") === id;

            link.classList.toggle("active", isActive);

            if (isActive) {
                link.setAttribute("aria-current", "true");
            } else {
                link.removeAttribute("aria-current");
            }
        });
    }

    setActive(activeId);

    const headerOffset = getHeaderOffset();

    const observer = new IntersectionObserver(
        (entries) => {
            const visible = entries
                .filter((entry) => {
                    return (
                        entry.isIntersecting &&
                        linkById.has(`#${entry.target.id}`)
                    );
                })
                .sort(
                    (a, b) =>
                        a.boundingClientRect.top -
                        b.boundingClientRect.top
                );

            if (!visible.length) {
                return;
            }

            const id = `#${visible[0].target.id}`;

            if (id !== activeId) {
                setActive(id);
            }
        },
        {
            root: null,
            rootMargin: `-${Math.max(
                90,
                headerOffset + 28
            )}px 0px -55% 0px`,
            threshold: [0, 0.1, 0.25, 0.5]
        }
    );

    targets.forEach((target) => {
        observer.observe(target);
    });

    links.forEach((link) => {
        link.addEventListener("click", (event) => {
            const href = link.getAttribute("href");

            if (!href || !href.startsWith("#")) {
                return;
            }

            const target = document.getElementById(href.slice(1));

            if (!target) {
                return;
            }

            setActive(href);

            window.setTimeout(() => {
                scrollToHash(href, "smooth");
            }, 0);
        });
    });

    window.addEventListener("hashchange", () => {
        const hash = window.location.hash;

        if (linkById.has(hash)) {
            setActive(hash);
            scrollToHash(hash, "smooth");
        }
    });

    window.addEventListener("pageshow", () => {
        const hash = window.location.hash;

        if (linkById.has(hash)) {
            setActive(hash);
            scrollToHash(hash, "auto");
        }
    });

    if (linkById.has(window.location.hash)) {
        scrollToHash(window.location.hash, "auto");
    }
}
