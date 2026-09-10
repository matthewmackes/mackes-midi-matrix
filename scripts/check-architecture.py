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
    # The bounded virtual-controller injection API keeps benchmark fixtures on the shared
    # endpoint contract; its small addition is explicitly accounted for here.
    "crates/midi-engine/src/lib.rs": 3120,
    "crates/tui/src/lib.rs": 4200,
    # The daemon's composition root retains a small amount of wiring while the
    # remaining service modules are extracted incrementally.  Keep this ceiling
    # explicit and reviewed rather than silently allowing unbounded growth.
    # PiPedal worker publication and the bounded physical-control dispatch bridge add a
    # small, reviewed composition-root seam pending the next module extraction.
    # The daemon now includes durable operation-journal integration at the
    # command boundary; keep a bounded ceiling while allowing that cohesive
    # path to remain together pending the next modular split.
    # The daemon root retains the IPC composition boundary while the v2 layer
    # projection lives in mapping_layers_runtime; the reviewed root budget is
    # 4,200 lines for this release slice.
    # The daemon retains the local IPC composition boundary; the reviewed
    # scene/project/setlist lifecycle and persisted PiPedal repair slices bring
    # the reviewed root budget to 4,650 lines pending the next module extraction.
    # The persisted PiPedal undo journal remains a small composition-root boundary
    # while its storage mechanics live in persistence_projection.
    "apps/mackesd/src/lib.rs": 4650,
    # The CLI composition root retains the bounded PiPedal apply/undo/repair
    # command family pending extraction into a dedicated command module.
    "apps/mackes/src/main.rs": 880,
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
    # Shared, transport-neutral HTTP/API value contracts.
    "mackes-web-contract": set(),
    # Same-origin HTTP adapter; all authoritative state remains daemon-owned over IPC.
    "mackes-web": {"mackes-ipc", "mackes-web-contract"},
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
