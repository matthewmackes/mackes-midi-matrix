#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 0 ]]; then
  printf 'usage: %s\n' "$0" >&2
  exit 64
fi

printf 'hardware-qualification=observation-only\n'
printf 'host=%s\n' "$(hostname)"
printf 'kernel=%s\n' "$(uname -sr)"
printf '\n[usb]\n'
if command -v lsusb >/dev/null 2>&1; then
  lsusb
else
  printf 'lsusb unavailable\n'
fi
printf '\n[alsa-cards]\n'
cat /proc/asound/cards 2>/dev/null || printf 'ALSA cards unavailable\n'
printf '\n[alsa-midi-nodes]\n'
find /dev/snd -maxdepth 1 -type c -name 'midi*' -printf '%f\n' 2>/dev/null | sort || true
printf '\n[application-endpoints]\n'
cargo run -q -p mackes-midi-matrix -- endpoints --json
printf '\n[alsa-diagnostics]\n'
for diagnostic in amidi aconnect; do
  if command -v "$diagnostic" >/dev/null 2>&1; then
    printf '%s=%s\n' "$diagnostic" "$(command -v "$diagnostic")"
  else
    printf '%s=unavailable\n' "$diagnostic"
  fi
done
if command -v amidi >/dev/null 2>&1; then
  midisport_ports="$(amidi -l 2>/dev/null | awk '/MidiSport 4x4 MIDI/{count++} END{print count+0}')"
  printf 'midisport_4x4_ports=%s\n' "$midisport_ports"
  if [[ "$midisport_ports" -eq 4 ]]; then
    printf 'midisport_4x4_acceptance=pass\n'
  else
    printf 'midisport_4x4_acceptance=pending\n'
  fi
else
  printf 'midisport_4x4_ports=unknown\nmidisport_4x4_acceptance=pending\n'
fi
printf '\n[write-qualification]\nPENDING: no vendor message/LED maps or physical-write validation are assumed by this report.\n'
