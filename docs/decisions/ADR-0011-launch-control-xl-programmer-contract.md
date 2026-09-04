# ADR-0011: Launch Control XL programmer contract

MACKES treats Novation's Launch Control XL Programmer's Reference Guide as
the device protocol authority. MIDI numbers are template data; they do not
identify a physical control unless verified for the selected template.

## LED address map

The background LED SysEx message is:

`F0 00 20 29 02 11 78 Template Index Value F7`

Template slots 0–7 are User templates and 8–15 are Factory templates.
Indices are fixed by physical geometry:

| Index | Physical controls |
| --- | --- |
| 0–7 | top knob row, left to right |
| 8–15 | middle knob row, left to right |
| 16–23 | bottom knob row, left to right |
| 24–31 | top channel-button row, left to right |
| 32–39 | bottom channel-button row, left to right |
| 40–43 | Device, Mute, Solo, Record Arm |
| 44–47 | Up, Down, Left, Right |

The platform must derive LED addresses from this physical map, never from a
button's observed MIDI note number. A template may assign arbitrary notes,
CCs, channels, and press/release behavior to the controls.

## Operational rule

When a template's assignments differ from the reviewed map, MACKES must store
the observed MIDI assignment separately from the physical control identity.
It must not relabel a physical control or infer a row from a note bank.

Source: Novation/Focusrite, *Launch Control XL Programmer's Reference Guide*,
sections 3 and 4.
