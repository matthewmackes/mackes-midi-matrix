# ADR-0019: Starter setup uses one reviewed atomic apply

**Status:** Accepted for W192 implementation

## Decision

Quick Start may discover and preview recommendations only from the versioned Studio capability
projection. A proposal must retain the stable physical-control ID, destination profile/effect/function,
generation, compatibility result, and LED-feedback truth. Previewing never sends MIDI or mutates the
mapping store.

Applying a reviewed set will use one daemon-owned, generation-checked batch transaction. The daemon
must validate every proposal and either commit the complete set or leave the prior mapping unchanged;
partial client-side loops are not an acceptable substitute. Conflicts, stale generations, unavailable
devices, and persistence failures return a structured result that preserves the review draft.

## Consequences

The current browser slice intentionally stops at capability-filtered preview until the typed batch IPC
contract exists. Existing single-assignment commits remain available through the normal assignment
editor and are not reused as a fake atomic setup operation.
