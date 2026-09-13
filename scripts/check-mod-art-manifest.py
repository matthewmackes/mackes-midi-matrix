#!/usr/bin/env python3
"""Validate the vendored MOD static-art inventory without network access."""
import hashlib
import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
BASE = ROOT / "apps/mackes-web/static/vendor/mod-art"
manifest = json.loads((BASE / "manifest.json").read_text(encoding="utf-8"))
assets = manifest["assets"]
assert manifest["permissionBasis"] == "operator-confirmed-remote-repository-approval-2026-09-13"
assert manifest["runtimePolicy"] == {
    "sameOriginOnly": True,
    "remoteFetch": False,
    "upstreamRuntimeImported": False,
    "brandingClassesExcluded": True,
}
files = set()
for asset in assets:
    rel = asset["file"]
    assert rel not in files, f"duplicate manifest file: {rel}"
    files.add(rel)
    path = BASE / rel
    assert path.is_file(), f"missing asset: {rel}"
    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    assert digest == asset["sha256"], f"hash mismatch: {rel}"
    if "upstreamSha256" in asset:
        assert len(asset["upstreamSha256"]) == 64, f"invalid upstream hash: {rel}"
    if path.suffix.lower() == ".svg":
        text = path.read_text(encoding="utf-8")
        lowered = text.lower()
        for forbidden in ("<script", " onload=", " onclick=", "javascript:", "<iframe", "<foreignobject"):
            assert forbidden not in lowered, f"unsafe SVG content {forbidden!r}: {rel}"
        assert "href=\"http://" not in lowered and "href=\"https://" not in lowered, f"external SVG URL: {rel}"
        assert "xlink:href=\"http://" not in lowered and "xlink:href=\"https://" not in lowered, f"external SVG URL: {rel}"

vendored = {
    str(path.relative_to(BASE))
    for path in BASE.rglob("*")
    if path.is_file() and path.name not in {"manifest.json", "UPSTREAM-LICENSE.txt"}
}
assert vendored == files, f"manifest/file mismatch: extra={vendored - files}, missing={files - vendored}"
print(f"mod-art manifest: PASS ({len(assets)} assets, {len(vendored)} files)")
