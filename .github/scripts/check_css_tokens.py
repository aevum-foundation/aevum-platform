#!/usr/bin/env python3
"""Fail if any CSS custom property is used but never defined.

Scans frontend/**/*.css, *.html and *.js. A `var(--name)` without a
fallback must have a matching `--name:` definition somewhere in the
frontend. Undefined tokens silently resolve to nothing: invisible
buttons, missing borders, unreadable text.
"""
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2] / "frontend"
DEF = re.compile(r"(--[A-Za-z0-9_-]+)\s*:")
USE = re.compile(r"var\(\s*(--[A-Za-z0-9_-]+)\s*(,)?")
# JS may define tokens at runtime: style.setProperty('--name', ...)
JS_DEF = re.compile(r"setProperty\(\s*['\"](--[A-Za-z0-9_-]+)['\"]")

files = [p for ext in ("css", "html", "js") for p in ROOT.rglob(f"*.{ext}")]
defined, uses = set(), []
for p in files:
    text = p.read_text(encoding="utf-8", errors="replace")
    defined.update(DEF.findall(text))
    defined.update(JS_DEF.findall(text))
    for n, line in enumerate(text.splitlines(), 1):
        for name, has_fallback in USE.findall(line):
            if not has_fallback:
                uses.append((p.relative_to(ROOT.parent), n, name))

missing = [(f, n, name) for f, n, name in uses if name not in defined]
for f, n, name in missing:
    print(f"{f}:{n}: undefined CSS custom property {name}")
names = sorted({m[2] for m in missing})
if missing:
    print(f"\n{len(names)} undefined token(s), {len(missing)} use(s): {', '.join(names)}")
    sys.exit(1)
print(f"OK: {len(uses)} var() uses, all defined ({len(defined)} tokens)")
