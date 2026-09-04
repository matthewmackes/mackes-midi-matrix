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

2026-09-04 — after host access was restored, the active `mackes-midi-matrix.service` was
verified healthy with native ALSA ownership and the Launch Control stable output
`midir-out-f7060e7462e070c`. With the verified qualification record and explicit operator
authorization, exactly one documented Factory 1 LED index-0 OFF frame was sent through daemon
IPC (`generation=511`, `bytes_sent=11`). Post-write status remained `health=ready`,
`native_backend=alsa-seq`, `dropped=0`, and `native_led_resync=true`. Transport delivery is
qualified; physical visual appearance is not inferred.

2026-09-04 — the operator visibly confirmed Factory 1 LED index 0 illuminated solid yellow
after the daemon-owned frame `F0 00 20 29 02 11 78 08 00 33 F7`. This qualifies the yellow
encoding and current stable output path; the next bounded probe tests the amber owner state.

2026-09-04 — the operator observed the subsequent index-0 amber probe as orange/amber and the
following red probe as red. This qualifies the visible amber and red owner colors on the current
Factory 1 output path. A final documented OFF frame was sent afterward (`generation=520`,
`bytes_sent=11`) to leave the controller clean; blink timing, full-row coverage, and preset-load
projection remain separate qualification items.

2026-09-04 — the operator confirmed the final index-0 cleanup state is visibly OFF. The bounded
single-index OFF/yellow/amber/red appearance matrix is therefore complete for the current output
and Factory 1 template; multi-control coverage, result blink timing, reconnect, persistence, and
preset projection remain open.

2026-09-04 — after the bounded 48-index transport sweep, `mackes-midi-matrix.service` was
restarted and returned `active`; daemon status returned `health=ready`, `native_backend=alsa-seq`,
`registered_inputs=7`, `mappings=34`, `dropped=0`, and `native_led_resync=true`. All three physical
device groups remained connected with stable identities. This qualifies restart persistence and
rebind readiness; it does not claim visual replay without an operator observation.

2026-09-04 — the operator confirmed the red indicator visibly progressed through all six
eight-LED rows during the 48-index sweep. Every index was then returned to OFF. This closes the
physical Factory 1 LED address/order coverage for the current controller.

2026-09-04 — after operator USB reconnect, the controller returned as ALSA client 28 with the
same stable identity. Daemon ingress 130:0 was connected from the controller input and daemon
output 132:0 was connected back to the primary controller MIDI endpoint. Status returned
`health=ready`, `native_backend=alsa-seq`, 7 registered inputs, 34 mappings, and `dropped=0`.
The LED diagnostics recorded 287 failed replay attempts during recovery, so reconnect transport
and mapping persistence are observed but LED replay is not yet qualified.

2026-09-04 — after the ALSA graph settled, a documented Factory 1 index-0 OFF frame was sent to
the same stable Launch Control output and returned `ok=true`, generation 16, `bytes_sent=11`.
The LED failure counter remained unchanged at 287. This proves the recovered daemon output can
deliver again; the transient replay failures remain recorded for W091/W098 follow-up.

2026-09-04 — the qualified daemon-owned Reflex Port D path accepted a `Concert Wave` preset
projection (`generation=2`, 63-byte Rev.1 active-setup frame, documented checksum `2A`). A
subsequent profile query remained healthy and exposed the translated preset catalog. This
qualifies the bounded preset-send path; independent processor readback/parameter appearance is
still required before claiming full preset projection acceptance.

2026-09-04 — an independent documented active-setup query `F0 06 02 30 60 00 F7` was accepted
on Reflex Port D (`ok=true`, 7 bytes). The daemon then received one SysEx response from the
qualified Reflex return `midir-in-b7003db6a8f35324`; status remained `health=ready`, `sent=2`,
`received=1`, and `dropped=0`. This qualifies bidirectional query delivery after preset load;
the current CLI status projection does not expose response payload bytes for an exact frame diff.

2026-09-04 — captured the qualified Reflex Port D return directly with `aseqdump -p 32:3`
while issuing the active-setup query. The returned 63-byte frame began `F0 06 02 00 38`, named
`Concert Wave`, and ended with checksum `2A F7`; it matched the daemon’s sent preset frame
byte-for-byte. This closes the bounded `Concert Wave` preset projection and independent readback
qualification without issuing a persistent store operation.

2026-09-04 — sent the documented Eventide `ACTIVE/BYPASS` control through daemon IPC to the
qualified MicroPitch output `midir-out-1800a4817d1d17ee`: CC14, zero-based channel 6, value 0.
The daemon returned `ok=true`, generation 8, bytes `[181,14,0]`. This qualifies the reversible
daemon-owned transport path; pedal audio/state appearance remains an operator observation.

