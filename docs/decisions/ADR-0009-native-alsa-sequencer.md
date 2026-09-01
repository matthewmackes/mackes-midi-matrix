# ADR-0009: Native ALSA Sequencer control-surface backend

- Status: proposed for W083
- Date: 2026-08-31

## Decision

On Linux, MACKES will use one daemon-owned native ALSA Sequencer client with named application
input/output ports. Hardware and bridge ports are selected by stable ALSA client/port identity plus
validated device metadata, then connected with explicit sequencer subscriptions. The daemon reads
events nonblockingly from its owned queue and dispatches decoded domain events through the existing
routing boundary.

`midir` callback input is not the authoritative production control-surface path. PipeWire and JACK
remain optional system patch bays; the daemon does not require either service or shell utilities.

## Rationale

The ALSA Sequencer API is designed for arbitrary application-to-device port subscriptions, dynamic
routing, scheduling, and client/port lifecycle changes. This matches a resident control-surface
daemon better than repeatedly opening ports by display name and depending on a callback-owned queue.

## Contract

- The backend exposes stable endpoint identity, ALSA source address, direction, display metadata,
  connection state, and lifecycle reason as typed values.
- Discovery is descriptive only. Subscription is explicit, idempotent, direction-checked, and
  fail-closed for duplicates or ambiguity.
- Reads are bounded and nonblocking. Reader code decodes into existing validated MIDI 1.0 domain
  messages; it never performs routing, mapping mutation, or hardware writes.
- ALSA client/port numbers are volatile runtime addresses and are never persisted as identity.
- Client/port announcements drive reconciliation. A disconnect preserves mappings and marks the
  endpoint unavailable; reconnect revalidates identity before restoring subscriptions or LEDs.
- Queue overflow, malformed events, unsupported event types, and permission failures become visible
  counters/health errors and never silently route data.
- The service remains least privilege (`mackes` plus required audio/sequencer access); running as
  root or changing the control socket to world-writable is not an acceptable workaround.

## Compatibility and rollback

The existing backend-neutral adapter and domain event contracts remain available while W084–W087
land behind a mutually exclusive feature gate. No configuration, mapping, profile, or SysEx schema
may be removed until W088 confirms migration and rollback evidence. Non-Linux builds retain their
existing feature-disabled behavior.

## Verification

W084–W086 must prove the contract with fake inventories, event fixtures, and announcement streams.
W087 must prove parity through daemon/TUI tests. W088 must verify the Launch Control XL Mk2 Device
packet, controls, arrows, reconnect, duplicate-name rejection, and simultaneous `aseqdump` observation
on the Fedora host.
