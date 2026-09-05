#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 0 ]]; then
  printf 'usage: %s\n' "$0" >&2
  exit 64
fi

scripts/install-fedora.sh --check >/dev/null
for required in packaging/10-appliance.conf packaging/mackes-midi-matrix-tui.service scripts/mackes-midi-matrix-local; do
  [[ -f "$required" ]] || { printf 'installer smoke: missing packaged dependency %s\n' "$required" >&2; exit 1; }
done
if scripts/install-fedora.sh --invalid-option >/dev/null 2>&1; then
  printf 'installer smoke: invalid option unexpectedly succeeded\n' >&2
  exit 1
fi
if MACKES_CONSOLE_USER='bad user' scripts/install-fedora.sh --check >/dev/null 2>&1; then
  printf 'installer smoke: invalid console user unexpectedly succeeded\n' >&2
  exit 1
fi
if MACKES_CONSOLE_HOME='relative/home' scripts/install-fedora.sh --check >/dev/null 2>&1; then
  printf 'installer smoke: relative console home unexpectedly succeeded\n' >&2
  exit 1
fi
printf 'installer-smoke: PASS (preflight and argument validation; no mutation)\n'
