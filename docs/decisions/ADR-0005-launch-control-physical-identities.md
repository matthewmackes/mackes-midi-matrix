# ADR-0005: Launch Control physical identities

## Decision

Launch Control XL Mk2 controls use stable profile-owned string identities. A
physical identity is separate from its MIDI source address and LED feedback
address; MIDI numbers and firmware LED indices are transport details, not
identity. The catalog contains 24 knobs, 16 channel buttons, 8 faders, and 8
utility controls exactly once.

Legacy numeric assignments remain readable for released configurations. Values
0–39 retain their documented knob/button meaning and 40–47 retain canonical
utility meaning. Because the old effects presentation used 40–47 ambiguously,
those entries are never inferred as faders: they load disabled with a
`needs-review` marker and original evidence preserved.

## Consequences

New configuration data uses `physical_control_id`; source and feedback
addresses are optional independent fields. Unknown or duplicate IDs fail
closed. Fader assignments must be recaptured from a stable identity.
