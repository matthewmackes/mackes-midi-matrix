#!/usr/bin/env python3
"""Validate the reusable PiPedal EQ mapping fixture."""
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else pathlib.Path("docs/fixtures/pipedal-eq-r3-example.json")
rows = json.loads(path.read_text())
if not isinstance(rows, list) or not 1 <= len(rows) <= 128:
    raise SystemExit("fixture must contain 1..128 mappings")
physical = set()
targets = set()
for row in rows:
    required = {"physical_control_id", "plugin_uri", "symbol", "scope"}
    if set(row) != required or not all(isinstance(row[k], str) and row[k] for k in ("physical_control_id", "plugin_uri", "symbol")):
        raise SystemExit("fixture mapping identity is invalid")
    if row["physical_control_id"] in physical:
        raise SystemExit("duplicate physical control")
    target = (row["plugin_uri"], row["symbol"], row["scope"])
    if target in targets:
        raise SystemExit("duplicate target")
    physical.add(row["physical_control_id"])
    targets.add(target)
print(f"PiPedal fixture valid: {len(rows)} mappings")
