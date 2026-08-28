#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 0 ]]; then
  printf 'usage: %s\n' "$0" >&2
  exit 64
fi

scenario_count=$(awk '/pub const INTEGRATION_SCENARIOS:/{inside=1; next} inside && /];/{exit} inside && /"/{count++} END{print count+0}' crates/testkit/src/lib.rs)
printf 'suite=hermetic-integration\nscenarios=%s\nmode=release\n' "$scenario_count"
cargo test --release -p mackes-testkit
printf 'suite-result=PASS\n'
