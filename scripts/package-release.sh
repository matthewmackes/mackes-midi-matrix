#!/usr/bin/env bash
set -euo pipefail

# Backward-compatible implementation lives in package-test-release.sh; this
# stable entry point reflects that the same deterministic bundle supports both
# test and production-tagged versions.
exec "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/package-test-release.sh" "$@"
