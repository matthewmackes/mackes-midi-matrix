# Novation controller qualification

Qualification uses the deterministic Novation test emulator for repeatable software and MIDI
transport checks. It does not claim visible hardware LED observation or physical USB behavior.

## Emulator gate

Command:

```text
scripts/qualify-novation-emulator.sh
```

Result on 2026-09-06: `PASS`.

| Check | Emulator evidence | Result |
| --- | --- | --- |
| Launch Control XL input/output identity | Bounded virtual MIDI pair and direction assertions | PASS |
| Controller input sequencing | Press/release sequence and NoteOn-zero release assertion | PASS |
| 48-index LED protocol | Factory-1 golden-byte and batch encoder tests | PASS |
| Template/reset diagnostics | Daemon LED-surface test suite | PASS |
| Deterministic bounded transport | Virtual endpoint capacity, 128-message mixed-traffic test, exact 64 sent/64 dropped counts, and queue drain | PASS |
| Visible seven-off / native yellow layout | Requires an actual controller faceplate | OPEN |
| USB reconnect cycles and latency | Requires physical USB detach/attach | OPEN |
| External Eventide/Lexicon/PiPedal response | Requires paired physical processors | OPEN |
| Thirty-minute mixed physical activity | Requires hardware soak telemetry | OPEN |

The emulator is the required pre-hardware qualification gate. Physical checks remain explicitly
open and must not be inferred from successful host writes.

## Native observation log

Operator-reported result: native Novation reconnect and LED observation passed on 2026-09-07.
Formal closure requires completing one row per reconnect cycle and retaining the corresponding
before/after snapshots; the report alone is not substituted for those records.

| Cycle | USB/ALSA identity before → after | Endpoint availability | Reconnect latency | Desired/sent/visible LED result | Drops/restarts | Operator |
| --- | --- | --- | --- | --- | --- | --- |
| 1 |  |  |  |  |  |  |
| 2 |  |  |  |  |  |  |
| 3 |  |  |  |  |  |  |
| 4 |  |  |  |  |  |  |
| 5 |  |  |  |  |  |  |
| 6 |  |  |  |  |  |  |
| 7 |  |  |  |  |  |  |
| 8 |  |  |  |  |  |  |
| 9 |  |  |  |  |  |  |
| 10 |  |  |  |  |  |  |
