# Hardware qualification matrix

This matrix records observation-only results from the Fedora 44 x86_64 host. It
does not authorize MIDI, SysEx, HID, or LED writes.

| Device | USB identity | Host transport observed | Current status | Required before production |
| --- | --- | --- | --- | --- |
| Lexicon Reflex | DIN via MIDISPORT Port A (`hw:2,0,0`) | ALSA MIDI Port A | Parameter and register operations verified | Checksum/recovery and physical reconnect are post-release qualification |
| Eventide MicroPitch | `1b12:003a` | ALSA MIDI (`MicroPitch Pedal MIDI 1`) | PC1, CC4 sweep, CC15, and ALSA reopen verified; an earlier CC2 transmission is not control evidence because the official map assigns ACTIVE/BYPASS to CC14 | Re-run reversible ACTIVE/BYPASS on CC14; independently confirm audio behavior |
| Novation Launch Control XL Mk2 | `1235:0061` | ALSA MIDI plus HUI endpoint | Input/output endpoints visible; controls are user-template programmable | Bind imported template assignments, then verify page mapping and LED reports |
| M-Audio MIDISPORT 4x4 | `0763:1020` loader → `0763:1021` runtime | ALSA USB-MIDI; four MIDI ports | Firmware-loaded and enumerated | Four-port routing and physical disconnect/reconnect are post-release qualification |

Reproduce the observation report with:

```text
scripts/qualify-hardware.sh
```

For a MIDISPORT that is present as loader identity `0763:1020`, install
`fxload` and `midisport-firmware`, then trigger its udev add rule for the
device path (the path varies by topology):

```text
sudo udevadm trigger --action=add /sys/bus/usb/devices/<midisport-path>
sudo udevadm settle
amidi -l
aconnect -l
```

Successful initialization changes the USB identity to `0763:1021` and exposes
four MIDI input/output ports.

Physical qualification must use a verified map record and explicit
`MACKES_CONFIRM_PHYSICAL_WRITE=1`; see `scripts/physical-write-guard.sh`.

## Latest observation

2026-08-31 — `scripts/qualify-hardware.sh` passed its observation-only checks on
`MACKES-MIDI-PROCESSOR`. USB identity `1235:0061` and ALSA input/output endpoints
for the Launch Control XL Mk2 were present; Eventide MicroPitch was present;
the MIDISPORT was present at runtime identity `0763:1021` with four MIDI ports;
and the MACKES input/output endpoint groups were enumerated. No MIDI, SysEx, or
LED write was sent. Physical control reaction, template verification, and
disconnect/reconnect behavior remain unverified.

During the same capture, the Launch Control XL Mk2 emitted a continuous
Control Change sweep on channel 8, controller 13, rising from value 1 through
127 and returning downward. This verifies live MIDI input and value continuity
through the ALSA path; the physical knob identity was not recorded, so it is
not treated as a completed stable-ID mapping qualification.
Because CC13 is outside the reviewed Mk2 User 1 inventory, this observation is
also evidence that the reviewed template is not yet verified on the device.
In Factory Template 1, the `Device` button was then observed as Note On/Off,
zero-based MIDI channel 8, note 105, with velocities 127/0. This is retained
as a factory-template universal-control observation and is not substituted for
the reviewed User 1 stable-ID map.
Factory Template 1 `Mute` was observed as Note On/Off on zero-based MIDI
channel 8, note 106, with velocities 127/0. The paired observation confirms
the reserved universal-button behavior without changing the controller.
Factory Template 1 `Solo` was observed as Note On/Off on zero-based MIDI
channel 8, note 107, with velocities 127/0.
Factory Template 1 `Record Arm` was observed as Note On/Off on zero-based MIDI
channel 8, note 108, with velocities 127/0.
Factory Template 1 `Up` was observed as Control Change on zero-based MIDI
channel 8, controller 104, with values 127/0.
Factory Template 1 `Down` was observed as Control Change on zero-based MIDI
channel 8, controller 105, with values 127/0.
Factory Template 1 `Left` was observed as Control Change on zero-based MIDI
channel 8, controller 106, with values 127/0.
Factory Template 1 `Right` was observed as Control Change on zero-based MIDI
channel 8, controller 107, with values 127/0 (the capture received the two
button-state messages in reverse order because the button was already in a
non-idle state when listening began).
The leftmost Factory Template 1 fader produced a continuous Control Change
sweep on zero-based MIDI channel 8, controller 77, reaching value 127. This
confirms an eligible continuous control can be learned through the same input
path; its stable physical-ID assignment remains subject to the reviewed User 1
template.
The top-left eligible Factory Template 1 channel button produced Note On/Off
on zero-based MIDI channel 8, note 41, with velocities 127/0. This confirms a
discrete non-universal control is also learnable through the input path.
The second Factory Template 1 fader produced a continuous Control Change
sweep on zero-based MIDI channel 8, controller 78, including values 0 and 127.
The top-left Factory Template 1 knob produced a continuous Control Change
sweep on zero-based MIDI channel 8, controller 13. This supplies learn evidence
for the knob control class as well as faders and buttons.

The Fedora `fxload` and `midisport-firmware` packages are installed on the
qualification host, and the connected MIDISPORT has transitioned from loader
identity `0763:1020` to runtime identity `0763:1021`. The four runtime ports
are therefore available for the remaining physical routing and reconnect
qualification; package installation is no longer a prerequisite.

## Native ALSA qualification capture

2026-09-01 — with the corrected daemon installed, a bounded live `aseqdump -p 24:0`
capture observed the Mk2 Device Note On/Off on channel 8, note 105, followed by a
top-left knob CC13 sweep. Simultaneous daemon status reported `received=452`,
`last_sequence=146`, the Launch Control stable endpoint, and assignment phase
`AwaitControl`. This proves native ALSA delivery through Device into Learn and
continuous knob activity. Arrow navigation, commit/LED result, 100-pair integrity,
and USB reconnect remain to be captured.

2026-09-01 — after fixing the dashboard poll to defer unmatched MIDI events, a daemon-only
physical Device press transitioned the authoritative assignment session from `Idle` to
`AwaitControl`. Status reported `registered_inputs=7`, `received=1`, `last_sequence=3`, and
the Mk2 stable endpoint with Note Off 105 activity. This closes the prior discrepancy where
the dashboard-binding poll consumed Device before the Learn dispatcher.

2026-09-01 — the installed Factory 1 control map then resolved a physical top-left knob CC13
movement to stable control `knob-r1-c1`. The authoritative assignment session advanced from
`AwaitControl` to `ChooseDevice`, `has_draft` became true, and daemon status reported 64 received
events with the Mk2 stable endpoint. This verifies physical continuous-control capture.