2026-09-04 — the operator observed the MicroPitch indicator light as green during the reversible
ACTIVE/BYPASS qualification. This confirms visible pedal indication was present; the semantic
meaning of the green state and its transition between values 0 and 127 remain to be confirmed.

2026-09-04 — after restoring documented ACTIVE/BYPASS value 0, the operator observed the
MicroPitch indicator as red. This establishes the observed baseline indication for value 0; the
earlier green observation was associated with the intervening toggle sequence and is not assigned
a semantic state without a controlled paired observation.

2026-09-03 — reinstalled the current release daemon from `target/release` through the governed
Fedora installer after detecting a stale installed artifact. The installed and target daemon
SHA-256 values now match (`2ca237ed2404cefb25eec6b7d6c3a3096f56e51daf330d299c18087560710f1f`).
The restarted service is active as `mackes:mackes-control`; status exposes the daemon-owned Learn
catalog, Eventide and Reflex profiles, and the exact configured Reflex Port D destination.
The installer retained a configuration backup under
`/var/lib/mackes-midi-matrix/config-backups/20260903T212003Z`.

2026-09-03 — materialized the requested Eventide layout through daemon IPC: 14 documented
non-Mix/non-bypass controls are assigned to knob rows 2–3, master Mix is assigned only to Slider 1,
and ACTIVE/BYPASS is assigned to Slider 1 Button 1. Slider 1 Button 2 has no mapping because the
Eventide protocol documents no independent Delay-bypass command. Live status reports 16 Eventide
mappings and LED delivery with 408 sent / 0 failed frames; physical color appearance remains to be
confirmed.

2026-09-03 — a fresh observation-only inventory still finds Eventide MicroPitch (`1b12:003a`),
Launch Control XL Mk2 (`1235:0061`), and runtime MIDISPORT 4x4 (`0763:1021`). The Mk2 exposes
its normal ALSA MIDI and HUI endpoints, the MIDISPORT exposes four MIDI ports, and both `amidi`
and `aconnect` are available. This confirms qualification-host readiness only; no MIDI, SysEx,
LED, or physical-control result is inferred from the inventory.

2026-09-03 — after the daemon reinstall, an operator-driven Mk2 button workflow completed
successfully: Device entered Learn, top-left channel button captured as Factory 1 channel 8/note
41, Reflex `Circular Reverbs` was selected, and the source replacement committed atomically to the
daemon-owned Port D destination. The duplicate `Concert Wave` attempt had previously failed closed
with zero sends; the successful terminal state reported `Succeeded`, 7 received events, and 0
dropped events. This qualifies one post-reinstall button/preset commit; a full 56-control
walkthrough is explicitly out of scope. Remaining targets are the requested Eventide controls:
all 16 documented controls on knob rows 2–3, Slider 1 master Mix, Slider 1 Button 1 for
ACTIVE/BYPASS, and Slider 1 Button 2 documented as unsupported for independent Delay bypass.
LED/preset-projection qualification for that selected set remains open.

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

2026-09-02 — after installing `4e7791a`, the local CLI queried the daemon-owned Eventide profile
and transmitted the documented, reversible Expression Pedal operation to exact endpoint
`midir-out-c0d934e6c08c6a1a`. Daemon accepted `[176,4,64]` (channel 1, CC4, value 64), incremented
`sent` to 1, and recorded an allowed `local-ipc` audit entry. The initial rejected shorthand
label exposed and then verified the IPC payload-boundary correction; no client MIDI port was opened.

2026-09-02 — current installed-service observation: `ActiveState=active`, `User=mackes`,
`Group=mackes-control`, `health=ready`, `native_backend=alsa-seq`, and `registered_inputs=7`.
The Mk2 client `24:0` is connected to daemon ingress `130:0`, and daemon output `131:0` is
connected back to `24:0`; HUI output `132:0` remains separately registered. LED diagnostics report
`attempted=96`, `sent=96`, `failed=0`, with no last error and no pending deadline. This confirms
the post-restart output subscription is currently restored; operator-visible LED color/pulse
qualification remains open.

2026-09-02 — `scripts/qualify-hardware.sh` observation-only run: USB `1235:0061` Launch
Control XL, `1b12:003a` MicroPitch, and `0763:1021` MidiSport 4x4 are present; ALSA exposes
`midiC2D0`, `midiC3D0`, and `midiC4D0`; the endpoint query completed; `amidi` and `aconnect`
are installed; all four MidiSport MIDI ports were enumerated (`midisport_4x4_acceptance=pass`).
The script intentionally performs no writes, so LED appearance and preset/SysEx qualification
remain pending.

