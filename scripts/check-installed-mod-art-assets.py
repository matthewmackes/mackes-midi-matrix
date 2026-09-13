#!/usr/bin/env python3
"""Verify every governed MOD asset is served byte-identically by the installed host."""
from __future__ import annotations
import hashlib
import json
import sys
from pathlib import Path
from urllib.request import urlopen

origin = sys.argv[1] if len(sys.argv) > 1 else "http://172.20.222.222:8081"
root = Path(__file__).resolve().parents[1]
manifest = json.loads((root / "apps/mackes-web/static/vendor/mod-art/manifest.json").read_text())
for asset in manifest["assets"]:
    local = root / "apps/mackes-web/static/vendor/mod-art" / asset["file"]
    local_digest = hashlib.sha256(local.read_bytes()).hexdigest()
    with urlopen(f"{origin}/assets/vendor/mod-art/{asset['file']}", timeout=5) as response:
        served = response.read()
        assert response.status == 200, f"HTTP {response.status}: {asset['file']}"
    served_digest = hashlib.sha256(served).hexdigest()
    assert served_digest == local_digest == asset["sha256"], f"hash mismatch: {asset['file']}"
print(f"installed MOD art parity: PASS ({len(manifest['assets'])} assets)")
