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
[[ -x scripts/capture-qualification-baseline.sh ]] || {
  printf 'installer smoke: qualification baseline capture is not executable\n' >&2
  exit 1
}
grep -q 'status-no-env.json' scripts/capture-qualification-baseline.sh
[[ -x scripts/capture-qualification-soak.sh ]] || {
  printf 'installer smoke: qualification soak capture is not executable\n' >&2
  exit 1
}
grep -q 'status_ok' scripts/capture-qualification-soak.sh
scripts/verify-systemd-units.sh >/dev/null
grep -q '^User=mackes$' packaging/mackes.service
grep -q '^Group=mackes-control$' packaging/mackes.service
grep -q '^DeviceAllow=/dev/snd/seq rw$' packaging/mackes.service
grep -q '^Requires=mackes-midi-matrix.service$' packaging/mackes-midi-matrix-tui.service
grep -q '^Environment=MACKES_CONFIG=' packaging/mackes-midi-matrix-tui.service
grep -q 'systemctl is-active --quiet mackes-midi-matrix.service' scripts/install-fedora.sh
grep -q 'systemctl is-active --quiet mackes-midi-matrix-tui.service' scripts/install-fedora.sh
grep -q 'systemctl is-enabled --quiet mackes-midi-matrix.service' scripts/install-fedora.sh
grep -q 'systemctl is-enabled --quiet mackes-midi-matrix-tui.service' scripts/install-fedora.sh
awk 'BEGIN { section=""; ok=0 } /^\[/{section=$0} /^StartLimitIntervalSec=60s$/ && section=="[Unit]" {ok++} /^StartLimitBurst=10$/ && section=="[Unit]" {ok++} END {exit ok == 2 ? 0 : 1}' packaging/10-appliance.conf
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
if scripts/capture-qualification-soak.sh relative 1 1 >/dev/null 2>&1; then
  printf 'installer smoke: relative soak output unexpectedly succeeded\n' >&2
  exit 1
fi
if scripts/capture-qualification-soak.sh /tmp/mackes-invalid-soak 0 1 >/dev/null 2>&1; then
  printf 'installer smoke: zero-duration soak unexpectedly succeeded\n' >&2
  exit 1
fi
printf 'installer-smoke: PASS (preflight and argument validation; no mutation)\n'