2026-09-03 — corrected the Reflex Port A destination identity from the input-side stable ID
`midir-in-1c86ec3dd492b1fc` to the independently derived daemon-owned output ID
`midir-out-1c86ed3dd492b3af`. The exact documented active-setup request
`F0 06 02 30 60 00 F7` was accepted (`ok=true`, seven bytes), and a paced sweep of the same
read-only request across all sixteen Reflex device channels was also accepted. Daemon status
remained `health=ready`, `dropped=0`, advanced `sent` from 0 to 17, and retained 17 allowed
`local-ipc` SysEx audit entries for Port A. `received` remained 6 and no Port A SysEx reply was
observed, so output ownership/registration and channel selection are proven while the physical
Reflex OUT-to-Port-A-IN return path, Reflex SysEx receive setting, or device power/state remains
the active qualification fault. No state-changing Reflex command was sent.

2026-09-03 — with explicit operator authorization, installed a narrowly scoped daemon-owned
Reflex System Reset path and sent the manual-supported one-byte MIDI Reset (`FF`) exactly once to
Port A. IPC returned `ok=true` with bytes `[255]`; the daemon audit records
`device-control:lexicon.reflex:system-reset`. After a three-second settle, all sixteen read-only
active-setup channel queries again transmitted successfully. The restarted daemon remained
`health=ready`, `dropped=0`, but `received=0`; reset therefore did not restore the return path.
The next discriminating test is front-panel parameter activity: the Reflex should emit a type-2
message on MIDI OUT when a parameter encoder changes. If that also produces no Port A input,
inspect/reseat the Reflex MIDI OUT → MIDISPORT A IN cable and the Reflex MIDI/SysEx settings.

2026-09-03 — front-panel activity identified the actual Reflex return as MIDISPORT Port D,
`midir-in-b7003db6a8f35324`. Read-only active-setup probes against Ports A–C produced no wire
reply; Port D output `midir-out-b7003eb6a8f354d7` immediately returned a valid 63-byte Rev.1
type-0 active setup beginning `F0 06 02 00 38` and ending with checksum byte `1A F7`. A separate
type-5 parameter-0 query returned `F0 06 02 50 00 09 04 00 00 F7` (`0x9400`) for the active
Reverb algorithm. The authorized reversible live test changed parameter 0 by one documented step
to `0x9800`, observed the matching `09 08 00 00` response, restored `0x9400`, and an independent
final query returned `09 04 00 00` again. No register-store task or persistent setup load was
sent. Final daemon state: `health=ready`, `received=58`, `sent=74`, `dropped=0`, with latest
activity a SysEx event from Port D. This qualifies bidirectional Reflex transport, request/reply,
one bounded parameter edit, and exact restoration on MIDISPORT Port D.

2026-09-02 — with explicit operator authorization and verified map record, a bounded reversible
Factory 1 LED OFF frame (`F0 00 20 29 02 11 78 08 00 F7`) was transmitted once through the
daemon to exact output `midir-out-8cb77e53a765b904`. IPC returned `ok=true`, `generation=4`,
`bytes_sent=10`; stale-binary and mistyped-destination attempts were rejected without sending.
The operator must still confirm the physical LED appearance for this frame.

2026-09-02 — a second bounded probe sent solid yellow at Factory 1 LED index 0 using
`F0 00 20 29 02 11 78 08 00 33 F7` to the same exact output. IPC returned `ok=true`,
`generation=6`, `bytes_sent=11`. Operator visibly confirmed LED index 0 illuminated yellow.

2026-09-02 — the clean-state OFF frame was repeated on LED index 0 using
`F0 00 20 29 02 11 78 08 00 00 F7`; IPC returned `ok=true`, `generation=7`, `bytes_sent=11`.
Operator visibly confirmed LED index 0 turned OFF.

Row-1 red chase: index 0 frame (`... 08 00 03 F7`) and index 1 frame
(`... 08 01 03 F7`) were accepted by the daemon (`generation=8` and `9`). The first two
positions illuminated red. The attempted index-2 command at generation 10 accidentally omitted
the Factory-template byte `08` (`... 78 02 03 F7`, 10 bytes), so the third position correctly did
not respond; it is not a hardware failure. Index 3 red returned `ok=true`, `generation=11`,
`bytes_sent=11`, and the fourth physical position illuminated. Index 4 red returned `ok=true`,
`generation=12`, `bytes_sent=11`, and the fifth physical position illuminated.

