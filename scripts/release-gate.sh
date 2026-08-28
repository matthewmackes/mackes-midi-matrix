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
printf 'release-gate: test-release artifact\n'
scripts/package-test-release.sh 0.1.0-test.1 >/dev/null
(cd dist && sha256sum -c mackes-midi-matrix-0.1.0-test.1-linux-x86_64.tar.gz.sha256)
archive_listing="$(tar -tzf dist/mackes-midi-matrix-0.1.0-test.1-linux-x86_64.tar.gz)"
rg -q 'mackes-midi-matrix-0.1.0-test.1-linux-x86_64/target/release/mackes-midi-matrix$' \
  <<<"$archive_listing"
rg -q 'mackes-midi-matrix-0.1.0-test.1-linux-x86_64/target/release/mackes-midi-matrixd$' \
  <<<"$archive_listing"
artifact_tmp="$(mktemp -d)"
trap 'rm -rf -- "$artifact_tmp"' EXIT
tar -xzf dist/mackes-midi-matrix-0.1.0-test.1-linux-x86_64.tar.gz -C "$artifact_tmp"
"$artifact_tmp/mackes-midi-matrix-0.1.0-test.1-linux-x86_64/scripts/install-fedora.sh" --check
target/release/mackes-midi-matrix validate fixtures/config-scenes-valid.json5 >/dev/null
target/release/mackes-midi-matrix scene plan fixtures/config-scenes-valid.json5 demo verse --json \
  | rg -q '"unsafe":true'
printf 'release-gate: PASS\n'
