#!/usr/bin/env bash
set -euo pipefail

version="${1:-$(awk -F'"' '/^version = / {print $2; exit}' Cargo.toml)}"
if [[ -z "$version" ]]; then
  echo "release-gate: unable to determine workspace version" >&2
  exit 2
fi
if [[ $# -gt 1 || ! "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-test\.[0-9]+)?$ ]]; then
  printf 'usage: %s [VERSION]\n' "$0" >&2
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
printf 'release-gate: release artifact\n'
scripts/package-release.sh "$version" >/dev/null
archive="dist/mackes-midi-matrix-${version}-linux-x86_64.tar.gz"
(cd dist && sha256sum -c "mackes-midi-matrix-${version}-linux-x86_64.tar.gz.sha256")
archive_listing="$(tar -tzf "$archive")"
rg -q "mackes-midi-matrix-${version}-linux-x86_64/target/release/mackes-midi-matrix$" \
  <<<"$archive_listing"
rg -q "mackes-midi-matrix-${version}-linux-x86_64/target/release/mackes-midi-matrixd$" \
  <<<"$archive_listing"
rg -q "mackes-midi-matrix-${version}-linux-x86_64/BUILD_PROVENANCE$" \
  <<<"$archive_listing"
artifact_tmp="$(mktemp -d)"
trap 'rm -rf -- "$artifact_tmp"' EXIT
tar -xzf "$archive" -C "$artifact_tmp"
"$artifact_tmp/mackes-midi-matrix-${version}-linux-x86_64/scripts/install-fedora.sh" --check
target/release/mackes-midi-matrix validate fixtures/config-scenes-valid.json5 >/dev/null
target/release/mackes-midi-matrix scene plan fixtures/config-scenes-valid.json5 demo verse --json \
  | rg -q '"unsafe":true'
printf 'release-gate: PASS\n'
