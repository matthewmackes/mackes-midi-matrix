#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 || ! "$1" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-test\.[0-9]+)?$ ]]; then
  printf 'usage: %s VERSION (example: 0.1.0 or 0.1.0-test.1)\n' "$0" >&2
  exit 64
fi

version="$1"
root_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source_commit="$(git -C "$root_dir" rev-parse HEAD)"
dist_dir="$root_dir/dist"
bundle_name="mackes-midi-matrix-$version-linux-x86_64"
archive="$dist_dir/$bundle_name.tar.gz"
checksum="$archive.sha256"
staging="$(mktemp -d)"
trap 'rm -rf -- "$staging"' EXIT

cargo build --release --locked \
  --package mackes-midi-matrix \
  --package mackesd

for binary in mackes-midi-matrix mackes-midi-matrixd; do
  path="$root_dir/target/release/$binary"
  [[ -x "$path" ]] || {
    printf 'missing release binary: %s; run cargo build --release --locked\n' "$path" >&2
    exit 79
  }
done

install -d -m 0755 \
  "$staging/$bundle_name/target/release" \
  "$staging/$bundle_name/scripts" \
  "$staging/$bundle_name/packaging" \
  "$staging/$bundle_name/docs"
install -m 0755 "$root_dir/target/release/mackes-midi-matrix" \
  "$staging/$bundle_name/target/release/mackes-midi-matrix"
install -m 0755 "$root_dir/target/release/mackes-midi-matrixd" \
  "$staging/$bundle_name/target/release/mackes-midi-matrixd"
install -m 0755 "$root_dir/scripts/install-fedora.sh" \
  "$staging/$bundle_name/scripts/install-fedora.sh"
install -m 0644 "$root_dir/packaging/mackes.service" \
  "$staging/$bundle_name/packaging/mackes.service"
install -m 0644 "$root_dir/README.md" "$root_dir/LICENSE" "$root_dir/Cargo.lock" \
  "$staging/$bundle_name/"
install -m 0644 "$root_dir/docs/installation-fedora.md" \
  "$root_dir/docs/hardware-qualification.md" "$staging/$bundle_name/docs/"
install -m 0644 "$root_dir/docs/releases/$version.md" "$staging/$bundle_name/RELEASE_NOTES.md"
printf 'version=%s\nsource_commit=%s\n' "$version" "$source_commit" >"$staging/$bundle_name/BUILD_PROVENANCE"

install -d -m 0755 "$dist_dir"
tar --sort=name --owner=0 --group=0 --numeric-owner \
  --mtime='UTC 2026-08-28' -C "$staging" -czf "$archive" "$bundle_name"
(cd "$dist_dir" && sha256sum "$(basename "$archive")") >"$checksum.tmp"
mv -f -- "$checksum.tmp" "$checksum"
printf 'release-archive=%s\nchecksum=%s\n' "$archive" "$checksum"
