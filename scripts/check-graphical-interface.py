#!/usr/bin/env python3
"""Guard the normal web GUI against code/protocol-oriented editor surfaces."""

from __future__ import annotations

import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
HTML = (ROOT / "apps/mackes-web/static/index.html").read_text(encoding="utf-8")
APP = (ROOT / "apps/mackes-web/static/app.js").read_text(encoding="utf-8")
LEDGER = ROOT / "docs/graphical-device-renderer-ledger.md"

patterns = {
    "raw JSON5 editor": r"id=[\"']configuration-json5-draft[\"']|Raw JSON5 draft",
    "route JSON editor": r"id=[\"']routes-json[\"']|Routes JSON",
    "scene action JSON editor": r"id=[\"']scene-actions-json[\"']|Scene actions JSON",
    "raw SysEx byte editor": r"id=[\"']sysex-bytes[\"']|SysEx data bytes",
    "state dump surface": r"<pre[^>]+id=[\"'](?:state|assignment-catalog|faceplate)[\"']",
}

findings: list[str] = []
for label, pattern in patterns.items():
    for source_name, source in (("index.html", HTML), ("app.js", APP)):
        if re.search(pattern, source, re.IGNORECASE):
            findings.append(f"{label}: {source_name}")

required_renderer_keys = {
    "novation.launch-control-xl",
    "eventide.micropitch",
    "lexicon.reflex",
    "pipedal",
    "m-audio.midisport-4x4",
    "rtp-midi",
    "generic-midi",
    "mackes.virtual-monitor",
    "generic.endpoint",
}
ledger = LEDGER.read_text(encoding="utf-8")
missing = sorted(key for key in required_renderer_keys if f"`{key}`" not in ledger)
if missing:
    findings.append("renderer ledger missing: " + ", ".join(missing))

if findings:
    print("graphical interface guard: findings remain")
    print("- " + "\n- ".join(findings))
    print("These findings are expected until W168–W175 remove the normal GUI violations.")
    sys.exit(1)

print("graphical interface guard passed")
