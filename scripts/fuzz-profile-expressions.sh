#!/usr/bin/env bash
set -euo pipefail

# Deterministic, offline parser fuzz smoke test. The Rust test owns the bounded
# malformed corpus and asserts every input returns an error without panicking.
exec cargo test -p mackes-profiles malformed_expression_corpus_never_panics --quiet