Protocol index 5 red (`F0 00 20 29 02 11 78 08 05 03 F7`) returned `ok=true`, `generation=13`,
`bytes_sent=11`; operator confirmed the sixth physical position illuminated. Physical indexing
is being recorded separately from zero-based protocol indices.

After the unresolved index-3 yellow probe, protocol index 3 was restored to OFF with
`F0 00 20 29 02 11 78 08 03 00 F7`; IPC returned `ok=true`, `generation=18`, `bytes_sent=11`.

Protocol index 6 red (`F0 00 20 29 02 11 78 08 06 03 F7`) returned `ok=true`,
`generation=14`, `bytes_sent=11`; the seventh physical position illuminated. The attempted
index-7 command at generation 15 also omitted the Factory-template byte `08`
(`... 78 07 03 F7`, 10 bytes), so the eighth position correctly did not respond; it is not a
hardware failure. Protocol index 6 was later resent (`generation=16`, `bytes_sent=11`) without
changing that conclusion.

Corrected protocol index 2 red (`F0 00 20 29 02 11 78 08 02 03 F7`) returned `ok=true`,
`generation=19`, `bytes_sent=11`; operator confirmed the third physical position illuminated.
The operator corrected the earlier report: the unresolved positions are the fourth and eighth,
not the third and eighth. Corrected protocol index 7 red
(`F0 00 20 29 02 11 78 08 07 03 F7`) returned `ok=true`, `generation=20`, `bytes_sent=11`;
operator confirmed the eighth physical position illuminated red.

At operator request, a bounded red Knight Rider sweep then addressed all six documented
eight-LED rows (indices 0–47), one LED at a time. Each index received one red frame, remained on
for 180 ms, and received one OFF frame before the next index; 96 daemon-owned writes completed
without a command failure. Operator observation of row order/completeness is pending.

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

After five minutes without a Factory 1 controller event, the daemon enters the standard idle
display: one red LED advances left-to-right across indices 0–47 at 600 ms per step. Any controller
event wakes the surface immediately and restores the persisted mapping/owner colors. Assignment
sessions suppress the idle display.

2026-09-03 — preset-load qualification on the now-qualified Reflex Port D first sent the
active-only translated `Concert Wave` type-0 setup. An independent query still returned the prior
`LongVerb` setup, so that translation was correctly treated as unverified. A documented
nonpersistent task-71 recall of register 9 then produced a parameter-64 setup-selection event for
9 and an independent active-state readback naming `DrumPlat` (algorithm 2 / Plate). No task-70
store or register-writing frame was sent.

2026-09-03 — corrected the translated-preset defect: normalized PCM70 targets had been emitted as
arbitrary in-range 16-bit values rather than snapped to each Reflex parameter's documented wire
step. `Concert Wave`, for example, contained `0xB3F7`; the corrected value is `0xB400`. After
installing the step-quantized encoder, the daemon sent the corrected 63-byte active-only frame on
Port D and an independent active-state query returned that frame byte-for-byte, including the
`Concert Wave` name and checksum `2A`. This supersedes the failed translated-load observation
above. No task-70 store or persistent register write was issued.

2026-09-03 — added the persistent generic-interface binding contract and configured endpoint alias
`reflex-port-d` (`MidiSport 4x4 MIDI 4`) for profile `lexicon.reflex`. On clean daemon restart,
the authoritative Learn catalog contains both `lexicon.reflex` and `eventide.micropitch`, and the
Reflex catalog destination is the exact qualified output `midir-out-b7003eb6a8f354d7`. Profile
selection recomputes the stable destination, with a regression proving Eventide selection cannot
retain Reflex Port D. A 45-second physical capture window received no controller input, so the
button-driven preset walkthrough remains pending operator action rather than being claimed.

The full release gate subsequently passed with this binding and the physically verified translated
preset fix: repository and architecture policy, all workspace tests, strict Clippy, benchmark,
hermetic integration, installer smoke, archive checksum, and preflight.

The documented rollback was then executed and reversed. The retained prior 0.1.11 daemon and
pre-binding configuration booted healthy with native ALSA, seven inputs, and one mapping. The
current release binary and saved configuration were restored and booted healthy with seven inputs,
both mappings, both authoritative profiles, and Reflex Port D selected. The installed daemon hash
matches the current release build (`3c0bba06…85029c3a7`); no processor state was written during
rollback.

