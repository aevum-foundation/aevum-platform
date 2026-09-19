/**
 * Aevum Support Page Module
 *
 * Page-specific logic for support.html.
 *
 * Contract:
 *   initSupportPage() — idempotent; safe to call multiple times.
 *
 * Responsibilities:
 *   - Copy support address to clipboard.
 */

const LOG_PREFIX = "[Aevum Support]";
const STATUS_TIMEOUT_MS = 2500;

let statusTimer = null;

export function initSupportPage() {
    const address = document.getElementById("supportAddress");
    const button = document.getElementById("copyAddress");
    const status = document.getElementById("copyStatus");

    if (!address || !button || !status) {
        return;
    }

    if (button.dataset.initialized === "true") {
        return;
    }

    if (address.textContent.trim() === "PENDING_VERIFICATION") {
        return;
    }

    if (!navigator.clipboard?.writeText) {
        status.textContent =
            "Clipboard API is unavailable in this browser.";
        return;
    }

    button.dataset.initialized = "true";
    button.disabled = false;

    button.addEventListener("click", async () => {
        try {
            await navigator.clipboard.writeText(
                address.textContent.trim()
            );

            status.textContent = "Address copied.";

            if (statusTimer) {
                clearTimeout(statusTimer);
            }

            statusTimer = window.setTimeout(() => {
                status.textContent = "";
                statusTimer = null;
            }, STATUS_TIMEOUT_MS);

        } catch (error) {
            console.error(
                `${LOG_PREFIX} Clipboard failed`,
                error
            );

            status.textContent =
                "Copy failed. Please copy the address manually.";
        }
    });
}
