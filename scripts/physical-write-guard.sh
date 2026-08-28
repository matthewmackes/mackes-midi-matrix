#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  printf 'usage: %s DEVICE-MAP-RECORD\n' "$0" >&2
  exit 64
fi
record=$1
if [[ ! -f "$record" ]]; then
  printf 'physical-write denied: map record does not exist: %s\n' "$record" >&2
  exit 83
fi
if ! grep -Eq '^status=verified$' "$record" || ! grep -Eq '^physical_test=pass$' "$record"; then
  printf 'physical-write denied: record must contain status=verified and physical_test=pass\n' >&2
  exit 84
fi
if [[ "${MACKES_CONFIRM_PHYSICAL_WRITE:-}" != "1" ]]; then
  printf 'physical-write denied: set MACKES_CONFIRM_PHYSICAL_WRITE=1 for explicit operator acknowledgement\n' >&2
  exit 85
fi
printf 'physical-write authorized by verified record: %s\n' "$record"
