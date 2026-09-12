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
  for attempt in 1 2 3 4 5; do
    if curl --silent --show-error --fail --max-time 10 "$origin/api/v1/health" >/dev/null; then
      recovered=1
      break
    fi
    sleep 1
  done
  if [[ "$recovered" != 1 ]]; then
    echo "qualify-web: health did not recover after $label" >&2
    return 1
  fi
  sleep 1
}
run_fixture "route smoke" bash scripts/browser-smoke.sh "$origin"
run_fixture "root Studio" python3 scripts/browser-root-studio-smoke.py "$origin"
run_fixture "Studio console" python3 scripts/browser-studio-console-smoke.py "$origin"
run_fixture "Studio accessibility" python3 scripts/browser-studio-accessibility-tree-smoke.py "$origin"
run_fixture "Studio visual accessibility" python3 scripts/browser-visual-accessibility-smoke.py "$origin"
run_fixture "Studio novice walkthrough" python3 scripts/browser-novice-walkthrough-smoke.py "$origin"
run_fixture "Studio mobile overflow" python3 scripts/browser-mobile-overflow-smoke.py "$origin"
run_fixture "Studio PiPedal catalog" python3 scripts/pipedal-catalog-smoke.py "$origin"
run_fixture "Studio draft reload" python3 scripts/browser-studio-draft-reload-smoke.py "$origin"
run_fixture "Studio assignment inputs" python3 scripts/browser-studio-assignment-input-smoke.py "$origin"
run_fixture "Studio compatibility" python3 scripts/browser-studio-compatibility-smoke.py "$origin"
run_fixture "Studio capture" python3 scripts/browser-studio-capture-smoke.py "$origin"
run_fixture "Studio conflict" python3 scripts/browser-studio-conflict-smoke.py "$origin"
run_fixture "Studio Quick Start Apply" python3 scripts/browser-studio-starter-apply-smoke.py "$origin"
run_fixture "Studio Undo" python3 scripts/browser-studio-undo-smoke.py "$origin"
run_fixture "Studio Panic" python3 scripts/browser-studio-panic-smoke.py "$origin"
run_fixture "Studio Eventide truth" python3 scripts/browser-studio-eventide-truth-smoke.py "$origin"
run_fixture "Studio Reflex truth" python3 scripts/browser-studio-reflex-truth-smoke.py "$origin"
run_fixture "Studio PiPedal events" python3 scripts/browser-studio-pipedal-event-smoke.py "$origin"
run_fixture "Studio scenes" python3 scripts/browser-studio-scenes-actions-smoke.py "$origin"
run_fixture "Studio resilience" python3 scripts/browser-studio-resilience-smoke.py "$origin"
echo "qualify-web: PASS origin=$origin"
