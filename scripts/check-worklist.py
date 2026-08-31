#!/usr/bin/env python3
"""Validate the mechanical invariants of the governed worklist."""

from __future__ import annotations

import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
TEXT = (ROOT / "WORKLIST.md").read_text(encoding="utf-8")
items = list(re.finditer(r"^#### \[[^\]]\] W(\d+) .*?(?=^#### \[[^\]]\] W|\Z)", TEXT, re.M | re.S))
if not items:
    raise SystemExit("no work items found")

ids = [int(match.group(1)) for match in items]
if len(ids) != len(set(ids)):
    raise SystemExit("duplicate work-item ID")

valid_statuses = {"NOT_STARTED", "READY", "IN_PROGRESS", "BLOCKED", "IN_REVIEW", "DONE", "DEFERRED"}
for match, item_id in zip(items, ids):
    body = match.group(0)
    status_match = re.search(r"^[-*] \*\*Status:\*\* `([^`]+)`", body, re.M)
    if not status_match or status_match.group(1) not in valid_statuses:
        raise SystemExit(f"W{item_id:03d}: missing or invalid status")
    owners = re.findall(r"^[-*] \*\*Owner:\*\* (.+)$", body, re.M)
    if len(owners) > 1:
        raise SystemExit(f"W{item_id:03d}: duplicate owner")
    if status_match.group(1) == "DONE" and not re.search(r"\*\*[^*\n]*evidence", body, re.I):
        raise SystemExit(f"W{item_id:03d}: DONE item lacks evidence")
    dependency_line = re.search(r"^[-*] \*\*Depends on:\*\* (.+)$", body, re.M)
    if dependency_line:
        for dependency in re.findall(r"W(\d+)", dependency_line.group(1)):
            if int(dependency) not in ids:
                raise SystemExit(f"W{item_id:03d}: unknown dependency W{int(dependency):03d}")

implementation = TEXT[TEXT.index("## 3. Implementation work items"):]
for forbidden in ("TLS/PSK frame/session suite", "MACKES TLS peer", "mode `0666`"):
    if forbidden in implementation:
        raise SystemExit(f"superseded implementation text remains: {forbidden}")

print(f"worklist checks passed ({len(ids)} items)")
