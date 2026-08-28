#!/usr/bin/env bash
set -euo pipefail

duration="${1:-60}"
if [[ ! "$duration" =~ ^[1-9][0-9]*$ ]]; then
  printf 'usage: %s [duration-seconds]\n' "$0" >&2
  exit 64
fi

started=$(date +%s)
deadline=$((started + duration))
iterations=0
failures=0
while (( $(date +%s) < deadline )); do
  if cargo test --release -p mackes-testkit throughput_regression_routes_ten_thousand_messages_without_drops >/dev/null; then
    iterations=$((iterations + 1))
  else
    failures=$((failures + 1))
  fi
done
ended=$(date +%s)
printf 'scenario=routing-soak\nduration_seconds=%s\niterations=%s\nfailures=%s\nelapsed_seconds=%s\n' \
  "$duration" "$iterations" "$failures" "$((ended - started))"
(( failures == 0 ))
