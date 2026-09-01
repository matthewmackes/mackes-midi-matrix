#!/usr/bin/env python3
"""Negative readiness checks for the offline controller contract."""

from __future__ import annotations

import json
import pathlib
import subprocess
import sys
import tempfile

ROOT = pathlib.Path(__file__).resolve().parents[1]
VERIFIER = ROOT / "scripts" / "verify-artifacts.py"
SOURCE = ROOT / "docs" / "mackes-launch-control-xl-mk2-factory1-manifest.json"


def main() -> int:
    valid = json.loads(SOURCE.read_text(encoding="utf-8"))
    with tempfile.TemporaryDirectory(prefix="mackes-artifact-") as directory:
        directory = pathlib.Path(directory)
        positive = directory / "factory1.json"
        positive.write_text(json.dumps(valid), encoding="utf-8")
        assert subprocess.run([sys.executable, str(VERIFIER), "--manifest", str(positive)], cwd=ROOT).returncode == 0
        cases = {
            "malformed": "{",
            "stale": {**valid, "template_version": "0.0.0"},
            "wrong-model": {**valid, "target_model": "Other Controller"},
            "wrong-slot": {**valid, "template_slot": 2},
            "modified": {**valid, "assignable_inventory": {**valid["assignable_inventory"], "faders": {**valid["assignable_inventory"]["faders"], "numbers": [77]}}},
        }
        for name, value in cases.items():
            path = directory / f"{name}.json"
            path.write_text(value if isinstance(value, str) else json.dumps(value), encoding="utf-8")
            result = subprocess.run([sys.executable, str(VERIFIER), "--manifest", str(path)], cwd=ROOT)
            if result.returncode == 0:
                raise SystemExit(f"negative artifact case unexpectedly passed: {name}")
        missing = directory / "missing.json"
        result = subprocess.run([sys.executable, str(VERIFIER), "--manifest", str(missing)], cwd=ROOT)
        if result.returncode == 0:
            raise SystemExit("missing artifact case unexpectedly passed")
    print("artifact negative checks passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
