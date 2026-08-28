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

## Consequences

Learned mappings survive restart and retain enough evidence for deterministic review. Exact
duplicates, dangling endpoints, invalid channels, unknown message/mode tags, empty evidence, and
oversized evidence are rejected before persistence.
