#!/usr/bin/env python3
"""Fail if active Studio sources reintroduce Carbon dependencies or requirements."""
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[1]
active_files = [
    ROOT / "apps/mackes-web/static/studio.html",
    ROOT / "apps/mackes-web/static/studio.css",
    ROOT / "apps/mackes-web/static/studio.js",
    ROOT / "apps/mackes-web/static/studio_controller.js",
    ROOT / "apps/mackes-web/static/studio_catalog.js",
    ROOT / "apps/mackes-web/static/studio_assignment.js",
    ROOT / "apps/mackes-web/static/studio_views.js",
    ROOT / "apps/mackes-web/static/device_renderer.js",
]
for path in active_files:
    text = path.read_text(encoding="utf-8")
    if re.search(r"carbon", text, re.IGNORECASE):
        raise SystemExit(f"active Studio source mentions Carbon: {path.relative_to(ROOT)}")
for path in (ROOT / "apps/mackes-web/package.json", ROOT / "apps/mackes-web/Cargo.toml"):
    if path.is_file() and re.search(r"carbon", path.read_text(encoding="utf-8"), re.IGNORECASE):
        raise SystemExit(f"active web manifest mentions Carbon: {path.relative_to(ROOT)}")
print("active Carbon guard passed")
