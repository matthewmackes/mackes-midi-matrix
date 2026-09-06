# Installed-platform qualification matrix

This matrix is the execution record for W104. Software checks are repeatable and may be run
without changing the installed service; host and hardware rows require an operator-controlled
qualification window. A passing software row does not close a hardware row.

## Baseline

| Item | Observed value |
| --- | --- |
| Build | `mackes-midi-matrix-0.1.11-linux-x86_64.tar.gz` |
| Host | Fedora 44 realtime kernel, x86_64 |
| Controller | Launch Control XL `1235:0061`, MIDI and HUI ports |
| Pedal | Eventide MicroPitch `1b12:003a`, MIDI 1 |
| Interface | MIDISPORT 4x4 `0763:1021`, four logical MIDI ports |
| Service | daemon and TUI active; daemon user/group `mackes:mackes-control`; `NRestarts=0` |

## Execution rows

| ID | Scenario and measurable result | Evidence | State |
| --- | --- | --- | --- |
| S01 | Full release gate, without cargo-audit | `scripts/release-gate.sh` | PASS |
| S02 | Packaged systemd units verify with appliance drop-in | `scripts/verify-systemd-units.sh` | PASS |
| S03 | Installer preflight and no-mutation validation | `scripts/install-fedora.sh --check`, `scripts/installer-smoke.sh` | PASS |
| S04 | Hermetic routing, identity, recovery, and safety scenarios | 13 pass; one paired-RTP case explicitly ignored | PASS |
| S05 | Ten cold boots, with arbitrary attachment order | boot logs, readiness latency, snapshot | OPEN |
| S06 | Ten warm reboots, with no mixed configuration generation | boot logs, snapshot, backup manifest | OPEN |
| S07 | Twenty reconnect/move cycles per supported device | before/after identity and subscription snapshots | OPEN |
| S08 | Unplug while holding a button; 100 press/release pairs | raw input plus operator-observed toggle count | OPEN |
| S09 | Complete LED replay after reconnect, including visible confirmation | LED observation and daemon diagnostics | OPEN |
| S10 | Eventide receive-channel/polarity acknowledgement | pedal response recording, separate from host send count | OPEN |
| S11 | Interrupted save, disk-full/permission fault, and power-loss recovery | isolated fault harness and old/new generation proof | OPEN |
| S12 | Clean-host install, upgrade, injected failure, and rollback | isolated host logs and artifact hashes | OPEN |
| S13 | Eight-hour representative run: CPU, memory, logs, drops, duplicates | `scripts/capture-qualification-soak.sh` CSV plus logs | OPEN |

## Run protocol

1. Capture UTC timestamp, operator/host metadata, `lsusb`, `amidi -l`, `aconnect -l`, both service
   property sets, executable SHA-256 hashes, repository revision, `mackes status --json`, and a
   no-environment status snapshot with
   `scripts/capture-qualification-baseline.sh /absolute/output-directory`.
2. Run one scenario at a time and record UTC start/end times, build hash, and operator.
3. For every reconnect, record stable identity, volatile ALSA address, direction, logical port,
   subscription set, mapping count, dropped count, and LED diagnostics before and after.
4. Mark host delivery, pedal acknowledgement, and visible LED behavior as separate observations.
5. If a required row is unavailable, leave it `OPEN`; do not infer it from a unit test or send
   counter. Attach raw logs and snapshots to the row before changing its state.

For S13, run `scripts/capture-qualification-soak.sh /absolute/output-directory 28800 60`.
The sampler is read-only with respect to the appliance: it records bounded status snapshots,
daemon/console active state, CPU/RSS, service restart counts, bounded daemon journal line counts,
and an explicit
`status_ok` probe-result column without
restarting services or sending MIDI. A short
one-second smoke capture was exercised on 2026-09-06; this does not substitute for the full soak.
