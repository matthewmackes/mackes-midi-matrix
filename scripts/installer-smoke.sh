#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 0 ]]; then
  printf 'usage: %s\n' "$0" >&2
  exit 64
fi

scripts/install-fedora.sh --check >/dev/null
if scripts/install-fedora.sh --invalid-option >/dev/null 2>&1; then
  printf 'installer smoke: invalid option unexpectedly succeeded\n' >&2
  exit 1
fi
printf 'installer-smoke: PASS (preflight and argument validation; no mutation)\n'
