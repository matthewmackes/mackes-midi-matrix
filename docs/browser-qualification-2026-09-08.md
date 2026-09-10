# Browser qualification record — 2026-09-08

Installed target: `http://172.20.222.222:8081` (`mackes-midi-matrix.service`).

## Live service and asset identity

```text
systemctl is-active mackes-midi-matrix.service -> active
/api/v1/health -> {"ok":true,"generation":927,"health":"ready"}
app.js SHA-256 -> cca98c0962d188c87aeeeca72fc47bfdb858e16b741acebca9bc140206199ca9
app.css SHA-256 -> c80d0d100ba3078bdd8334f93c1383127580acc65eaed771101d3a5c287bba8d
```

## Browser evidence

- `scripts/browser-smoke.sh http://172.20.222.222:8081 /tmp/mackes-browser-w163-retry2`
  passed all six workspace routes, the Novation deep link, 320x900 and 768x1024 captures,
  light-theme rendering, current assignments, readback truthfulness, and installed asset hashes.
- `scripts/browser-interaction-smoke.py http://172.20.222.222:8081`
  passed with 56 controls and 89 named accessibility buttons. It exercised pointer selection and
  Enter activation. Accessibility tree: `/tmp/mackes-accessibility-tree-final.json`.
  Browser/performance logs: `/tmp/mackes-browser-logs-final.json`.
- `scripts/browser-feature-isolation-smoke.py http://172.20.222.222:8081` passed both delayed and
  failed mappings fixtures without displaying `Backend offline`.
- `scripts/browser-generation-smoke.py http://172.20.222.222:8081` passed the out-of-order
  generation fixture, retaining the newer Devices response.
- `scripts/browser-abort-smoke.py http://172.20.222.222:8081` passed after delaying a Devices
  request and navigating to Routing; the superseded request observed `AbortSignal` and the newer
  route remained authoritative.
- `scripts/browser-resume-smoke.py http://172.20.222.222:8081` passed after dispatching a
  visibility-change event and observing an immediate health refresh.
- `scripts/browser-draft-preservation-smoke.py http://172.20.222.222:8081` passed after entering
  a dirty routing draft and triggering background refresh; the draft remained unchanged.
- Combined W159 run (2026-09-08): generation, abort, resume, and draft-preservation fixtures were
  executed sequentially against the same installed service; all four returned `PASS`.
- Latest live checkpoint: the same four fixtures were repeated against the installed service and
  all passed; `/api/v1/health` returned `{"ok":true,"generation":1851,"health":"ready"}`.
- Latest full route smoke: `scripts/browser-smoke.sh http://172.20.222.222:8081` passed all
  primary workspaces, Recovery, Novation deep links, responsive sizes, light theme, and installed
  asset hashes; artifacts: `/tmp/mackes-browser-smoke.k4haND`.
- Reproducible aggregate command: `scripts/qualify-web-installed.sh
  http://172.20.222.222:8081` runs the route smoke and all four W159 lifecycle fixtures. The
  installed run completed with `qualify-web: PASS`.
- `scripts/browser-focus-preservation-smoke.py http://172.20.222.222:8081` passed, retaining
  focus on `routing-refresh` across an authoritative refresh. The aggregate command now includes
  this fifth W159 lifecycle fixture.
- Aggregate rerun after focus integration completed `qualify-web: PASS`; generation, abort,
  resume, draft, and focus fixtures all passed in sequence against the installed service.
- `scripts/browser-scroll-preservation-smoke.py http://172.20.222.222:8081` passed at 768×1024,
  retaining a 420px scroll position across refresh. The aggregate command now includes this
  sixth W159 lifecycle fixture.
- Aggregate rerun after scroll integration completed `qualify-web: PASS`; all six W159 lifecycle
  fixtures passed sequentially against the installed service.
- Latest aggregate run after navigation extraction completed `qualify-web: PASS`; route smoke and
  all six lifecycle fixtures passed. Route artifacts: `/tmp/mackes-browser-smoke.DydWI3`.
- Asset identity refresh: browser smoke now hashes `navigation.js` in addition to `app.js` and
  `app.css`; the installed run passed with artifacts at `/tmp/mackes-browser-smoke.FklP1g`.
