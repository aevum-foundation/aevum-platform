#!/usr/bin/env python3
"""Fail on inline <script> blocks and inline event handlers.

The site is designed to run under a Content-Security-Policy without
'unsafe-inline' in script-src (deploy/apache/zz-aevum-hardening.conf).
Inline code would be blocked by the browser in production.

Allowed: <script src="..."> and <script type="application/ld+json">
(JSON-LD is data, not code, and is not subject to script-src).
"""
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2] / "frontend"
SCRIPT = re.compile(r"<script\b([^>]*)>(.*?)</script>", re.S | re.I)
HANDLER = re.compile(r"<[^>]+\s(on[a-z]+)\s*=\s*['\"]", re.I)
errors = []
for p in sorted(ROOT.rglob("*.html")):
    text = p.read_text(encoding="utf-8", errors="replace")
    rel = p.relative_to(ROOT.parent)
    for m in SCRIPT.finditer(text):
        attrs, body = m.group(1), m.group(2)
        if re.search(r"\bsrc\s*=", attrs, re.I):
            continue
        if re.search(r"type\s*=\s*['\"]application/ld\+json['\"]", attrs, re.I):
            continue
        if body.strip():
            line = text.count("\n", 0, m.start()) + 1
            errors.append(f"{rel}:{line}: inline <script> (move it to /js/entries/)")
    for m in HANDLER.finditer(text):
        line = text.count("\n", 0, m.start()) + 1
        errors.append(f"{rel}:{line}: inline event handler {m.group(1)}= (use addEventListener)")
print("\n".join(errors) if errors else "OK: no inline scripts or event handlers")
sys.exit(1 if errors else 0)
