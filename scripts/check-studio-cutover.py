#!/usr/bin/env python3
"""Check the installed Studio cutover and rollback artifact without mutating the service."""
from __future__ import annotations
import hashlib
import subprocess
import sys
import urllib.request
from pathlib import Path

origin = sys.argv[1] if len(sys.argv) > 1 else "http://172.20.222.222:8081"
rollback = Path("/var/lib/mackes-midi-matrix/mackes-web.rollback")
if subprocess.run(["systemctl", "is-active", "--quiet", "mackes-web.service"], check=False).returncode != 0:
    raise RuntimeError("mackes-web.service is not active")
if not rollback.is_file():
    raise RuntimeError(f"rollback artifact missing: {rollback}")
digest = hashlib.sha256(rollback.read_bytes()).hexdigest()
if digest != "14074731612a191606e0bc220dc8a45ae1b6dde244da261632d63905fc942908":
    raise RuntimeError(f"rollback artifact checksum mismatch: {digest}")
for path in ("/", "/studio"):
    with urllib.request.urlopen(f"{origin}{path}", timeout=5) as response:
        body = response.read().decode("utf-8")
    if response.status != 200 or "MACKES Studio" not in body or "studio-controller" not in body:
        raise RuntimeError(f"Studio cutover response invalid at {path}")
print(f"studio-cutover: PASS root_and_studio rollback_sha256={digest}")
