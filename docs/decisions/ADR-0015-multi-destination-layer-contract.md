# ADR-0015: Daemon-owned multi-destination and modifier-layer contract

**Status:** Proposed — implementation remains gated by validation and hardware qualification.
**Date:** 2026-09-08

## Decision

W165 must extend the daemon-owned mapping model rather than encode layers in the browser. The
existing `ControlMapping` is deliberately single-destination (`destination_endpoint`, profile,
effect, and parameter) and `ControlMappingStore` is generation-guarded and persisted through the
configuration document. A future versioned contract should therefore represent an ordered list of
destination bindings plus an optional mutually-exclusive modifier-layer selector, with the
effective mapping projection computed by the daemon for snapshots, events, LED intent, and MIDI
dispatch.

The browser may edit drafts and display the effective projection, but it must not independently
combine base, scene, and modifier mappings. Scene activation is already daemon-owned through
`Project.scenes`, `active_scene`, and `compile_scene_actions`; scene changes must clear the active
modifier before publishing the new effective projection.

## Current compatibility baseline

The persisted `ControlMapping` currently contains `id`, controller profile, physical source id,
source endpoint/kind/channel/number, one optional destination channel, one destination endpoint,
destination profile/effect/parameter, `MappingBehavior`, `enabled`, and `profile_version`. The
store rejects duplicate physical sources and duplicate destination tuples during generation-guarded
commit. A future versioned destination-list shape must preserve every one of these fields for the
single-binding case and make any normalization explicit rather than silently selecting a target.
Authoritative source anchors: `crates/config/src/lib.rs:206-236` for the persisted field set and
`crates/config/src/lib.rs:344-396` for generation-guarded collision checks.

## Required invariants before implementation

- Every destination binding has a stable identity, endpoint, profile, effect, parameter, channel,
  and behavior; ordering is deterministic and duplicates are rejected.
- At most one modifier layer is active. A layer switch replaces the effective assignment set;
  toggling the active layer returns to base.
- Scene activation clears modifier state atomically with the scene generation update.
- Existing single-destination mappings round-trip without migration loss; displaced controls are
  reported rather than silently deleted.
- Effective mappings, save acknowledgements, reconnect snapshots, and LED intent share one daemon
  generation. Unsupported LED readback remains explicitly unknown.

## Evidence and source boundaries

The source-backed baseline is `crates/config/src/lib.rs` (`ControlMapping`,
`ControlMappingStore`, `Project`, and `active_scene`) and `apps/mackesd/src/startup_restore.rs`
(`compile_scene_actions` and active-scene persistence). The current IPC mapping envelope exposes
single-destination records and has no persisted modifier field. No new JSON shape or MIDI/LED
behavior is claimed by this ADR; schema and protocol changes require a follow-up implementation
ADR, fixtures, migration tests, and exact Novation manufacturer/profile references. The hardware
constraints are governed by `docs/decisions/ADR-0010-launch-control-xl-mk2-factory-template-1.md`,
`docs/decisions/ADR-0011-launch-control-xl-programmer-contract.md`, and the Novation reconciliation
section of `docs/device-feature-inventory.md`; in particular, unsupported LED readback must remain
unknown and no fictional control addresses may be introduced.

## Consequence

W165 remains intentionally open. This ADR prevents a UI-only implementation from creating state
that the daemon cannot persist, reconcile, or recover. Existing mappings and the current W161
surface remain unchanged until the contract is implemented and qualified.

The lossless schema checkpoint is captured separately in ADR-0017; it is a proposal only and does
not authorize runtime adoption.
