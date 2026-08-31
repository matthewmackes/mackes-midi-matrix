#!/usr/bin/env python3
"""Reject malformed generated schemas and private/local fixture data."""

from __future__ import annotations

import json
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
SCHEMAS = ROOT / "schemas"
FIXTURES = ROOT / "fixtures"
USER1_MANIFEST = ROOT / "docs" / "mackes-launch-control-xl-mk2-user1-manifest.json"

PRIVATE_PATTERNS = (
    re.compile(r"/(?:home|root)/[^\s\"']+"),
    re.compile(r"(?i)(?:password|secret|private[_ -]?key|preshared[_ -]?key)\s*[:=]"),
    re.compile(r"(?i)usb[_ -]?serial\s*[:=]"),
)


def check_schemas() -> None:
    for path in sorted(SCHEMAS.rglob("*.json")):
        try:
            value = json.loads(path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as exc:
            raise SystemExit(f"invalid schema {path}: {exc}") from exc
        if not isinstance(value, dict) or "$schema" not in value:
            raise SystemExit(f"schema missing $schema: {path}")


def check_fixtures() -> None:
    if not FIXTURES.exists():
        return
    for path in sorted(FIXTURES.rglob("*")):
        if not path.is_file() or path.name == "README.md":
            continue
        text = path.read_text(encoding="utf-8", errors="replace")
        for pattern in PRIVATE_PATTERNS:
            if pattern.search(text):
                raise SystemExit(f"possible private data in fixture {path}: {pattern.pattern}")


def check_user1_manifest() -> None:
    """Validate the reviewable inventory without treating the pending artifact as verified."""
    try:
        manifest = json.loads(USER1_MANIFEST.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise SystemExit(f"invalid User 1 manifest: {exc}") from exc
    if manifest.get("target_generation") != "Mk2" or manifest.get("expected_user_slot") != 1:
        raise SystemExit("User 1 manifest has the wrong target or slot")
    inventory = manifest.get("assignable_inventory")
    if not isinstance(inventory, dict):
        raise SystemExit("User 1 manifest is missing assignable inventory")
    tuples: set[tuple[str, int]] = set()
    physical_ids: set[str] = set()
    expected_counts = {"knobs": 24, "channel_buttons": 16, "faders": 8}
    for name, expected in expected_counts.items():
        item = inventory.get(name)
        if not isinstance(item, dict) or item.get("count") != expected:
            raise SystemExit(f"User 1 manifest has an invalid {name} count")
        numbers = item.get("cc_numbers")
        if not isinstance(numbers, list) or len(numbers) != expected:
            raise SystemExit(f"User 1 manifest has an invalid {name} MIDI inventory")
        if name == "knobs":
            ids = [f"knob-r{row}-c{column}" for row in range(1, 4) for column in range(1, 9)]
        elif name == "channel_buttons":
            ids = [f"button-r{row}-c{column}" for row in range(1, 3) for column in range(1, 9)]
        else:
            ids = [f"fader-{column}" for column in range(1, 9)]
        expected_pattern = {
            "knobs": "knob-r{row}-c{column}",
            "channel_buttons": "button-r{row}-c{column}",
            "faders": "fader-{column}",
        }[name]
        if item.get("physical_id_pattern") != expected_pattern or len(ids) != expected:
            raise SystemExit(f"User 1 manifest has an invalid {name} physical-ID pattern")
        if physical_ids.intersection(ids):
            raise SystemExit(f"duplicate User 1 physical control ID in {name}")
        physical_ids.update(ids)
        for number in numbers:
            key = (item.get("midi_kind", ""), number)
            if key in tuples:
                raise SystemExit(f"duplicate User 1 source tuple: {key}")
            tuples.add(key)
    reserved = manifest.get("reserved_controls")
    if not isinstance(reserved, list) or len(reserved) != 8 or len(set(reserved)) != 8:
        raise SystemExit("User 1 manifest must contain eight unique reserved controls")
    if manifest.get("sha256") is None:
        print("User 1 Components artifact checksum pending review")
    elif not re.fullmatch(r"[0-9a-fA-F]{64}", manifest["sha256"]):
        raise SystemExit("User 1 artifact checksum is not a SHA-256 digest")


if __name__ == "__main__":
    check_schemas()
    check_fixtures()
    check_user1_manifest()
    print("artifact checks passed")
