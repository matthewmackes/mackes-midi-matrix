# ADR-0003: Durable native device identity

Status: Accepted for W099 implementation
Date: 2026-09-05

## Decision

MACKES persists application-owned endpoint aliases and logical port/direction
identities. ALSA client numbers, port numbers, USB bus paths, enumeration order,
and generated endpoint hashes are runtime addresses only; they are never durable
device identity.

Identity matching uses the strongest verified tuple available, in this order:

1. vendor ID, product ID, verified serial, logical port index, and direction;
2. vendor ID, product ID, persisted operator binding, logical port index, and
   direction for serial-less hardware;
3. no automatic match when either duplicate candidates or insufficient identity
   data remain.

Client and port numbers may be used to reopen a currently resolved endpoint, but
must be discarded when the endpoint disappears. A moved or replacement
serial-less device requires an explicit operator rebind. Display names, MIDI
channel/note/CC tuples, and matching profile labels are never sufficient to
select a device.

The resolver is daemon-owned and shared by routing, mappings, Learn, profile
outputs, inventory, and LED targets. Input and output identities are separate;
HUI ports are excluded from MIDI-controller bindings, and MIDISPORT logical port
indices are preserved. Ambiguous or permission-denied candidates remain visible
as repairable faults and fail closed without activating a mapping.

## Migration and recovery

Legacy volatile endpoint references are retained as unresolved references until a
unique identity is proven. A migration writes a backup before replacing the
configuration and preserves mapping IDs, channels, values, enabled state, and
disabled mappings. Failed migration leaves the prior document unchanged and
offers undo. Reconnect replay may restore desired LED state only after the
resolver proves the same output identity; it never replays stale button presses
or effect writes.

## Qualification obligations

Tests must cover changed ALSA numbers, moved ports, input/output returning
separately, duplicate serial-less devices, HUI exclusion, permission recovery,
restart, and unrelated controllers emitting identical MIDI tuples. Hardware
qualification must record the identity and subscription snapshot before and
after each reconnect; unresolved or unavailable observations remain open in
W099/W104.
