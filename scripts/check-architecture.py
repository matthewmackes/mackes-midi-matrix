#!/usr/bin/env python3
"""Enforce the workspace boundaries documented in docs/architecture.md."""

from __future__ import annotations

import re
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent

MAX_LINES = {
    # Novation XL protocol, LED batch encoding, and first-class controller
    # capability descriptors remain in the profile boundary pending extraction.
    "crates/profiles/src/lib.rs": 3200,
    "crates/midi-engine/src/lib.rs": 3100,
    "crates/tui/src/lib.rs": 4200,
    # The daemon's composition root retains a small amount of wiring while the
    # remaining service modules are extracted incrementally.  Keep this ceiling
    # explicit and reviewed rather than silently allowing unbounded growth.
    # PiPedal worker publication adds a small, reviewed composition-root seam.
    "apps/mackesd/src/lib.rs": 4000,
    "apps/mackes/src/main.rs": 840,
}

ALLOWED_LOCAL_DEPS = {
    # Configuration validation includes profile-owned hardware tuple checks;
    # this is an intentional, read-only contract dependency (not transport I/O).
    "mackes-config": {"mackes-domain", "mackes-profiles"},
    "mackes-domain": set(),
    "mackes-ipc": {"mackes-config", "mackes-domain"},
    "mackes-midi-engine": {"mackes-domain"},
    "mackes-profiles": {"mackes-domain"},
    "mackes-scene-engine": {"mackes-domain"},
    "mackes-tui": {"mackes-config", "mackes-domain", "mackes-ipc", "mackes-midi-engine", "mackes-profiles"},
    "mackes-testkit": {"mackes-config", "mackes-domain", "mackes-ipc", "mackes-midi-engine", "mackes-profiles", "mackes-scene-engine", "mackes-tui"},
    "mackes-midi-matrix": {"mackes-config", "mackes-domain", "mackes-ipc", "mackes-midi-engine", "mackes-profiles", "mackes-scene-engine", "mackes-tui"},
    "mackesd": {"mackes-config", "mackes-domain", "mackes-ipc", "mackes-midi-engine", "mackes-pipedal-adapter", "mackes-profiles", "mackes-scene-engine"},
    "mackes-pipedal-connector": set(),
    # The daemon-boundary adapter may depend on the transport-independent
    # connector; the daemon itself remains insulated from both packages.
    "mackes-pipedal-adapter": {"mackes-ipc", "mackes-pipedal-connector"},
}


def package_name(manifest: Path) -> str:
    match = re.search(r'^name\s*=\s*"([^"]+)"', manifest.read_text(), re.MULTILINE)
    if match is None:
        raise ValueError(f"missing package name: {manifest.relative_to(ROOT)}")
    return match.group(1)


def local_dependencies(manifest: Path) -> set[str]:
    return set(re.findall(r'^(mackes-[\w-]+)\s*=\s*\{\s*path\s*=', manifest.read_text(), re.MULTILINE))


def main() -> int:
    failures: list[str] = []
    if not (ROOT / "docs/architecture.md").is_file():
        failures.append("missing docs/architecture.md")
    for relative, maximum in MAX_LINES.items():
        source = ROOT / relative
        if not source.is_file():
            failures.append(f"missing canonical source: {relative}")
            continue
        line_count = len(source.read_text().splitlines())
        if line_count > maximum:
            failures.append(f"{relative} has {line_count} lines; ceiling is {maximum}")
    manifests = sorted((ROOT / "crates").glob("*/Cargo.toml")) + sorted((ROOT / "apps").glob("*/Cargo.toml"))
    for manifest in manifests:
        name = package_name(manifest)
        allowed = ALLOWED_LOCAL_DEPS.get(name)
        if allowed is None:
            failures.append(f"unmapped workspace package: {name}")
            continue
        unexpected = sorted(local_dependencies(manifest) - allowed)
        if unexpected:
            failures.append(f"{name} has forbidden local dependencies: {', '.join(unexpected)}")
    if failures:
        print("architecture policy failed:", *failures, sep="\n- ", file=sys.stderr)
        return 1
    print("architecture policy passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
