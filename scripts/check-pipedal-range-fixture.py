#!/usr/bin/env python3
"""Validate the source-derived PiPedal native-range join fixture."""

import json
from pathlib import Path


def main() -> int:
    path = Path(__file__).parents[1] / "docs/fixtures/pipedal-native-range-example.json"
    data = json.loads(path.read_text(encoding="utf-8"))
    controls = data["catalog"]["controls"]
    mappings = data["mapping_resolution"]
    assert len(controls) == 1 and len(mappings) == 1
    control = controls[0]
    mapping = mappings[0]
    assert mapping["plugin_uri"] == control["plugin_uri"]
    assert mapping["symbol"] == control["symbol"]
    expected = data["expected"]
    assert control["min_value"] == expected["input_min"]
    assert control["max_value"] == expected["input_max"]
    assert expected["input_min"] <= expected["accepted_value"] <= expected["input_max"]
    assert not expected["input_min"] <= expected["rejected_value"] <= expected["input_max"]
    print("PiPedal native-range fixture passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
