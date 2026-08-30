# ADR-0003: Persisted MIDI Learn contract

- Status: Accepted
- Date: 2026-08-28

## Context

MIDI Learn previously produced only an in-memory routing draft. That omitted the selected global
input and the source signature needed to reconstruct, audit, and safely edit a learned mapping
after restart.

## Decision

Schema version 1 gains two backward-compatible, defaulted fields:

- `settings.learn_input_alias` stores the one operator-selected capture endpoint alias.
- `learned_mappings` stores one source signature, explicit channel policy, bounded raw wire
  evidence, one destination, execution mode, enabled state, and priority per mapping.

All endpoint references are validated. Channel-bearing and channel-less messages use distinct
policies. Mapping creation remains transactional and is available from the TUI only after its
mandatory live test and explicit commit. Existing version-1 documents deserialize with empty
defaults, so no migration or version increment is required.

### Live-test boundary

The live test is a separate, versioned local-IPC operation owned by the daemon. Its request carries
the selected source endpoint ID, validated candidate signature, channel policy, destination ID, and
a bounded request identifier. The daemon returns exactly one terminal result: `passed`, `failed`,
`timed_out`, `denied`, or `unavailable`, with a bounded operator-safe reason and audit reference.
Only a `passed` result may unlock the existing explicit commit. The TUI must never infer success from
keypresses, elapsed time, or an unverified echo. Device profiles own the actual probe/observation
semantics; unsupported or non-readable destinations return `unavailable` and remain uncommitted.
Requests are generation-checked and idempotent by request identifier, and all failure paths fail
closed without mutating persisted mappings.

## Consequences

Learned mappings survive restart and retain enough evidence for deterministic review. Exact
duplicates, dangling endpoints, invalid channels, unknown message/mode tags, empty evidence, and
oversized evidence are rejected before persistence.

The explicit live-test boundary makes hardware acknowledgment an auditable daemon decision while
keeping the TUI renderer-neutral and preventing a local UI action from manufacturing qualification
evidence.
