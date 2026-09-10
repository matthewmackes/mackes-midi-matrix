# ADR-0013: Revision-guarded configuration commits

- Status: Accepted for implementation
- Date: 2026-09-07
- Owners: daemon/configuration boundary (W154/W155)

## Decision

Configuration writes use a content-addressed revision token (`sha256:<hex>`)
computed from canonical JSON serialization of the validated `ConfigDocument`.
When supplied, the expected token is checked while holding the shared writer
lock and before backup rotation or replacement. A mismatch returns a structured
conflict and leaves the target unchanged. Requests without a token remain a
temporary compatibility path while callers migrate.

The daemon, rather than HTTP or editors, owns the check and atomic replacement.
Reads and unrelated daemon generation changes never alter the configuration
revision. A successful commit publishes the new revision; runtime application,
durable operation outcomes, and restart recovery remain separate follow-up
steps described in `docs/configuration-boundary-design.md`.

## Rationale and limits

This prevents supported writers from silently overwriting one another while
remaining deterministic across reconnects. It does not provide compare-and-swap
against arbitrary external editors that ignore the advisory lock; reread and
divergence detection must surface that case. JSON5 comments are not represented
by the revision and must be preserved by the draft layer before commit.

## Current implementation

`crates/config/src/lib.rs` provides `document_revision`,
`save_if_revision`, and `save_with_optional_revision`; the daemon configuration
mutation path and web admission boundary pass the token through. Durable
operation intents, idempotency records, and migration of remaining writers are
tracked in W155.
