(function() {
    const savedTheme = localStorage.getItem('aevum-theme') || 'dark';
    document.documentElement.setAttribute('data-theme', savedTheme);
})();

function toggleTheme() {
    const html = document.documentElement;
    const current = html.getAttribute('data-theme');
    const next = current === 'dark' ? 'light' : 'dark';
    html.setAttribute('data-theme', next);
    localStorage.setItem('aevum-theme', next);

    // SVG-safe: only update aria-pressed, do NOT touch textContent.
    document.querySelectorAll('.theme-toggle').forEach(function (btn) {
        btn.setAttribute('aria-pressed', next === 'light' ? 'true' : 'false');
    });
}

// Delegated handler instead of inline onclick="toggleTheme()" in the header
// component (inline handlers are blocked by a strict Content-Security-Policy).
// Works for buttons injected later by js/components.js.
document.addEventListener('click', function (event) {
    if (event.target.closest && event.target.closest('.theme-toggle')) {
        toggleTheme();
    }
});
