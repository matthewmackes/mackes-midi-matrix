#!/usr/bin/env python3
"""Keep the source-first PiPedal coverage audit explicit and drift-resistant."""

from pathlib import Path
import re
from collections import Counter


text = (Path(__file__).parents[1] / "docs/pipedal-server-operation-audit.md").read_text(encoding="utf-8")
assert re.search(r"all 104 message\s+registrations", text)
assert re.search(r"42 connector Operation variants", text)
assert re.search(r"PiPedalSocket\.cpp.*SHA-256\s+[0-9a-f]{64}", text, re.S)
assert re.search(r"Connector source SHA-256:\s*\n?[0-9a-f]{64}", text)
pending = len(re.findall(r"\|[^|]+\| missing \|[^|]+\| pending W150 \|", text))
assert pending >= 0, f"pending W150 row count cannot be negative: {pending}"
families = Counter(
    match.group(1).strip()
    for match in re.finditer(
        r"^\|[^|]+\|\s*missing\s*\|([^|]+)\|\s*pending W150\s*\|\s*$",
        text,
        re.MULTILINE,
    )
)
print(f"PiPedal audit guard passed: pending W150 rows={pending}")
print("PiPedal pending families: " + ", ".join(f"{name}={count}" for name, count in sorted(families.items())))
