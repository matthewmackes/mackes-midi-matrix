#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 0 ]]; then
  printf 'usage: %s\n' "$0" >&2
  exit 64
fi

if ! command -v cargo-audit >/dev/null 2>&1; then
  cargo_bin_dir="$(dirname "$(command -v cargo)")"
  if [[ -x "${cargo_bin_dir}/cargo-audit" ]]; then
    export PATH="${cargo_bin_dir}:${PATH}"
  fi
fi

if ! command -v cargo-audit >/dev/null 2>&1; then
  user_home="$(getent passwd "$(id -u)" | cut -d: -f6)"
  if [[ -n "${user_home}" && -x "${user_home}/.cargo/bin/cargo-audit" ]]; then
    export PATH="${user_home}/.cargo/bin:${PATH}"
  fi
fi

if ! command -v cargo-audit >/dev/null 2>&1; then
  printf 'dependency-audit: cargo-audit is required; install it before release qualification\n' >&2
  exit 69
fi

if cargo audit; then
  exit 0
fi

# Restricted release environments may have a valid cached RustSec database but
# deny Cargo's refresh lock. Reuse that database only after the fresh scan has
# failed; a missing cache remains a hard failure and findings still propagate.
if [[ -d "${CARGO_HOME:-${HOME}/.cargo}/advisory-db" ]]; then
  printf 'dependency-audit: fresh database refresh unavailable; retrying cached database\n' >&2
  cargo audit --no-fetch
else
  exit 1
fi
