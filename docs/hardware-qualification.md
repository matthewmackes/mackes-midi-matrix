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
Because CC13 is outside the retired User 1 inventory, this observation is
also evidence that the reviewed template is not yet verified on the device.
In Factory Template 1, the `Device` button was then observed as Note On/Off,
zero-based MIDI channel 8, note 105, with velocities 127/0. This is retained
as a factory-template universal-control observation and is not substituted for
the Factory Template 1 stable-ID map.
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
path; its stable physical-ID assignment is governed by the Factory Template 1
contract.
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

2026-09-01 — fresh host qualification confirms USB identity `1235:0061`, ALSA client `24`,
seven registered native daemon inputs, connected Launch Control XL input/output ports, and
least-privilege service identity `User=mackes`, `Group=mackes-control`,
`SupplementaryGroups=audio`. Service health is `ready`; physical commit/LED, 100-pair integrity,
and USB reconnect evidence remain open.

2026-09-01 — a fresh ten-second direct `aseqdump -p 24:0` observation window reported no
events, while the daemon remained `ready` with seven registered inputs. The ALSA subscription
graph was verified separately (`24:0` connected to daemon ingress `130:0`), so this run does not
support application-level failure attribution; controller transmission/template mode remains the
next physical investigation.

2026-09-01 — after installing the current pushed build, a fresh twenty-second direct
`aseqdump -p 24:0` window again reported `Waiting for data` with no events during the requested
Device press. USB/ALSA enumeration remained healthy and the service was active. This reproduces
the upstream physical-transmission/template-mode blocker; no application-level qualification
claim is made from this run.

2026-09-01 — a separate fifteen-second capture on the Mk2 HUI port (`aseqdump -p 24:1`) also
reported no events during a requested Device press. Both exposed controller input ports are
therefore covered by direct observation; the remaining fault is upstream of the native ALSA
reader and requires the controller to transmit a MIDI template/event.

2026-09-01 — operator observation after installing the current build confirmed the physical
Launch Control control LED visibly acknowledged a captured Learn control with the yellow blinking
feedback state. This verifies the action-level LED acknowledgment path; persistent owner colors
and preset-load projection remain to be qualified.

2026-09-01 — after installing the daemon-side Right-arrow correction, the operator completed
Device → knob → Right Arrow testing. The native daemon recorded CC107 from the Mk2 and advanced
the authoritative assignment session to `ChooseParameter`, proving the physical Right Arrow now
performs the same Enter transition as the keyboard while Learn is active.

## Native ALSA walkthrough 2026-09-02

Clean `systemctl restart mackes-midi-matrix.service`: `ActiveState=active`, `MainPID=1055809`,
`User=mackes`, `Group=mackes-control`, `SupplementaryGroups=audio`, `DeviceAllow=/dev/snd/seq rw`,
`/dev/snd/seq` mode `crw-rw---- root:audio`. Status: `health=ready`, `native_backend=alsa-seq`,
`registered_inputs=7`, `received=0`, assignment `Idle`, `native_failure` none. USB `1235:0061`
enumerates as ALSA client `24`; `24:0` is subscribed only by daemon ingress `130:0`, and daemon
output `131:0` is connected to `24:0`.

Physical Factory Template 1 Device press/release: daemon `received=1`, phase `Idle` →
`AwaitControl`, last activity Note Off 105 on zero-based channel 8 from Mk2 stable input
`midir-in-8cb77f53a765bab7`. Operator selected Device pressed-and-released.

Physical top-left knob: CC13 sweep, `has_draft=true`, phase `AwaitControl` → `ChooseDevice`.
Operator reported the captured control LED went yellow.

Physical Right arrow (CC107 press/release) advanced exactly one catalog level per press:
`ChooseDevice` → `ChoosePreset` → `ChooseEffect` → `ChooseType` → `ChooseParameter`. Received
counters advanced by one decoded press per step.

Physical Right on `ChooseParameter` did **not** commit. CC107 was received (`received` 258 → 259)
and the session remained `ChooseParameter` with `control_mappings` absent from status and
`sent=0`. Native dispatch applies `Enter` for Right only when the phase is not Idle and not
`ChooseParameter`; the parameter-level Right is recorded as TUI `ui_navigation` instead of a
typed Commit with destination IDs. Commit/LED result is therefore incomplete on the controller
path and remains open under W088/W096. This is not recorded as a passing commit.

USB reconnect, 100-pair integrity, simultaneous `aseqdump` starvation, restart mapping survival,
and duplicate-name fail-closed remain.

## Native ALSA walkthrough 2026-09-02 (operator-driven)

Service `mackes-midi-matrix.service` `ActiveState=active`, `MainPID=1056166`, `User=mackes`,
`SupplementaryGroups=audio`, `health=ready`, `native_backend=alsa-seq`, `registered_inputs=7`.
USB `1235:0061` present with Eventide `1b12:003a` and MIDISPORT `0763:1021`. Mk2 ALSA client
`24:0` subscribed by daemon ingress `130:0`.

