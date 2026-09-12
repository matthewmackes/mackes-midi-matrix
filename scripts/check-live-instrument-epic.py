#!/usr/bin/env python3
"""Validate the governed plug-and-play/live-feedback epic contract."""

from __future__ import annotations

import pathlib
import re

ROOT = pathlib.Path(__file__).resolve().parents[1]
worklist = (ROOT / "WORKLIST.md").read_text(encoding="utf-8")
spec = (ROOT / "docs" / "plug-and-play-live-instrument-epic.md").read_text(encoding="utf-8")
spec_flat = re.sub(r"\s+", " ", spec)

required_spec = (
    "per-feature",
    "sent-unverified",
    "100 ms p95",
    "500 ms p95",
    "one reviewed Apply",
    "no Carbon",
    "Acceptance walkthroughs",
)
for phrase in required_spec:
    if phrase.lower() not in spec_flat.lower():
        raise SystemExit(f"epic specification missing required contract: {phrase}")

for item_id in range(186, 203):
    if not re.search(rf"^#### \[[^\]]\] W{item_id} \u2014", worklist, re.M):
        raise SystemExit(f"missing W{item_id}")

for item_id in range(180, 186):
    match = re.search(rf"^#### \[[^\]]\] W{item_id} .*?(?=^#### |\Z)", worklist, re.M | re.S)
    if not match or "**Status:** `DEFERRED`" not in match.group(0) or "Superseded" not in match.group(0):
        raise SystemExit(f"W{item_id} is not explicitly superseded")

epic_start = worklist.index("### Plug-and-Play Live Instrument Web Experience")
epic_end = worklist.index("## 4. Dependency and parallelization map", epic_start)
epic = worklist[epic_start:epic_end]
print("plug-and-play/live-instrument epic checks passed (W186-W202)")