- Latest aggregate after asset-hash integration completed `qualify-web: PASS`; all six lifecycle
  fixtures and the three installed asset hashes passed. Route artifacts: `/tmp/mackes-browser-smoke.gRPrPq`.
- Asset identity now covers both extracted modules (`navigation.js` and `health.js`) plus the
  application stylesheet/script; the live browser smoke passed with artifacts at
  `/tmp/mackes-browser-smoke.bqbg3T`.
- Latest installed aggregate rerun (2026-09-07): route smoke plus generation, abort, resume,
  draft, focus, and scroll fixtures all passed with `qualify-web: PASS`; route artifacts are at
  `/tmp/mackes-browser-smoke.pK6b1l`.
- After release installation, the live catalog hash matched the source exactly
  (`a203c4b453d87a54f2a1350d9c788e3416df7d818c44e4e5fcf2c856204ae0ad`), and route smoke passed
  with the expanded asset set; artifacts: `/tmp/mackes-browser-smoke.bP1GYd`.
- Full installed qualification after that deployment passed route smoke plus all six lifecycle
  fixtures and expanded asset hashes; artifacts: `/tmp/mackes-browser-smoke.lgOnLJ`.
- Route smoke after the feature projection deployment passed all workspaces, responsive/theme
  checks, deep links, and expanded asset hashes; artifacts: `/tmp/mackes-browser-smoke.CryNPW`.
- Full installed qualification after feature projection passed route smoke plus all six lifecycle
  fixtures and expanded asset hashes; artifacts: `/tmp/mackes-browser-smoke.y67owC`.
- Latest installed qualification (2026-09-08): route smoke plus generation, abort, resume, draft,
  focus, scroll, mobile-overflow, and task-ownership fixtures all passed with
  `qualify-web: PASS`; artifacts: `/tmp/mackes-browser-smoke.LPnjSO`.
- Latest installed qualification after DeviceQuery/Endpoints coalescing and the bounded mapping
  timeout (2026-09-08): route smoke, device inventory (32 cards), generation, abort, resume, draft,
  focus, scroll, mobile-overflow, assignment inspector, lossless route guard, and task ownership all
  passed with `qualify-web: PASS`; artifacts: `/tmp/mackes-browser-smoke.T6qBIB`.
- Device inventory recovery qualification (2026-09-08): after restarting the daemon following a
  transient resource-exhaustion 503, the installed Devices workspace rendered 32 authoritative
  cards including Novation/Launch Control; `browser-device-inventory: PASS`.
- Recovery qualification rerun (2026-09-08): route smoke, device inventory (32 cards), PiPedal
  catalog (3076 controls, 265 targets, 18 operation families), generation, abort, resume, draft,
  focus, and scroll fixtures passed. The mobile-overflow fixture later passed independently at
  width 500 after a transient long-run timeout.
- PiPedal source/live checkpoint (2026-09-08): source audit reported 104 registrations, 18
  connector operation families, and 86 pending rows across six families; the installed catalog
  returned 3076 controls and 265 targets with all 18 operation families.
- Assignment inspector recovery (2026-09-08): after extending `/endpoints` and `/novation`
  bounded reads to 12 seconds, the installed inspector fixture passed with an assigned Novation
  control and authoritative destination, source, behavior, LED, mapping ID, and readback status.
- Assignment inspector stability recheck (2026-09-08): repeated installed fixture passed in 14.4s
  with `knob-r1-c1` mapped to Eventide MicroPitch modulation/control-5 and explicit unknown
  readback state.
- Post-daemon aggregate (2026-09-08): route smoke, device inventory, PiPedal catalog, generation,
  abort, resume, draft, focus, and scroll all passed after daemon redeploy. The orchestration health
  check then timed out before mobile/assignment/route/task fixtures; this remains an open long-run
  saturation observation rather than a green aggregate claim.
- Post-focused-set soak (2026-09-08): immediately after the focused device/catalog/assignment/
  routing/task fixtures, 1,000 health probes passed with p95 8.964ms and max 13.213ms, with no
  observed HTTP 503 responses.

## Release and limitations

`bash scripts/release-gate.sh` passed after the current Web Interface and reliability changes.
Native hardware observations are not inferred from these browser fixtures. Human visual sign-off,
long-duration injected-load qualification, and the daemon-owned multi-destination/modifier contract
remain open in W160/W163/W165.
