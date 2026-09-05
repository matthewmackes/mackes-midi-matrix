# Eventide mapping repair — 2026-09-05

Updated the installed configuration at `/etc/mackes-midi-matrix/config.json5`. All 16 Eventide records now use current Novation input `midir-in-96f7bf329cb24e03` and destination channel 5 (wire MIDI channel 6), matching the existing bypass mapping (the pedal receive channel is unconfirmed). The prior knob/fader destination channel was 6 (wire channel 7). Existing physical assignments and enable states were retained. Unrelated mappings were compared before and after restart and are unchanged.

Backup: `/etc/mackes-midi-matrix/config.json5.eventide-repair-20260905-175459`.

Validation: installed CLI accepts the configuration; restarted daemon reports ready, zero dropped events, no native failure; all 16 loaded Eventide bindings match the repair. No enabled input tuples collide. Six Eventide daemon regression tests pass (`cargo test -p mackesd eventide -- --nocapture`); these are software checks, not proof of physical pedal response.

| Physical control | Eventide parameter | Enabled |
| --- | --- | --- |
| knob-r2-c2 | Expression Pedal | False |
| knob-r2-c3 | TAP TEMPO | False |
| knob-r3-c1 | FLEX | False |
| knob-r1-c1 | Pitch A | True |
| knob-r1-c2 | Pitch B | True |
| knob-r3-c2 | Depth | True |
| knob-r3-c1 | Rate/Sens | True |
| fader-2 | Pitch Mix | True |
| knob-r3-c3 | Tone | True |
| knob-r2-c1 | Delay A | True |
| knob-r2-c2 | Delay B | True |
| fader-3 | Mod | True |
| knob-r2-c3 | Feedback | True |
| knob-r3-c4 | Out Lvl | False |
| fader-1 | Mix | True |
| button-r1-c1 | ACTIVE/BYPASS | True |

Physical verification initially remained open because no mapped controller event had arrived at the post-repair audit.

## Physical failure reported after repair

The operator initially reported that the Novation LED changed but Eventide did not respond. Live status confirmed channel-9 note 41 input and a mapping send of `[181,14,0]` (wire channel 6, CC14, bypass), with zero dropped events. LED status was `delivered_unconfirmed`. The official Eventide quick reference identifies CC14 >=64 as active and <64 as bypass, with a configurable receive channel and factory default Omni.

The MicroPitch was then factory-reset, direct CC14 tests were sent through the daemon and raw ALSA USB output, and all MIDI channels were exercised to remove uncertainty about its receive-channel setting. The operator subsequently confirmed that the issue appears solved. A final runtime audit reports the MicroPitch connected, all 16 Eventide mappings loaded, daemon health `ready`, zero dropped messages, and no native backend failure.
