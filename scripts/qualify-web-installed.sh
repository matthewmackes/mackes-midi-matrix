#!/usr/bin/env bash
set -euo pipefail

origin=${1:-http://172.20.222.222:8081}
run_fixture() {
  local label=$1; shift
  echo "qualify-web: $label"
  "$@"
  # Let the local daemon drain closed browser IPC writers before the next fixture. A
  # short transient timeout is retried, but persistent unhealth remains fatal.
  local recovered=0
  for attempt in 1 2 3; do
    if curl --silent --show-error --fail --max-time 5 "$origin/api/v1/health" >/dev/null; then
      recovered=1
      break
    fi
    sleep 2
  done
  if [[ "$recovered" != 1 ]]; then
    echo "qualify-web: health did not recover after $label" >&2
    return 1
  fi
  sleep 1
}
run_fixture "route smoke" bash scripts/browser-smoke.sh "$origin"
run_fixture "device inventory" python3 scripts/browser-device-inventory-smoke.py "$origin"
run_fixture "graphical inventory" python3 scripts/browser-graphical-inventory-smoke.py "$origin"
run_fixture "PiPedal catalog" python3 scripts/pipedal-catalog-smoke.py "$origin"
run_fixture "generation ordering" python3 scripts/browser-generation-smoke.py "$origin"
run_fixture "request cancellation" python3 scripts/browser-abort-smoke.py "$origin"
run_fixture "visible-tab resume" python3 scripts/browser-resume-smoke.py "$origin"
run_fixture "draft preservation" python3 scripts/browser-draft-preservation-smoke.py "$origin"
run_fixture "focus preservation" python3 scripts/browser-focus-preservation-smoke.py "$origin"
run_fixture "scroll preservation" python3 scripts/browser-scroll-preservation-smoke.py "$origin"
run_fixture "mobile overflow" python3 scripts/browser-mobile-overflow-smoke.py "$origin"
run_fixture "assignment inspector" python3 scripts/browser-assignment-inspector-smoke.py "$origin"
run_fixture "lossless route guard" python3 scripts/browser-route-lossless-guard-smoke.py "$origin"
run_fixture "task ownership" python3 scripts/browser-task-ownership-smoke.py "$origin"
echo "qualify-web: PASS origin=$origin"
