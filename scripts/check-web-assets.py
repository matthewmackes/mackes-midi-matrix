#!/usr/bin/env python3
"""Enforce the initial bundled web asset budget."""

from __future__ import annotations

import gzip
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ASSETS = (
    ROOT / "apps" / "mackes-web" / "static" / "index.html",
    ROOT / "apps" / "mackes-web" / "static" / "feature_catalog.js",
    ROOT / "apps" / "mackes-web" / "static" / "feature_renderer.js",
    ROOT / "apps" / "mackes-web" / "static" / "state_store.js",
    ROOT / "apps" / "mackes-web" / "static" / "app.js",
    ROOT / "apps" / "mackes-web" / "static" / "app.css",
)
MAX_COMPRESSED_BYTES = 500 * 1024
external_url = re.compile(rb"(?:https?:)?//")
asset_bytes = [path.read_bytes() for path in ASSETS]
html_ids = set(re.findall(rb'\bid="([A-Za-z][A-Za-z0-9_-]*)"', asset_bytes[0]))
selectors = set(re.findall(rb"querySelector\(['\"]#([A-Za-z][A-Za-z0-9_-]*)['\"]\)",
                           asset_bytes[ASSETS.index(ROOT / "apps" / "mackes-web" / "static" / "app.js")]))
missing_selectors = sorted(selectors - html_ids)
if missing_selectors:
    names = ", ".join(value.decode("ascii") for value in missing_selectors)
    raise SystemExit(f"web JavaScript selectors missing from HTML: {names}")
for path, content in zip(ASSETS, asset_bytes):
    if external_url.search(content):
        raise SystemExit(f"web asset has an external URL: {path}")
payload = b"".join(asset_bytes)
compressed = gzip.compress(payload, compresslevel=9, mtime=0)
if len(compressed) > MAX_COMPRESSED_BYTES:
    raise SystemExit(f"web initial asset budget exceeded: {len(compressed)} > {MAX_COMPRESSED_BYTES}")
print(f"web asset budget passed: {len(compressed)} compressed bytes")
