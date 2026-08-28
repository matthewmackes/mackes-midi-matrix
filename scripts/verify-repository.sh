#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

if command -v cargo >/dev/null 2>&1; then
  cargo fmt --check
else
  echo "cargo is required for repository verification" >&2
  exit 1
fi

python3 scripts/verify-artifacts.py
python3 scripts/check-worklist.py
while IFS= read -r -d '' script; do
  bash -n "$script"
done < <(find scripts -maxdepth 1 -type f -name '*.sh' -print0 | sort -z)
echo "repository policy checks passed"
