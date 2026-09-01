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
FACTORY1_MANIFEST = ROOT / "docs" / "mackes-launch-control-xl-mk2-factory1-manifest.json"

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


def check_factory1_manifest() -> None:
    """Validate the complete offline Factory Template 1 production contract."""
    try:
        manifest = json.loads(FACTORY1_MANIFEST.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise SystemExit(f"invalid Factory 1 manifest: {exc}") from exc
    if (
        manifest.get("target_model") != "Novation Launch Control XL"
        or manifest.get("target_generation") != "Mk2"
        or manifest.get("template_name") != "Factory Template 1"
        or manifest.get("template_slot") != 1
        or manifest.get("midi_channel") != 8
    ):
        raise SystemExit("Factory 1 manifest has the wrong target or template")
    inventory = manifest.get("assignable_inventory")
    if not isinstance(inventory, dict):
        raise SystemExit("Factory 1 manifest is missing assignable inventory")
    tuples: set[tuple[str, int]] = set()
    physical_ids: set[str] = set()
    expected_counts = {"knobs": 24, "channel_buttons": 16, "faders": 8}
    for name, expected in expected_counts.items():
        item = inventory.get(name)
        if not isinstance(item, dict) or item.get("count") != expected:
            raise SystemExit(f"Factory 1 manifest has an invalid {name} count")
        numbers = item.get("numbers")
        if not isinstance(numbers, list) or len(numbers) != expected:
            raise SystemExit(f"Factory 1 manifest has an invalid {name} MIDI inventory")
        expected_kind = "note" if name == "channel_buttons" else "cc"
        if item.get("midi_kind") != expected_kind:
            raise SystemExit(f"Factory 1 manifest has an invalid {name} message kind")
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
            raise SystemExit(f"Factory 1 manifest has an invalid {name} physical-ID pattern")
        if physical_ids.intersection(ids):
            raise SystemExit(f"duplicate Factory 1 physical control ID in {name}")
        physical_ids.update(ids)
        for number in numbers:
            key = (manifest["midi_channel"], item["midi_kind"], number)
            if key in tuples:
                raise SystemExit(f"duplicate Factory 1 source tuple: {key}")
            tuples.add(key)
    reserved = manifest.get("reserved_controls")
    if not isinstance(reserved, list) or len(reserved) != 8 or len(set(reserved)) != 8:
        raise SystemExit("Factory 1 manifest must contain eight unique reserved controls")


if __name__ == "__main__":
    check_schemas()
    check_fixtures()
    check_factory1_manifest()
    print("artifact checks passed")
