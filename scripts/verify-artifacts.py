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


if __name__ == "__main__":
    check_schemas()
    check_fixtures()
    print("artifact checks passed")
