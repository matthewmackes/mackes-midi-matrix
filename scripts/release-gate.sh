#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 0 ]]; then
  printf 'usage: %s\n' "$0" >&2
  exit 64
fi

printf 'release-gate: formatting\n'
cargo fmt --check
printf 'release-gate: repository policy and worklist\n'
scripts/verify-repository.sh
printf 'release-gate: locked dependency metadata\n'
cargo metadata --locked --all-features --format-version 1 >/dev/null
printf 'release-gate: dependency advisories\n'
scripts/dependency-audit.sh
printf 'release-gate: workspace tests\n'
cargo test --workspace --all-features
printf 'release-gate: workspace clippy\n'
cargo clippy --workspace --all-targets --all-features -- -D warnings
printf 'release-gate: routing benchmark\n'
scripts/benchmark-routing.sh
printf 'release-gate: hermetic integration\n'
scripts/integration-suite.sh
printf 'release-gate: installer smoke\n'
scripts/installer-smoke.sh
printf 'release-gate: PASS\n'
