#!/usr/bin/env python3
"""Static resilience/accessibility guard for the local MOD-inspired art layer."""
from pathlib import Path
import json
import re

ROOT = Path(__file__).resolve().parents[1]
static = ROOT / "apps/mackes-web/static"
manifest_path = static / "vendor/mod-art/manifest.json"
manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
assert manifest["runtimePolicy"]["remoteFetch"] is False
assert manifest["runtimePolicy"]["sameOriginOnly"] is True
assert {asset["role"] for asset in manifest["assets"]} >= {"missing-art-state", "broken-plugin-state", "blocked-state"}
assert len(manifest["assets"]) <= 64
assert sum((static / "vendor/mod-art" / asset["file"]).stat().st_size for asset in manifest["assets"]) <= 512 * 1024

for name in ("mod_art.js", "mod_art_gallery.js", "studio_pipedal_canvas.js", "studio_library.js"):
    source = (static / name).read_text(encoding="utf-8")
    assert not re.search(r"https?://", source), f"remote URL in {name}"
    assert "Unavailable" in source or "unavailable" in source, f"missing fallback state in {name}"

css = (static / "studio.css").read_text(encoding="utf-8")
assert "min-height: 44px" in css and "min-width: 44px" in css
assert "prefers-reduced-motion" in css
assert ".pipedal-topology-list" in css
print("mod-art resilience: PASS (offline, fallback, size, target, motion guards)")
