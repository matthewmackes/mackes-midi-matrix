# Studio browser qualification record — 2026-09-12

Installed target: `http://172.20.222.222:8081/` (`mackes-web.service`). The service is enabled and
active. The root route and `/studio` deep link serve the same clean-sheet Studio controller.

## Automated evidence

| Check | Result |
| --- | --- |
| `scripts/release-gate.sh` | `PASS` |
| `scripts/browser-root-studio-smoke.py` | Root and `/studio`, 56 controls |
| `scripts/browser-smoke.sh` | Studio-backed page inventory and installed asset hashes |
| `scripts/browser-studio-console-smoke.py` | No `SEVERE` console entries |
| `scripts/browser-studio-accessibility-tree-smoke.py` | 56 named controls, 44 px targets |
| `scripts/browser-studio-latency-smoke.py` | Selection p95 4.20 ms; feedback 6.00 ms |
| `scripts/browser-studio-resilience-smoke.py` | Refresh failure, stale values, late events |
| `scripts/browser-studio-capability-view-smoke.py` | Ready/readback counts and protocol hiding |
| `scripts/browser-studio-scenes-actions-smoke.py` | Save and confirmation-gated recall |
| `scripts/browser-studio-starter-apply-smoke.py` | One atomic reviewed batch Apply |
| `scripts/browser-studio-draft-reload-smoke.py` | Draft restored after reload without mutation |
| `scripts/browser-studio-undo-smoke.py` | Typed generation-checked Undo |
| `scripts/check-active-carbon.py` | Active Studio sources contain no Carbon dependency or token |
| `scripts/qualify-web-installed.sh` | Expanded installed Studio suite passes, including Apply, Undo, Panic, device truth, and event fixtures |
| `scripts/capture-qualification-soak.sh` | 12-second installed soak: 0 health failures, drops, or restarts |

## Installed cutover

`python3 scripts/check-studio-cutover.py` passes with executable rollback artifact checksum
`14074731612a191606e0bc220dc8a45ae1b6dde244da261632d63905fc942908`. Compatibility page paths serve
the Studio shell; API routes remain unchanged. Existing configuration backups are retained under
`/var/lib/mackes-midi-matrix/config-backups/`.

The current worktree `target/release/mackes-web` and the running installed
`/usr/local/libexec/mackes-midi-matrix/mackes-web` now match at SHA-256
`183c1922a7cc682f5fd6f1f7f6655d314c6292c3989c679a58ba24fa9daedb47`. The release was installed at
`2026-09-12T16:01:43Z` with configuration backup `20260912T160143Z`; both `/` and `/studio` return
HTTP 200 and serve the Studio shell.

## Native observation baseline

`bash scripts/qualify-hardware.sh` was run in observation-only mode on host `NAM-MIDI`. The
captured inventory is recorded in [docs/hardware-qualification-2026-09-12.txt](hardware-qualification-2026-09-12.txt).
It confirms connected Eventide MicroPitch, Novation Launch Control XL, and M-Audio MidiSport 4x4
hardware, four MidiSport MIDI ports, and the expected PiPedal/application endpoints. This evidence
does not authorize or claim physical writes, LED delivery, or device readback.

## Remaining qualification

Native controller LED delivery, physical pickup timing, device readback, reconnect on real hardware,
and moderated human sign-off remain explicitly open. Automated fixtures and emulators do not close
those requirements by inference.
