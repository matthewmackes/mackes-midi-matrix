# Native qualification session — 2026-09-13

This session is prepared for the installed host `http://172.20.222.222:8081`.
The inventory and software gates pass; the fields below require a human observer and must not be
filled from source, endpoint presence, or transport acceptance.

## Before starting

```text
bash scripts/qualify-hardware.sh
systemctl is-active mackes-midi-matrix.service mackes-web.service
/usr/local/bin/mackes-midi-matrix status --json
```

Record the hardware serials, firmware, browser, viewport, and timestamp here:

```text
observer=
hardware_serials=
firmware=
browser_viewport=
started_utc=
```

Step 1 result: PASS for physical input transport and stable-ID resolution. A 15-second capture
observed Launch Control XL ALSA `24:0`, channel 0 / CC13, values `1 → 127 → 64`; the operator
identified the control as R1C1, and daemon status resolved the latest activity to `knob-r1-c1`
(`received=254`). This does not qualify an output LED or destination readback.

## Reversible observation sequence

1. Open `/studio`, confirm the Launch Control XL identity and move one knob. Record the stable
   control ID, visible value, and whether the update is timely.
2. Start the named Learn flow, press Device, move one knob, and record the captured control. Do
   not commit a mapping unless the displayed destination and physical control are verified.
3. Run the existing reversible LED probe only after an approved device-specific map record passes
   `scripts/physical-write-guard.sh`; record the visible color and cleanup state.
4. Disconnect and reconnect one device, then record stale state, recovery time, mapping retention,
   and any replay failures. Do not convert transport delivery into visual confirmation.
5. For PiPedal, record one advertised control’s before value, requested value, server readback,
   persisted reload value, and any error/reconnect response. Repeat only for operations the
   connected server explicitly advertises.

## Evidence

| Scenario | Result | Evidence path / notes |
| --- | --- | --- |
| Controller identity and physical input | OPEN | |
| Learn capture and stable ID | OPEN | |
| LED visible color and cleanup | OPEN | |
| Device disconnect/reconnect | OPEN | |
| PiPedal control readback and reload | OPEN | |
| Scene recall with mixed device availability | OPEN | |
| Human accessibility/walkthrough | OPEN | |

A reviewer may mark a row PASS only with a direct observation or recording. Failed or unavailable
rows remain OPEN with a disposition. This record is intentionally not a substitute for evidence.
