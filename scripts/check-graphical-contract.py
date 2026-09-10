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
fixture = json.loads((ROOT / "tests/fixtures/graphical-capability-v1.json").read_text(encoding="utf-8"))
if fixture.get("schema_version") != 1 or fixture.get("fallback_renderer") != "generic.endpoint":
    raise SystemExit("graphical capability golden fixture has invalid version or fallback")
fixture_keys = {item.get("key") for item in fixture.get("renderers", [])}
if fixture_keys != required_keys:
    raise SystemExit("graphical capability golden fixture renderer inventory drift")
if not any(item.get("renderer_key") == "generic.endpoint" for item in fixture.get("devices", [])):
    raise SystemExit("graphical capability golden fixture lacks unknown fallback device")
print("graphical capability contract checks passed")
