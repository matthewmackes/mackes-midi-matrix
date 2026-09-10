# Worklist execution board

Updated 2026-09-10. This board is subordinate to `WORKLIST.md`; it records the next executable
step for the active Web Interface drain packets and must be updated with command output, not intent.

Latest automated checkpoint (2026-09-10): `cargo test -p mackes-web` passed 56/56; graphical
interface guard, web asset budget (38,643 compressed bytes), worklist validation, JavaScript syntax,
and `git diff --check` all pass. Installed browser smoke passed against
`http://172.20.222.222:8081` after release rebuild/reinstall, including deep links, 320/768 responsive
captures, light theme, GUI surface checks, and installed asset hashes. current worklist state: 147 complete, 14 in progress, 0 not started (161 total; measured 2026-09-10).
The installed graphical-inventory, renderer-registry, and novice-surface fixtures pass (32 live device
cards, 7 researched feature cards, all nine governed renderer keys, no visible code editor); generation ordering and visual draft-preservation fixtures also
pass. The PiPedal catalog fixture now passes after the bounded 265-entry decoder fix, reporting 3,076
controls and 265 targets from PiPedal v2.0.110.
The PiPedal generation boundary was corrected and reinstalled; snapshot and connector generations now
match, and the approved typed `loadPreset(16)` request was accepted/queued. The generation fixture now
asserts the Devices workspace's applied-generation marker, avoiding interference from an independent
PiPedal subrefresh. The final installed aggregate passed all 17 fixtures, including visual
accessibility, lossless-route, and task-ownership checks. This includes light theme, reduced motion,
200% zoom, 56 non-color-labeled controls, and the race-hardened assignment inspector.
The visual review pass also confirms the installed signal-flow canvas keeps 32 devices distinct in a
scrollable layout and the normal Devices/Routing text avoids internal URIs and runtime identities.
The renderer registry now also supplies an accessible summary list for every family and fallback;
the rebuilt installed host passed renderer, graphical-inventory, and visual-accessibility fixtures.
The formal `scripts/release-gate.sh` also passes end to end, including workspace tests, emulator,
clippy, benchmark, hermetic integration, installer smoke, and release artifact checksum/contents.
The installed baseline and bounded service soak also pass: four samples, zero status failures, zero
dropped events, zero restarts, and four MIDISPORT ports observed on `NAM-MIDI`.
W177 is therefore in progress with software release evidence recorded; W176 novice/human sign-off
and final release sign-off remain open.
The existing focused
fixtures and post-fixture health soak remain valid; these checks do not constitute human visual
sign-off or native hardware observation.

| Packet | Current evidence | Next executable step | Exit evidence | External dependency |
|---|---|---|---|---|
| W159 | Navigation, health, feature-catalog, renderer, and state-store modules extracted; cancellation, stale-generation, abort, hidden-tab, draft, focus, scroll, inventory, assignment, route-guard, and task fixtures pass independently; aggregate remains long-run sensitive | Continue component/state boundaries and stabilize full aggregate orchestration with bounded recovery | Component tests plus bundle/source provenance and full gate | Browser harness choice must honor offline/no-CDN policy |
| W160 | DONE: read coalescing/single-flight deployed; post-daemon 1,000/1,000 health soak and 250/250 feature-isolation soak pass; broken-pipe logging repaired; ADR-0014 records root cause | No further W160 software action; retain soak evidence | Evidence and closure record in `WORKLIST.md`/ADR-0014 | Native hardware qualification remains separate |
| W161 | DONE: pinned profile/emulator tests, 56-control SVG/mapping join, truthful readback, accessibility, responsive/theme, pointer/keyboard, and out-of-order generation evidence pass | No further W161 software action; retain native observation separately | Evidence in W161 and browser qualification record | Native LED/readback observation remains separate |
| W162 | Compact labeled action groups and divider rules deployed | Build control-to-route inventory and split configuration/recovery/hardware actions into focused deep-linked views | Zero omitted/duplicated actions plus keyboard task walkthroughs | Route-owned visibility slice added; full visual review remains |
| W163 | DONE: release gate, responsive/light-theme smoke, pointer/Enter trace, 125-node accessibility tree, delayed/failed feature fixtures, empty console, performance logs, installed hashes, and operator visual sign-off pass | No further W163 action; preserve artifacts | Qualification record and artifacts in `docs/browser-qualification-2026-09-08.md` | Native hardware observation remains separate |
| W165 | DONE: versioned schema, validated persistence, generation-checked layer selection, effective replacement dispatch, coordinated LED projection, scene reset, reconnect snapshot projection, browser selector, and release-gate evidence recorded in `WORKLIST.md` | No further W165 software action; retain native qualification separately | Contract, persistence/reconnect, projection, browser, and full-gate evidence in `WORKLIST.md` | Manufacturer/profile LED and native behavior remain post-release qualification |

## Execution rules

1. Start with the next executable step in dependency order: W160 and W159 may proceed in parallel;
   W161 and W162 consume their state contracts; W163 is the final qualification packet.
2. Do not close a packet from shell-string assertions, a release checksum, or an operator assumption
   alone. Attach the exact command, fixture, artifact, and observed result.
3. Keep authoritative vendor/developer source references in `docs/device-feature-inventory.md`
   and `docs/pipedal-server-operation-audit.md`; reverse engineering is a last resort and must be
   labeled as such.