Device short-press: `Idle` → `AwaitControl`, Note Off 105 channel 8. Operator reported Device LED
lit. Top-left channel button: `AwaitControl` → `ChooseDevice`, Note Off 41, operator reported pad
LED lit. Four Right arrows (CC107) advanced `ChooseDevice` → `ChoosePreset` → `ChooseEffect` →
`ChooseType` → `ChooseParameter` (received 2 → 6). Fifth Right was received (received 7) and
left phase `ChooseParameter` with `sent=0` and no mapping; controller commit is still incomplete
on this installed daemon. Device hold did not cancel; last accidental pad event was Note Off 41.
A later Device hold produced Note Off 105 but phase stayed `ChooseParameter`. Host IPC `Cancel`
at assignment generation 6 returned `Idle`.

Second Learn: Device tap → `AwaitControl`; top-left knob CC13 → `ChooseDevice` (received 12 → 75);
operator reported knob LED lit.

USB reconnect of Mk2 only: USB device `087` → `088`, identity still `1235:0061`, ALSA client
still `24`. Learn remained `ChooseDevice`. Input `24:0` resubscribed to `130:0`. Daemon output
`131` did **not** reconnect to `24:0`. Snapshot `native_led_resync=false`. Operator reported knobs
lit after replug; that is not treated as daemon LED replay. Post-reconnect Right arrow advanced
`ChooseDevice` → `ChoosePreset`, CC107, received 110 → 111.

100 press/release pairs on Mute (Note 106): received 111 → 161 (25 pairs) then 161 → 311 (75
pairs). Exactly 200 Mute events, `dropped=0`, last event Note Off 106, phase stayed
`ChoosePreset`. This closes the 100-pair integrity step for this walkthrough.

Post-count Down arrow: received 311 → 312, CC105 channel 8, phase stayed `ChoosePreset` (cursor
unchanged at preset 0). Up arrow: received 312 → 313, CC104 channel 8. Left arrow: received
313 → 314, CC106 channel 8; phase stayed `ChoosePreset` (this installed daemon dispatched Left
as ordinary CC rather than Assignment Back). Solo: received 314 → 316, Note Off 107 channel 8.
Record Arm: received 316 → 318, Note Off 108 channel 8. Leftmost fader: received 318 → 572,
CC77 channel 8 including value 0. Operator reported no LED change on Solo, Record Arm, or the
fader; Factory 1 faders have no LED address, and this Learn session only lights Device plus the
already-captured control.

Rightmost-fader attempt: no new daemon event (`received` stayed 572, last activity still CC77).
Retry with a full travel: received 572 → 826, CC84 channel 8 value 0.

Bottom-left channel button first attempt: received 826 → 905 with last activity still CC84
value 79 (fader 8 still moving). Note 57 was not the last event. Two further pad attempts
produced no new daemon events (`received` stayed 1033, last activity CC13). Top-right knob:
received 1033 → 1097, CC20 channel 8 value 64. Operator then requested the faceplate sweep be cut to minimum; last sampled activity was CC36
channel 8 (`received` 1161). Remaining Factory 1 knob/pad cells are not required for this
walkthrough. Physical session stopped at operator `done`; no Eventide/Reflex write was sent.

Controller commit/LED result, daemon output resubscribe after USB reconnect, restart mapping
survival, duplicate-name fail-closed, and physical Reflex/Eventide send remain.

2026-09-02 — a clean `systemctl restart mackes-midi-matrix.service` produced a new active
daemon (`MainPID=1084815`) running as `mackes:mackes-control` with supplementary group `audio`.
After bounded startup, `/run/mackes-midi-matrix/control.sock` was recreated as
`mackes:mackes-control 0660`; the native subscription graph restored all seven source ports to
the single daemon ingress `130:0`, including `24:0` (Mk2), and daemon-owned output `131:0`
reconnected to `24:0`. This closes the clean-service-start and subscription-restoration portions
of W088 without claiming an operator control/LED outcome.

## LED contract (software; physical still open)

W091 software owner is the daemon LED surface. Runtime feedback uses the operator-facing
Factory Template 1 only. Its documented LED `SysEx` wire byte is `8` (Factory bank offset), while
the human-facing Factory slot remains `1`; these are separate values and must never be conflated.
Writes require exactly one Launch Control XL Mk2 MIDI output;
HUI endpoints are ignored; two MIDI endpoints fail closed with a snapshot `led.last_error`.
Base colors come from persisted mappings: unmapped OFF, Lexicon amber, Eventide red, other
owners green. Learn capture is yellow. Successful persist uses two 400 ms green pulses, then
the owner color; failure uses the matching red pulse sequence.

Faders have no individual LED address. Software policy: a mapped fader lights that column's
two channel-button LEDs as a proxy, unless a button in the column already has its own
assignment, in which case the button owner wins. Blue is not a Launch Control XL Mk2 color
and is rejected as `Unknown`. Physical OFF/yellow/amber/red/pulse/proxy qualification remains
open under W091/W092.
