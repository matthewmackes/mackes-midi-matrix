#!/usr/bin/env python3
"""Verify the live PiPedal read-only catalog projection is populated and bounded."""

from __future__ import annotations

import json
import sys
import urllib.request


origin = sys.argv[1] if len(sys.argv) > 1 else "http://172.20.222.222:8081"
with urllib.request.urlopen(f"{origin}/api/v1/pipedal", timeout=20) as response:
    body = json.load(response)
if body.get("ok") is not True:
    raise RuntimeError(f"PiPedal projection is not authoritative: {body.get('ok')!r}")
catalog = body.get("catalog") or {}
controls = catalog.get("controls") if isinstance(catalog, dict) else None
targets = catalog.get("targets") if isinstance(catalog, dict) else None
operations = body.get("supported_operations")
version = body.get("version")
if not isinstance(controls, list) or not controls:
    raise RuntimeError("PiPedal catalog has no controls")
if not isinstance(targets, list) or not targets:
    raise RuntimeError("PiPedal catalog has no targets")
if not isinstance(operations, list) or len(operations) < 18:
    raise RuntimeError(f"PiPedal operation family projection is incomplete: {operations!r}")
if not isinstance(version, dict) or not isinstance(version.get("serverVersion"), str):
    raise RuntimeError("PiPedal version readback is missing or malformed")
print(
    f"pipedal-catalog: PASS origin={origin} generation={body.get('generation')} "
    f"controls={len(controls)} targets={len(targets)} operation_families={len(operations)} "
    f"server_version={version['serverVersion']}"
)