2026-09-03 — completed an operator-driven Learn/preset block on the installed release. The
captured Mk2 control was `button-r2-c1` (channel 8, note 57); Forward navigation reached the
Reflex preset catalog; Down plus Forward committed `Concert Wave` to qualified Reflex Port D.
The daemon status reached `Succeeded`, persisted `pcm70_reflex:concert-wave`, and reported zero
dropped input events. The operator observed two green confirmation blinks followed by yellow.
Forward navigation works, but the Mk2 arrow-key LED itself did not illuminate; this remains an
explicit physical appearance limitation and is not claimed as passed LED qualification.

2026-09-03 — follow-up preset-load attempts after restart received a channel-8 note-73
press/release, while the persisted qualified block is channel-8 note 57 (`button-r2-c1`). No
mapping dispatch or Reflex write occurred (`sent=0`, `audit_count=0`). This is retained as a
layout/physical-position discrepancy for W093; the daemon correctly failed closed rather than
guessing a control or rewriting the mapping.

Track Control Button 1 was then isolated as channel-8 note 73 (velocity 127 on press, matching
release observed). This confirms the connected controller is presenting a non-Factory-1 tuple for
that physical control; the value is retained as raw qualification evidence pending the W093
template decision.

2026-09-03 — daemon-owned template selection frame `F0 00 20 29 02 11 77 01 F7` was sent and
audited (`ok=true`, 9 bytes). The subsequent Track Control Button 1 press/release arrived as
**MIDI channel 1, note 41**, confirming Factory 1 but exposing a channel-convention mismatch with
the frozen channel-8 contract. Existing channel-8 mappings therefore remain safely inactive until
the layout contract is reconciled.

Track Control Button 2 was then isolated as channel-8 note 74 (press/release, zero dropped
events), establishing the consecutive observed Button 1/2 tuple `73/74` for the pending layout
reconciliation.

Track Control Button 3 is channel-8 note 75 (press/release, zero drops), extending the observed
Track Control sequence to notes `73/74/75`.

Track Control Button 4 press is channel-8 note 76 (velocity 127, zero drops), extending the
observed sequence to `73/74/75/76`.

Track Control Button 5 produced channel-8 note 89 (velocity 127 on press; zero dropped events).
Because this differs from both the Factory-1 channel-button range and the observed 73–76 bank,
its page/bank identity remains unresolved under W093; no mapping alias was inferred.

Track Control Button 6 is channel-8 note 90 (press/release, zero dropped events), pairing with
Button 5's note 89 in the second observed bank.

Track Control Button 7 is channel-8 note 91 (press/release, zero dropped events), extending the
second observed bank to notes `89/90/91`.

Track Control Button 8 is channel-8 note 92 (press/release, zero dropped events), completing the
observed second bank `89/90/91/92`.

The Slider 1 upper button was then confirmed as channel-8 note 41 (press/release, zero dropped
events), matching the Factory-1 top channel-button tuple for column 1.

Slider 2 upper button is channel-8 note 42 (press/release, zero dropped events), continuing the
Factory-1 top channel-button sequence.

After Factory 1 was pushed, Slider 3 upper button was confirmed as channel-1 note 43
(press/release, zero dropped events), demonstrating the active Factory-1 channel convention.

The daemon now automatically reselects Factory 1 on every detected Launch Control reconnect,
using the documented 9-byte frame before LED replay. A live restart exercised this path with
`health=ready` and zero drops; the board's Track Control Button 1 event remained channel 1/note 41,
so source-channel reconciliation is still required.

Slider 4 upper button is channel-1 note 44 (press/release, zero dropped events), continuing the
Factory-1 top-button sequence.

The operator-labeled Slider 5 upper button produced channel-1 note 57 (press/release, zero
dropped events), rather than the expected sequential note 45. This physical-label/bank-position
discrepancy is retained as raw W093 evidence; no stable ID was inferred.

Slider 6 upper produced channel-1 note 58 (press/release, zero dropped events), continuing the
same observed bank as Slider 5 upper.

Slider 7 upper produced channel-1 note 59 (press/release, zero dropped events), continuing the
observed bank.

Slider 8 upper produced channel-1 note 60 (press/release, zero dropped events), completing the
observed `57/58/59/60` upper-button bank.

Slider 1 lower produced channel-1 note 73 (press/release, zero dropped events), confirming the
Factory-1 lower-button bank begins at note 73 on channel 1.

2026-09-03 — observation-only host recheck found the Mk2 (`1235:0061`), MicroPitch (`1b12:003a`),
and MidiSport 4x4 (`0763:1021`) present. Native ALSA exposed four MidiSport ports, and the
daemon endpoint inventory included the Mk2 input/output plus all four MidiSport ports. `amidi`
and `aconnect` were available. No hardware write was performed; this advances environment
readiness only and does not qualify layout, LED appearance, or preset projection.
