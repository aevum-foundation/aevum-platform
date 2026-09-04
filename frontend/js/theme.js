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
    document.querySelectorAll('.theme-toggle').forEach(btn => {
        btn.textContent = next === 'dark' ? '🌙' : '☀️';
    });
}
