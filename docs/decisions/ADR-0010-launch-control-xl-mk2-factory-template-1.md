# ADR-0010: Launch Control XL Mk2 Factory Template 1 contract

## Status

Accepted — 2026-09-01

## Decision

MACKES uses the Launch Control XL Mk2 Factory Template 1 as its only production Mk2
layout. Operators select Factory Template 1 on the controller; MACKES does not install,
define, or claim a Novation Components User template. The target is USB model
`1235:0061`, with the firmware-supplied Factory Template 1 selected.

The profile-owned `launch_control_mk2_factory1_layout` table is the sole runtime source
of truth. All controls use zero-based MIDI channel 8 (wire channel 9). Knobs use CC
13–20, 29–36, and 49–56; faders use CC 77–84; channel buttons use notes 41–48 and
57–64. Device, Mute, Solo, and Record Arm use notes 105–108. Up, Down, Left, and Right
use CC 104–107. Continuous controls accept values 0–127. Buttons use nonzero press and
zero release semantics. The stable physical IDs remain those defined by ADR-0005.

Knob and channel-button feedback addresses remain the documented indices 0–39.
Utility feedback uses indices 40–47. Faders have no individual LED address; their proxy
policy remains governed by ADR-0008. Traditional input messages and Mk1 programmer-
reference LED SysEx are separate protocols and must not be inferred from one another.

### Clean-start procedure

The Launch Control XL Mk1/Mk2 has no factory-reset function, and Novation provides no
Factory Pack for these models. The dependable clean start is the immutable firmware template:
hold `FACTORY`, press bottom-row pad `1`, and release `FACTORY` to select Factory Template 1.
Do not use Components to send a User template as a substitute; MACKES does not write template
definitions.

## Migration and failure behavior

Existing Factory Template 1 mappings already store stable physical IDs and their captured
source tuple, so they retain their destinations. A stored tuple is reconciled only when its
stable ID maps exactly to this table. Contradictory User 1 tuples, other channels, wrong
message families, unknown model identities, and ambiguous devices fail closed and require
explicit recapture; destinations are never silently rewritten.

The former MACKES User 1 manifest and onboarding contract are retired. W094 replaces
their warning-only release check with a versioned Factory Template 1 contract fixture.

## Evidence

The exact input tuples are physical captures recorded in `docs/hardware-qualification.md`.
Profile and daemon regression tests prove uniqueness, full inventory, utility semantics,
wrong-layout rejection, and shared decoding.
