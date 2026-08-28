#!/usr/bin/env bash
set -euo pipefail

if [[ "${1:-}" == "--help" ]]; then
  printf 'Usage: %s\n' "$0"
  printf 'Runs the deterministic 10,000-message routing regression in release mode.\n'
  exit 0
fi
if [[ $# -ne 0 ]]; then
  printf 'error: no arguments are accepted\n' >&2
  exit 64
fi

printf 'scenario=throughput_regression_routes_ten_thousand_messages_without_drops\n'
printf 'host=%s\n' "$(hostname)"
printf 'kernel=%s\n' "$(uname -sr)"
printf 'rust=%s\n' "$(rustc --version)"
printf 'cargo=%s\n' "$(cargo --version)"

time cargo test --release -p mackes-testkit throughput_regression_routes_ten_thousand_messages_without_drops
