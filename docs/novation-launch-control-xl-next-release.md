# Novation Launch Control XL Mk2 — next-release future state

This document defines the operator-facing behavior targeted for the next release. It is a
release specification, not a claim about features already available in the current build.

## Fixed physical layout

MACKES will use a static, permanently labeled Launch Control XL Mk2 layout. Controls will not be
dynamically reassigned between effects.

### Knob rows

| Row | Left group | Right group |
|---|---|---|
| 1 | Gain | Gate |
| 2 | Compressor | Modulation, including Detune/MicroPitch |
| 3 | Delay | Reverb |

Each group will have four fixed parameter knobs, selected from the owning device’s available
parameters. Unsupported parameters remain visibly unavailable; unused controls remain unassigned.

### Channel buttons

The channel buttons will provide fixed group actions:

- One Enable button per group.
- One Type/Model button per group.
- Enable LED: green when enabled, solid red when disabled.
- Type/Model LED: blue/teal for the selected type.
- Unavailable group: blinking red LED and an explicit `unavailable` label.

### Faders

The eight faders will be permanently labeled:

1. Gain
2. Gate
3. Compressor
4. Modulation
5. Delay
6. Reverb
7. Master
8. Output

### Utility controls

Device, Mute, Solo, Record Arm, Up, Down, Left, and Right will remain utility/navigation inputs.
They will not be assigned to effect parameters in this release.

## Signal-chain automation

The fixed processing topology is:

`Eventide MicroPitch → Lexicon Reflex → master output`

The controller will route each parameter to the device that owns it:

- MicroPitch: detune, micro-pitch, pitch shift, digital delay, and feedback-pitch effects.
- Reflex: documented reverb, modulation, and delay algorithms.

Changing an effect group will update only that group’s owning device. Unrelated blocks
will be bypassed and hidden in generated configurations.

## Reusable configurations

MACKES will generate minimal effect configurations from the enabled groups. A configuration
will contain only the required blocks, types, parameters, values, and MIDI assignments.

Names will be generated in signal-path order, for example:

- `Gain + Gate`
- `Gain + Gate + Compressor`
- `Modulation + Delay`
- `Delay + Reverb`
- `Gain + Gate + Compressor + Modulation + Delay + Reverb`

Changing enabled groups will regenerate the minimal configuration automatically. Imported editor
maps will be validated for device identity, firmware, schema, ranges, ownership, duplicates, and
source-artifact hash before reuse.

## Operator behavior

- The TUI and CLI will expose the same static labels and assignments.
- Effect changes will load stored values onto the controller.
- Pickup mode will prevent a physical knob from jumping a stored parameter.
- Scene activation and reconnect will resynchronize desired LED and parameter state.
- Every hardware write will be audited and retried at most twice.
- Devices without acknowledgment or read-back will be reported as `sent-unverified`.
- No control will be silently redirected to another device.

## Release boundary

The next release must include deterministic simulator, LED test, and effects demo modes covering
all six groups, fixed faders, enable/type LEDs, configuration generation, reconnect, and scene
activation without paired hardware. Physical appearance and paired-device qualification remain
post-release evidence.

Implementation is tracked by W062–W069 in [`WORKLIST.md`](../WORKLIST.md).
