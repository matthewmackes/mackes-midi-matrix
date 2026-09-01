#!/usr/bin/env python3
"""Enforce that the operator app cannot open or enumerate physical MIDI."""
from __future__ import annotations
import pathlib
import re

ROOT = pathlib.Path(__file__).resolve().parents[1]
OPERATOR = ROOT / "apps" / "mackes" / "src"
FORBIDDEN = (re.compile(r"\benumerate_midir_ports\b"), re.compile(r"\bMidir(?:Input|Output)Adapter\b"), re.compile(r"\bMidi(?:Input|Output)\b"), re.compile(r"/dev/snd"))
violations = []
for path in sorted(OPERATOR.rglob("*.rs")):
    for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        for pattern in FORBIDDEN:
            if pattern.search(line):
                violations.append(f"{path}:{line_number}: {pattern.pattern}")
if violations:
    raise SystemExit("operator physical MIDI ownership violation:\n" + "\n".join(violations))
print("MIDI ownership policy passed")
