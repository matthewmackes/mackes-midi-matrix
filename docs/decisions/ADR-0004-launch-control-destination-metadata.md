# ADR-0004: Optional Launch Control destination metadata

Status: Accepted

## Decision

Launch Control XL assignments may include an optional, human-readable `destination`
summary. It is bounded to 96 bytes, must be non-empty when present, and is displayed
with validated live activity in the TUI faceplate.

The field is optional and uses serde defaults so existing templates remain compatible.
It is descriptive metadata only; MIDI routing and learned values remain authoritative.

## Rationale

Physical control labels identify the device surface, while destination names identify
what a mapping controls. Keeping the destination explicit prevents the UI from
guessing routing from labels or MIDI numbers.
