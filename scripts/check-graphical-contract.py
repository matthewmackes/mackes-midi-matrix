#!/usr/bin/env python3
"""Validate the governed graphical capability contract shape and renderer inventory."""

from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
schema = json.loads((ROOT / "schemas/graphical-capability-v1.schema.json").read_text(encoding="utf-8"))
if schema.get("$id") != "https://mackes.invalid/schema/graphical-capability-v1.schema.json":
    raise SystemExit("graphical capability schema identity drift")
if schema.get("additionalProperties") is not False or schema.get("properties", {}).get("schema_version", {}).get("const") != 1:
    raise SystemExit("graphical capability schema must be closed and versioned")
required_keys = {
    "novation.launch-control-xl", "eventide.micropitch", "lexicon.reflex", "pipedal",
    "m-audio.midisport-4x4", "rtp-midi", "generic-midi", "mackes.virtual-monitor", "generic.endpoint",
}
ledger = (ROOT / "docs/graphical-device-renderer-ledger.md").read_text(encoding="utf-8")
missing = sorted(key for key in required_keys if f"`{key}`" not in ledger)
if missing:
    raise SystemExit("graphical renderer ledger missing: " + ", ".join(missing))
print("graphical capability contract checks passed")
