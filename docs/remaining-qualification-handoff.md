# Remaining qualification handoff

Updated 2026-09-13. Local software, installed browser, release, asset, rollback, and governance
checks are current. The remaining work is deliberately limited to evidence that cannot be inferred
from source or browser fixtures.

| Packet | Remaining requirement | Required evidence | Current boundary |
| --- | --- | --- | --- |
| W111 | PiPedal operation-by-operation reply/readback coverage | Connected server traces for each promoted family, including error and reconnect replies | Typed connector and adapter boundaries exist; source-only tests do not prove live replies |
| W114 | EQ mapping/editor parity with live PiPedal | Physical/plugin target resolution, repair, apply, undo, and persisted reload trace | Local persistence and repair paths are implemented and tested |
| W138 | Exhaustive device-native PiPedal coverage | Live qualification for pending catalog families: presets/banks, files, system settings, routing, and plugin editing | Unsupported families remain visible; no iframe or catalog count is treated as implementation |
| W199 | Cross-device scene/preset coordination | Mixed-capability save/preview/recall trace with one device unavailable and authoritative recovery | Scenes now show readiness and unconfirmed device state; partial-failure browser fixture passes |
| W201 | Moderated human/native qualification | Reviewer records each checklist scenario with hardware/browser/viewport and screenshot or recording | Automated matrix passes; human sign-off remains intentionally blank |

## Automated prerequisites

```text
bash scripts/release-gate.sh
bash scripts/qualify-web-installed.sh http://172.20.222.222:8081
python3 scripts/check-installed-mod-art-assets.py http://172.20.222.222:8081
python3 scripts/check-studio-cutover.py
```

Do not close a packet from these prerequisites alone when its table row requires live or human
evidence. Record failed or unavailable scenarios as dispositions rather than converting them into
successful results.
