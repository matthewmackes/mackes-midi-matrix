# ADR-0008: Daemon-owned assignment session and layered controller feedback

## Status

Accepted — 2026-08-31

## Decision

The daemon owns one `AssignmentSession` for controller-driven reassignment. Hardware
buttons and keyboard arrows submit the same typed actions and therefore cannot diverge
in navigation, cancellation, retry, or commit behavior. The session is generation
checked and keeps the prior screen as an explicit return target.

An assignment starts in `AwaitControl`, accepts only one unique eligible physical
control within the bounded 250 ms candidate window, and remains inert until a complete
parameter mapping is committed through the W074 transaction. A 750 ms-or-longer
Device hold cancels the active session. Disconnects become `Interrupted`; recovery
requires explicit resume or discard.

Controller feedback is layered. The authoritative base mapping/activity state is
underneath assignment guidance, which is underneath the terminal result overlay.
Result overlays emit exactly two 400 ms pulses, green for success and red for failure,
then restore the selected base state. Fader columns use their paired channel-button
LEDs as a deterministic proxy. No layer writes template definitions.

## Consequences

TUI consumers render daemon projections and never own session transitions or direct
configuration writes. Fake-clock tests can prove gesture boundaries, candidate
ambiguity, interruption recovery, pulse timing, and base restoration without sleeps
or hardware. Any future protocol or LED change must preserve stable physical IDs and
return to the owning session/profile contract for review.
