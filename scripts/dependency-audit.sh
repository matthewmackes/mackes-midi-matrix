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

cargo audit
