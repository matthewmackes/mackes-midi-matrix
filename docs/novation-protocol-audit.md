# Novation protocol and incident audit

Audit date: 2026-09-06. This document distinguishes observation from inference; successful
host writes are not treated as visible hardware acknowledgement.

## Protocol authority

| Field | Evidence |
| --- | --- |
| Source | [Launch Control XL Programmer's Reference Guide](https://fael-downloads-prod.focusrite.com/customer/prod/s3fs-public/downloads/launch-control-xl-programmers-reference-guide.pdf) |
| Retrieved | 2026-09-06 |
| Retrieved PDF SHA-256 | `98b6183c4d03fcf7b64a8db2f33f50f63ab192af689613a3b633372d814be0fe` |
| Relevant pages | 3–9; sections 3–4 |
| Device identity observed | USB `1235:0061`, Launch Control XL Mk2 |
| Port evidence | Native ALSA input/output plus separate HUI port, client `24` in host qualification |

## Build and writer provenance

Current release-profile hashes:

```text
target/release/mackes-midi-matrix   eca69d8a03d7a2bd0be81354b75653f0d55db2b4a191084bd8038046163f8bb8
target/release/mackes-midi-matrixd  dbe6be0fc3bdf85e0be6e57918d9ce6c85ff21b74fbb6267fccf473e2ba3ca46
target/release/mackes-web           5b2ea360ccde6bf0e2cc491e5c5df02621889c8c98c2594e423b903b2c9bfef6
```

The packaged daemon writer is `mackes.service`; the packaged browser process is
`mackes-web.service`. Native MIDI output writes are daemon-owned through the output registry,
profile bindings, and `LedSurface`; HUI is classified and rejected as an LED owner. CLI/TUI and
web paths submit IPC commands and do not open independent controller writers. The historical
interrupted-build deployment state is not reproducible from the current host, so that portion is
recorded as UNKNOWN rather than inferred resolved.

## Sanitized protocol inventory

| Operation | Observation | Inference / unknown | Source |
| --- | --- | --- | --- |
| Background LED address | `F0 00 20 29 02 11 78 Template Index Value F7` | Visible state still requires faceplate observation | Guide pp. 3–4 |
| LED geometry | Indices 0–47 map to four 8-control rows plus utility controls | Fader proxy behavior is platform policy | Guide pp. 3–4 |
| Template selection | Template byte is bounded to 0–15 by the encoder contract | Current selected template is not independently read back | Guide pp. 5–6 |
| Reset | Template-scoped reset is encoded as bounded MIDI reset data | Firmware buffer/reset semantics are not read back | Guide pp. 6–7 |
| Input layout | Factory-1 profile maps knobs, faders, buttons, and utility controls to stable IDs | A different controller template must fail closed | Host captures; profile contract |
| HUI separation | HUI is observed as a distinct endpoint role | HUI is not evidence of a second physical unit | Host qualification |

## Incident observations

| Interval | Observation | Inference / unknown |
| --- | --- | --- |
| Idle | Daemon remains ready; LED send counters can be successful | No claim about visible LED state |
| Knob movement | Host captures include Factory-1 control tuples and assignment phase transitions | Processor response and LED pickup remain hardware checks |
| Reconnect | USB identity remained `1235:0061`; input resubscription was observed | Output replay and visible restoration require physical confirmation |
| Queue starvation | Software emitter bounds work and preserves ordered pending indices | Historical all-lit cause is not reclassified without a fresh physical trace |

The emulator gate is recorded separately in `docs/novation-controller-qualification.md`. Physical
faceplate, reconnect-latency, paired-processor, and soak checks remain OPEN under W126.
