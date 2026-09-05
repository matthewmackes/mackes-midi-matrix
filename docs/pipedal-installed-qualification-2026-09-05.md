# PiPedal installed qualification evidence — 2026-09-05

This is read-only evidence for W112. It does not change PiPedal or MACKES mappings.

PiPedal runs as `pipedald.service` (PID observed 41454) and listens on port 8080. ALSA
exposes `PiPedal:in` at `128:0` and `Device Monitor:PiPedal:portMonitor` at `130:0`.
The installed `SystemMidiBindings.json` is a JSON array with fields `symbol`, `bindingType`,
`note`, `control`, `minControlValue`, `maxControlValue`, `rotaryScale`, `minValue`,
`maxValue`, `linearControlType`, and `switchControlType`. Existing system bindings are
`prevBank`, `nextBank`, `prevProgram`, and `nextProgram` on notes 41–44.

The active `Default+Bank.bank` contains these EQ plugin targets:

| Plugin URI | Observed controls |
|---|---|
| `http://two-play.com/plugins/toob-parametric-eq` | `gain`, `lfC`, `lfLevel`, `lmfC`, `lmfLevel`, `lmfQ`, `hmfC`, `hmfLevel`, `hmfQ`, `hfC`, `hfLevel`, `loCut`, `hiCut` |
| `http://two-play.com/plugins/toob-parametric-eq-stereo` | Same parametric controls as mono, plus stereo I/O controls in some presets |
| `http://two-play.com/plugins/toob-three-band-eq` | `bass`, `mid`, `treble`, `gain` |
| `http://two-play.com/plugins/toob-three-band-eq-stereo` | `bass`, `mid`, `treble`, `gain` |

Observed presets contain multiple instances and repeated plugin URIs. Instance IDs are
runtime values and must not be used as reusable mapping identity. No EQ MIDI bindings were
present in the inspected presets. The five requested R3C4–R3C8 targets therefore require
explicit plugin-instance selection and parameter selection; they cannot be inferred as
generic “five EQ bands”.

The service journal also reports previous crash recovery. W115 must test reconnect and
recovery without replaying stale controls. W112 remains open until the connector protocol
revision and exact WebSocket operations are pinned against this installed PiPedal build.

Read-only strings inspection of `/usr/sbin/pipedald` exposed operation/event candidates:
`setControl`, `setSelectedPedalboardPlugin`, `setPedalboardItemEnable`,
`setPedalboardItemUseModUi`, `updateCurrentPedalboard`, `setSnapshot`, `setSnapshots`,
`setSystemMidiBindings`, `getSystemMidiBindings`, `setPedalboardItemTitle`,
`onSelectedSnapshotChanged`, `onSystemMidiBindingsChanged`, `onSnapshotModified`, and
`onPedalboardChanged`. These names are discovery evidence only; W112 must capture their
JSON envelopes and semantics from the matching client/server source before implementation.
