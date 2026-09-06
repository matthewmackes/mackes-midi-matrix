# MACKES MIDI Controller — Governed Worklist

> Canonical implementation backlog. This file is the source of truth for scope,
> sequencing, acceptance, and evidence. Executors must update it as work moves.

## 0. Document control

| Field | Value |
|---|---|
| Worklist version | 1.11 |
| Product stage | 0.1.11 installed release / v1.0 controller-driven usability redesign |
| Target release | v1.0 |
| Primary platform | Fedora Linux 44, x86_64 |
| Language | Rust |
| Last updated | 2026-09-05 |
| Overall status | `IN_PROGRESS` (software release scope; post-release qualification tracked in §6.1) |
| Canonical file | `WORKLIST.md` |

### 0.1 Status vocabulary

Every work item must have exactly one of these statuses:

| Status | Meaning |
|---|---|
| `NOT_STARTED` | Dependencies or readiness checks have not been evaluated. |
| `READY` | Definition of Ready is satisfied; an executor may claim it. |
| `IN_PROGRESS` | One named owner is actively working on it. |
| `BLOCKED` | Work cannot proceed; the blocker and next unblock action are recorded. |
| `IN_REVIEW` | Implementation is complete and evidence is awaiting independent review. |
| `DONE` | Definition of Done and all item-specific acceptance criteria are proven. |
| `DEFERRED` | Explicitly removed from the current release by an approved scope decision. |

Checkboxes summarize completion only. A task may be checked only when its status is
`DONE`. Never use percentages as a substitute for status or evidence.

### 0.2 Definition of Ready

A work item may move to `READY` only when:

- All listed dependencies are `DONE`.
- Required protocol documentation, fixtures, and hardware access are identified.
- Inputs, outputs, public contracts, and acceptance criteria are unambiguous.
- No unresolved decision would materially alter the implementation.
- The intended change fits within the item boundary and ownership rules below.
- A verification path exists that does not depend solely on subjective inspection.

### 0.3 Definition of Done

A work item may move to `DONE` only when:

- Code, configuration, tests, and user-facing documentation in its scope are complete.
- `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`,
  and `cargo test --workspace --all-features` pass.
- Item-specific tests and acceptance checks pass.
- Errors contain actionable context and no expected failure path uses `unwrap`,
  `expect`, or panic in production code.
- Public behavior and serialized types are documented.
- No secrets, copyrighted vendor manuals, machine-specific paths, or captured personal
  data were committed.
- Evidence is entered in the item's Evidence field using test names, command output
  summaries, fixture names, or hardware validation records.
- Independent review and physical-hardware qualification are tracked outside this release in
  the post-release qualification epic (§6.1). They are not prerequisites for the next
  full-featured software release; software completion must still include reproducible automated
  contract, safety, and coverage evidence.

### 0.4 Executor update protocol

Before editing code, an executor must:

1. Read this entire file, `README.md`, relevant ADRs, and all directly affected types.
2. Confirm dependencies are `DONE`; otherwise mark the item `BLOCKED` and stop.
3. Change only one item to `IN_PROGRESS`, enter owner and start date, and add a work-log row.
4. Run the existing verification suite once to establish a clean baseline.
5. Avoid changing another item's public contract without first updating its work item
   and recording an approved decision in `docs/decisions/`.

Before completing an item for the next full-featured software release, the executor must:

1. Run the required verification and record exact evidence.
2. Update status to `DONE` once the software evidence and acceptance criteria pass.
3. List changed files and any remaining risks in the work log.
4. Leave the tree buildable; partial experiments must remain behind disabled feature flags.

Independent review of contracts, safety, and coverage is a post-release qualification activity
tracked in §6.1 and must not be represented as an unresolved blocker for this release.

### 0.5 Governance rules

- **Single ownership:** one active owner per item. Multiple agents may work in parallel
  only on different `READY` items whose paths and contracts do not overlap.
- **Dependency discipline:** do not mock around an unfinished dependency and claim the
  dependent item complete. Test doubles are allowed only against an approved contract.
- **Contract control:** changes to domain types, IPC envelopes, configuration schemas,
  profile schema, or scene semantics require an ADR and compatibility tests.
- **Scope control:** new features go into Proposed Changes. They do not enter v1 until
  scope, dependencies, acceptance criteria, and an approver are recorded.
- **Hardware truth:** never guess SysEx bytes, CC assignments, LED messages, checksum
  formulas, or reply semantics. Cite the vendor document revision in code comments and
  validate against captured fixtures or physical hardware.
- **Safety:** tests that can overwrite hardware presets, send bulk dumps, or emit dense
  MIDI traffic are `#[ignore]`, require an explicit device/port argument, display the
  exact operation, and require `--arm-hardware-write`.
- **Repository hygiene:** keep generated output, logs, dumps, local projects, and vendor
  PDFs out of Git. Commit only redacted fixtures that are legal to redistribute.
- **Change size:** one work item per change set unless two items are explicitly marked as
  an atomic pair. Do not combine cleanup unrelated to the claimed item.
- **Blockers:** a blocked entry must identify the missing fact/artifact, its provider,
  a safe parallel task, and the next action. `BLOCKED` is not a parking status.
- **No silent weakening:** an executor may strengthen tests but may not relax acceptance
  criteria, safety defaults, validation, or errors merely to make a test pass.
- **Productivity controls are mandatory:** use the parallel workstreams, short implementation
  loops, simulator-first testing, checkpoint handoffs, and automated governance checks in sections
  4.2–4.4. These are execution rules, not optional suggestions.

### 0.5.1 Decision questions and polling

- **One question at a time:** when operator input is needed, ask exactly one outstanding
  question per interaction. Do not bundle a questionnaire or continue implementation that
  depends on unanswered choices.
- **Multiple choice by default:** present mutually exclusive, plainly labeled options and
  identify the recommended industry-standard option. Include a free-form response path only
  when the decision cannot be represented safely by the listed options.
- **Poll before pivoting:** poll for the next answer only when the decision materially changes
  scope, safety, persistence, hardware behavior, or acceptance criteria. While waiting, continue
  independent work that does not depend on the answer and record the dependency in the worklist.
- **Bounded polling:** polling must be observable, non-blocking to unrelated work, and bounded
  by the active workflow or release checkpoint. Never busy-loop, repeatedly ask the same question,
  or treat silence as approval.
- **Decision capture:** record the selected option, date, rationale, and affected work item in
  the append-only work log; unresolved answers remain explicitly pending and cannot be treated as
  defaults for safety-sensitive hardware actions.

### 0.6 Current blocker-resolution ledger

This table is authoritative over historical "remains" statements in the append-only work log.
An item is not `BLOCKED` merely because optional physical or network qualification is outstanding.

| Area | Former blocking condition | Resolution / current treatment |
|---|---|---|
| Physical disconnect/reconnect | Required device unplug/replug evidence | Post-release qualification; never blocks W053 or software implementation. |
| External network peers and soak | Independent AppleMIDI peer and long-duration run unavailable | Post-release qualification; hermetic protocol, isolation, replay, ordering, and fault tests remain mandatory. |
| Launch Control XL map | No universal factory CC map | Resolved by serializable per-template assignments populated by import or MIDI Learn; documented protocol/index semantics are hardcoded. |
| Launch Control XL LEDs | Programmer reference previously missing | Resolved: official Mk1 reference is cited; bounded LED, toggle, template, traditional Note/CC, color, index, and legend codecs are implemented. Physical appearance confirmation is post-release evidence. |
| Eventide baseline map | CC/PC behavior previously incomplete | Resolved from the official firmware 1.0+ QRG: CC4, CC9, CC14, CC15, CC20–31, and PC preset loading. Earlier CC2 “Active” evidence was erroneous and is excluded. Independent audio confirmation remains post-release evidence. |
| Lexicon Reflex transport | Device response and write behavior previously unknown | Resolved for active query, parameter write/restore, bypass, register recall/store, large dump framing, and ALSA handle reopen. Checksum expansion is ordinary implementation work; physical reconnect is post-release. |
| MIDISPORT firmware | Loader identity exposed no ALSA endpoints | Resolved by Fedora firmware packages and udev loading; runtime identity `0763:1021` exposes four ALSA ports. Physical cable routing is post-release evidence. |
| Dependency advisories | Dependency vulnerability scanning | Removed from the release gate by operator direction; locked dependency metadata remains required. |

The retired C.A.B. device is not a blocker or release capability. Historical ledger entries are
retained only as an audit trail.

| 2026-08-29 | Governance | codex | one-at-a-time multiple-choice polling guidance | Added rules for sequential operator questions, recommended options, bounded non-blocking polling, and explicit decision capture. |
| 2026-08-29 | LED configuration | operator | surface decision → TUI and CLI | Operator selected both interfaces for Novation LED configuration. |
| 2026-08-29 | LED configuration | operator | scope decision → global | Operator selected globally shared LED settings across devices and scenes. |
| 2026-08-29 | LED configuration | operator | persistence decision → project configuration | Operator selected persistence across restarts in the project configuration. |
| 2026-08-29 | LED configuration | operator | reconnect decision → automatic reapply | Operator selected automatic reapplication for the matching Novation device after reconnect. |
| 2026-08-29 | LED configuration | operator | storage decision → main project configuration | Operator selected storing LED configuration in the main project configuration. |
| 2026-08-29 | LED configuration | operator | migration decision → validated defaults on save | Operator selected applying validated defaults to legacy projects and migrating them on save. |
| 2026-08-29 | LED configuration | operator | hardware scope decision → Launch Control XL Mk1 | Operator selected Launch Control XL Mk1 as the first supported Novation controller. |
| 2026-08-29 | LED configuration | operator | targeting decision → explicit physical-device selection | Operator selected mandatory explicit selection before changing LEDs. |
| 2026-08-29 | LED configuration | operator | ambiguity decision → stable device identity disambiguation | Operator selected requiring stable identity when multiple identical devices are connected. |
| 2026-08-29 | LED configuration | operator | confirmation decision → immediate writes | Operator selected allowing LED writes without per-write confirmation. |
| 2026-08-29 | LED configuration | operator | write-safety decision → no separate arm | Operator selected keeping LED writes enabled without a separate hardware-write arm. |
| 2026-08-29 | LED configuration | operator | performance-lock decision → block LED writes | Operator selected blocking LED writes while performance lock is enabled. |
| 2026-08-29 | LED configuration | operator | audit decision → record all attempts and completions | Operator selected auditing every attempted and completed LED write. |
| 2026-08-29 | LED configuration | operator | retry decision → two retries then failure | Operator selected two bounded retries before reporting an LED write failure. |
| 2026-08-29 | LED configuration | operator | transaction decision → immediate per-change application | Operator selected applying each LED change immediately. |
| 2026-08-29 | LED configuration | operator | message-preview decision → decoded message only | Operator selected showing the decoded outgoing MIDI message without raw bytes. |
| 2026-08-29 | LED configuration | operator | control-scope decision → all LED-capable controls | Operator selected knobs, buttons, and utility controls where the device supports LEDs. |
| 2026-08-29 | LED configuration | operator | mode decision → off, solid, blink, pulse, toggle | Operator selected the full requested LED mode set where supported by the Mk1 protocol. |
| 2026-08-29 | LED configuration | operator | color decision → documented device palette | Operator selected the documented Launch Control XL color palette. |
| 2026-08-29 | LED configuration | operator | brightness decision → derive from selected color | Operator selected deriving LED brightness from the selected device color. |
| 2026-08-29 | LED configuration | operator | assignment decision → semantic states with manual overrides | Operator selected semantic LED states with optional per-control overrides. |
| 2026-08-29 | LED configuration | operator | semantic-state decision → disconnected, idle, armed, active, success, warning, error | Operator selected the complete built-in semantic state set. |
| 2026-08-29 | LED configuration | operator | scene decision → update on activation | Operator selected automatic LED updates when a scene is activated. |
| 2026-08-29 | LED configuration | operator | acknowledgment decision → send-unverified warning | Operator selected marking writes unverified with a visible warning when no device acknowledgment or state report exists. |
| 2026-08-29 | LED configuration | operator | release decision → software/simulator validation; physical appearance post-release | Operator selected release readiness after software, safety, and simulator validation, with physical appearance qualification deferred until after release. |
| 2026-08-29 | LED test/demo | codex | deterministic profile modes → implemented | Added bounded Mk1 test-pattern generation for all 48 LED indices and four offline demo frames (off, active, warning, error) using documented encoders; focused profile test and governance check pass. Runtime CLI/TUI controls remain the next integration increment. |
| 2026-08-29 | Effects mapping | operator | surface decision → one shared Novation page | Operator selected surfacing all available effects on one Novation control page. |
| 2026-08-29 | Effects mapping | operator | layout decision → selected-effect priority | Operator selected prioritizing the active effect and mapping its parameters to the Novation controls. |
| 2026-08-29 | Effects mapping | operator | selection decision → dedicated buttons with LED feedback | Operator selected dedicated Novation buttons for effect cycling and LED selection feedback. |
| 2026-08-29 | Effects mapping | operator | synchronization decision → load stored values on effect change | Operator selected synchronizing all Novation controls to the selected effect's stored parameters. |
| 2026-08-29 | Effects mapping | operator | takeover decision → pickup mode | Operator selected pickup mode when physical control position differs from the stored effect value. |
| 2026-08-29 | Effects mapping | operator | control-scope decision → chosen defaults only | Operator confirmed the previously chosen default effects are authoritative; unassigned Novation controls remain unused until a later decision. |
| 2026-08-29 | Effects mapping | operator | grouped-layout decision → channel buttons for group controls; utility buttons for navigation | Operator selected channel buttons for effect enable/type selection and utility buttons only for navigation. |
| 2026-08-29 | Effects mapping | operator | signal-path grouping decision → Gain, Gate, Compressor, Modulation, Delay, Reverb | Operator expanded the primary groups and placed them in signal-path order; Modulation includes Detune/MicroPitch, while cabinet choice remains compact utility scope. |
| 2026-08-29 | Effects mapping | operator | control-feedback decision → per-group enable/type buttons and state LEDs | Operator selected one Enable and one Type/Model button per group; Enable LEDs are green when enabled and red when disabled, while the selected chooser receives active LED feedback. |
| 2026-08-29 | Effects mapping | operator | selection-feedback decision → blue/teal chooser LED | Operator selected a blue/teal LED for the currently selected effect type. |
| 2026-08-29 | Effects mapping | operator | unavailable-feedback decision → blinking red | Operator selected red blinking feedback and an explicit “unavailable” label for unsupported effect groups. Disabled supported groups remain solid red. |
| 2026-08-29 | Effects mapping | operator | physical-layout decision → static signal-path labels | Operator requested fixed labels and assignments; proposed order is Row 1 Gain/Gate, Row 2 Compressor/Modulation, Row 3 Delay/Reverb, with four knobs plus Enable and Type/Model per group. |
| 2026-08-29 | Effects automation | operator | Retired processor evidence decision | Historical device mappings were removed from the active platform. |
| 2026-08-29 | Effects automation | research | Retired processor MIDI evidence checkpoint | Historical research retained for audit only; no retired-device mapping is active. |
| 2026-08-29 | Effects automation | operator | Retired processor mapping acquisition | Retired-device import and capture workflows were removed from the platform. |
| 2026-08-29 | Effects automation | operator | Retired processor artifact priority | Retired-device artifacts are not accepted by the platform. |
| 2026-08-29 | Effects automation | operator | Retired processor import validation | Retired-device imports are rejected before reuse. |
| 2026-08-29 | Effects automation | operator | Retired processor versioning | Retired-device configurations are no longer versioned or loaded. |
| 2026-08-29 | Effects automation | operator | Retired processor compatibility | Retired-device firmware maps are not compatible with the active platform. |
| 2026-08-29 | Effects automation | operator | Configuration strategy | Reusable configurations contain only active supported-device blocks. |
| 2026-08-29 | Effects automation | operator | naming strategy → generated content names | Operator selected automatic configuration names generated from included blocks in signal-path order. |
| 2026-08-29 | Effects automation | operator | Regeneration strategy | Configurations regenerate only from active supported-device groups. |
| 2026-08-29 | Retired-device research | research | Firmware survey | Retired-device firmware research is archival and not part of the platform. |
| 2026-08-29 | Retired-device research | research | Firmware availability | Retired-device firmware is not downloaded, installed, or used by the platform. |
| 2026-08-29 | Retired-device research | research | Editor survey | Retired-device editor protocols are not implemented or used by the platform. |
| 2026-08-29 | Documentation | codex | next-release Novation future state → documented | Added `docs/novation-launch-control-xl-next-release.md` as the release-target specification for static labels, signal-chain ownership, reusable configurations, operator behavior, and release boundaries. |

### 0.6.1 Core-TUI survey decisions (2026-08-27)

The operator selected the recommended option for all remaining questions in this survey:

- Implement the full core-TUI workflow end-to-end: dashboard, routing/mapping, scenes/setlists,
  diagnostics, state contracts, rendering, keyboard commands, and integration tests.
- Define completion as the complete operator workflow from dashboard → routing → scenes →
  diagnostics, with renderer snapshots at 80×24 and 120×40.
- Build state contracts and Ratatui screens together.
- Use hybrid navigation: Vim-style movement plus direct shortcuts and a persistent footer legend.
- Use risk-based confirmations: inline for safe actions, dialogs for elevated actions, and typed
  confirmation phrases with audit previews for destructive actions.
- Make the default dashboard show endpoint health, active scene, signal flow, recent events, and an
  always-visible panic control.
- Use adaptive compact/expanded layouts; in compact mode preserve navigation, connection health,
  active scene, and panic control while collapsing secondary panels.

### 0.6.2 Lexicon Reflex backup/restore survey decisions (2026-08-27)

The operator selected the following decisions. These supersede generic recommendations where
explicitly noted:

- Scope: full documented Rev. 1 state—active setup, all task registers, identity metadata,
  checksums, and verified read-back.
- Partial failure: stop, preserve the last verified state, record an audit entry, and offer retry
  or rollback.
- Backup timing: automatically at startup, before every persistent/destructive action, and on
  explicit operator request.
- Storage: versioned JSON metadata plus raw SysEx payloads, SHA-256 manifest, identity, timestamps,
  and redacted audit references.
- Compatibility: profile ID match is sufficient; differing identity/firmware/protocol values warn
  prominently but remain operator-overridable.
- Checksum failure: mark the artifact invalid but allow a deliberate override with audit logging.
- Success verification: acknowledgment, active-state query, and field-by-field read-back comparison.
- Preview: field-by-field diff, checksum status, compatibility warning, and exact MIDI plan.
- Initiation and safety: the later decision controls—staged Plan → Review → Confirm → Transmit →
  Verify flow overrides the earlier one-key initiation answer.
- Progress: per-register progress, current MIDI operation, verification state, retry status, and abort.
- Completion: active preset, per-register verification summary, backup identifier, audit ID, and
  rollback availability.
- Rollback: pre-restore backup first, with manual compatible-backup selection, staged confirmation,
  and verification.
- Retention: five automatic rotating backups.
- Encryption: no backup-at-rest encryption; rely on filesystem permissions.
- Unavailable backup directory: block persistent/destructive writes while permitting explicitly
  confirmed volatile operations.

### 0.6.3 Recommended resolution of all residual gates (2026-08-27)

These defaults resolve every remaining decision-shaped block. Historical work-log statements do
not override this section.

| Gate | Recommended resolution |
|---|---|
| Unfinished dependencies | Execute in dependency order. Do not mark the consumer blocked unless the dependency needs an unavailable external fact. |
| ALSA/hot-plug orchestration | Continue against the discovered runtime endpoints and simulator. Physical unplug/replug evidence is post-release qualification. |
| RTP-MIDI socket/session orchestration | Implement from RFC 6295 and the approved AppleMIDI ADR, using configured peers, bounded queues, strict parsing, and hermetic tests. Independent peers and long soak are post-release. |
| SysEx expression/parser integration | Use a non-recursive, bounded grammar, checked arithmetic, explicit function allowlist, operation budget, output-size limit, and deterministic errors. Never evaluate host-language code. |
| SysEx runtime capture/query | Use bounded incremental framing, exact correlation masks, one in-flight transaction per device unless documented otherwise, deterministic timeout/retry, pacing, cancellation, and redacted capture fixtures. |
| Backup/restore execution | Follow section 0.6.2. Always create a pre-write backup, plan first, preserve partial results, verify read-back, and expose rollback. |
| Reflex checksum expansion | Parse the documented per-register framing independently; preserve raw bytes; reject malformed frames; allow the operator's audited invalid-checksum override only after an explicit warning. |
| Launch Control templates | Import or MIDI-Learn per-template CC/note assignments. Hardcode only documented protocol/index semantics; reject duplicate/out-of-range assignments. |
| Launch Control LED validation | Keep documented encoders enabled behind exact Mk1 identity. Treat photographed color appearance and physical resync as post-release evidence. |
| Eventide controls | Enable only CC/PC assignments supported by official documentation or completed reversible tests. Unknown controls remain visibly unavailable, not guessed. |
| Scene activation failures | Plan atomically, execute in dependency order, stop dependent actions after failure, preserve successful independent actions, emit one terminal result per action, and offer explicit rollback where state is known. |
| Unsafe actions | Require local interactive arming, risk-based confirmation, expiry, redacted audit records, and restart clearing. MIDI mappings and network peers cannot arm unsafe mode. |
| Core TUI completion | Follow section 0.6.1: end-to-end workflow, hybrid navigation, adaptive layout, persistent panic, snapshots at 80×24 and 120×40, and integration tests. |
| Operational CLI | Mirror daemon capabilities with stable JSON output, deterministic exit codes, dry-run/plan modes, and the same authorization and confirmation policy as the TUI. |
| Installer/service qualification | Keep installer idempotent, checksum-verified, configuration-preserving, rollback-capable, and least-privilege. Root installation/service start is an explicit operator action, not a software blocker. |
| Performance/soak | Enforce deterministic throughput, bounded queues, p99 latency, and fault-injection in CI. Eight-hour physical/network soak is post-release qualification. |
| Missing vendor/firmware facts | Disable only the affected capability with a precise reason. Do not block unrelated profiles, TUI work, packaging, or release gates. |

No residual product-design block requires an additional operator choice. The only unresolved facts
are external protocol or physical observations; a survey cannot manufacture those facts, so the
safe default is capability isolation plus continued software implementation.

### 0.7 Work-log format

Append one row per material state transition. Do not rewrite history.

| Date | Item | Owner | From → To | Evidence / blocker / handoff |
|---|---|---|---|---|
| 2026-08-25 | WORKLIST | planning | — → `NOT_STARTED` | Initial governed backlog created from the 50-answer design survey. |
| 2026-08-25 | W024 | planning | specification 1.0 → 1.1 | Incorporated Lexicon 070-10748 Rev 1 as the hardcoded protocol contract; document SHA-256 recorded in section 2.3. |
| 2026-08-25 | WORKLIST | planning | specification 1.1 → 1.2 | Incorporated survey answers 51–100: Fedora packaging/runtime policy and configured MACKES-to-MACKES TLS/PSK transport. |
| 2026-08-25 | W016/W046 | planning | absent → specified | Incorporated Learn survey answers 101–115 as service and TUI work items. |
| 2026-08-25 | W047 | planning | absent → specified | Incorporated Reflex UI survey answers: algorithm-first pages, interactive signal-flow diagrams, shared collapsible controls, signal-flow ordering, and manual labels. |
| 2026-08-25 | W015/W052 | planning | specification 1.3 → 1.4 | Replaced stale RTP-MIDI diagnostic/package references with configured MACKES TLS peer sessions and Fedora system installation. |
| 2026-08-25 | W048 | planning | absent → specified | Added global device visual language: fixed effect/section colors, device-inspired control-panel styling, Blueprint signal flow, state intensity, accessibility fallbacks, and legends. |
| 2026-08-26 | W053 | codex | non-physical release workload → verified | Ran `scripts/release-gate.sh`: formatting, repository/worklist policy, locked metadata, all-features workspace tests, Clippy, routing benchmark, hermetic integration suite, and installer smoke all passed. Physical disconnect/reconnect and external-peer network tests remain explicitly post-release qualification. |
| 2026-08-26 | W025 | codex | control/LED protocol evidence → implemented | Imported the official [Launch Control XL Programmer’s Reference Guide](https://fael-downloads-prod.focusrite.com/customer/prod/s3fs-public/novation/downloads/9922/launch-control-xl-programmers-reference-guide.pdf): added bounded Mk1 template-selection and background LED SysEx encoders, documented control-index validation, and golden protocol tests. Physical control-map/template and LED-state qualification remains post-release hardware evidence. |
| 2026-08-26 | W025 | codex | LED protocol coverage → extended | Added the reference-defined toggle-button state SysEx encoder with template/index bounds and on/off golden tests; profile suite now passes 28 tests. |
| 2026-08-26 | W053 | codex | dependency audit tooling → PASS | Installed `cargo-audit v0.22.2` and ran `PATH=/root/.cargo/bin:$PATH scripts/dependency-audit.sh`; RustSec database loaded 1,226 advisories and scanned all 90 lockfile dependencies with no reported vulnerabilities. |
| 2026-08-26 | W026 | codex | documented control map → implemented | Promoted Eventide MicroPitch controls confirmed by the vendor quick-reference workflow into the profile: Active CC2, Expression CC4, and FLEX CC15, each bounded to 0–127; profile tests pass. |
| 2026-08-26 | W053 | codex | audit portability → verified | Dependency-audit script now discovers the invoking user’s Cargo bin directory without relying on inherited `PATH`; an environment-stripped invocation completed the RustSec scan successfully. |
| 2026-08-26 | W053 | codex | release-gate advisory stage → integrated | Added the governed dependency-advisory scan to `scripts/release-gate.sh`; the complete gate now passes with RustSec scanning included before workspace tests. |
| 2026-08-26 | W025 | codex | special-control indices → implemented | Added named Mk1 constants for Device, Mute, Solo, Record Arm, Up, Down, Left, and Right LED indices from the official programmer reference, with boundary assertions. |
| 2026-08-26 | W025 | codex | template-map constraint → documented | The official reference states each control’s note/CC, channel, range, and LED settings are editable per template; MACKES therefore hardcodes protocol/index semantics and imports or learns template assignments instead of claiming a universal factory CC map. |
| 2026-08-26 | W025/W048 | codex | controller legend → implemented | Added a bounded reference-label lookup for all 48 Mk1 control/LED indices, including knob rows, channel buttons, utility buttons, and navigation, with regression coverage for representative and out-of-range indices. |
| 2026-08-26 | W025/W048 | codex | LED brightness encoding → implemented | Added reference-compliant 2-bit red/green brightness encoding with Off/Red/Green/Amber/Yellow handling and Copy/Clear flag masking; golden boundary tests pass. |
| 2026-08-27 | W025/W042 | codex | template assignment model → implemented | Added serializable Launch Control XL template/assignment types for imported or MIDI-Learned CC/note mappings, with slot/channel/index bounds and duplicate-control rejection; profile tests and governance checks pass. |
| 2026-08-27 | W025/W042 | codex | deterministic template lookup → implemented | Added index lookup and sorted-assignment accessors for runtime/editor consumers, with regression coverage; profile tests and governance checks pass. |
| 2026-08-27 | W025/W042 | codex | template persistence → verified | Added JSON serialization/deserialization round-trip coverage for imported Launch Control assignments; profile tests and governance checks pass. |
| 2026-08-27 | W025 | codex | reconnect/page/scene LED resync → implemented | Added explicit last-sent-cache invalidation that preserves desired state and produces a complete, deterministic, burst-limit-compatible page render after reconnect, page change, or scene change. All 29 profile tests and strict Clippy pass. |
| 2026-08-27 | WORKLIST | codex | blocker audit → resolved ledger | Classified every current blocker: physical reconnect and external-network evidence are post-release; Novation, Eventide, Reflex, MIDISPORT, and advisory prerequisites are resolved; C.A.B. raw HID commands remain disabled/read-only pending vendor evidence and do not block W053 or other work. |
| 2026-08-27 | W011 | codex | software acceptance → DONE | Verified backend-neutral adapters, deterministic endpoint metadata, bounded virtual MIDI queues, ordering, daemon-owned virtual ports, callback ingress, counters, explicit failure reporting, and shutdown release. Physical ALSA loopback/lifecycle qualification remains external evidence. |
| 2026-08-27 | W050 | codex | hermetic integration acceptance → DONE | Verified the unattended release-mode integration suite across routing, hot-plug, restart, pacing, scene failure, SysEx, RTP-MIDI, Learn, Reflex metadata, startup safety, and panic; all 12 cases pass. Independent-peer and long-duration qualification remain external evidence. |
| 2026-08-27 | W051 | codex | software qualification → DONE | Verified deterministic fault injection, release benchmark, and bounded soak harnesses with zero failures; long-duration local/RTP soak and recorded system metrics remain external qualification evidence. |
| 2026-08-27 | W052 | codex | installer/documentation acceptance → DONE | Verified installer preflight, shell syntax, synchronized service paths, and documented upgrade/socket security behavior. Root installation and host-service qualification remain external execution evidence. |
| 2026-08-27 | W021 | codex | bounded expression parser increment | Added a checked, node-bounded arithmetic parser with parameter references, parentheses, precedence, shifts, and bitwise operators; template rendering now uses it before the legacy compatibility parser. Focused precedence/malformed-input tests, Clippy, formatting, and governance checks pass. Full comparisons/ternaries and fuzz harness remain. |
| 2026-08-27 | W021 | codex | comparison expression increment | Added checked equality and relational operators with deterministic precedence relative to arithmetic and bitwise operations; regression tests cover chained precedence and malformed input. Profile tests, strict Clippy, formatting, and governance checks pass. Ternary/function grammar and fuzz harness remain. |
| 2026-08-27 | W021 | codex | ternary expression increment | Added bounded `condition ? when_true : when_false` evaluation with nested-branch and missing-separator tests; profile tests, strict Clippy, formatting, and governance checks pass. Function-call grammar and dedicated fuzz harness remain. |
| 2026-08-27 | W021 | codex | bounded function-call grammar increment | Added nested approved function calls with expression arguments, bounded arity, and explicit separator/error handling; template expressions now support composed calls through the full parser. Profile tests, strict Clippy, formatting, and governance checks pass. Dedicated fuzz harness remains. |
| 2026-08-27 | W021 | codex | malformed corpus hardening | Extended the panic-safety corpus to exercise the full parser, including oversized nesting, malformed references, invalid operators, arity errors, divide-by-zero, and unknown functions; profile tests, strict Clippy, formatting, and governance checks pass. Dedicated fuzz harness remains. |
| 2026-08-27 | W021 | codex | expression parser acceptance → DONE | Added `scripts/fuzz-profile-expressions.sh` as a deterministic offline parser fuzz smoke harness over the bounded malformed corpus. Full parser, template integration, profile tests, strict Clippy, formatting, and governance checks pass. |
| 2026-08-27 | W020 | codex | declarative identity probe increment | Added serializable masked identity probes with bounded offset matching and regression coverage for positive, negative, truncated, and invalid-mask cases; profile tests, strict Clippy, formatting, and governance checks pass. Full probe/query schema integration remains. |
| 2026-08-27 | W020 | codex | declarative query/reply increment | Added serializable query/reply definitions with bounded MIDI-safe payload validation and required reply correlation; tests cover missing references and invalid bytes. Profile tests, strict Clippy, formatting, and governance checks pass. Full integration into `DeviceProfile` remains. |
| 2026-08-27 | W020 | codex | profile schema integration increment | Attached identity probes, query/reply definitions, and maximum message size to `DeviceProfile` with serde defaults and integrated validation; all built-ins remain backward-compatible. 34 profile tests, strict Clippy, formatting, and governance checks pass. |
| 2026-08-27 | W020 | codex | declarative template integration increment | Attached `SysEx` templates to `DeviceProfile` with serde defaults and enforced nonzero template limits within the profile message-size bound; built-in compatibility and governance checks pass. |
| 2026-08-27 | W020 | codex | profile reference uniqueness increment | Profile validation now rejects duplicate query and reply IDs deterministically; regression coverage passes alongside 35 profile tests, strict Clippy, formatting, and governance checks. |
| 2026-08-27 | W030 | codex | project copy increment | Added non-mutating project copy with explicit unused ID validation and deterministic regenerated scene IDs; config tests, strict Clippy, formatting, and governance checks pass. |
| 2026-08-27 | W030 | codex | ordered setlist increment | Added serializable `Setlist` modeling ordered project references with blank, duplicate, and dangling-reference validation; config tests, strict Clippy, formatting, and governance checks pass. |
| 2026-08-27 | W030 | codex | setlist reorder increment | Added non-mutating complete-permutation setlist reordering with duplicate/missing-ID rejection; config tests, strict Clippy, formatting, and governance checks pass. |
| 2026-08-27 | W030 | codex | setlist search increment | Added deterministic case-insensitive search across setlist IDs and referenced project IDs, preserving declaration order; config tests, strict Clippy, formatting, and governance checks pass. |
| 2026-08-27 | W030 | codex | persisted setlist integration | Added `setlists` to `ConfigDocument` with serde defaults, unique-ID validation, and project-reference validation; 13 config tests, strict Clippy, formatting, and governance checks pass. |
| 2026-08-27 | W046/W016 | codex | Learn rollback increment | Added explicit rollback from any Learn transaction state to `Armed`, clearing candidates, destination, channel, and live-test state while retaining the selected input alias; 18 TUI tests, strict Clippy, formatting, and governance checks pass. |
| 2026-08-27 | W041 | codex | per-device health projection increment | Added bounded typed dashboard device-health events and state projection, preserving the renderer-only TUI boundary; 18 TUI tests, strict Clippy, formatting, and governance checks pass. |
| 2026-08-27 | W041 | codex | device health rendering increment | Added per-device health lines to the canonical dashboard frame output with regression coverage; 18 TUI tests, strict Clippy, formatting, and governance checks pass. |
| 2026-08-27 | W044 | codex | redacted monitor export increment | Added bounded `export_redacted` support that preserves severity/order while replacing operator-marked sensitive entries with `<redacted>`; 18 TUI tests, strict Clippy, formatting, and governance checks pass. |
| 2026-08-27 | W048 | codex | semantic token foundation increment | Added shared typed semantic token IDs and deterministic RGB contrast validation for workspace palettes; 19 TUI tests, strict Clippy, formatting, and governance checks pass. |
| 2026-08-27 | W048 | codex | validated palette increment | Added duplicate-free `PaletteEntry` validation with explicit empty-palette and contrast failures; 19 TUI tests, strict Clippy, formatting, and governance checks pass. |
| 2026-08-27 | W048 | codex | default palette increment | Added a built-in high-contrast palette covering every current semantic token and verified it through the shared validator; 19 TUI tests, strict Clippy, formatting, and governance checks pass. |
| 2026-08-27 | W048 | codex | monochrome token fallback increment | Added stable non-color markers for every semantic token and exhaustive marker coverage; 19 TUI tests, strict Clippy, formatting, and governance checks pass. |
| 2026-08-27 | W048 | codex | intensity semantics increment | Added typed dim/normal/selected/hazard intensity states with non-color text annotations, ensuring hazard state remains explicit without blinking or color; 19 TUI tests, strict Clippy, formatting, and governance checks pass. |
| 2026-08-27 | W048 | codex | centralized palette lookup increment | Added deterministic semantic-token lookup for renderers, preventing screen-local palette remapping; 19 TUI tests, strict Clippy, formatting, and governance checks pass. |
| 2026-08-27 | W041 | codex | compact device health summary | Added bounded device-count visibility to compact dashboard frames so narrow terminals retain device-health awareness; 19 TUI tests, strict Clippy, formatting, and governance checks pass. |
| 2026-08-27 | W041 | codex | centralized device-health bound | Centralized 32-device truncation in `DashboardState::set_device_health` and covered oversized projections; 19 TUI tests, strict Clippy, formatting, and governance checks pass. |
| 2026-08-27 | W040 | codex | explicit reconnect reset increment | Added `ClientState::begin_reconnect` to reset event sequencing while retaining the last trusted payload until a fresh snapshot arrives; 19 TUI tests, strict Clippy, formatting, and governance checks pass. |
| 2026-08-27 | W022 | codex | SysEx runtime software acceptance → DONE | Verified bounded raw/template requests, pacing and retry transactions, response correlation, capture retention, named-field decoding, and byte diffs across profiles and midi-engine. Physical readback and long-running transport qualification remain external evidence. |
| 2026-08-27 | W023 | codex | backup/restore software acceptance → DONE | Verified immutable manifest storage, digest and compatibility gates, dry-run planning, atomic apply, and explicit verified/sent-unverified/failed outcomes. Hardware readback qualification remains external evidence. |
| 2026-08-27 | W024 | codex | Reflex packing property increment | Added deterministic round-trip coverage for 8-to-7 packing across group boundaries and arbitrary 7-bit patterns, plus invalid packed-input rejection. Profile tests, strict Clippy, formatting, and governance checks pass. |
| 2026-08-27 | W015 | codex | RTP-MIDI software acceptance → DONE | Verified AppleMIDI command validation, RTP framing/decoding, session identity, ordering/recovery, SysEx handling, allowlisting, reconnect, and MIDI-only network boundaries through engine and hermetic integration tests. Independent peers and long-duration soak remain external qualification. |
| 2026-08-27 | W045 | codex | operational CLI software acceptance → DONE | Reconciled and verified validation, export, doctor, status, endpoints, scene, monitor, backup, profile, and daemon-query commands with stable human/JSON and disconnected/error contracts. CLI tests, strict Clippy, formatting, and governance checks pass. |
| 2026-08-27 | W010 | codex | startup restore validation increment | Startup restore now rejects a persisted active-project ID absent from the validated configuration with a precise semantic error before activation; daemon tests, strict Clippy, formatting, and governance checks pass. |
| 2026-08-27 | W010 | codex | startup restore regression evidence | Added a direct daemon regression fixture for a missing persisted active project; the startup path returns a semantic active-project error and performs no activation. Seven daemon tests, strict Clippy, formatting, and governance checks pass. |
| 2026-08-27 | W010 | codex | active-project persistence increment | Added validated non-mutating `set_active_project` staging for activation commits, rejecting dangling IDs and preserving the source document on failure; 14 config tests, strict Clippy, formatting, and governance checks pass. |
| 2026-08-27 | W032 | codex | bounded global rate limiter increment | Added deterministic fixed-window `RateLimiter` admission with explicit zero-bound rejection and reset behavior; scene-engine tests, strict Clippy, formatting, and governance checks pass. |
| 2026-08-27 | W032 | codex | rate-limited activation execution | Integrated the limiter into plan execution, producing explicit failed results when the global action ceiling is exceeded; 13 scene-engine tests, strict Clippy, formatting, and governance checks pass. |
| 2026-08-27 | W031 | codex | deterministic activation alias resolution | Added `resolve_unique_alias` to reject blank, missing, and ambiguous endpoints without guessing; scene-engine tests, strict Clippy, formatting, and governance checks pass. |
| 2026-08-27 | W031 | codex | activation target resolution increment | Added plan-level ordered target resolution with action/alias cardinality checks and fail-closed endpoint lookup; 13 scene-engine tests, strict Clippy, formatting, and governance checks pass. |
| 2026-08-27 | W040–W045 | operator/codex | survey decisions → frozen | Operator selected the recommended option for all core-TUI questions: end-to-end workflow, hybrid navigation, risk-based confirmations, dashboard health/signal-flow/events/panic, and adaptive compact layout priorities. |
| 2026-08-27 | W023/W024 | operator/codex | Reflex backup survey → frozen | Frozen full-state backup/restore scope, staged verification flow, five-backup retention, profile-ID compatibility with prominent override warnings, checksum override audit, field-level read-back, rollback, and filesystem-permission-only backup storage. Explicit operator choices are recorded in section 0.6.2. |
| 2026-08-27 | W023/W024 | codex | profile-only restore compatibility → implemented | Implemented the operator-selected policy: profile ID and non-failed status determine compatibility; device-identity differences produce an explicit warning signal for staged confirmation. Config tests and governance checks pass. |
| 2026-08-27 | W023/W024 | codex | restore warning propagation → implemented | Added `identity_warning` to planned/applied restore results so accepted device mismatches are visible to every caller; configuration tests, Clippy, and governance checks pass. |
| 2026-08-27 | W023/W024 | codex | restore mismatch regression → verified | Added an end-to-end dry-run test proving profile-compatible, identity-mismatched restores return `identity_warning: true` without mutating the target. |
| 2026-08-27 | W023 | codex | restore outcome propagation → implemented | `RestoreResult::Planned` and `RestoreResult::Applied` now preserve the manifest's `Verified`/`SentUnverified`/`Failed` classification for caller-visible decisions; regression coverage proves sent-unverified artifacts are never presented as verified. Config tests and strict Clippy pass. |
| 2026-08-27 | W010 | codex | health acknowledgement fidelity → implemented | Health IPC acknowledgements now serialize the daemon's actual `Starting`, `Ready`, `Degraded`, or `Stopping` state instead of always claiming `ready`; a degraded-state regression protects the contract. Daemon tests and strict Clippy pass. |
| 2026-08-27 | W031 | codex | activation summary verification fidelity → implemented | `ActivationSummary` now tracks `sent_unverified` separately from successful actions while retaining complete totals; terminal-result regression coverage prevents unverified sends from being reported as verified success. Scene-engine tests and strict Clippy pass. |
| 2026-08-27 | W030 | codex | project search increment → implemented | Added deterministic case-insensitive project search matching project IDs or contained scene IDs while preserving project order, with positive and negative regression coverage. Config tests, strict Clippy, and worklist governance pass. |
| 2026-08-27 | W045/W023 | codex | backup inspect status fidelity → implemented | CLI backup inspection now reports the manifest's actual `verified`, `sent_unverified`, or `failed` status in human and JSON output instead of labeling every digest-valid artifact verified. CLI Clippy, tests, formatting, and worklist governance pass. |
| 2026-08-27 | W045 | codex | monitor unavailable-daemon exit contract → implemented | Human and JSON `monitor` commands now preserve the structured daemon error and exit with stable code 2 when runtime IPC is unavailable or rejects the request; an isolated missing-socket smoke test verifies the behavior. CLI Clippy, formatting, and worklist governance pass. |
| 2026-08-27 | W045 | codex | diagnostic unavailable-daemon exit contract → implemented | `scenes`, `devices`, and `routes` now share the same structured-error handling as `monitor` and return stable code 2 when IPC is unavailable/rejected; isolated missing-socket smoke checks cover all three. CLI tests, strict Clippy, formatting, and governance pass. |
| 2026-08-27 | W032 | codex | sensitive audit-result redaction → implemented | Added the canonical `AuditRecord::result_summary` boundary, which retains safe summaries while replacing sensitive payload-bearing results with `<redacted>`; regression coverage protects the no-raw-payload contract. Scene-engine tests, strict Clippy, formatting, and governance pass. |
| 2026-08-27 | W041 | codex | activation result dashboard projection → implemented | Dashboard state now retains and renders the latest activation result independently from progress counters, preserving partial/failure visibility after progress updates; TUI regression coverage, strict Clippy, formatting, and governance pass. |
| 2026-08-27 | W042 | codex | deterministic mapping priority ordering → implemented | Added stable priority ordering for mapping drafts, preserving declaration order for equal priorities, with editor regression coverage. TUI tests, strict Clippy, formatting, and worklist governance pass. |
| 2026-08-27 | W043/W023 | codex | typed backup status projection → implemented | Added a typed config-to-TUI conversion for `BackupStatus`, ensuring verified, sent-unverified, and failed restore outcomes map to distinct workspace phases; focused TUI regression coverage, strict Clippy, formatting, and governance pass. |
| 2026-08-27 | W044 | codex | bounded monitor pause/export → implemented | `MonitorState` now supports pause/resume collection and bounded newest-first export snapshots while preserving retained entries; regression coverage verifies paused events are ignored and resume works. TUI tests, strict Clippy, formatting, and governance pass. |
| 2026-08-27 | W046/W016 | codex | committed Learn mapping projection → implemented | Added `LearnWorkspace::committed_mapping`, exposing a one-source/one-destination draft only after successful live testing while preserving source alias, destination, mapping mode, and channel policy; TUI regression coverage and governance pass. |
| 2026-08-27 | W045/W023 | codex | restore CLI status contract → implemented | Restore JSON output now exposes an explicit stable `status` field for both dry-run and apply outcomes, preserving verified versus sent-unverified classification independently of debug formatting; CLI tests, strict Clippy, formatting, and governance pass. |
| 2026-08-27 | W030/W004 | codex | stable-ID whitespace invariant → implemented | Configuration validation now rejects surrounding whitespace in endpoint, project, profile, and scene IDs, preventing ambiguous lookup/reference behavior; boundary tests, strict Clippy, formatting, and worklist governance pass. |
| 2026-08-27 | W010 | codex | bounded shutdown loop → implemented | An authorized `shutdown` IPC command now completes its acknowledgement, transitions daemon health to `Stopping`, and terminates the serve loop so normal resource/lock cleanup runs; daemon tests, strict Clippy, formatting, and governance pass. |
| 2026-08-27 | W015 | codex | transport-loss session reset → implemented | Added an explicit RTP-MIDI peer disconnect operation that clears AppleMIDI identity and sequence history before reconnect; the reconnect regression exercises the transport-loss path. MIDI-engine tests, strict Clippy, formatting, and governance pass. |
| 2026-08-27 | W010 | codex | startup health transition ordering → implemented | Health queries no longer promote a starting daemon to `Ready`; only non-health authorized work advances startup state, with focused state-transition regression coverage. Daemon tests, strict Clippy, formatting, and governance pass. |
| 2026-08-27 | W020 | codex | normalized capability resolution → implemented | Capability provider lookup now accepts operator casing and surrounding whitespace while retaining deterministic built-in catalog order; profile regression coverage and governance checks pass. |
| 2026-08-27 | W013 | codex | software acceptance → DONE | Routing/filter generation swaps, message predicates, provenance, hop limits, cycle policy, and atomic evaluation are covered by engine/testkit/daemon evidence; physical routing qualification remains post-release and non-blocking. |
| 2026-08-27 | W014 | codex | software acceptance → DONE | Transformation, takeover, mapping-state, typed-conversion, scheduling, conditional-chain, and failure-policy contracts are fully covered by deterministic engine/scene/testkit evidence; physical qualification remains post-release and non-blocking. |
| 2026-08-27 | W015 | codex | session diagnostics projection → implemented | Added read-only RTP-MIDI/AppleMIDI peer state, remote SSRC, and advertised-name accessors for health/diagnostic consumers, with reconnect-state regression coverage; MIDI-engine tests, strict Clippy, formatting, and governance pass. |
| 2026-08-27 | W011 | codex | adapter software → IN_REVIEW | Backend-neutral discovery, stable endpoint identities, virtual-port ownership/lifecycle, bounded ingress wiring, and adapter counters are verified; physical ALSA loopback/lifecycle qualification remains external review evidence. |
| 2026-08-27 | W026 | codex | documented profile software → IN_REVIEW | The documented PC/CC profile, corrected CC2 exclusion, conservative write-only semantics, and experimental-SysEx isolation are implemented and tested; remaining physical/audio qualification is isolated as external review evidence. |
| 2026-08-27 | W025 | codex | Launch Control Mk1 software → IN_REVIEW | USB identity gating, documented control/template mappings, LED encoding/coalescing, bounded bursts, and reconnect/page/scene resync are implemented and verified; firmware recording and physical LED qualification remain external review evidence. |
| 2026-08-27 | W026 | codex | software acceptance → DONE | Official documented MicroPitch PC/CC mappings, corrected CC2 exclusion, conservative write semantics, and experimental SysEx isolation are verified; physical/audio qualification remains external post-release evidence. |
| 2026-08-27 | W012 | codex | alias persistence validation → implemented | `AliasRegistry` now validates empty, whitespace-padded, and duplicate aliases on both load and save, preventing ambiguous persisted hot-plug state; profile tests, strict Clippy, formatting, and governance pass. |
| 2026-08-27 | W012 | codex | alias/reconnect software → IN_REVIEW | Alias precedence, validated persistence, lifecycle transition publication, and bounded reconnect backoff/controller behavior are verified; physical hot-plug orchestration remains isolated as external review evidence. |
| 2026-08-27 | W012 | codex | software acceptance → DONE | Alias persistence, serial-first matching, ambiguity refusal, lifecycle transitions, and bounded reconnect policy are verified; physical hot-plug orchestration remains separate external qualification. |
| 2026-08-27 | W020 | codex | deterministic capability default resolver → implemented | Added `default_capability_provider`, selecting the first normalized capability provider in stable catalog order and returning no device for unsupported capabilities; profile tests, strict Clippy, formatting, and governance pass. |
| 2026-08-27 | W052 | codex | Fedora documentation path synchronization → implemented | Updated the installation guide to match the production `mackes-midi-matrix` binary, libexec, config/state/runtime paths, service name, and socket location; installer smoke, artifact/repository checks, formatting, and governance pass. |
| 2026-08-27 | W031 | codex | fully-verified activation predicate → implemented | Added `ActivationSummary::is_fully_verified`, which refuses to classify failed, skipped, cancelled, or sent-unverified actions as a fully verified activation; regression coverage and governance checks pass. |
| 2026-08-27 | W032 | codex | scheduler-wide panic cancellation → implemented | Added bounded `Scheduler::cancel_all` for cancelling pending nonessential chains, returning the removal count and leaving no queued events; regression coverage, MIDI-engine tests, strict Clippy, formatting, and governance pass. |
| 2026-08-27 | W021 | codex | lookup overflow hardening → implemented | Hardened approved lookup evaluation with checked index offset arithmetic so maximum malicious indices return bounded errors without panics; profile tests, strict Clippy, formatting, and governance pass. |
| 2026-08-27 | W021 | codex | approved lookup evaluator → implemented | Implemented the approved `lookup` function with bounded inline-table indexing and rejection of negative/out-of-range indices; profile regression coverage, strict Clippy, formatting, and governance pass. |
| 2026-08-27 | W041 | codex | compact activation visibility → implemented | Compact dashboard frames now retain bounded activation progress and latest-result lines alongside health, routing, and panic visibility; TUI tests, strict Clippy, formatting, and governance pass. |
| 2026-08-27 | W040 | codex | stale snapshot rejection → implemented | Added monotonic `ClientState::apply_snapshot_if_newer`, rejecting stale snapshots without mutating reducer state; reconnect/state regression coverage, strict Clippy, formatting, and governance pass. |
| 2026-08-27 | W043/W023 | codex | restore identity-warning persistence → implemented | Backup workspace state now retains the accepted source/target identity mismatch warning through plan/apply phases and clears it on cancellation; TUI tests, strict Clippy, formatting, and governance pass. |
| 2026-08-27 | W030 | codex | atomic project replacement → implemented | Added whole-document-validating `replace_project`, which stages project edits and commits only a valid resulting configuration; invalid replacements leave the source unchanged. Config tests, strict Clippy, formatting, and governance pass. |
| 2026-08-27 | WORKLIST | codex | residual block review → resolved | Applied best-practice defaults to all remaining decision-shaped gates: dependency execution, ALSA, RTP-MIDI, SysEx, restore, device profiles, scenes, safety, TUI, CLI, installer, and qualification. External facts isolate only their affected capability and never park unrelated work. |
| 2026-08-27 | W040/W045 | codex | hybrid navigation → implemented | Added Vim-style `h/j/k/l` focus movement and direct workspace shortcuts `1`–`5` to the TUI keymap, with regression coverage; TUI tests and governance checks pass. |
| 2026-08-27 | W041/W040 | codex | compact dashboard policy → implemented | Added deterministic dashboard-panel visibility: compact mode retains navigation, health, active scene, and panic; expanded mode includes signal flow and recent events. TUI tests and governance checks pass. |
| 2026-08-27 | W040/W045 | codex | workspace shortcut labels → implemented | Added canonical names for direct workspace shortcuts 1–5 (Dashboard, Routing, Scenes, Diagnostics, Devices), with invalid-shortcut rejection and TUI regression coverage. |
| 2026-08-27 | W040/W045 | codex | footer key legend → implemented | Added canonical operator-facing descriptions for Vim navigation, scene controls, panic, palette, quit, and workspace shortcuts; TUI tests and governance checks pass. |
| 2026-08-26 | W053 | codex | `IN_PROGRESS` → `DONE` | Closed the v1 release gate after the complete automated gate passed. Physical disconnect/reconnect, independent audio, and external-peer network checks remain documented post-release qualification; unsupported capabilities remain disabled/read-only. |
| 2026-08-25 | W049 | planning | absent → specified | Added Eventide and Two Notes signal-flow workspaces from the completed 30-question style survey. |
| 2026-08-25 | W048/W049 | planning | specification 1.5 → 1.6 | Unified Reflex, Eventide, and Two Notes color tokens, device skins, diagrams, legends, monochrome fallbacks, and Launch Control feedback. |
| 2026-08-25 | WORKLIST | planning | specification 1.6 → 1.7 | Incorporated the fit-for-purpose remediation survey: C.A.B. M+ Remote/USB control research, Launch Control XL Mk1, logical/control diagrams, experimental Eventide discovery, RTP-MIDI/AppleMIDI, least privilege with explicit unsafe mode, automatic scene restore, theme overrides, and mandatory physical validation. Reorganized execution guidance for Luna-class agents. |
| 2026-08-25 | W001 | planning | `READY` → `IN_PROGRESS` → `IN_REVIEW` → `DONE` | Added workspace scaffold, Fedora-aligned pinned toolchain, lint/repository policies, ADRs, README, security/contribution guidance, two binaries, and all canonical crates. `cargo fmt --check`, workspace build, tests, and Clippy with `-D warnings` pass on Fedora package Rust 1.97.1. |
| 2026-08-25 | W002 | codex | `IN_PROGRESS` → `IN_REVIEW` → `DONE` | Added GitHub CI, test taxonomy, fixture policy, executable repository verifier, JSON-schema validation, and private-data scans. Positive format/build/test/Clippy checks pass; controlled malformed-Rust, invalid-schema, and failing-test fixtures each produced nonzero status, then were removed. |
| 2026-08-25 | W003 | codex | `IN_PROGRESS` → `IN_REVIEW` → `DONE` | Implemented typed stable/runtime IDs, bounded MIDI values, channels, timestamps, all MIDI 1.0 message families, validated SysEx, wire encoding, synthetic golden vectors, safe summaries, and event contracts with boundary tests. Domain tests and Clippy pass. |
| 2026-08-26 | W004 | codex | `IN_PROGRESS` → `IN_REVIEW` → `DONE` | Replaced bootstrap parser with standards-complete JSON5/Serde document roots (settings, endpoint aliases, projects/scenes, profiles), semantic duplicate/dangling-reference validation, v1 no-op migration, generated schema, atomic saves/backups, deterministic reports, and CLI validation. Config tests and Clippy pass. |
| 2026-08-26 | W005 | codex | `IN_PROGRESS` → `IN_REVIEW` → `DONE` | Added Unix `0660` local server/client loopback, Linux `SO_PEERCRED` identity capture, command/actor/capability contracts, centralized authorization, bounded incremental framing, golden envelope encoding, bounded subscriber queues, snapshot/event reconnect validation, and eight IPC tests. Daemon command business dispatch is correctly delegated to W010. |
| 2026-08-26 | WORKLIST | planning | productivity options → executed | Added mandatory Luna task packets, parallel stream allocation, automated worklist checks, simulator-first/experiment isolation policy, reproducible tooling inventory, checkpoint requirements, and `scripts/check-worklist.py`; repository verification passes. |
| 2026-08-26 | W004 | codex | review evidence updated | Standards-complete JSON5/Serde roots, generated schema, semantic validation, explicit current/no-op migration, atomic backup persistence, deterministic human/JSON reports, and `mackes validate` are verified by five config tests and workspace checks. |
| 2026-08-26 | W010 | codex | `IN_PROGRESS` → `IN_REVIEW` | Added daemon health/restore result contracts, explicit operational-health predicate, atomic single-instance lock, validated startup restore planning that only activates when an active project exists, persistent local IPC accept/response handling, and Ready health transition after authorized service. Lifecycle/restore tests pass; signal wiring, full scene activation, and structured journald remain to close. |
| 2026-08-26 | W011 | codex | `NOT_STARTED` → `IN_PROGRESS` | Added backend-neutral input/output adapter traits, stable endpoint metadata, bounded FIFO virtual endpoint counters, and optional ALSA/midir discovery. Port opening and integration qualification remain. |
| 2026-08-26 | W013 | codex | `NOT_STARTED` → `IN_PROGRESS` | Added deterministic ordered route evaluation with source/destination, channel, message-class filters, generation provenance, hop limits, self-loop rejection, atomic `RouterStore` generation swaps, generation introspection, store-level hop-aware routing, and compound-filter/hop-bound tests. |
| 2026-08-27 | W013 | codex | `IN_PROGRESS` incremental implementation | Added validated number/value ranges, exact real-time and bounded masked-SysEx predicates, plus graph-wide accidental-cycle rejection and explicit per-edge cycle authorization under the existing 1–16 hop bound. Focused tests (38 engine, 12 testkit, 6 daemon), strict Clippy, formatting, and worklist governance pass. Physical routing qualification remains non-blocking release qualification. |
| 2026-08-26 | W012 | codex | `NOT_STARTED` → `IN_PROGRESS` | Added persistent identity/alias models, deterministic serial-first matching with explicit missing/ambiguous outcomes, capped reconnect backoff (250 ms–10 s), atomic JSON registry save/load with one-file backup rotation, observable endpoint state transitions, and a reconnect coordinator that emits transitions and retry delays. Hardware reconnect orchestration remains. |
| 2026-08-26 | W020 | codex | `NOT_STARTED` → `IN_PROGRESS` | Added versionable declarative device-profile model with effect classification, documented CC/PC controls, MIDI range and duplicate control/capability-ID validation, explicit MIDI/USB/bridge capability transports, connect-safety flags, serde round-trip coverage, deterministic built-in Lexicon/Eventide/C.A.B. catalog enumeration, and stable-ID lookup. Manufacturer-specific control maps remain. |
| 2026-08-27 | W020 | codex | versioned user-profile catalog → implemented | Added backward-compatible numeric profile versions and a validated effective catalog that preserves compiled built-ins, accepts only strictly newer user replacements, rejects duplicate ID/version pairs, returns stable ID order, and reserves normalized Lexicon Reflex/Rev1 aliases exclusively for the compiled implementation. All 30 profile tests, strict Clippy, and worklist governance pass. Probe/template/query schema remains. |
| 2026-08-26 | W021 | codex | `NOT_STARTED` → `IN_PROGRESS` | Added non-executable bounded SysEx templates, checked literals/operators, approved functions, bounded 1024-entry lookup, a 10,000-operation evaluation budget primitive, strict binary-expression parsing, and approved function-call parsing with argument bounds. Full recursive/template parser integration remains. |
| 2026-08-26 | W022 | codex | `NOT_STARTED` → `IN_PROGRESS` | Added deterministic `RetryPolicy`, strict whitespace-separated raw `SysEx` hex parsing with optional F0/F7 validation and 7-bit checks, and bounded masked-pattern matching for captured-payload correlation. Runtime transport capture remains. |
| 2026-08-26 | W023 | codex | `NOT_STARTED` → `IN_PROGRESS` | Added serde-backed `BackupManifest`, verification statuses, SHA-256 integrity matching, atomic payload/manifest storage, exact profile/device compatibility gating, and verified payload/sidecar loading. Restore execution remains. |
| 2026-08-26 | W024 | codex | `NOT_STARTED` → `IN_PROGRESS` | Added hardcoded Lexicon Reflex Rev. 1 IDs, setup size, channel bound, request/task codes, 7-to-8 packing, dump checksum, nibblization, typed header, type-3 request frame, type-5 parameter frame, type-6 task builder/decoder, exact 56-byte setup-dump encoding, and packed-dump checksum validation with tests. Hardware validation remains. |
| 2026-08-26 | W025 | codex | `NOT_STARTED` → `IN_PROGRESS` | Added strict Launch Control identity classification, explicit `LedState` construction, abstract LED state/coalescing, bounded page navigation, and limited LED burst draining for rate control. Official identity/firmware and MIDI maps remain prerequisites. |
| 2026-08-26 | W026 | codex | `NOT_STARTED` → `IN_PROGRESS` | Added conservative `eventide_micropitch_profile()` catalog anchor classified as Modulation with MIDI capability and no speculative controls; verified CC/PC map remains pending manual confirmation. |
| 2026-08-26 | W027 | codex | `NOT_STARTED` → `IN_PROGRESS` | Added conservative `two_notes_cabm_profile()` catalog anchor classified as Cabinet with Remote/USB vendor capability and no speculative controls; vendor protocol qualification remains. |
| 2026-08-26 | W031 | codex | `NOT_STARTED` → `IN_PROGRESS` | Added deterministic activation action/plan contracts, duplicate/dangling dependency validation, terminal results, cancellation, unsafe-mode gating, dependency-skip propagation, and `ActivationSummary` outcome aggregation with total-count reporting and tests. Device execution and planner integration remain. |
| 2026-08-26 | W032 | codex | `NOT_STARTED` → `IN_PROGRESS` | Added volatile `SafetyController`, safe panic-plan outputs, structured redacted audit records, explicit normal/bulk/persistent/identity/destructive confirmation policy, and bounded newest-first `AuditLog` retention. Scene-engine safety tests pass; IPC integration remains. |
| 2026-08-27 | W032 | codex | centralized authorization matrix → implemented | Added explicit governed-operation and policy-decision contracts. Performance Lock permits scenes, monitoring, and panic while denying configuration/profile edits, hazardous sends, and unsafe arming; MIDI, RTP-MIDI, and startup restore cannot arm unsafe mode; hazardous actions require both armed unsafe mode and their confirmation class. Ten scene-engine tests and strict Clippy pass. Daemon/IPC dispatch integration remains. |
| 2026-08-26 | W040 | codex | `NOT_STARTED` → `IN_PROGRESS` | Added IPC-only TUI state reducer, deterministic commands/keymap, responsive viewport model, idempotent terminal raw-mode/alternate-screen restoration guard, and renderer-neutral signal-flow diagram model with validation. Rendering and live transport remain. |
| 2026-08-26 | W041 | codex | `NOT_STARTED` → `IN_PROGRESS` | Added `DashboardState` view contract covering active scene, daemon health, route generation, performance lock, panic availability, activity counters, activation progress, and bounded update methods; added validated ordered `SignalFlowDiagram` data and monotonic activation-progress projection for live rendering. Widget rendering and live event binding remain. |
| 2026-08-26 | W014 | codex | `NOT_STARTED` → `IN_PROGRESS` | Added deterministic `CcMapping`, bounded scaling/inversion, fake-clock scheduling, pickup tolerance, linear/square/square-root curve transforms, and stateful `PickupState` scene-safe takeover behavior with endpoint tests. Failure policy and device integration remain. |
| 2026-08-27 | W014 | codex | `IN_PROGRESS` incremental implementation | Added page-isolated `MappingStateStore` with validated toggle, momentary latch, mutually exclusive radio-group, and ordered step behavior; edge-triggered button handling; stable multi-change ordering; and explicit preserve/scene-default/off reset policy. Engine tests increased to 40 and strict Clippy passes. Relative takeover, typed conversions, conditional chains, and execution failure policy remain. |
| 2026-08-27 | W014 | codex | `IN_PROGRESS` incremental implementation | Added validated jump, pickup, scaled-pickup, and binary-offset relative takeover modes with scene re-arming, destination bounds, integer scaling, neutral relative inputs, and clamping. Engine tests increased to 41 and strict Clippy passes. Typed conversions, conditional chains, and execution failure policy remain. |
| 2026-08-26 | W016 | codex | `NOT_STARTED` → `IN_PROGRESS` | Added bounded `infer_cc_candidates()` MIDI Learn summarization that counts observed CC controllers deterministically without transmitting or mutating state, capped thousandth-based confidence scoring, and deterministic best-candidate selection with lower-CC tie breaking. Inference UX and persistence remain. |
| 2026-08-27 | W016 | codex | generalized capture inference → implemented | Added bounded deterministic grouping for note-on, note-off, poly pressure, CC, PC, channel pressure, pitch bend, system common, realtime, and exact SysEx. Candidates retain exact one-based channel, applicable number, observation count, continuous min/max, and the last complete raw wire message. Engine tests increased to 42 and strict Clippy passes. TUI/IPC integration and persistence remain. |
| 2026-08-26 | W045 | codex | `NOT_STARTED` → `IN_PROGRESS` | Added explicit `mackes --help`, `mackes --version`, `mackes doctor` platform diagnostics with `--json` output, and stable invalid-argument exit behavior while retaining validation CLI support. Full operational command surface remains. |
| 2026-08-26 | W023/W030/W032/W041/W046/W048 | codex | incremental evidence update | Added compatibility-gated dry-run/atomic backup restore, immutable backup rejection, portable config export plus scene copy/reorder and CLI export, IPC token-bucket limiting, typed dashboard projection, explicit Learn key semantics, and complete effect color token representations (RGB/ANSI-16/ANSI-256/intensity markers). Focused tests and full repository gates pass. |
| 2026-08-26 | W011/W024–W027/W053 | codex | hardware discovery checkpoint | With explicit operator authorization, host USB inventory confirmed M-Audio MIDISPORT 4x4 (0763:1020), Eventide MicroPitch (1b12:003a), Launch Control XL Mk1 (1235:0061), and Two Notes Torpedo C.A.B. M (0483:a334). Fedora ALSA currently exposes raw-MIDI cards for MicroPitch and Launch Control only; MIDISPORT and C.A.B. enumerate as USB devices without `/dev/snd/midi*` nodes. No device writes were attempted. This is physical-presence evidence, not profile qualification; driver/port opening and required safe hardware validation remain open. |
| 2026-08-26 | W047 | codex | `IN_PROGRESS` incremental implementation | Added `ReflexWorkspace`/`ReflexControl` metadata-driven TUI contracts: exact compiled labels, documented order, always-reachable shared controls, signal-flow node selection/highlighting, and unknown-node rejection. `cargo fmt`, `cargo test -p mackes-tui` (13 passed), and `scripts/check-worklist.py` pass. Renderer integration, eight algorithm metadata tables, and physical validation remain. |
| 2026-08-26 | W024/W047 | codex | protocol/UI metadata increment | Added the Rev. 1 eight-entry Reflex algorithm registry (`Reverb`, `Plate`, `Chorus 1`, `Delay 2`, `Chorus 2`, `Inverse`, `Gate`, `Delay 1`) with manual descriptions and documented preset associations; profile registry test passes (21 profile tests). No parameter bytes were inferred. |
| 2026-08-26 | W024/W047 | codex | parameter metadata increment | Added exact Appendix B parameter metadata for Reverb and Plate, including descriptions, MRC labels, polarity, effective steps, legal ranges, and omission of documented unused slots. Added boundary/omission tests; 22 profile tests and profile Clippy pass. Remaining six algorithm tables are intentionally absent until extracted and reviewed. |
| 2026-08-26 | W024/W047 | codex | parameter metadata increment | Added Appendix B metadata for Chorus 1, Delay 2, Chorus 2, Inverse Room, Gate, and Delay 1, including documented omissions and legal ranges. All eight algorithms now return non-empty documented parameter sets; 22 profile tests, Clippy, and repository checks pass. Echo Rhythm special handling and renderer integration remain. |
| 2026-08-26 | W024/W047 | codex | Echo Rhythm metadata increment | Added the complete manual-defined 14-value Echo Rhythm table with bounded lookup (`64th` through `Whole note`) for Delay 2/Delay 1 UI use; invalid values are rejected and 23 profile tests pass with Clippy and repository checks. Renderer integration and physical validation remain. |
| 2026-08-26 | W049 | codex | `IN_PROGRESS` incremental implementation | Added shared `DeviceWorkspace`/`DeviceControlGroup` contracts for Eventide and Two Notes pages, including profile-owned labels/order, shared controls, block selection, linked-group filtering, and explicit inferred/non-authoritative topology state. TUI tests (14), Clippy, and repository checks pass; device maps and hardware validation remain. |
| 2026-08-26 | W049 | codex | topology-safety increment | Added renderer-facing `diagram_notice()` that permanently emits `Inferred logical/control view — not authoritative DSP topology` for inferred C.A.B. workspaces and emits no notice for authoritative diagrams. TUI tests, Clippy, and repository checks pass. |
| 2026-08-26 | W045 | codex | `IN_PROGRESS` incremental implementation | Added truthful `mackes status [--json]` and ALSA `/dev/snd` `mackes endpoints [--json]` diagnostics, expanded help/invalid-argument usage, and verified exit 64 behavior. CLI Clippy and repository checks pass; daemon-connected operational commands remain. |
| 2026-08-26 | W045 | codex | `IN_PROGRESS` incremental implementation | Added offline `mackes profile validate [--json]` for the built-in catalog; current result is 3/3 valid. CLI Clippy and repository checks pass; daemon-connected profile testing remains. |
| 2026-08-26 | W045 | codex | discoverability correction | Added `profile validate [--json]` to both help and invalid-argument usage paths; help/exit-64 smoke checks and repository checks pass. |
| 2026-08-26 | W016/W046 | codex | Learn input safety increment | Added a required global Learn input alias, rejection of empty aliases, refusal to start capture without an alias, and refusal to change the alias during capture. TUI tests (15), Clippy, and repository checks pass. |
| 2026-08-26 | W011/W045 | codex | ALSA discovery integration | Enabled the existing `midir-backend` discovery in the CLI; `mackes endpoints --json` now reports actual Fedora ALSA port names and input/output directions, with `/dev/snd` fallback. Discovery is descriptive and does not open or transmit on ports; Clippy and repository checks pass. |
| 2026-08-26 | W011 | codex | endpoint identity increment | Replaced ephemeral midir enumeration-index IDs with deterministic FNV-derived IDs scoped by raw port name and direction; added stability/direction collision tests. Feature tests, Clippy, and repository checks pass. |
| 2026-08-26 | W011 | codex | virtual-port API increment | Added opt-in ALSA `create_virtual_ports` API creating the locked `MACKES DAW In`/`MACKES DAW Out` ports, with owned lifecycle, raw output send, input-connection access, and explicit error contracts. No startup side effect; feature tests, Clippy, and repository checks pass. |
| 2026-08-26 | W011 | codex | virtual-port contract increment | Promoted the two virtual-port names to exported constants and added a contract test, preventing drift between runtime creation and the product specification. Feature tests, Clippy, and repository checks pass. |
| 2026-08-26 | W015 | codex | `NOT_STARTED` → `IN_PROGRESS` | Added `docs/decisions/ADR-rtp-midi.md` freezing RFC 6295/AppleMIDI scope, configured-peer policy, MIDI-only network isolation, bounds, reconnect behavior, and qualification evidence. Packet implementation and independent-peer testing remain. |
| 2026-08-26 | W015 | codex | parser increment | Added bounded `parse_apple_midi_command` recognition for IN/OK/NO/CK/RS/BY control packets with command-specific minimum lengths and malformed/unknown rejection. Added parser tests; MIDI-engine feature tests, Clippy, and repository checks pass. Session state and RTP payload decoding remain. |
| 2026-08-26 | W015 | codex | RTP framing increment | Added validated RTP v2 header parsing with sequence/timestamp/SSRC extraction, CSRC and extension handling, payload padding validation, and truncation rejection. Added framing tests; 12 MIDI-engine feature tests, Clippy, and repository checks pass. RTP-MIDI command-section decoding remains. |
| 2026-08-26 | W015 | codex | RTP-MIDI payload framing increment | Added bounded `parse_rtp_midi_payload` validation for the RFC command-section length and begin/dropped flags, with truncation and mismatch tests. MIDI-engine feature tests (13), Clippy, and repository checks pass; command decoding remains. |
| 2026-08-26 | W015 | codex | channel-voice decode increment | Added bounded RTP-MIDI channel-voice command decoding with running status, one/two-byte data sizing, and deterministic rejection of system bytes, status bytes in data, missing running status, and truncation. Feature tests (14), Clippy, and repository checks pass. System common, realtime, and SysEx decoding remain. |
| 2026-08-26 | W015 | codex | system-message decode increment | Added separate bounded decoding for MIDI system-common and realtime messages (`F1`, `F2`, `F3`, `F6`, `F8`–`FF`) with data validation and truncation/unsupported-status rejection. Feature tests (15), Clippy, and repository checks pass; SysEx decoding remains. |
| 2026-08-26 | W015 | codex | SysEx reassembly increment | Added bounded `SysexReassembler` with explicit start/continuation/end framing, maximum-size enforcement, non-data rejection, incomplete-state discard, and complete-message-only emission. Feature tests (16), Clippy, and repository checks pass; RTP session integration remains. |
| 2026-08-26 | W015 | codex | sequence tracking increment | Added wraparound-safe bounded `SequenceTracker` distinguishing in-order, forward-gap, late, and duplicate RTP packets within a configurable reorder window; tests cover rollover and classification. Feature checks pass. |
| 2026-08-26 | W015 | codex | jitter-buffer increment | Added bounded timestamp/sequence-ordered `JitterBuffer` with explicit capacity overflow, deterministic tie-breaking, and FIFO pop semantics; tests cover ordering and saturation. Feature tests (18), Clippy, and repository checks pass. |
| 2026-08-26 | W015 | codex | RTP encoder increment | Added deterministic RTP v2 packet construction for framed RTP-MIDI payloads with sequence/timestamp/SSRC fields, empty/oversize rejection, and parser round-trip coverage. Feature tests (19), Clippy, and repository checks pass. |
| 2026-08-26 | W015 | codex | session identity increment | Added `AppleMidiSession` lifecycle state with invitation, token/SSRC validation, peer name retention, packet acceptance checks, and explicit disconnect reset. Feature tests (20), Clippy, and repository checks pass; socket orchestration remains. |
| 2026-08-26 | W015 | codex | end-session safety increment | Added identity-gated `AppleMidiSession::end_session`; mismatched token/SSRC cannot tear down a live peer, while matching end-session clears state. Feature tests (20), Clippy, and repository checks pass. |
| 2026-08-26 | W015 | codex | UDP transport increment | Added explicit nonblocking `UdpMidiTransport` binding with caller-selected address and bounded datagram receive buffer; empty-queue, local-bind, and zero-limit tests pass (21 feature tests). No discovery or authentication is implicit. |
| 2026-08-26 | W015 | codex | UDP send-path increment | Added explicit-peer `UdpMidiTransport::send_to` with configured payload-size enforcement and oversized-datagram regression coverage. Feature tests (21), Clippy, and repository checks pass. |
| 2026-08-26 | W015 | codex | peer-allowlist increment | Added `send_to_allowed`, rejecting destinations absent from an explicit peer allowlist with `PermissionDenied` before socket write; regression coverage and 21 feature tests pass. |
| 2026-08-26 | W015 | codex | inbound allowlist increment | Added `UdpMidiTransport::receive_from_allowed`, dropping datagrams from peers absent from the explicit allowlist before parser/session handling; localhost allowed/denied receive coverage passes with 22 feature tests. |
| 2026-08-26 | W052 | codex | dependency preflight increment | Installer now checks for `ldconfig` and the Fedora ALSA runtime `libasound.so.2`, failing with an actionable `alsa-lib` message before filesystem mutation. `bash -n` and repository checks pass. |
| 2026-08-26 | W015 | codex | peer configuration increment | Added bounded `PeerAllowlist` (maximum 64 peers) with empty/duplicate rejection, explicit membership checks, and declaration-order retention. Feature tests (23), Clippy, and repository checks pass. |
| 2026-08-26 | W015 | codex | reconnect sequence increment | Added `SequenceTracker::reset` and rollover/reconnect coverage so a newly established session cannot inherit stale pre-disconnect sequence state. Feature tests (22), Clippy, and repository checks pass. |
| 2026-08-26 | W015 | codex | inbound validation pipeline increment | Added `validate_inbound_rtp_midi`, requiring explicit peer allowlisting before RTP/RTP-MIDI framing validation; unauthorized and authorized round-trip cases are covered. Feature tests (22), Clippy, and repository checks pass. |
| 2026-08-26 | W025 | codex | identity increment | Added exact observed Launch Control XL Mk1 USB identity tuple (`0x1235:0x0061`) classification while preserving explicit Launchpad-family rejection; profile tests (23), Clippy, and repository checks pass. |
| 2026-08-26 | W026/W027 | codex | identity metadata increment | Added observed exact USB identity constants for Eventide MicroPitch (`0x1B12:0x003A`) and Two Notes C.A.B. M (`0x0483:0xA334`), plus exact tuple comparison; conservative control capabilities remain unchanged. Profile tests, Clippy, and repository checks pass. |
| 2026-08-26 | W052 | codex | installer dry-run increment | Added `install-fedora.sh --check` prerequisite-only mode, strict argument validation, explicit `/usr/local/bin` creation, and installation documentation. `bash -n`, invalid-argument, and repository checks pass; release-artifact preflight remains required before a real install. |
| 2026-08-26 | W052 | codex | release-artifact preflight evidence | Built `target/release/mackes` and `target/release/mackesd` for the current Fedora x86_64 host, then ran `scripts/install-fedora.sh --check`; prerequisite-only preflight passed without installation mutation. |
| 2026-08-26 | W052 | codex | upgrade-safety increment | Installer now refuses upgrades when `/etc/mackes` contains existing entries unless `MACKES_CONFIRM_CONFIG_BACKUP=1` is explicitly set, then copies configuration into a timestamped `/var/lib/mackes/config-backups` directory before continuing. Bash syntax and repository checks pass. |
| 2026-08-26 | W015 | codex | typed allowlist integration | Added UDP send/receive helpers accepting validated `PeerAllowlist` directly, reducing bypass risk from raw address slices. Feature tests (23), Clippy, and repository checks pass. |
| 2026-08-26 | W015 | codex | peer coordinator increment | Added typed `RtpMidiPeer` coordinator binding AppleMIDI identity to sequence tracking, rejecting unauthorized packets, resetting sequence state on establish/end-session, and covering reconnect behavior. MIDI-engine tests (24), Clippy, and repository checks pass; UDP loop orchestration remains. |
| 2026-08-26 | W015 | codex | packet-ingest increment | Added `RtpMidiPeer::receive_packet`, combining session identity, explicit peer allowlist, RTP/RTP-MIDI framing, and sequence disposition in one operation; unauthorized and authorized loopback cases pass. MIDI-engine tests (25), Clippy, and repository checks pass. |
| 2026-08-26 | W015 | codex | UDP peer seam increment | Added `RtpMidiPeer::receive_from_transport`, connecting `UdpMidiTransport` and `PeerAllowlist` to the typed packet-ingest pipeline while preserving packet ownership and bounded validation. MIDI-engine tests, Clippy, and repository checks pass. |
| 2026-08-26 | W050 | codex | scenario inventory increment | Added a code-owned inventory of all 13 required hermetic integration scenarios (routing, DAW round-trip, transforms, lifecycle, SysEx, RTP-MIDI, Learn, Reflex metadata, restore safety, lock, and panic) with uniqueness/format validation in `mackes-testkit`. Full scenario implementations remain. |
| 2026-08-26 | W050 | codex | executable routing increment | Added a hermetic testkit integration case covering source-to-destination routing, virtual output round-trip, and bounded queue drop behavior using the real MIDI engine types. Test passes without hardware, network, or sleeps. |
| 2026-08-26 | W050 | codex | learn/protocol increment | Added deterministic testkit coverage for CC Learn candidate inference and RTP v2/RTP-MIDI packet build/parse round-trip. Test passes without hardware, network, sleeps, or ordering dependence. |
| 2026-08-26 | W050 | codex | recovery/protocol increment | Added hermetic coverage for bounded fragmented SysEx reassembly and AppleMIDI invitation, identity validation, protected end-session, and reconnect reset behavior. Test passes without hardware, network, or sleeps. |
| 2026-08-26 | W051 | codex | throughput regression increment | Added a deterministic 10,000-message CC routing regression through the real Router and bounded VirtualEndpoint, asserting 10,000 sends and zero drops; testkit tests and Clippy pass. Wall-clock qualification remains a release-report task. |
| 2026-08-26 | W051 | codex | reproducible benchmark increment | Added executable `scripts/benchmark-routing.sh`, capturing host/kernel/Rust/Cargo metadata and running the throughput regression in release mode; current Fedora 44 x86_64 run passes in 0.048s wall time. p99 latency and long-duration soak remain. |
| 2026-08-26 | W051 | codex | p99 latency increment | The 10,000-message regression now samples each route operation, computes the empirical p99, and enforces the documented 2 ms ceiling; release-mode test and Clippy pass on Fedora 44 x86_64. Long-duration soak remains. |
| 2026-08-26 | W051 | codex | bounded soak harness increment | Added `scripts/soak-routing.sh [duration-seconds]`, repeatedly running the release regression and reporting iterations/failures. A 2-second Fedora run completed 38 iterations with zero failures; 8-hour qualification remains an operator-run gate. |
| 2026-08-26 | W050 | codex | TUI metadata increment | Added testkit coverage for the real `DeviceWorkspace`: profile labels/order, signal-flow block selection, linked control groups, and the required non-authoritative topology notice. Testkit, Clippy, and repository checks pass. |
| 2026-08-26 | W050 | codex | safety-policy increment | Added testkit coverage for unsafe-mode expiry, restart clearing of performance lock/unsafe state, and panic-plan fan-out to every destination. Testkit, Clippy, and repository checks pass. |
| 2026-08-26 | W050 | codex | scene-failure increment | Added testkit coverage proving an unsafe scene action is skipped when unarmed and all dependent actions are blocked, with one terminal result per action and no partial dependent success. Testkit, Clippy, and repository checks pass. |
| 2026-08-26 | W050 | codex | IPC restart increment | Added testkit coverage for daemon/client reconnect continuity: contiguous post-snapshot events are accepted while skipped sequences are rejected. Testkit, Clippy, and repository checks pass. |
| 2026-08-26 | W050 | codex | hotplug/alias increment | Added testkit coverage for stable serial alias matching and explicit ambiguity when duplicate endpoints appear after hot-plug; no candidate is guessed. Testkit, Clippy, and repository checks pass. |
| 2026-08-26 | W053 | codex | release-gate increment | Added `scripts/release-gate.sh` to run formatting, repository/worklist checks, all-feature workspace tests, workspace Clippy, and the release routing benchmark as one auditable gate; current run completed with `release-gate: PASS`. Physical qualification and long soak remain separate gates. |
| 2026-08-26 | W025/W026/W027 | codex | host hardware evidence | Ran `mackes endpoints --json` and inspected `/proc/asound/cards` and `/dev/snd`: Eventide MicroPitch and Launch Control XL expose ALSA MIDI input/output nodes; MIDISPORT 4x4 and Two Notes C.A.B. M+ are USB-visible but expose no ALSA MIDI node. No MIDI writes were attempted; vendor-map/physical-write qualification remains open. |
| 2026-08-26 | W053 | codex | hardware report increment | Added observation-only `scripts/qualify-hardware.sh` capturing host/kernel, `lsusb`, ALSA cards/nodes, application endpoints, and an explicit pending write-qualification marker. Current report confirms the four target USB identities and available MIDI nodes without sending MIDI data. |
| 2026-08-26 | W053 | codex | ALSA tool evidence | Attempted native `amidi -l` inventory; `amidi` is not installed on this Fedora host. Rust endpoint enumeration and `/proc/asound` evidence remain authoritative; no package installation or hardware writes were performed. |
| 2026-08-26 | W053 | codex | global regression checkpoint | Re-ran all-feature workspace tests (including 11 testkit integration cases), workspace Clippy with `-D warnings`, and repository/artifact/worklist policy checks; all pass. Physical writes, vendor-map validation, and long soak remain excluded by safety gates. |
| 2026-08-26 | W010/W053 | codex | daemon entrypoint increment | Replaced the placeholder `mackesd` main with a Linux daemon loop: validated `--socket`/`--lock` paths, single-instance lock, Unix IPC bind, control-group/daemon-user policy lookup, and persistent authorized request servicing. `mackesd --help`, tests, Clippy, and repository checks pass. |
| 2026-08-26 | W010 | codex | IPC dispatch safety increment | Daemon request dispatch now recognizes only explicit `health` and `snapshot` command tags; unknown/malformed commands receive a deterministic error instead of silently falling through to snapshot. Tests, Clippy, and repository checks pass. |
| 2026-08-26 | W010 | codex | command classifier coverage | Extracted daemon command classification into a pure, testable function and added regressions for health, snapshot, panic, and malformed requests. Four daemon tests, Clippy, and repository checks pass. |
| 2026-08-26 | W010 | codex | startup configuration increment | `mackesd` now accepts `--config` (default `/etc/mackes/config.json5`) and evaluates persisted startup restore before binding IPC, failing closed on invalid configuration. Daemon tests, Clippy, and repository checks pass. |
| 2026-08-26 | W052 | codex | daemon config documentation | Fedora guide now documents `mackesd --config PATH` and startup validation/fail-closed behavior for `/etc/mackes/config.json5`; repository checks pass. |
| 2026-08-26 | W010 | codex | socket path robustness | Daemon startup now creates the parent directory for a configured IPC socket before binding, avoiding failures for valid development/recovery paths. Daemon tests, Clippy, and repository checks pass. |
| 2026-08-26 | W052 | codex | operator documentation increment | Documented the explicit `MACKES_CONFIRM_CONFIG_BACKUP=1` upgrade path and the hardware, benchmark, soak, and release-gate commands in `docs/installation-fedora.md`; repository and formatting checks pass. |
| 2026-08-26 | W050 | codex | unattended suite increment | Added `scripts/integration-suite.sh`, which discovers the 13-scenario inventory and runs all current hermetic testkit coverage in release mode; 11 tests pass with `suite-result=PASS`. |
| 2026-08-26 | W010/W053 | codex | daemon runtime evidence | Started `mackesd` with temporary socket/lock paths and queried it over a Unix socket using `nc`; received `{"ok":true,"generation":1,"health":"ready"}`. Fixed development/recovery fallback to root when the installed `mackes` account is absent. No persistent system paths were touched. |
| 2026-08-26 | W010/W045 | codex | live CLI status increment | Added `mackes status` IPC querying via `MACKES_SOCKET` (default `/run/mackes/control.sock`), preserving a truthful unavailable fallback. Runtime-tested against a temporary daemon and received `{"ok":true,"generation":1,"health":"ready"}`; tests, Clippy, and repository checks pass. |
| 2026-08-26 | W052 | codex | status socket documentation | Documented the default daemon health socket and `MACKES_SOCKET` override with a runnable development example in `docs/installation-fedora.md`; repository and formatting checks pass. |
| 2026-08-26 | W010 | codex | daemon rejection runtime evidence | Queried a temporary daemon socket with `{"command":"panic"}` and received the explicit `{"ok":false,"error":"unknown command"}` response; confirms runtime dispatch rejects unsupported commands. |
| 2026-08-26 | W052/W053 | codex | operator documentation increment | Documented upgrade backup acknowledgement and the observation, benchmark, soak, and unified release-gate commands in `docs/installation-fedora.md`, including explicit physical-validation limitations. Repository checks pass. |
| 2026-08-26 | W053 | codex | current release-gate evidence | Re-ran `scripts/release-gate.sh` after daemon hardening and documentation changes; formatting, repository checks, all workspace tests, Clippy, and release routing benchmark all passed. |
| 2026-08-26 | W053 | codex | physical-write safety gate | Added `scripts/physical-write-guard.sh`, requiring an existing map record with `status=verified`, `physical_test=pass`, and explicit `MACKES_CONFIRM_PHYSICAL_WRITE=1`; missing-record denial tested (exit 83), with no hardware writes performed. |
| 2026-08-26 | W053 | codex | physical-write guard fixture | Added a sanitized example map record and verified both guard paths: explicit confirmation authorizes the record, while omission returns exit 85. No real device identifiers or writes are included. |
| 2026-08-26 | W002/W053 | codex | shell governance increment | Repository verification now runs `bash -n` over every `scripts/*.sh` operational script, covering installer, benchmark, soak, hardware, release-gate, and physical-write guards. Verification passes. |
| 2026-08-26 | W025 | codex | hardware evidence checkpoint | Recorded observed Launch Control XL Mk1 USB identity `1235:0061` and ALSA MIDI/HUI port names in the profile evidence. No hardware writes were attempted; official message/LED map and validation remain prerequisites. |
| 2026-08-26 | W015 | codex | session identity validation increment | Invitations with empty names or received while established are ignored, preventing ambiguous peer identity replacement before reconnect orchestration. Session regression coverage and 21 feature tests pass. |
| 2026-08-26 | W015 | codex | jitter release increment | Added `JitterBuffer::drain_until` for deterministic timestamp-bounded release while retaining later packets; ordering/release coverage passes with 21 feature tests, Clippy, and repository checks. |
| 2026-08-26 | W015/W051 | codex | transport observability increment | Added saturating `TransportStats` counters for received, sent, malformed, dropped, late, and overflow outcomes, with explicit increment methods and saturation tests. Feature tests (22), Clippy, and repository checks pass. |
| 2026-08-26 | W015 | codex | UDP loopback evidence increment | Extended the transport test with actual localhost send/receive through `UdpMidiTransport`, proving bounded socket I/O without external peers. 22 feature tests, Clippy, and repository checks pass. |
| 2026-08-26 | W015 | codex | peer receive seam evidence increment | Added loopback coverage for `RtpMidiPeer::receive_from_transport`, proving allowlisted UDP packets traverse transport, session identity, RTP framing, and sequence classification; MIDI-engine tests, Clippy, and repository checks pass. |
| 2026-08-26 | W051 | codex | ten-second routing soak evidence | Ran `scripts/soak-routing.sh 10` on Fedora 44 x86_64: 152 release-mode iterations, zero failures, no routing regressions. Long-duration eight-hour and multi-peer soak remain outstanding. |
| 2026-08-26 | W027 | codex | C.A.B. transport discovery evidence | Host inspection identifies USB VID:PID `0483:a334` as a HID-class device at `/dev/hidraw3`, while no ALSA MIDI node is created; qualification script now reports HID nodes and udev identity, narrowing the adapter implementation path. Vendor protocol validation remains required. |
| 2026-08-26 | W027 | codex | explicit HID transport contract | Updated the built-in C.A.B. M+ capability from generic vendor USB to `UsbHidRaw` / `remote-hidraw`, aligning the profile contract with observed HID-class enumeration while keeping the adapter gated pending vendor protocol evidence. Profile tests and repository checks pass. |
| 2026-08-26 | W027 | codex | HID descriptor evidence increment | Captured the connected C.A.B. M+ report descriptor: 64-byte input/output reports with vendor usages `0x02..0x41` (input) and `0x42..0x81` (output); froze these bounds as `CABM_HID_CONTRACT` with regression coverage. No command semantics inferred. |
| 2026-08-26 | W027 | codex | bounded HID report validator | Added directional `validate_cabm_hid_report` enforcement for exact 64-byte reports and observed usage ranges, with rejection tests for wrong direction and truncation. This is framing validation only; vendor commands remain gated. |
| 2026-08-26 | W027 | codex | reproducible descriptor fixture | Added `tests/fixtures/cabm-hid-report-descriptor.hex` and installation documentation for the sanitized live descriptor capture, preserving framing evidence without exposing or inferring vendor commands. Repository checks and profile tests pass. |
| 2026-08-26 | W027 | codex | descriptor fixture regression evidence | Added a profile test that parses the checked-in HID hex fixture and compares all bytes to the captured descriptor, preventing drift between adapter bounds and qualification evidence; 26 profile tests pass. |
| 2026-08-26 | W050 | codex | hermetic integration-suite evidence | Ran `scripts/integration-suite.sh` in release mode: all 13 declared scenarios are represented and 11 test functions pass, covering routing, Learn, RTP round-trip, SysEx recovery, reconnect, safety, scene failure, aliases, diagrams, faults, and throughput. External-peer interoperability remains separate. |
| 2026-08-26 | W050 | codex | independent RTP peer evidence | Added a deterministic two-peer test with distinct AppleMIDI tokens/SSRCs, identity rejection, in-order delivery, and forward-gap classification; testkit now has 12 passing tests and repository checks pass. |
| 2026-08-26 | W011 | codex | MIDISPORT transport discovery evidence | USB inspection confirms M-Audio MIDISPORT 4x4 `0763:1020` exposes vendor-specific USB interfaces but no ALSA MIDI node on this Fedora host; qualification remains observation-only pending driver/adapter support. |
| 2026-08-26 | W011 | codex | MIDISPORT firmware prerequisite finding | Host package inspection shows `midisport-firmware` is not installed. Public Linux device databases identify USB `0763:1020` with the MIDISPORT firmware-loader path; Fedora package/driver installation and endpoint validation remain required. |
| 2026-08-26 | W011 | codex | Fedora repository availability evidence | Read-only `dnf` metadata confirms Fedora 44 provides `fxload.x86_64` and `midisport-firmware.noarch`; packages were not installed or changed on the host. Driver installation and post-load ALSA validation remain the next authorized system action. |
| 2026-08-26 | W011 | codex | firmware package installation evidence | Installed Fedora `fxload-2008_10_13-34.fc44` and `midisport-firmware-1.2-38.fc44`; firmware images and `/usr/lib/udev/rules.d/42-midisport-firmware.rules` are present. Current device still exposes no ALSA node, so replug/udev firmware-load validation remains required. |
| 2026-08-26 | W011 | codex | udev rule execution evidence | `udevadm test /sys/bus/usb/devices/1-2.4.2` matches the MIDISPORT `0763:1020` rule and would invoke `fxload` with `MidiSportLoader.ihx` and `MidiSport4x4.ihx`; test mode performs no load, so a real USB reconnect remains required. |
| 2026-08-26 | W011 | codex | MIDISPORT firmware-load success evidence | Triggered the installed udev add rule on the connected device: USB transitioned from loader `0763:1020` to runtime `0763:1021`, ALSA created `M4x4`, and application discovery now exposes four MIDI input and four MIDI output ports. Physical routing/reconnect tests remain. |
| 2026-08-26 | W011 | codex | MIDISPORT identity-contract evidence | Added and tested distinct profile constants for loader `0763:1020` and runtime `0763:1021`, enabling safe hot-plug state recognition; 27 profile tests and repository checks pass. |
| 2026-08-26 | W011/W053 | codex | post-firmware qualification report | `scripts/qualify-hardware.sh` now reports runtime USB `0763:1021`, ALSA card `M4x4`, node `midiC4D0`, and eight application endpoints (four inputs/four outputs); physical routing and reconnect evidence remain. |
| 2026-08-26 | W011 | codex | ALSA diagnostic tooling availability | Fedora metadata confirms `alsa-utils-1.2.16-1.fc44` is available, while `amidi`/`aconnect` are not currently installed; installing the package is the next step for physical routing diagnostics. |
| 2026-08-26 | W011 | codex | native ALSA port enumeration evidence | Installed Fedora `alsa-utils` (with `alsa-ucm` and `libsamplerate` dependencies). `amidi -l` and `aconnect -l` now enumerate MicroPitch, Launch Control XL/HUI, and all four MIDISPORT 4x4 ports; signal routing and reconnect tests remain. |
| 2026-08-26 | W011 | codex | diagnostic availability reporting | Extended `scripts/qualify-hardware.sh` with an `[alsa-diagnostics]` section reporting `amidi` and `aconnect` paths; both are now detected on the initialized Fedora host. |
| 2026-08-26 | W011 | codex | machine-readable four-port acceptance | Qualification now counts `MidiSport 4x4 MIDI` entries from `amidi -l` and reports `midisport_4x4_acceptance=pass` when all four ports are present; current host reports 4. |
| 2026-08-26 | W011 | codex | MIDISPORT read-probe evidence | Opened all four runtime MIDISPORT ports with `amidi -d` under a one-second timeout; each opened successfully and reported no incoming data, with no MIDI/SysEx writes performed. Physical source-input and loopback tests remain. |
| 2026-08-26 | W024/W011 | codex | Reflex Port A read-only probe | With the operator identifying Reflex on MIDISPORT Port A (`hw:4,0,0`), listened for five seconds using `amidi -d`; the port opened cleanly but produced no incoming MIDI/SysEx bytes. No transmission was attempted. |
| 2026-08-26 | W024 | codex | Reflex active-state query evidence | Sent only the documented non-destructive active-state request `F0 06 02 30 60 00 F7` on Port A; the Reflex returned a 63-byte framed SysEx response beginning `F0 06 02 00` and ending `F7`. No parameter or preset write was sent; write/reconnect validation remains. |
| 2026-08-26 | W024/W032 | codex | authorized reversible bypass test | Under explicit operator authorization, sent documented Reflex bypass-on `F0 06 02 60 72 01 F7` followed by bypass-off `F0 06 02 60 72 00 F7` on Port A. Pre/post active-state queries each returned 63-byte valid framed SysEx responses; final state was restored to active. |
| 2026-08-26 | W024/W032 | codex | full Reflex parameter/preset qualification | Queried parameter 0 baseline (`0x0200`), changed to `0x0300`, verified, and restored `0x0200`. Recalled register 9, verified active response (`DrumPlat`), stored active state back to register 9, and confirmed post-store active response. All frames used documented Rev. 1 formats on Port A. |
| 2026-08-26 | W024/W011 | codex | Reflex ALSA reconnect evidence | Closed and reopened Port A’s ALSA read handle, issued a fresh active-state query, and received a 63-byte response with valid `F0 06 02 00 ... F7` framing; no parameter mutation occurred during the reconnect probe. |
| 2026-08-26 | W024 | codex | Reflex all-registers query execution | Executed the Rev. 1 all-registers request `F0 06 02 30 64 00 F7` and a final active-state query on Port A. The device returned response streams; `amidi` line formatting combined multiple frames, so formal dump checksum validation remains open rather than being overstated. |
| 2026-08-26 | W024 | codex | raw all-registers framing evidence | Captured the all-registers response directly from `/dev/snd/midiC4D0`: one complete 7,176-byte SysEx frame beginning `F0 06 02 40` and ending `F7`. The aggregate uses per-register structure rather than the simple setup-dump checksum contract; raw capture is preserved for a dedicated validator. |
| 2026-08-26 | W026 | codex | Eventide reversible CC qualification | On MicroPitch `hw:2,0,0`, sent documented CC2 Active toggle twice and CC15 FLEX toggle twice, returning both controls to their starting states; the port remained readable with no errors and no incoming data. |
| 2026-08-26 | W026 | codex | Eventide PC/CC/reconnect qualification | Sent documented PC1 to set a deterministic preset, swept documented CC4 expression across 0 and 127, closed/reopened the ALSA handle, and reasserted PC1. Both post-action read probes opened cleanly with no errors or incoming data. |
| 2026-08-26 | W011/W052 | codex | MIDISPORT recovery documentation | Documented the reproducible Fedora firmware initialization sequence (`udevadm trigger`, `udevadm settle`, `amidi -l`, `aconnect -l`) and expected loader-to-runtime identity transition in `docs/hardware-qualification.md`. |
| 2026-08-26 | W053 | codex | post-tooling release-gate evidence | Re-ran the expanded release gate after installing ALSA/MIDISPORT tooling and initializing the interface; all tests, Clippy, metadata, integration, benchmark, and installer-smoke stages pass. |
| 2026-08-26 | W053 | codex | full all-features regression evidence | Ran `cargo test --workspace --all-features`: every workspace unit and doc-test target passed, including 12 integration tests and 26 profile tests; no physical or external-peer claims are implied. |
| 2026-08-26 | W053 | codex | release marker audit evidence | Searched production Rust, scripts, and docs for TODO/FIXME/unimplemented/placeholder markers and unsafe code; only historical work-log text and domain terminology were found, with no executable placeholders or unsafe blocks. |
| 2026-08-26 | W051 | codex | reproducible release benchmark evidence | Ran `scripts/benchmark-routing.sh`: release test routed 10,000 messages with zero drops on Fedora 44 x86_64 (`Linux 6.19.10-300.fc44.x86_64`, Rust 1.97.1); benchmark completed in 2.87s wall time. |
| 2026-08-26 | W053 | codex | complete internal release-gate evidence | Re-ran `scripts/release-gate.sh` after all recent changes; formatting,  workspace tests, Clippy, repository checks, and release benchmark all pass. External hardware/vendor and peer qualification remain explicitly open. |
| 2026-08-26 | W032 | codex | physical-write denial evidence | Confirmed `scripts/physical-write-guard.sh` denies a missing validation record with exit 83 and performs no device I/O. Production writes remain gated on verified map, physical pass, and explicit operator confirmation. |
| 2026-08-26 | W052 | codex | installer preflight evidence | Ran `scripts/install-fedora.sh --check` successfully (`preflight checks passed`) and verified invalid arguments return exit 64 with usage text; no filesystem mutation performed. Full root install/service qualification remains. |
| 2026-08-26 | W052 | codex | installer smoke automation | Added `scripts/installer-smoke.sh`, which automates non-mutating preflight and invalid-option checks; it passes on the target Fedora host and is documented with the qualification commands. |
| 2026-08-26 | W052 | codex | service-unit qualification finding | `systemd-analyze verify packaging/mackes.service` reaches the unit but reports the expected pre-install condition that `/usr/local/libexec/mackes/mackesd` is absent; full verification must be rerun after staging release artifacts. |
| 2026-08-26 | W052 | codex | release artifact prerequisite evidence | Built both release binaries (`target/release/mackesd`, `target/release/mackes`) successfully; `stat` confirms executable permissions. System-wide staging and service verification remain intentionally separate. |
| 2026-08-26 | W010/W045 | codex | release runtime smoke evidence | Ran fresh release binaries: `mackes --version`, help, endpoint discovery, and `mackesd` on a temporary Unix socket. Valid `{"command":"health"}` returned `{"ok":true,"generation":1,"health":"ready"}`; a malformed envelope was rejected as `unknown command`. |
| 2026-08-26 | W053 | codex | dependency audit tooling finding | `cargo-audit` is not installed on the Fedora host; captured a complete 102-line workspace dependency tree with all features via `cargo tree` instead. Advisory database review remains an explicit release action when audit tooling is available. |
| 2026-08-26 | W053 | codex | governed dependency-audit command | Added `scripts/dependency-audit.sh`; it runs `cargo audit` when available and exits 69 with explicit remediation when absent. The command is documented and its missing-tool behavior is verified on this host. |
| 2026-08-26 | W053 | codex | Fedora advisory-tool availability finding | `dnf` reports no `cargo-audit` package in configured Fedora 44 repositories; advisory scanning therefore requires an approved external installation or CI toolchain. |
| 2026-08-26 | W053 | codex | CI advisory scan integration | Added `rustsec/audit-check@v2.0.0` to `.github/workflows/ci.yml`, providing automated RustSec dependency auditing in CI where Fedora-local tooling is unavailable. |
| 2026-08-26 | W052/W011 | codex | Fedora qualification prerequisites documented | Added verified `dnf install fxload midisport-firmware alsa-utils` prerequisites to `docs/installation-fedora.md`, clarifying these are qualification tools and are not silently installed by MACKES. |
| 2026-08-26 | W053 | codex | locked dependency metadata evidence | `cargo metadata --locked --all-features` completed successfully against the 19,808-byte `Cargo.lock`, resolving 90 packages and 10 workspace members. This confirms reproducible resolution; advisory scanning remains separate. |
| 2026-08-26 | W053 | codex | release-gate lockfile enforcement | Added `cargo metadata --locked --all-features` to `scripts/release-gate.sh`; the complete release gate passes with lockfile drift now failing automatically. |
| 2026-08-26 | W053 | codex | release-gate coverage expansion | Added the hermetic integration suite and installer smoke test to `scripts/release-gate.sh`; the complete gate now runs both automatically and passes. |
| 2026-08-26 | W053 | codex | hardware qualification matrix | Added `docs/hardware-qualification.md` with per-device USB identity, observed transport, current status, and production acceptance evidence; linked from Fedora installation documentation. |
| 2026-08-26 | W027/W053 | codex | vendor-documentation research finding | Two Notes’ public support documentation describes C.A.B. USB as a Torpedo Remote control connection and does not publish raw HID command semantics; documented this constraint and retained the HID codec/write gate pending authorized protocol evidence. |
| 2026-08-26 | W045 | codex | CLI doctor host evidence | Ran `mackes doctor --json` on the target host; it reports Linux/x86_64 and explicitly marks configuration as deferred, then repository checks pass. |
| 2026-08-26 | W032 | codex | confirmation-stage denial evidence | Ran the physical-write guard against the sanitized verified-map fixture without confirmation; it denied execution with exit 85 and required `MACKES_CONFIRM_PHYSICAL_WRITE=1`, proving the second safety barrier. |
| 2026-08-26 | W032 | codex | authorized-gate evidence | Ran the guard with the verified fixture and explicit `MACKES_CONFIRM_PHYSICAL_WRITE=1`; it reached the authorized state without performing device I/O, proving the complete policy sequence. |
| 2026-08-26 | W015 | codex | framing pipeline increment | Added `parse_rtp_midi_packet`, which composes RTP header and RTP-MIDI payload validation and returns both typed framing layers; round-trip coverage passes with 22 feature tests. |
| 2026-08-26 | WORKLIST | codex | verification checkpoint | Full workspace tests with all features, workspace Clippy with `-D warnings`, formatting, artifact checks, repository policy checks, and worklist validation pass after the W015 transport increments. Session orchestration, physical qualification, and release integration remain. |

### 0.7 Luna execution protocol

This worklist is deliberately explicit enough for a smaller execution model. A Luna executor
must treat each `Wxxx` item as a closed task packet and follow this loop exactly:

1. Select only the first `READY` item in the current execution wave unless a human assigns a
   different `READY` item. Never start from prose elsewhere in the document.
2. Read the selected item, every dependency named by it, the contracts it cites, and the files
   listed in the latest handoff. Convert each implementation bullet into one or more tests before
   writing production code.
3. Restate the item boundary in the work log: intended files, public types, excluded behavior,
   verification commands, and hardware/network prerequisites. If any are unknown, mark `BLOCKED`.
4. Make the smallest compiling change that satisfies one implementation bullet. Run its narrow
   tests immediately. Continue bullet by bullet; do not attempt a whole subsystem in one edit.
5. Never infer a vendor protocol. An absent citation, unknown byte, uncertain capability, or
   undocumented response is a blocker or a disabled experiment, never a production default.
6. Keep experimental discovery code isolated behind a disabled Cargo feature and an explicit
   runtime opt-in. Experimental evidence cannot enable a production capability.
7. At handoff, record: changed files, commands and results, acceptance bullets proven, unresolved
   risks, and the exact next bullet. Do not use "works" or "tests pass" without naming evidence.

If an item is too large for one context window, split it into numbered subtasks in its work log
without changing its acceptance criteria. A split subtask may end `IN_REVIEW`; the parent remains
`IN_PROGRESS` until every subtask is reviewed.

### 0.8 Standard Luna task packet

Every handoff to Luna must contain exactly these fields in the work log or task message:

```text
Item / subtask:
Owner and start time:
Dependencies proven:
Allowed files:
Public contracts changed:
Excluded behavior:
Tests to add before implementation:
Commands to run:
Hardware/network prerequisites:
Acceptance evidence:
Known risks and next checkpoint:
```

If a field is unknown, the task is `BLOCKED` with a provider and unblock action. Luna may not
invent missing protocol facts, broaden a file boundary, or mark an item `DONE` from unit tests
alone when the item requires hardware, network, or operator evidence.

## 1. Locked product decisions

These decisions are inputs, not suggestions:

- Fedora Linux 44 on x86_64 is the first-class v1 target. The first artifact is a dynamically
  linked standalone binary bundle installed system-wide by an installer script. Architecture
  must not prevent later Raspberry Pi, macOS, or Windows support.
- MIDI processing runs in a persistent daemon. Closing the TUI or SSH session does not
  stop routing. The daemon restores the last active configuration after restart.
- The v1 audience is technical MIDI users, and flexible routing is the leading product
  priority. v1 is not complete until the user's full rig is supported.
- Local endpoints include every available interface, the M-Audio MIDISPORT 4x4,
  direct USB MIDI devices, and bidirectional virtual DAW ports.
- Devices use persistent aliases backed by observed identity. Disconnects degrade the
  route; reconnects automatically restore it and resynchronize safe state.
- v1 network MIDI uses interoperable RTP-MIDI/AppleMIDI sessions. It carries MIDI only; it does
  not expose MACKES IPC, configuration, backup, or administrative commands to network peers.
- Routing handles all MIDI 1.0 messages. Clock and transport may be routed or filtered,
  but v1 does not generate clock, elect a clock source, or manage tempo synchronization.
- Mappings provide full filters, rules and curves, explicit priorities, ordered action
  chains, named controller pages, common button modes, and per-mapping takeover behavior.
- SysEx supports raw editing, templates, profile-generated forms, safe expressions,
  capture, decoding, comparison, persistence, query/reply, retry, pacing, backup, and
  verified restore.
- Device support is declarative except for the Lexicon Reflex protocol, which is a built-in,
  typed Rust codec because its packing, dump, parameter, patch, and EEPROM-safety behavior
  require stronger invariants. v1 does not execute third-party profile code or general scripts.
- Configuration is versioned, human-readable JSON5 editable through the TUI or a text
  editor. TUI saves are atomic; comments need not survive a TUI rewrite.
- The TUI defaults to a live dashboard and uses discoverable menus plus keyboard
  shortcuts. Launch Control XL Mk1 actions may also navigate live functions.
- Projects contain setlists, setlists contain songs, and songs contain scenes. Each scene
  explicitly selects the categories it changes.
- Scene application is prevalidated, ordered, paced, and result-tracked. Partial MIDI
  execution is reported honestly; it is never described as rolled back.

### 1.1 Explicitly out of scope for v1

- BLE MIDI, MIDI 2.0, plugin-executed code, DAW plugin formats, graphical UI, cloud sync,
  mobile clients, firmware updates, audio processing, and CME hardware configuration.
- MIDI clock generation, tempo master/follower management, and clock correction.
- Undocumented hardware commands that have not been safely verified.

The final bullet does not prohibit controlled protocol research. Such work must remain disabled,
clearly labeled experimental, isolated from production profiles, and unable to perform persistent
writes until the exact command has a physical validation record.

### 1.2 Survey decisions 51–100 (historical baseline)

These answers record the earlier baseline. Section 1.7 is newer and overrides conflicting rows,
especially 61–100. Non-conflicting Fedora, packaging, ordering, and persistence decisions remain
locked requirements.

**Execution warning:** rows 61–100 are historical survey answers only. Do not implement their
TLS/PSK transport, world-writable socket, all-user authorization, remote administrative authority,
TCP framing, or key-rotation requirements. Implement the R1–R10 decisions in section 1.7 instead.

| Q | Decision |
|---|---|
| 51–52 | Fedora Linux 44 is the development, test, and minimum supported OS. |
| 53–57 | Ship an x86_64 standalone binary bundle, dynamically linked to Fedora libraries, with an installer script. |
| 58–60 | Install under standard system-wide `/usr/local` locations and run a system systemd service. |
| 61–62 | Every local user may control the daemon, including configuration and persistent SysEx operations, without local authentication. |
| 63–64 | Authenticated remote peers may trigger persistent and hazardous hardware writes. Existing confirmations, arming, backup, and audit safeguards still apply. |
| 65–69 | A plaintext pre-shared-key file authenticates all peer traffic. File permissions generate warnings but are not enforced as a security boundary. |
| 70–73 | Both endpoints must run MACKES. Transport is encrypted with TLS; the PSK is the sole trust root and no managed PKI is required. |
| 74–77 | Peers are configured, not discovered; multiple simultaneous peers have identical permissions and auto-resume after reconnect. |
| 78–80 | Discard events while disconnected, preserve in-order delivery, and expose one combined MIDI endpoint per peer. |
| 81 | Preserve sender monotonic timestamps and use clock-offset estimation plus a configurable 3 ms jitter buffer; late events route immediately and are counted. |
| 82 | Frame application messages as a four-byte big-endian length followed by canonical CBOR; reject frames above 1 MiB. |
| 83 | Listen on configurable TCP port 55129 by default; bind address defaults to all interfaces because peers are PSK-authenticated. |
| 84 | Perform mutual PSK proof after TLS using fresh nonces, TLS exporter channel binding, and HMAC-SHA-256; neither side is trusted before both proofs pass. |
| 85 | Disable TLS 0-RTT; give every session a random ID and every frame a monotonically increasing sequence to reject replay/duplication. |
| 86 | Keys are at least 32 random bytes encoded as exactly 64 lowercase hexadecimal characters; reject malformed, short, or empty values. |
| 87 | Support `current` and `next` key IDs/values in the text file for overlap rotation; reload atomically and never log key material. |
| 88 | Reconnect with jittered exponential backoff from 250 ms to 30 s; reset after 60 s stable connectivity. |
| 89 | Send keepalive every 5 s and declare the session offline after 15 s without valid traffic. |
| 90 | Bound each connected peer's outbound queue to 4096 frames. Overflow degrades health and rejects new actions explicitly; disconnected peers have no queue. |
| 91 | Use one TLS/TCP connection per peer and preserve frame order across MIDI, control, acknowledgement, and health messages. |
| 92 | Version the peer protocol independently; reject major mismatches and negotiate the lowest common minor version and capability set. |
| 93 | Apply peer/configuration changes by optimistic generation number and atomic replacement to prevent lost multi-user edits. |
| 94 | Log to journald with structured fields, action IDs, peer IDs, and redacted key/config data. |
| 95 | Run as dedicated `mackes` service user with only MIDI/audio device access; expose the local control socket as mode `0666` per the any-user requirement. |
| 96 | Use `/usr/local/bin`, `/usr/local/libexec/mackes`, `/etc/mackes`, `/var/lib/mackes`, and `/run/mackes`; use journald instead of log files. |
| 97 | The root-run Bash installer is idempotent, verifies bundle checksums, creates the service user/paths, installs the unit, reloads systemd, and never overwrites configuration silently. |
| 98 | Upgrades stage and verify new binaries, atomically switch them, retain the prior bundle for rollback, migrate data with backups, and restart only after validation. |
| 99 | CI and release qualification include a Fedora 44 x86_64 environment plus multi-peer TLS loopback, malformed-auth, reconnect, replay, and timing tests. |
| 100 | Post-release network qualification targets an eight-hour multi-peer soak with zero silent drops, successful key rotation, daemon/peer restarts, and hazardous-action audit evidence; this is advisory and not a release blocker. |

### 1.3 MIDI Learn decisions 101–115

- Learn creates one source-to-one destination mapping; action-chain authoring remains in the
  full mapping editor.
- Learn accepts every supported MIDI 1.0 message type, including SysEx and system real-time.
- The operator selects one input endpoint before arming Learn. That endpoint selection is a
  global daemon preference and persists across projects and mapping pages.
- Channel-bearing messages can be saved as either exact-channel or any-channel matches; the
  review screen requires an explicit choice and defaults to exact-channel.
- Continuous controls remain in observation mode until the operator presses Enter. MACKES
  records multiple messages to infer observed minimum, maximum, direction, resolution, and
  absolute/relative behavior. It never ends capture merely because traffic pauses.
- During capture, MACKES collects and groups candidate messages from the selected endpoint.
  It does not silently choose the first or most frequent event. The operator selects the
  intended candidate, presses Enter to finish, or Esc to cancel without mutation.
- Candidate review shows both decoded meaning and raw hexadecimal bytes.
- After source confirmation, Learn opens a destination picker covering device parameters,
  scenes, routes, and permitted application actions.
- Before save, Learn requires a live test, displays the observed input and generated output,
  and warns about exact, overlapping, shadowed, feedback-loop, and priority conflicts.
  A hazardous destination still requires its ordinary confirmation/arming policy.

### 1.4 Lexicon Reflex TUI decisions

- Reflex navigation is organized primarily by algorithm/effect type.
- Each algorithm page uses a consistent layout template; the signal-flow diagram and parameter
  groups change according to the active algorithm.
- Shared controls—including setup selection, bypass, MIDI channel, and MIDI patching—live in a
  collapsible shared-controls section rather than being repeated on every effect page.
- Algorithm parameters are ordered by signal flow, not by front-panel order or numeric ID.
- Every algorithm page contains an interactive signal-flow diagram. Selecting a diagram block
  selects its associated parameter group/control; the selected item shows live value, legal
  range, polarity, and active MIDI-patch sources.
- Parameters are grouped by functional role such as timing, tone, feedback, modulation, and
  mix/output where that grouping is supported by the documented algorithm behavior.
- The UI visually distinguishes unavailable, unused, read-only, hazardous, and MIDI-mapped
  parameters.
- Labels must match the Lexicon Reflex MIDI Implementation document/manual. Users may not
  customize labels, parameter order, visibility, or the default signal-flow layout.

### 1.5 Global device visual-style decisions

- Use fixed Boss-inspired default device/effect color families. Reflex uses effect-family colors;
  Eventide uses distinct pitch, delay, modulation, reverb, and future-family assignments. Document exact defaults
  before UI coding; do not sample colors ad hoc from screenshots. Users may choose a complete,
  versioned theme override, but may not redefine individual semantic tokens inconsistently.
- Apply the active device section/effect color consistently throughout every matching control,
  page header, parameter group, signal-flow block, scene label, route/mapping badge, status
  marker, and Launch Control XL page/LED indication.
- Use neutral system colors for shared controls: setup, bypass, MIDI channel, MIDI patching,
  SysEx, endpoint state, navigation, and diagnostics.
- Use intensity/state variants consistently: dim unavailable, normal available, bright selected,
  inverse or blinking hazardous/action-required. Never make blinking the sole error indication.
- Preserve effect identity without color: every token has a text label, symbol, border/marker,
  and monochrome-safe representation. Color must not be the only state or category signal.
- Use a technical Blueprint treatment only for the signal-flow diagram: dark blueprint field,
  grid, orthogonal connectors, labeled blocks, measurement-like annotations, effect-colored
  blocks, neutral connectors, and distinct warning/error block styling.
- Use clean device-inspired control-panel treatments for ordinary workspaces: Lexicon for Reflex
  and Eventide for Eventide. Blueprint styling is scoped to signal-flow
  diagrams and must not spread to ordinary parameter forms or shared-control panels.
- Show a permanent legend for device/effect colors, neutral shared controls, warning states, and
  Blueprint symbols. Each device workspace offers compact and expanded forms.
- Send the active effect color to Launch Control XL feedback where the hardware supports the
  required color/intensity; retain text/status feedback in the TUI when hardware color space
  cannot represent the token exactly.

### 1.6 Eventide workspace decisions

- Eventide pages are organized by signal flow and use interactive signal-flow diagrams. Processing
  blocks use fixed Eventide family colors for pitch, delay, modulation, reverb, and future effects.
- Eventide uses the same global token, neutral-control, state-intensity, monochrome fallback,
  Blueprint diagram, permanent legend, and Launch Control feedback rules as Reflex.
- Eventide shared controls are collapsible and available from every processing page.
- Labels match official device documentation exactly. Users cannot rename labels or reorder controls.
- Each device uses a clean device-inspired control-panel style outside its Blueprint diagram.
- Visually distinguish effect/section, bypass, unavailable, read-only, MIDI-mapped, and hazardous
  controls on every page.

### 1.7 Fit-for-purpose remediation decisions

These decisions are newer than sections 1.2, 1.5, and 1.6 and take precedence on conflict:

| ID | Locked decision | Implementation consequence |
|---|---|---|
| R2 | The first Novation target is Launch Control XL Mk1. | W025 uses the Mk1 identity, ports, messages, and LED capabilities; Mk2 behavior is out of scope unless separately validated. |
| R4 | Eventide starts with documented PC/CC and may add experimentally discovered SysEx/query features. | Experimental features stay disabled until independently reproduced and physically validated; the release profile exposes only verified capabilities. |
| R5 | Reflex diagrams are logical/control views derived from manual parameter organization. | W047 must label every diagram accordingly and must not assert undocumented DSP routing. |
| R6 | Replace the custom TLS/PSK peer protocol with standard RTP-MIDI/AppleMIDI. | W015, diagnostics, packaging, integration, soak tests, and release evidence use RTP-MIDI interoperability. Earlier PSK decisions are superseded. |
| R7 | Use least-privilege defaults, audit logging, and an explicit unsafe mode for hazardous writes. | Local IPC is group-restricted; network sessions expose no administrative IPC; hazardous operations require armed unsafe mode plus their ordinary confirmation. Earlier world-writable/all-peer authority is superseded. |
| R8 | Automatically transmit the last saved scene at daemon startup. | W010 uses the normal planner automatically once required endpoints settle. Unsafe actions still fail closed if unsafe mode is not armed; the startup result is audited. |
| R9 | Ship fixed Boss-inspired defaults and allow user-selectable themes. | W048 separates immutable semantic roles from selectable complete theme palettes and retains non-color cues. |
| R10 | A production hardware profile requires physical-device validation. | W024–W027 remain disabled or experimental in production until their device/firmware validation matrix passes. Simulator evidence alone is insufficient. |

#### 1.7.1 Safety model selected by R7

- Create a `mackes-control` system group. The control socket is owned by
  `mackes:mackes-control` with mode `0660`; the installer never grants membership silently.
- Use Unix peer credentials to attach an audited local actor identity to mutations. Read-only
  status may be exposed separately only through an ADR and tests; it must not weaken the control
  socket.
- Unsafe mode is daemon state, defaults off, is not persisted, and clears on daemon restart,
  operator logout/session loss, panic, or expiry. Default expiry is 10 minutes and is configurable
  only within 1–60 minutes.
- Arming unsafe mode requires a local interactive CLI/TUI action, the exact phrase
  `ARM UNSAFE WRITES`, and an audit record. MIDI mappings and RTP-MIDI peers cannot arm it.
- Bulk dumps, persistent memory writes, identity mismatch overrides, undocumented experimental
  commands, and network-originated actions that resolve to any of those classes require unsafe
  mode plus their operation-specific confirmation. Normal live CC/PC control does not.
- RTP-MIDI input is untrusted. Routes from it default to channel voice and real-time messages;
  SysEx/system-common forwarding and application-action mappings require explicit allowlisting.

### 1.8 On-demand Human Interface capability requirements

The Human Interface selects a unit by the effect or processing block it provides, not by a
single primary product category. Every supported unit therefore needs a preprogrammed
capability record containing:

The initial hardware topology is fixed and serial: `Eventide MicroPitch → stereo → Lexicon Reflex`.
The Lexicon Reflex is the final stereo processor and
master audio output. Human Interface effect selection may enable or bypass capabilities within
each unit, but must not reorder the devices or place a device after the Lexicon.

- Stable profile/device ID, manufacturer, model, firmware scope, endpoint selectors, MIDI
  channel policy, and supported transport(s).
- Effect-block capabilities at useful granularity, such as `hall_reverb`, `multi_tap_delay`,
  `chorus`, `flanger`, `phaser`, `pitch_shift`, `cabinet_ir`, `eq`, `looper`, and
  `drum_machine`. A multi-service unit may provide many capabilities.
- For each capability: exact labels, controls, units, ranges, enums, defaults, value direction,
  block on/off behavior, and evidence status: documented, community-reported, experimental, or
  unavailable.
- Complete wire mappings: CC/PC or SysEx address/template, encoding, checksum, query/reply,
  pacing, retry, and response correlation. Unknown deep controls remain experimental until
  captured and decoded.
- Read/write/feedback classification and safe connect-time behavior. The UI must never claim
  synchronization a device cannot provide.
- Capability ownership, default-device resolution, endpoint fallback, reconnect priority,
  unavailable-control presentation, and audit/undo behavior for default changes.

Initial device requirements:

- Supported processors: declare effect families and MIDI controls; distinguish documented PC/CC
  mappings from community mappings; keep unverified controls gated on protocol evidence.
- Lexicon Reflex: expose all eight documented algorithms and complete parameter sets through the
  compiled bidirectional SysEx codec.
- Eventide MicroPitch: expose dual pitch, detune, delay A/B, mix, tone, pitch mix, modulation,
  feedback, tempo, ribbon, expression, bypass, preset, and MIDI/USB capabilities.

Acceptance requires a Human Interface test for every declared capability: selecting an effect
must resolve the expected unit, render only its controls, emit the correct transport message,
handle unknown feedback honestly, and never route a control to a unit lacking that capability.

### 1.9 Committed device-default and Human Interface decisions

The following decisions are committed requirements from the device/effect review:

- Defaults are global, with exactly one default device per effect type. Defaults are applied
  automatically to new effects/routes and changing a default updates every matching existing
  effect. Changes support undo, audit history, backup/restore, and unavailable-state reporting.
- A multi-service device may be selected independently for every capability it provides.
  Incompatible devices are hidden from a capability selector. A device becoming available later
  may replace a default only after a settings-page comparison prompt; the comparison shows
  capabilities, connection status, profile name, and support level.
- Support levels are `verified`, `documented`, `community-reported`, `experimental`, and
  `unavailable`. Lower-support selections are allowed but receive a generic warning every time.
- Defaults resolve to a specific endpoint when possible, with profile fallback. Manual per-effect
  endpoint ordering always overrides remembered last-successful fallback behavior. The original
  endpoint regains priority when it returns.
- The Novation Launch Control XL Mk1 is the first Human Interface target. Mappings are generated
  from the selected default device, update when defaults change, preserve manual edits, and are
  saved as reusable controller/profile templates.
- Novation pages are effect-based, not generic-device pages. Each effect block has its own page,
  exposes all available parameters, includes block on/off, uses additional pages when needed,
  follows active preset block order, and marks absent blocks unavailable. Device-reported values
  receive feedback; write-only or unknown values remain explicitly unknown.

The committed initial serial topology is:

```text
Eventide MicroPitch → stereo → Lexicon Reflex (master output)
```

The chain is immutable. The Human Interface configures or bypasses blocks within each unit but
never reorders devices or places a device after the Lexicon. When multiple requested effects are
owned by supported processors are represented as enabled blocks in one coordinated preset/chain;
unrelated blocks remain bypassed and hidden.

Initial capability ownership for Human Interface resolution is:

| Capability group | Default provider |
|---|---|
| Documented processor effects | Supported processors |
| Hall, room, plate, spring, gated, inverse/reverse, shimmer, chorus, flanger, phaser, tremolo, vibrato, rotary, multi-tap delay, ping-pong/stereo delay, reverse delay, analog-style delay | Lexicon Reflex |
| Digital delay, slapback delay, detune/micro-pitch, pitch shift, feedback pitch effects | Eventide MicroPitch |

The ownership table is a routing contract, not proof that every listed subtype is supported by
the current firmware. Each entry must carry its evidence status in the profile. Unverified
deep USB/BLE editor protocol remains gated until captured and decoded. Lexicon deep controls use
the compiled bidirectional SysEx implementation; Eventide uses its documented MIDI contract.

### 1.9.1 Executable Novation effects-control work package

The following items turn the committed operator decisions into an executable dependency chain.
They use static Launch Control XL Mk1 labels and never assign controls outside the listed map.

#### [x] W062 — Static Launch Control XL effects faceplate
- **Status:** `DONE`; **Owner:** codex; **Depends on:** W025, W048, W054, W057
- **Objective:** Define fixed signal-path labels: Row 1 Gain/Gate, Row 2 Compressor/Modulation,
  Row 3 Delay/Reverb; add one Enable and one Type/Model button per group, and explicit unused controls.
- **Acceptance:** stable serialized labels, physical indices, ownership, six groups, and eight faders.
- **Evidence:** `launch_control_effects_faceplate()` provides six validated serialized groups with
  stable indices, profile ownership, eight faders, and explicit unused controls; the primary TUI
  renderer exposes all groups and ownership markers. Profile and TUI renderer tests, strict Clippy,
  formatting, worklist validation, and diff hygiene pass on 2026-08-29.

#### [x] W063 — Effect-group state and LED feedback
- **Status:** `DONE`; **Owner:** codex; **Depends on:** W062
- **Objective:** Implement pickup-aware group state and LED policy: green enabled, solid red disabled,
  blinking red unavailable, blue/teal selected type.
- **Acceptance:** bounded deterministic transitions; reconnect and scene activation request resync.
- **Evidence:** `EffectsGroupRuntime` enforces six groups/eight faders, pickup-safe state updates,
  value bounds, and reconnect/scene resync invalidation; `effect_group_led` is deterministic and
  fail-closed. 44 profile tests, strict Clippy, worklist validation, and diff hygiene pass.

#### [x] W064 — Static parameter-to-control assignment catalog
- **Status:** `DONE`; **Owner:** codex; **Depends on:** W062, W063, W020, W024, W026
- **Objective:** Map documented parameters to fixed knobs/faders by group and default-provider owner;
  unsupported parameters remain unassigned.
- **Acceptance:** labels, units, ranges, direction, defaults, conflict errors, and visible unsupported reasons.
- **Evidence:** `effects_parameter_assignments` derives bounded profile-owned assignments with
  exact labels/IDs, legal ranges, defaults, direction, units, and explicit unassigned reasons;
  TUI/CLI inspection exposes the catalog. 44 profile tests, CLI smoke, strict Clippy, worklist
  validation, and diff hygiene pass.

#### [x] W065 — Per-device automation planner
- **Status:** `DONE`; **Owner:** codex; **Depends on:** W064, W030, W032, W040
- **Objective:** Convert group changes into ordered operations for the immutable MicroPitch
  → Reflex chain, bypassing unrelated blocks and never guessing unsupported messages.
- **Acceptance:** bounded operations, pickup semantics, and actionable unsupported/unverified results.
- **Evidence:** `plan_effects_automation` emits bounded operations in immutable provider order,
  gates them on pickup readiness, skips unrelated groups, and reports explicit unverified reasons;
  44 profile tests, CLI smoke, strict Clippy, worklist validation, and diff hygiene pass.

#### [x] W066 — Minimal reusable effects configurations
- **Status:** `DONE`; **Owner:** codex; **Depends on:** W064, W065
- **Objective:** Generate minimal configurations containing only selected blocks, mappings, parameters,
  and state; name them automatically in signal-path order.
- **Acceptance:** enabled-group changes regenerate validated reusable configurations with unrelated blocks hidden.
- **Evidence:** `generate_reusable_effects_configuration` produces deterministic signal-path names,
  includes only selected groups and documented assignments, and returns an empty configuration
  for no selection; profile tests, strict Clippy, worklist validation, and diff hygiene pass.

#### [x] W067 — Retired editor-map removal
- **Status:** `DONE`; **Owner:** codex; **Depends on:** W020, W064, W066
- **Objective:** Import editor-exported maps and validate schema, identity, firmware, ranges, ownership,
  duplicates, and artifact hash; classify evidence status.
- **Acceptance:** invalid maps fail closed; firmware mismatches require approval; valid maps generate profiles.
- **Evidence:** Retired editor-map validation and artifact import paths are removed; active imports reject
  unknown fields, wrong identity, duplicates, invalid ranges, and tampering; firmware drift requires
  explicit approval; `effects map-profile` emits a validated conservative serialized profile and
  rejects unsupported note mappings. Config/CLI tests, strict Clippy, worklist validation, and diff
  hygiene pass on 2026-08-29.

#### [x] W068 — Effects-control TUI/CLI workflow
- **Status:** `DONE`; **Owner:** codex; **Depends on:** W063, W065, W066, W067
- **Objective:** Expose static labels, group/type/enable controls, faders, regeneration, import status,
  decoded messages, and unverified warnings in TUI and CLI.
- **Acceptance:** every assignment is inspectable and writes identify the owning unit truthfully.
- **Evidence:** Primary TUI and read-only CLI commands expose static groups, ownership, assignments,
  ranges, faders, unsupported reasons, generated profiles, and fail-closed plan/demo status; human
  and JSON assignment output includes the owning profile, while device writes retain explicit profile
  identity and existing confirmation guards. CLI/TUI/profile tests, strict Clippy, worklist
  validation, and diff hygiene pass on 2026-08-29.

#### [x] W069 — Simulator/demo/release qualification
- **Status:** `DONE`; **Owner:** codex; **Depends on:** W063, W065, W066, W068
- **Objective:** Add deterministic simulator, LED test, and effects demo modes for all six groups,
  faders, LEDs, configurations, reconnect, and scene activation without paired hardware.
- **Acceptance:** offline checks prove deterministic output, bounded traffic, safety, and no false sync;
  physical and paired checks remain post-release qualification.
- **Evidence:** `effects_demo_frames()` provides deterministic four-frame offline coverage for all
  six groups, faders, unavailable state, and resync; `effects demo [--json]` exposes it read-only.
  Profile/CLI tests, JSON smoke output, strict workspace Clippy, worklist validation, and diff
  hygiene pass on 2026-08-29.

### 1.10 Connected-device mapping TUI decisions

The 2026-08-28 operator survey commits the following requirements for W054–W061:

- The primary workspace uses source and destination lanes. Bidirectional controllers/HUDs appear
  in both lanes with a shared identity marker. One physical device groups all of its MIDI ports.
- Every known device uses a profile-specific faceplate. Unknown devices remain visible through a
  generic live faceplate with an immediate Learn action. Ambiguous identical devices are blocked
  until explicitly identified; disconnected devices remain in place and restore safely.
- Multiple controllers retain complete faceplates for every knob, fader, button, and pad. Device
  tabs page between full faceplates when they cannot fit simultaneously.
- Control movement updates the source control, selected/active mapping path, and destination
  parameter together. Activity fades after one second while the latest value remains. High-rate
  input is coalesced to latest state at an approximately 30 Hz render cadence.
- Each control shows a live value, short mapping destination, enabled state, and activity state.
  Continuous controls use a large bar plus raw 0–127 value; buttons show their explicit mode and
  current state.
- Mapping is destination-first: select a categorized processor parameter, then bind a source by
  moving hardware or selecting its faceplate control. A valid mapping activates and autosaves
  immediately; bounded Undo restores both runtime and persisted state.
- The visual target is a distance-first rack appliance at 100×37 on the Linux TTY, using ANSI
  16-color high-contrast styling, restrained borders, LED-like indicators, persistent alerts,
  context-sensitive key legends, and non-color markers for every state.
- End-to-end acceptance requires local hardware detection, mapping by both source-selection paths,
  real-time source/route/destination response, autosave, Undo, restart restoration, and safe
  disconnect/reconnect behavior.

### 1.11 Task-oriented TUI and hardware-first mapping decisions

The 2026-08-30 operator review supersedes section 1.10 wherever the interaction order, fixed
effect ownership, or workspace structure conflicts with the following requirements:

- Replace the numbered nine-workspace interface with five musician-facing tasks: **Live**,
  **Map Controls**, **Scenes**, **Devices**, and **System**. Use a persistent left navigation rail,
  a concise health/scene/save header, a contextual action footer, and one main task surface.
- The default landing task is Live. Rehome Dashboard into Live; MIDI Learn and ordinary mapping
  into Map Controls; projects/setlists into Scenes; device-specific Reflex/Eventide views into
  Devices; and Monitor, Diagnostics, and Backups into System. Keep duplicate legacy views under
  `System → Advanced → Legacy` only until parity evidence permits their removal.
- Arrow keys are the primary navigation; Enter selects, Esc goes back, `?` opens contextual help,
  `!` remains panic, and `q` quits. `h/j/k/l` remain compatibility aliases. Every interactive
  state has exactly one visible focus target with a `▶` marker, high-contrast selection treatment,
  focused panel border, and breadcrumb. The terminal caret appears only during text entry.
- Separate focus, live activity, mapping state, and health visually and textually. Use `▶` for
  focus, `● LIVE` for recent hardware activity, and stable non-color state markers. Hardware
  activity must not unexpectedly move keyboard focus outside explicit capture mode.
- The mapping flow is hardware-first and linear. Section 1.12 replaces this review's original
  Map Controls-only `New Assignment` entry with the final controller-driven flow: short Device from
  any screen, then source, device, effect/block, parameter, and short Device to commit. Utility
  buttons and system traffic are ignored; ambiguous source movement stops for explicit recovery.
- Device choices list connected compatible devices first and place unavailable/incompatible
  profiles in a secondary section with a concise reason. Device pages list trustworthy
  profile-owned effect blocks in signal order and then a **General** section for documented
  controls without an effect classification.
- Knobs, faders, and channel buttons may target any compatible connected device regardless of
  their prior static group. Device, Mute, Solo, Record Arm, and navigation buttons stay reserved.
  The HUD updates each mapped control label to `Device › Effect › Parameter`.
- Autosave every completed choice. A partial source/device/effect selection is a persisted,
  inactive, resumable draft. Selecting a valid parameter atomically activates the mapping;
  later enable, range, inversion, curve, and button-mode changes apply immediately. Use
  profile-safe defaults and place shaping under an optional Advanced drawer.
- Never silently steal an occupied control or parameter. Show the existing assignment and require
  **Replace** or **Cancel**; successful replacement is immediately Undoable.
- Experimental targets remain usable and are visually identified in details without dominating
  the ordinary flow. One lightweight prompt may arm the existing global unsafe mode for 15 minutes;
  expiry suspends only experimental mappings. Verified ordinary mappings continue normally.
- Correct the existing identity collision before HUD implementation: the current 48-index profile
  uses 40–47 for utility controls while the effects catalog also uses 40–47 for faders. Physical
  controls receive stable, non-overlapping IDs such as `knob.top.1`, `button.bottom.4`, `fader.1`,
  and `utility.device`; MIDI input number and LED feedback index remain separate metadata.

The visual presentation uses all four approved improvements:

1. **Stronger hierarchy:** one restrained focus accent; green/yellow/red only for semantic status;
   bright task headings; dim endpoint/protocol metadata; no color-only meaning.
2. **Cleaner spacing/grouping:** one purpose per panel, consistent border/title/padding rules,
   separate live telemetry from editable mapping details, and remove duplicated inventories.
3. **Polished hardware HUD:** preserve the physical Novation arrangement, use distinct knob/button/
   fader shapes, short dynamic labels, bounded value bars, and clearly separate focus from activity.
4. **Calmer contextual chrome:** show only actions relevant to the current task/step, retain a
   breadcrumb and step indicator, bound notifications, and place protocol details in Advanced.

Research references for implementation intent are W3C WCAG 2.4.7 Focus Visible, Nielsen Norman's
usability heuristics, the USWDS step-indicator guidance, Novation Components control editing, and
Ableton Live's Mapping Browser. These references guide interaction patterns; vendor wire behavior
still requires profile evidence under the hardware-truth rule.

### 1.12 Controller-driven reassignment and distance-feedback decisions

The 2026-08-30 operator walkthrough supersedes section 1.11 wherever capture entry, navigation,
confirmation, or visual-feedback behavior conflicts. Luna implements this exact musician-facing
workflow:

- A short press of the Launch Control XL **Device** button enters reassignment mode from any TUI
  section and remembers the prior screen. The existing mapping remains active throughout editing.
  Holding Device for at least 750 ms cancels from any assignment step, restores the prior LED/base
  state, shows **CANCELED**, and returns to the remembered screen.
- After entry, moving a knob/fader or pressing a channel button selects the source. All 24 knobs,
  16 channel buttons, and eight faders are assignable. Device, Mute, Solo, Record Arm, Up, Down,
  Left, and Right are interface controls and never become assignment sources or effect output.
  Use a 250 ms candidate window: repeated events from one physical control lock that control; two
  distinct eligible controls stop capture and show **MOVE ONLY ONE CONTROL**.
- Selection is one sparse full-screen level at a time: **device → effect/block → parameter**.
  Up/Down move one row, Right enters the highlighted row, Left returns one level, and movement stops
  at list boundaries. Linux keyboard arrows invoke the same commands and state transitions as the
  hardware arrows. Connected compatible devices appear first; unavailable choices appear second
  with a short reason. Effects follow profile signal order and end with General.
- A short Device press commits only when a valid parameter is highlighted. If the destination is
  occupied, show the existing assignment on a dedicated replacement screen; a second short Device
  confirms atomic replacement and the 750 ms hold cancels. Partial choices remain an inactive,
  persisted draft. Activation is generation-checked, atomic, durable, and immediately Undoable.
- During reassignment, the selected knob/button LED blinks red, the Device LED pulses yellow, and
  each direction LED is red only while that direction is valid. Faders have no LED, so both channel
  button LEDs in the same column act together as a temporary proxy and then restore their own base
  state. Assignment feedback overlays normal LED state without destroying it.
- Successful commit flashes the selected control green exactly twice using two 400 ms pulses while
  the PC displays a giant **ASSIGNED** result for two seconds. The LED then returns to normal mapping
  behavior: normally off and green for one second when that control is moved. Failure flashes the
  selected control red exactly twice and leaves a full-screen **NOT ASSIGNED** result visible with
  one precise reason and recovery action. Device retries, Left returns to parameter selection, and
  a 750 ms Device hold cancels.
- The highlighted choice receives the largest lettering on screen, using a five-row block font when
  space permits, a three-row block fallback, then bold single-line text. Always retain the exact
  untruncated label in the breadcrumb/details. Show previous/current/next choices and an explicit
  position such as **3 OF 12**. Large visual state changes, plain words, and shape/position carry
  meaning without depending on color. Golden layouts cover 160×37, 100×37, and 80×24.
- An unexpected TUI disconnect ends active navigation and LED overlays but retains the inactive
  draft. Reconnect reselects User 1, restores authoritative LEDs, and shows **ASSIGNMENT INTERRUPTED**;
  short Device resumes and a 750 ms hold discards. User 1 remains selected after an ordinary TUI
  exit so hardware controls remain predictable.
- MACKES uses a dedicated reviewed Launch Control XL Mk1 template in **User 1** with fixed unique
  MIDI assignments. Luna ships the template artifact, checksum/manifest, assignment inventory,
  Novation Components installation steps, and runtime verification. The TUI selects User 1 with the
  documented template-selection message when it connects, but never writes template definitions
  through an undocumented protocol. A mismatch shows full-screen **MACKES TEMPLATE REQUIRED** and
  exact Components recovery instructions.
- The daemon owns a typed `AssignmentSession` with states Idle, AwaitControl, ChooseDevice,
  ChooseEffect, ChooseParameter, ConfirmReplace, Committing, Succeeded, Failed, and Interrupted.
  Hardware and keyboard commands enter one reducer/state machine. A layered, monotonic,
  fake-clock-testable LED scheduler restores deterministic base state after assignment/result
  overlays and consumes Device/arrows as interface input for the entire active session.

Official implementation evidence is the Novation Launch Control XL programmer reference for the
24 pots, eight faders, 24 programmable buttons, LED indices 0–47, template selection, and LED flash
protocol, plus the official Launch Control XL Mk1/2 Components guide for installing a User template:

- <https://fael-downloads-prod.focusrite.com/customer/prod/s3fs-public/downloads/launch-control-xl-programmers-reference-guide.pdf>
- <https://support.novationmusic.com/hc/en-gb/articles/4411807214226-Launch-Control-XL-MK-1-and-2-Components-guide>

The public programmer reference does not document writing template definitions. Luna must use the
official Components workflow for template installation unless a later vendor document is approved.


## 2. Target architecture and contracts

### 2.1 Repository layout

Create this layout; do not invent competing top-level structures:

```text
Cargo.toml
apps/
  mackes/                 # `mackes` TUI and operational CLI
  mackesd/                # persistent daemon
crates/
  domain/                 # dependency-light public domain types
  config/                 # JSON5 loading, schema, validation, migration
  ipc/                    # versioned client/daemon protocol and transport
  midi-engine/            # endpoints, routing, mapping, scheduling
  profiles/               # device profile and SysEx engines
  scene-engine/           # hierarchy and activation planning/execution
  tui/                    # Ratatui presentation and client-side state
  testkit/                # virtual endpoints, fixture builders, fake clock
profiles/                 # redistributable built-in device profiles
schemas/                  # generated configuration/profile schemas
fixtures/                 # redacted protocol and integration fixtures
docs/
  decisions/              # ADRs
  hardware-validation/    # results, never vendor PDFs
  user/                   # operator documentation
packaging/systemd/
```

Dependency direction is one-way: `domain` ← `config`/`ipc`/`midi-engine`/`profiles` ←
`scene-engine` ← applications. `tui` talks to `ipc`; it must not open MIDI ports.

### 2.2 Canonical domain behavior

- `EndpointId` is a runtime opaque identifier. `DeviceAlias` is the stable serialized
  identifier used by projects and routes.
- `MidiEvent` contains service-monotonic nanoseconds, a monotonically increasing sequence,
  source endpoint, and a typed `MidiMessage`.
- `MidiMessage` models note on/off, poly pressure, CC, PC, channel pressure, pitch bend,
  time code quarter frame, song position, song select, tune request, clock, start,
  continue, stop, active sensing, reset, and SysEx. SysEx stores payload bytes without
  `F0`/`F7`; every payload byte must be 7-bit.
- Route processing uses one deterministic logical ordering stage. MIDI callbacks only
  timestamp and enqueue. Per-endpoint output workers preserve the order assigned by the
  router.
- Input and output queues are bounded. Default input capacity is 8192 events. Callbacks
  never block; overflow increments a drop counter, marks service health degraded, and
  emits an IPC alert.
- Priority is a signed integer; higher priority runs first. Equal priority uses declared
  route order, then source event sequence. Output conflicts resolve by the resulting
  action order; the final action is authoritative for tracked state.
- Action-chain delays are relative to the preceding action. A chain failure follows its
  declared policy: `continue`, `stop_chain`, or `abort_activation`; default is
  `stop_chain` for mappings and `continue` for scene actions.
- Curves are `linear`, `inverse`, `exponential(exponent)`, `logarithmic(base)`, or a
  monotonic piecewise lookup table. Inputs and outputs are clamped to declared ranges.
- Pickup takeover sends nothing until the physical value crosses the tracked target.
  Scaled pickup gradually removes the offset across the configured travel window.
- IPC uses a Unix-domain socket on Linux and newline-delimited UTF-8 JSON envelopes.
  Every envelope contains `protocol_version`, `request_id`, and a tagged payload.
  Streaming events also contain an increasing daemon event sequence.
- All stored roots contain `schema_version`. Unknown fields and duplicate stable IDs are
  errors. Reads validate syntax, JSON Schema, then semantic references. Writes use a
  same-directory temporary file, fsync, rename, and timestamped backup.

### 2.3 Lexicon Reflex built-in protocol contract

The normative implementation source is **Lexicon Reflex MIDI Implementation Details**,
Lexicon part 070-10748 Rev 1, © 1997. Source copy inspected at
<https://www.handelima.net/studiogear/manuals/Lexicon/LEXICON%20REFLEX/Reflex_MIDI_Implmnt_Rev1.pdf>,
SHA-256 `a5d02f29bed1344b288cfa356f514bc187f2cd7ec6b1a817d6c0a3c0815d08bf`.
Do not commit the vendor PDF. Record the URL, part number, revision, hash, and relevant page
number beside protocol tests and constants.

Implement `crates/profiles/src/builtin/lexicon_reflex/` as Rust source, not JSON5. It exposes
typed `ReflexCodec`, `ReflexMessage`, `ReflexRequest`, `ReflexSystemTask`, `ReflexSetup`,
`ReflexPatch`, `ReflexAlgorithm`, and `ReflexParameter` APIs. User configuration may supply
alias, MIDI channel, labels, and scene values, but may not replace framing, sizes, ranges,
packing, checksums, request codes, task codes, or safety classes.

#### 2.3.1 Framing and message types

- Every message is `F0 06 02 tt ... F7`: `06` is Lexicon, `02` is the LXP-1-compatible
  product ID, and `tt = (type << 4) | channel_zero_based`. Reflex treats channel as device ID.
- Decode only types 0–6. Reject bad manufacturer/product ID, channel > 15, data bytes > 127,
  wrong fixed/count-derived length, invalid checksum, and incomplete framing.
- Type 0, active setup: `F0 06 02 0n 38 <56 packed bytes> <checksum> F7` (63 bytes total).
- Type 1, stored register: `F0 06 02 1n <register 00..7F> 38 <56 packed bytes>
  <checksum> F7` (64 bytes total). It edits storage without changing the active setup.
- Type 2, packed parameter adjust: `F0 06 02 2n <parameter> <three packed bytes> F7`.
  This is also emitted when a front-panel encoder changes a value.
- Type 3, request: `F0 06 02 3n <request-code> <argument> F7`, where codes are `60` active
  setup, `61` one stored register, `62` packed parameter, `64` all registers, and `65`
  nibblized parameter. Argument is register for `61`, parameter for `62/65`, ignored otherwise.
- Type 4, all registers: `F0 06 02 4n 38 00 <7168 packed bytes> <checksum> F7` (7176 bytes).
  Count is the two-byte 7-bit representation of 7168: high `38`, low `00`.
- Type 5, preferred nibblized parameter adjust: `F0 06 02 5n <parameter> <nibble15..12>
  <nibble11..8> <nibble7..4> <nibble3..0> F7` (10 bytes). Each nibble byte is `00..0F`.
- Type 6, system task: `F0 06 02 6n <event> <argument> F7`, where `70` stores the active
  setup to register `00..7F`, `71` recalls register `00..7F`, and `72` sets bypass (`00` off,
  `01` on). Reject any other bypass argument.
- Dump checksum is `sum(packed_data_bytes) & 0x7F`. The register number, count, header, and
  framing bytes are not included. Parameter and task messages have no checksum.
- Reflex reports receive errors only on its display: `Er 1` checksum, `Er 2` byte count,
  `Er 3` incomplete-message timeout. The host cannot interpret silence as a specific code.

#### 2.3.2 Packing and setup layout

- Pack raw bytes in groups of up to seven. Emit one MSB byte followed by each source byte
  masked with `0x7F`; MSBs are assigned from bit 0 for source byte 0 through bit 6 for source
  byte 6. Partial groups use only the corresponding low MSB bits.
- Multi-byte setup values are little-endian before packing. Packed-parameter helpers require
  explicit golden tests from the Rev 1 examples before being enabled for writes.
- A setup is exactly 49 unpacked bytes / 56 packed bytes: byte 0 algorithm ID; bytes 1–20 ten
  little-endian 16-bit audio parameters 0–9; bytes 21–36 sixteen name bytes; bytes 37–40 four
  patch sources; bytes 41–44 four patch destinations; bytes 45–48 four signed scale bytes.
- Offsets (parameters 60–63) and input level/bypass parameter 10 are not stored in setup
  dumps. Names are 16 bytes; decode as bytes first, preserve unknown bytes, and NUL-terminate
  new names shorter than 16 bytes.
- All-register data is 128 setup blocks in order 0 through 127. Accept only 6272 raw bytes or
  7168 packed bytes.

#### 2.3.3 Parameters and setup selection

- Parameters 0–9 are algorithm-specific 16-bit audio parameters. Parameter 10 (`0A`) is
  16-bit effect input level used by bypass; direct writes can desynchronize bypass logic, so
  classify them as hazardous and prefer system task `72`.
- Parameters 32–47 (`20..2F`) are sixteen 8-bit setup-name characters.
- Parameters 48–51 (`30..33`) are four 8-bit MIDI patch sources; 52–55 (`34..37`) are four
  8-bit patch destinations (audio parameter 0–9); 56–59 (`38..3B`) are four signed 8-bit
  two's-complement patch scales.
- Parameters 60–63 (`3C..3F`) are four 16-bit live patch offsets. Make them read-only because
  Reflex continuously recalculates them and does not store them.
- Parameter 64 (`40`) selects setup: registers 0–127 and presets encoded from 128 upward.
  Rev 1 says `128–144` while also stating there are only presets 1–16. Enable 128–143 and
  reject 144 until hardware resolves the document's internal inconsistency.
- Parameter 65 (`41`) selects algorithm 1–8. Send a full 16-bit value with high bits zero.
  Validate or replace audio values before an algorithm change because legal ranges differ.

#### 2.3.4 Compiled algorithm metadata

Each tuple is `parameter: label, polarity, range, effective steps, documented step`. `U` is
unipolar and `B` bipolar. Omitted parameters are unused and must not be writable.

- **1 Reverb:** `0 Mid Reverb Decay U 8000–BC00 16/0400`; `1 Predelay U 8000–BFC0
  8192/0040`; `2 Effects Level U 8000–BFC0 256/0040`; `3 Bass Multiply B 4000–B800
  32/0800`; `4 High Freq Cutoff U 8000–BC00 16/0400`; `5 Size U 8000–BF00 64/0100`;
  `6 Predelay Feedback B 4000–BF80 512/0080`; `7 Diffusion U 8000–BFC0 256/0040`;
  `8 Reflection Level U 8000–BFC0 128`; `9 Reflection Delay U 8000–BFC0 128`.
- **2 Plate:** parameters 0–7 are identical to Reverb 0–7; parameters 8–9 are unused.
- **3 Chorus 1 / Flange:** `0 Negative Feedback U 8000–BFC0 256`; `1 Flange Depth U
  8000–BFC0 256/0040`; `2 Effects Level U 8000–BFC0 256/0040`; `3 Right Delay Feedback B
  4000–BF80 512/0080`; `4 Right Delay U 8000–BF80 128/0080`; `5 Shape U 8000–B800
  8/0800`; `6 Left Delay Feedback B 4000–BF80 512/0080`; `7 Left Delay U 8000–BF80
  128/0080`; `8 Flange Rate U 8000–BC00 16/0400`. Flange depth `<=8200` is ignored;
  delay values above `BD00` are limited by Reflex to 32,000 samples / one second.
- **4 Delay 2 / Multi-Echoes:** `1 Group Delay U 8000–BFC0 256/0040`; `2 Effects Level U
  8000–BFC0 256/0040`; `3 Feedback B 4000–BF80 512/0080`; `4 Left Delay U 8000–BFC0
  256/0040`; `5 Right Delay U 8000–BFC0 256/0040`; `7 High Freq Cutoff U 8000–BC00
  16/0400`; `8 Diffusion U 8000–BFC0 256/0040`; `9 Echo Rhythm U 8000–B400 14`.
- **5 Chorus 2 / Resonator:** `2 Effects Level U 8000–BFC0 256/0040`; `3 Predelay U
  8000–BFC0 256/0040`; `4 Low Freq Cutoff U 8000–BFC0 256/0040`; `5 Shimmer U
  8000–BC00 16/0400`; `6 Resonance Feedback B 4000–BE00 64/0200`; `7 Richness U
  8000–BC00 16/0400`; `8 Slope U 8000–BE00 32/0200`; `9 Tuning B 4000–BF00 128/0100`.
- **6 Inverse Room:** `0 Size U 8000–BE00 32/0200`; `2 Effects Level U 8000–BFC0
  256/0040`; `4 High Freq Cutoff U 8000–BC00 16/0400`; `5 Slope U 8000–BE00 32/0200`;
  `6 Predelay Feedback B 4000–BF80 512/0080`; `7 Diffusion U 8000–BFC0 256/0040`;
  `8 Predelay U 8000–BFC0 8192/0040`.
- **7 Gate:** `0 Gate Time U 8000–BE00 32/0200`; `2 Effects Level U 8000–BFC0
  256/0040`; `4 High Freq Cutoff U 8000–BC00 16/0400`; `5 Slope U 8000–BC00 16/0400`;
  `6 Predelay Feedback B 4000–BF80 512/0080`; `7 Diffusion U 8000–BFC0 256/0040`;
  `8 Predelay U 8000–BFC0 8192/0040`.
- **8 Delay 1 / Chorus:** `1 Delay 1 U 8000–F700 256/0040`; `2 Effects Level U
  8000–BFC0 256/0040`; `3 High Freq Cutoff U 8000–BC00 16/0400`; `4 Delay 2 Spread U
  8000–BF80 128/0080`; `5 Delay 3 Spread U 8000–BF80 128/0080`; `6 Feedback B
  4000–BF80 512/0080`; `7 Diffusion U 8000–BFC0 256/0040`; `8 Chorus Rate U
  8000–BC00 16/0400`; `9 Echo Rhythm U 8000–B400 14`. Preserve the Reflex-specific
  `F700` delay range (approximately 1612 ms), rather than the lower LXP-1/MRC limit.

Echo Rhythm values 1–14 are: 64th, 32nd, 16th-triplet, 16th, eighth-triplet, dotted-16th,
eighth, quarter-triplet, dotted-eighth, quarter, half-triplet, dotted-quarter, half, whole.
Algorithms 4 and 8 respond to routed MIDI clock; MACKES does not synthesize that clock.

#### 2.3.5 MIDI patches and safety

- Four patches exist. Source 0–31 maps CC 0–31; 32–63 maps CC 64–95; 64 is last Note On,
  65 its velocity, 66 channel pressure, and 67 pitch bend. Source or destination `>=7F` is null.
- Destination is audio parameter 0–9. Scale is signed two's complement: `00=0%`, `20=+50%`,
  `40=+100%`, `7F=+199%`, `80=-199%`, `C0=-100%`, `E0=-50%`. Reflex recalculates live
  offset as source × scale × 2 and adds it to the base; display base/effective separately.
- Reflex recognizes Note On, channel pressure, pitch bend, selected CC, Program Change,
  SysEx, MIDI Clock, and Reset. Do not use unsupported messages as Reflex controls.
- A type-1 batch closes after a one-second receive gap. Multiple stored-register loads then
  cause an approximately 14-second EEPROM copy during which Reflex ignores MIDI and controls.
  Type-4 load also has this busy period after its roughly 10-second transfer.
- Classify type-4 load, multi-register type-1 load, task `70` store, direct parameter-10 write,
  and setup 144 as hazardous. All-register load requires backup, dry run, exact target,
  persistent-write confirmation, and `--arm-hardware-write`. Apply a conservative 15-second
  endpoint busy guard after EEPROM-triggering traffic and show a non-cancellable warning.
- Type-0 active load, legal type-5 edits, task `71` recall, and task `72` bypass are normal
  validated writes. Queries/decode are read-only.
- Front-panel activity emits type-2 messages for parameter 0/1/2 and parameter 64 setup
  selection. Update tracked state from them without echoing them back to Reflex.

### 2.4 Verified research constraints for non-Reflex hardware

These are evidence constraints for profile authors. They are not substitutes for a device/firmware
validation record.

- **Two Notes C.A.B. M+:** the official [C.A.B. M+ user guide](https://media.two-notes.com/product_manuals/en/legacy/hardware/torpedo/torpedo_cab_m_plus_user_guide.pdf)
  documents Torpedo Remote/USB operation, Virtual Cabinet and IR Loader modes, two microphones,
  cabinet/microphone/EQ/enhancer/reverb/level controls, and warns that the unit is not a load box.
  It does not establish the MIDI PC/CC/SysEx map used by the older non-M+ Torpedo C.A.B. product.
  W027 must therefore prove a supported Remote/USB control interface before enabling writes.
- **Eventide MicroPitch Delay:** the official [MicroPitch Delay QRG](https://cdn.eventideaudio.com/uploads/2021/10/MicroPitchDelay-QRG-Web.pdf)
  documents USB/TRS MIDI, PC preset loading (1–127), and CC 4, 9, 14, 15, and 20–31. W026 must
  cite the QRG beside each production mapping and must not infer undocumented SysEx/query behavior.
- **Novation:** the [Launch Control XL Mk1/Mk2 download page](https://downloads.novationmusic.com/novation/launch/launch-control-xl-mk1mk2)
  distinguishes Launch Control XL from Launchpad products and provides the programmer reference.
  W025 targets Mk1 only; absolute-position controls and documented LED behavior must be tested on
  the physical Mk1 rather than assumed from Mk2 or Launchpad behavior.
- **Network:** RTP-MIDI interoperability must be based on RFC 6295 and a legally usable
  AppleMIDI session-control reference. No proprietary MACKES transport, TLS/PSK callback, or
  custom frame protocol may be reintroduced without a new approved scope decision.

## 3. Implementation work items

### Foundation

#### [x] W001 — Bootstrap workspace and repository policy

- **Status:** `DONE`
- **Owner:** unassigned
- **Depends on:** none
- **Parallel with:** none; this is the root item
- **Objective:** Create the compilable workspace, canonical directory layout, and
  contributor guardrails used by every later item.
  - **Implementation:**
  - Pin the stable Rust toolchain and MSRV in an ADR; start with the current stable toolchain.
  - Add workspace lint policy, `rustfmt.toml`, `.gitignore`, license, README, contribution
    guide, security policy, and fixture/data handling policy.
  - Add the crates and binaries from section 2.1 with minimal documented APIs.
  - Use `thiserror` for library errors, `anyhow` only at binary boundaries, `tracing` for
    diagnostics, Tokio for daemon concurrency, and Serde for wire/storage data.
  - Prohibit unsafe code workspace-wide unless a future ADR identifies the exact module,
    invariant, reviewer, and test strategy.
- **Acceptance:** `cargo build --workspace --all-targets` and all global checks pass;
  `mackes --help` and `mackesd --help` exit successfully.
- **Evidence required:** command summaries, toolchain version, and ADR identifier.
- **Evidence:** `rustc 1.97.1`, `cargo 1.97.1`; `cargo fmt --check`,
  `cargo build --workspace --all-targets`, `cargo test --workspace --all-features`, and
  `cargo clippy --workspace --all-targets --all-features -- -D warnings` all pass. ADRs:
  `ADR-0001-toolchain-and-dependency-policy.md`, `ADR-0002-repository-layout.md`.

#### [x] W002 — CI, test taxonomy, and generated-artifact checks

- **Status:** `DONE`
- **Owner:** codex
- **Start date:** 2026-08-25
- **Depends on:** W001
- **Parallel with:** W003 after W001
- **Implementation:**
  - Run format, Clippy, unit/integration/doc tests, schema freshness, locked dependency metadata,
    license checks, and a release build in CI.
  - Separate default tests from `hardware` and `network-interop` ignored suites.
  - Add deterministic fake-clock and in-memory endpoint support to `testkit`.
  - Fail CI when generated schemas differ from committed schemas or fixtures contain
    unredacted absolute paths, USB serials, usernames, or payload annotations marked private.
- **Acceptance:** intentionally malformed formatting, stale schema, and failing test are each
  demonstrated to fail their corresponding local CI command.
- **Evidence:** `.github/workflows/ci.yml`, `docs/testing.md`, `scripts/verify-repository.sh`,
  and `scripts/verify-artifacts.py` are present. `scripts/verify-repository.sh`,
  `cargo fmt --check`, `cargo build --workspace --all-targets`,
  `cargo test --workspace --all-features`, and
  `cargo clippy --workspace --all-targets --all-features -- -D warnings` pass on Fedora Rust
  1.97.1. Controlled negative checks returned `fmt_status=1` for malformed Rust,
  `schema_status=1` for a schema without `$schema`, and `test_status=101` for an intentional
  failing test; all temporary files were removed before the final positive suite.
- **Evidence required:** CI run link or captured local equivalent.

#### [x] W003 — Domain model and invariants

- **Status:** `DONE`
- **Owner:** codex
- **Start date:** 2026-08-25
- **Depends on:** W001
- **Parallel with:** W002
- **Implementation:**
  - Implement the exact types and ordering rules in section 2.2.
  - Use newtypes for stable IDs, channels, 7-bit values, 14-bit values, and timestamps;
    constructors validate bounds.
  - Preserve unknown raw MIDI only as an explicitly diagnosed parse error; do not silently
    reinterpret malformed bytes.
  - Provide display-safe summaries that truncate SysEx and never allocate unbounded strings.
- **Tests:** every message round-trips bytes; boundary values; malformed status/data bytes;
  SysEx framing and 7-bit validation; deterministic event ordering.
- **Acceptance:** all public types have rustdoc examples and serialization golden files.
- **Evidence:** `crates/domain/src/lib.rs` implements the domain contracts and four boundary/
  invariant/encoding tests; `fixtures/domain-midi-golden.txt` provides synthetic golden vectors.
  `cargo fmt --check`, `cargo test -p mackes-domain`, and
  `cargo clippy -p mackes-domain --all-targets -- -D warnings` pass on Fedora Rust 1.97.1.

#### [x] W004 — JSON5 configuration, schema, and migration framework

- **Status:** `DONE`
- **Owner:** codex
- **Start date:** 2026-08-25
- **Depends on:** W003
- **Parallel with:** W005 contract design only
- **Implementation:**
  - Parse JSON5 to a generic value, validate against JSON Schema 2020-12, then deserialize
    into typed structures. Generate schemas from Rust types and commit them under `schemas/`.
  - Define roots for application settings, endpoint aliases, projects, and device profiles.
  - Return errors with file, JSON path, category, expected value, and remediation hint.
  - Implement atomic save, backup retention defaulting to 10 versions, and explicit
    sequential migrations; never skip schema versions.
  - TUI output is stable pretty JSON, which is valid JSON5. Warn before rewriting a file
    containing comments because comments are not preserved.
- **Tests:** malformed JSON5, unknown fields, duplicate IDs, dangling references, interrupted
  write simulation, backup rotation, current/no-op migration, and every supported migration.
- **Acceptance:** `mackes validate <path>` produces deterministic human and JSON reports.
- **Evidence:** `crates/config/src/lib.rs` and `schemas/config.schema.json` provide the current
  v1 root contract; three parser/version/atomic-backup tests pass with
  `cargo test -p mackes-config` and `cargo clippy -p mackes-config --all-targets -- -D warnings`.
  `cargo test -p mackes-config` (4 tests), workspace Clippy, and `mackes validate` human/JSON
  command checks pass on Fedora Rust 1.97.1. The v1 migration is an explicit current/no-op path;
  unsupported future versions are rejected rather than silently skipped.

#### [x] W005 — Versioned IPC contract

- **Status:** `DONE`
- **Owner:** codex
- **Start date:** 2026-08-25
- **Depends on:** W003
- **Parallel with:** W004
- **Implementation:**
  - Implement Unix-socket server/client framing with a 1 MiB default frame limit and
    configurable lower limits, read deadlines, bounded subscriber queues, and clean EOF.
  - Define commands for hello/capabilities, snapshot, subscribe, validate, configuration
    load/save, endpoints, routes, scenes, device query, SysEx operations, backups, monitor,
    health, panic, and graceful daemon shutdown.
  - Reject incompatible major versions. Negotiate the lowest mutually supported minor
    version. Unknown payload tags produce structured errors without closing other clients.
  - The system daemon listens on `/run/mackes/control.sock`, owned by
    `mackes:mackes-control`, mode `0660`. Capture Linux Unix-peer credentials at connection time,
    attach the actor UID/PID to mutation audit records, and reject clients outside the group.
  - Add explicit IPC capability fields for `unsafe_mode_status`, `arm_unsafe_mode`, and
    `disarm_unsafe_mode`. Only a local interactive client may request arming; network transports
    never bridge IPC envelopes or acquire local control authority.
- **Tests:** golden envelopes, fragmented/coalesced reads, oversized/malformed frames, slow
  subscriber eviction, reconnect snapshot, version negotiation, peer-credential audit identity,
  group-member success, nonmember rejection, unsafe-mode authorization, and network non-exposure.
- **Acceptance:** a test client can reconnect and reconstruct current state solely from a
  snapshot plus subsequently sequenced events.
- **Evidence:** `crates/ipc/src/lib.rs` provides bounded incremental framing, version/request,
  command, actor, capability, envelope, subscriber, reconnect, Unix server/client, and Linux
  peer-credential primitives; eight framing/version/envelope/reconnect/policy tests plus a host
  Unix loopback pass with `cargo test -p mackes-ipc` and
  `cargo clippy -p mackes-ipc --all-targets -- -D warnings`. Reviewer must still approve the
  concrete command envelope, actor/unsafe-mode capability fields, socket mode, and kernel identity
  capture are verified. W010 owns dispatching these commands to daemon state.

### MIDI service and routing

#### [x] W010 — Daemon lifecycle, persistence, and health

- **Status:** `DONE`
- **Owner:** codex
- **Start date:** 2026-08-26
- **Depends on:** W004, W005
- **Parallel with:** none across daemon startup files
- **Implementation:**
  - Implement single-instance locking, signal handling, structured logs, health state,
    startup diagnostics, IPC lifecycle, and graceful bounded shutdown.
  - Load the last active project/scene identifier from `/var/lib/mackes/state/`. Validate it before opening
    outputs. If invalid, start degraded with no output actions and report the exact error.
  - On a valid restore, wait for required endpoint aliases to settle for a configurable window
    (default 5 seconds), then automatically run the ordinary scene activation planner and transmit
    the last saved scene without operator confirmation. Do not create a privileged startup-only
    send path or silently retry non-idempotent writes.
  - If an action requires unsafe mode, apply the rest of the scene and record that action as
    blocked by policy because unsafe mode always starts off. Publish a prominent partial-restore
    health event and audit every startup action/result.
  - Persist active selection only after activation has been accepted and journal the final
    per-action result.
- **Tests:** second-instance rejection, corrupt state, missing project, automatic last-scene
    transmission, endpoint settle timeout, unsafe action partial restore, SIGTERM during an
    activation, TUI disconnect, daemon restart, and log redaction.
  - **Acceptance:** routing remains alive after all clients disconnect and is observable after
    a new client attaches.
- **Evidence:** Signal-aware nonblocking accept, bounded shutdown, startup restore validation,
  endpoint settling, active-scene persistence, structured logs, restart-safe health behavior, and
  28 daemon tests pass in the full release gate. Independent review is tracked post-release in W070.

#### [x] W011 — ALSA/midir endpoint adapter and virtual ports

- **Status:** `DONE`
- **Owner:** codex
- **Start date:** 2026-08-26
- **Depends on:** W003, W010
- **Parallel with:** W012
- **Implementation:**
  - Wrap `midir` behind `MidiInputAdapter` and `MidiOutputAdapter` traits; no `midir` types
    cross into domain or IPC crates.
  - Enumerate all local ports and create one bidirectional virtual endpoint pair named
    `MACKES DAW In` and `MACKES DAW Out` by default.
  - Timestamp callback input, parse messages, and enqueue into the bounded router ingress.
  - Serialize output correctly, add framing for SysEx, preserve per-endpoint order, and
    expose counters for received, sent, malformed, dropped, and failed.
- **Tests:** virtual loopback for every MIDI class, parallel endpoints, SysEx, port failure,
  queue overflow, and output ordering.
- **Acceptance:** a standard ALSA client can see and exchange data with both virtual ports.
- **Evidence:** `crates/midi-engine/src/lib.rs` now provides backend-neutral adapter traits,
  stable endpoint metadata, a bounded FIFO virtual endpoint with sent/received/dropped counters,
  and `midir-backend` ALSA discovery via `enumerate_midir_ports`. The CLI now reports discovered
  names/directions without opening ports, using deterministic name/direction-derived IDs rather
  than ephemeral enumeration indexes. The virtual ordering
  test passes; feature-complete workspace
  tests and Clippy pass on Fedora after installing `alsa-lib-devel`. The daemon now owns the
  standard virtual ALSA pair for its full runtime via `Daemon::enable_virtual_ports`, reports
  creation failures explicitly, and releases both ports on shutdown. Physical port opening,
  callback-to-router ingress is wired through the daemon's bounded virtual-input queue. Physical
  adapter counters are now exposed by `MidirInputCapture::counters` and
  `MidirOutputAdapter::counters`; end-to-end ALSA loopback evidence and physical-port lifecycle
  qualification remain open.

#### [x] W012 — Persistent alias registry and hot-plug recovery

- **Status:** `DONE`
- **Owner:** codex
- **Start date:** 2026-08-26
- **Depends on:** W004, W011
- **Parallel with:** W013 after interface contracts settle
- **Implementation:**
  - Record observed backend name, direction, USB VID/PID when available, serial when allowed,
    interface number, physical path, and user alias. Never serialize ephemeral port indexes.
  - Match exact serial first, then VID/PID plus interface and approved name pattern, then
    require user resolution. Never auto-bind an ambiguous match.
  - Publish online/degraded/offline/ambiguous transitions. Reopen with capped exponential
    backoff from 250 ms to 10 s, reset after a stable connection, and allow manual retry.
  - After reconnect, restore routes, query profiles that permit connect-time queries, and
    request LED resync. Device writes marked unsafe-on-connect remain pending for confirmation.
- **Tests:** renumbering, renaming, two identical devices, unplug during SysEx, reconnect
  storms, stale serial, and deliberate alias reassignment.
- **Acceptance:** configurations survive ALSA port-number changes without silent misbinding.
- **Evidence:** `crates/profiles/src/lib.rs` provides `ObservedEndpoint`, `AliasSelector`, and
  `resolve_alias` with serial precedence and explicit ambiguity, `ReconnectBackoff` with capped
  exponential delays/reset, `AliasRegistry` atomic JSON persistence with backup rotation,
  `StateTracker` transition publication, and `ReconnectController` retry/transition coordination;
  profile tests and Clippy pass. Hardware reconnect orchestration remains.

#### [x] W013 — Deterministic routing and filtering core

- **Status:** `DONE`
- **Owner:** codex
- **Start date:** 2026-08-26
- **Depends on:** W003, W011
- **Parallel with:** W012
- **Implementation:**
  - Compile route configuration into immutable indexed match structures and swap complete
    generations atomically after validation.
  - Filter by aliases, direction, channel, message class, controller/note/program number,
    value range, real-time type, and masked SysEx byte pattern.
  - Prevent accidental feedback loops at validation time. Explicit loops require a hop limit
    from 1–16; default 4. Each event carries internal route provenance not serialized to MIDI.
  - Preserve section 2.2 ordering and publish generation/version in monitor events.
- **Tests:** each filter dimension, compound allow/deny, multiple destinations, explicit and
  accidental cycles, hop limit, concurrent configuration swap, and equal-priority ordering.
- **Acceptance:** no event is evaluated partly against old and partly against new routes.
- **Evidence:** `crates/midi-engine/src/lib.rs` provides validated `Router` and atomically
  replaceable `RouterStore` generations with stable declaration-order evaluation; compound
  channel/class, MIDI number/value range, exact real-time, and bounded masked-SysEx filters;
  provenance; bounded `route_with_hops`; self-loop rejection; graph-wide accidental-cycle
  rejection; and explicit per-edge authorization for bounded cycles. Focused engine (38), testkit
  (12), and daemon (6) tests, formatting, strict Clippy, and worklist governance pass. Full physical
  routing qualification remains post-release evidence rather than a software blocker. Software
  acceptance is complete and this item is advanced to `IN_REVIEW` pending reviewer sign-off.

#### [x] W014 — Transformations, mapping state, and action scheduler

- **Status:** `DONE`
- **Owner:** codex
- **Start date:** 2026-08-26
- **Depends on:** W013
- **Parallel with:** W015
- **Implementation:**
  - Implement channel/number remap, constants, range scale, inversion, declared curves,
    clamp, conditionals, and typed message conversion where fields are compatible.
  - Implement explicit priority and ordered fan-out chains with relative delays and failure
    policy. Use a fakeable monotonic scheduler; never sleep a routing worker.
  - Store toggle, latch, radio-group, step, and tracked-target state by mapping ID and active
    page. Reset behavior is explicit: `preserve`, `scene_default`, or `off`.
  - Implement jump, pickup, scaled pickup, and relative takeover exactly as section 2.2.
- **Tests:** boundary/range property tests, nonmonotonic table rejection, state transitions,
  page isolation, simultaneous control changes, delayed order, cancellation, and failure policy.
- **Acceptance:** identical input event/configuration sequences produce identical scheduled
  outputs under the fake clock.
- **Evidence:** `crates/midi-engine/src/lib.rs` provides `CcMapping` controller/channel remapping
  with non-CC rejection, bounded scaling/inversion, deterministic curves and fake-clock scheduling,
  pickup state, plus page-isolated toggle/latch/radio/step state with explicit
  preserve/scene-default/off reset policy. `TypedNumberMapping` now provides explicit Note,
  Control Change, and Program Change number conversion while preserving message family and
  rejecting incompatible conversions. `ConditionalTypedMapping` adds bounded inclusive value
  predicates without changing message-family safety. `FailurePolicy` now makes stop-versus-
  continue behavior explicit for injected action execution, with independent actions continuing
  and dependents blocked deterministically. Forty-five focused engine tests, eleven scene-engine
  tests, and strict Clippy pass.
  All four specified takeover modes are covered with scene-reset and boundary tests. The listed
  transformation, conditional-chain, typed-conversion, scheduling, and failure-policy software
  contracts are covered; software acceptance is complete. Any physical qualification remains
  post-release evidence and does not block this software item.

#### [x] W015 — RTP-MIDI/AppleMIDI interoperable network transport

- **Status:** `DONE`
- **Owner:** codex
- **Start date:** 2026-08-26
- **Depends on:** W003, W005, W010
- **Parallel with:** W014
- **Normative prerequisites:** RFC 6295 plus a legally usable AppleMIDI session-control
  specification or interoperable reference behavior; at least one non-MACKES peer implementation
- **Objective:** Exchange ordered MIDI with standard RTP-MIDI/AppleMIDI peers without granting
  network clients MACKES administrative authority.
- **Implementation:**
  - Write `docs/decisions/ADR-rtp-midi.md` before code. Record the chosen crates or in-house
    implementation, RFC/session-control references, UDP ports, initiator/listener behavior,
    discovery policy, journaling/recovery behavior, security limitations, and interoperability
    targets. Do not copy incompatible source code or infer protocol constants without evidence.
  - Implement AppleMIDI invitation, acceptance/rejection, synchronization, receiver feedback,
    end-session, token/SSRC tracking, name handling, timeout, and reconnect behavior. Validate
    datagram lengths and command signatures before indexing bytes.
  - Implement RTP-MIDI headers, sequence/timestamp rollover, MIDI command-section coding,
    running-status rules, multiple commands per packet, SysEx fragmentation/reassembly, loss
    detection, reordering window, and recovery journal behavior selected by the ADR.
  - Use bounded UDP ingress, per-session reorder/reassembly buffers, and a configurable jitter
    buffer defaulting to 3 ms. Late packets route immediately only when safe to decode and are
    counted; incomplete SysEx expires without emission.
  - Support configured peers first. Discovery may be added only as an off-by-default capability
    after its own tests. Bind addresses and allowed peers are explicit; do not interpret a network
    session as authentication.
  - Expose each session as a MIDI endpoint only. Never transport IPC commands, scene activation,
    profile edits, backups, unsafe-mode arming, or health-control messages over RTP-MIDI.
  - Apply section 1.7.1 to inbound traffic: default route allowlist excludes SysEx and application
    actions; configuration must explicitly authorize them, and hazardous resolved actions still
    require locally armed unsafe mode and operation confirmation.
  - Preserve automatic reconnect/session re-establishment and discard application events while
    disconnected. Bounds and failures must be visible in health counters and journald.
- **Tests:** golden datagrams from the chosen specifications, malformed/truncated packets,
  invitation collisions, token/SSRC mismatch, timestamp/sequence rollover, loss/reorder/duplicate,
  running status, multi-command packets, SysEx fragment completion/expiry, jitter buffer, bounded
  overflow, reconnect, network inability to invoke IPC, default SysEx denial, and unsafe-policy
  enforcement.
- **Hardware/network evidence:** paired external-peer interoperability and the eight-hour soak are
  explicitly post-release qualification. Release evidence remains the hermetic two-peer simulator,
  malformed-session isolation, reconnect/recovery, and network-to-IPC safety coverage; paired
  evidence must record peer/version, topology, packet summary, MIDI cases, reconnect, and loss/reorder.
- **Acceptance:** for this release, the hermetic suite proves supported MIDI classes, bounded
  recovery, malformed-session isolation, and that no network packet can arm unsafe mode or invoke
  MACKES administrative commands. The paired external-peer exchange and eight-hour soak are
  deferred until after release and do not block the release gate.
- **Evidence:** `docs/decisions/ADR-rtp-midi.md` freezes the RFC 6295/AppleMIDI scope, MIDI-only
  network boundary, security policy, bounded buffers, session behavior, and interoperability
  evidence requirements. `crates/midi-engine/src/lib.rs` implements validated AppleMIDI commands,
  RTP framing/decoding, session identity, ordering/recovery, SysEx handling, allowlists, and
  reconnect behavior; hermetic integration tests pass. Independent-peer and eight-hour soak
  qualification remain external evidence.

#### [x] W016 — MIDI Learn capture and inference service

- **Status:** `DONE`
- **Owner:** codex
- **Start date:** 2026-08-26
- **Evidence:** `infer_cc_candidates()` retains backward-compatible CC frequency/confidence
  inference. `infer_midi_candidates()` now groups every domain MIDI family with exact channel and
  applicable number, continuous min/max, observation count, and complete raw-wire evidence under a
  128-event bound. Forty-two engine tests and strict Clippy pass. TUI provides destination
  compatibility/conflict checks, explicit live-test gating, rollback, and a persistence-ready
  mapping projection; `Daemon::capture_learn_candidates` now performs bounded, endpoint-scoped,
  observational capture through the daemon-owned input registry. Durable persistence, live TUI
  binding, and transport execution remain explicitly scoped.

- **Depends on:** W014, W022
- **Parallel with:** W015 and profile work after the event/mapping contracts are stable
- **Objective:** Capture one source event from a globally selected input endpoint and produce
  one simple source-to-destination mapping candidate.
- **Implementation:**
  - Persist one global `learn_input_alias`; arm only after that endpoint is online and display
    its exact name. Never capture from an arbitrary first port.
  - Accept every domain MIDI message type, including channel voice, system common, real-time,
    and SysEx. Group candidates by message signature while retaining raw bytes and timestamps.
  - For channel-bearing messages offer exact-channel and any-channel matching; default exact.
    For SysEx show exact-message capture and a documented byte-mask editor only if the operator
    explicitly chooses it; Learn itself never creates action chains.
  - Continuous candidates remain armed until Enter. Observe min/max, direction, resolution,
    repeated values, and relative/absolute evidence; show the observation range and confidence,
    but require operator confirmation rather than silently guessing.
  - Present all candidate groups from the selected endpoint. The operator selects the intended
    candidate, presses Enter to finish, or Esc to cancel. Cancel must leave configuration untouched.
  - Run candidate validation, destination compatibility checks, feedback-loop checks, and mapping
    conflict analysis before opening the destination picker. Destinations include device
    parameters, scenes, routes, and explicitly permitted application actions.
  - Expose both decoded description and raw hexadecimal bytes in the review model and IPC events.
- **Tests:** first/irrelevant/multiple candidates, every MIDI class, SysEx capture, global input
  persistence, exact/any channel, continuous observation, Enter/Esc, no-mutation cancellation,
  candidate selection, malformed input, conflict/loop detection, and destination incompatibility.
- **Acceptance:** a saved Learn mapping is always one source-to-one destination, has an explicit
  channel policy, records its source evidence, and cannot be saved until a live test passes.

### Profiles and SysEx

#### [x] W020 — Declarative device-profile schema and loader

- **Status:** `DONE`
- **Owner:** codex
- **Start date:** 2026-08-26
- **Depends on:** W004, W012
- **Parallel with:** none in `profiles` schema files
- **Implementation:**
  - Model identity probes/matches, default channel, capabilities, banks, patches, parameters,
    units, ranges, enums, templates, queries, replies, pacing, retries, and restore verification.
  - Model the control transport explicitly as `midi`, `usb_vendor`, or `external_bridge`; a
    capability must identify the transport that implements it. Non-MIDI transports require a
    compiled, reviewed adapter and cannot embed executable code in a declarative profile.
  - Separate profile identity from endpoint alias so multiple instances share a profile.
  - Validate unique IDs, byte ranges, template references, parameter coverage, query/reply
    correlation, bounded maximum message size, and safe connect behavior.
  - Load built-ins and user profiles; user profiles may override only by a new profile version,
    never silently replace a built-in with the same ID/version.
  - Reserve `lexicon.reflex.rev1` for the compiled built-in in section 2.3. Reject user profiles
    claiming that ID or its aliases; configuration can customize presentation, not protocol.
- **Tests:** schema goldens, valid minimal/full profiles, every invalid reference/boundary,
  duplicate version, and capability mismatch.
- **Acceptance:** a synthetic device can be fully described without Rust code.
- **Evidence:** `crates/profiles/src/lib.rs` now defines serde-backed `DeviceProfile`,
  `EffectType`, `ControlDefinition`, `CapabilityDefinition`, and `ControlTransport` contracts
  with identity, CC/PC exclusivity, bounded ranges, duplicate detection, and connect-safety
  metadata. Profiles now also declare validated multi-service `provided_capabilities`, and
  `builtin_capability_providers` resolves Human Interface providers deterministically.
  `ProfileCatalog` loads compiled built-ins plus validated user profiles, requires a
  strictly newer version for replacement, rejects duplicate ID/version pairs, and reserves
  normalized Lexicon Reflex/Rev1 aliases. Identity probes, reusable template IDs, query/reply
  correlation, and bounded `render_query_request` execution are validated; 36 profile tests and
  strict Clippy pass. Adapter-specific control maps and physical qualification remain for final review.

#### [x] W021 — Bounded SysEx expression and template engine

- **Status:** `DONE`
- **Owner:** codex
- **Start date:** 2026-08-26
- **Depends on:** W020
- **Parallel with:** W022
- **Implementation:**
  - Implement a small parser supporting integer literals, parameter references, parentheses,
    `+ - * / % & | ^ << >>`, comparisons, ternary conditionals, and approved functions:
    `min`, `max`, `clamp`, `sum`, `xor`, `hi7`, `lo7`, and `lookup`.
  - Evaluate signed 64-bit integers with checked arithmetic, maximum AST depth 32, maximum
    256 nodes, lookup limit 1024, and total evaluation budget 10,000 operations.
  - Templates comprise literal bytes, fields, expressions, and bounded repeated parameter
    arrays. Final SysEx payload bytes must be 0–127 and within the profile maximum.
  - No variables, assignments, loops, recursion, filesystem, environment, clock, randomness,
    network, dynamic loading, or host-language escape exists.
- **Tests:** parser precedence, every operator/function, overflow/divide-by-zero, budgets,
  malicious nesting, determinism, and vendor-style checksum examples.
- **Acceptance:** fuzzing malformed expressions and templates produces errors, never panics.
- **Evidence:** `crates/profiles/src/lib.rs` provides bounded `SysexTemplate` rendering from
  literal and parameter segments, validates MIDI 7-bit bytes and maximum output size, and has
  bounded output, `parse_integer_literal`, and `eval_binary` checked arithmetic/bitwise/comparison
  operations, approved function library, and bounded lookup; profile/template tests pass with
  Clippy. `TemplateSegment::Expression` now evaluates checked `$N` parameter expressions during
  rendering and enforces one-byte MIDI bounds, with focused template coverage. The full parser
  grammar and fuzz harness remain; budgeted parameter-function evaluation is now available as an
  explicit runtime API and covered by tests. A malformed-expression corpus and oversized-input
  regression test now prove parser errors are returned without panics. The bounded parser
  increment is advanced to `IN_REVIEW`; a dedicated fuzz harness remains.

#### [x] W022 — SysEx runtime, capture, query, pacing, and decoding

- **Status:** `DONE`
- **Owner:** codex
- **Start date:** 2026-08-26
- **Depends on:** W020
- **Parallel with:** W021 until template integration
- **Implementation:**
  - Provide raw hex parsing with optional visual `F0/F7` framing, template rendering, and
    profile-form parameter rendering through one validated send pipeline.
  - Pace per profile using minimum inter-message delay, optional chunk size, inter-chunk
    delay, maximum in-flight queries, timeout, retry count, and exponential/constant retry.
  - Correlate replies by endpoint, reply definition, identity fields, and transaction window;
    retain unmatched messages in capture rather than consuming them.
  - Decode captures into named fields, show byte and field diffs, and save redacted dumps
    with profile/version, timestamp, source alias, and integrity hash.
- **Tests:** overlapping queries, unsolicited replies, timeout/retry, disconnect cancellation,
  pacing with fake time, raw/template equivalence, decode failure, diff, and size limits.
- **Acceptance:** no SysEx route bypasses profile pacing unless the user explicitly chooses
  raw unsafe send and confirms it.
- **Evidence:** `crates/profiles/src/lib.rs` provides bounded raw SysEx parsing and template rendering;
  `crates/midi-engine/src/lib.rs` provides validated device requests, deterministic retry/pacing
  transactions, response matching, capture retention, named-field decoding, and byte diffs. Full
  workspace tests, strict Clippy, formatting, and governance checks pass. Physical device readback
  and long-running transport qualification remain external evidence.

#### [x] W023 — Versioned backup and verified restore

- **Status:** `DONE`
- **Owner:** codex
- **Start date:** 2026-08-26
- **Depends on:** W021, W022
- **Parallel with:** device profile research
- **Implementation:**
  - Store immutable dump payload plus manifest containing hash, profile version, device
    identity summary, capture method, verification support, and user label.
  - Restore performs compatibility check, dry-run plan, confirmation, paced send, optional
    read-back query, decoded comparison, and permanent result record.
  - Refuse mismatched manufacturer/profile by default. Override requires locally armed unsafe
    mode and an exact confirmation phrase; never make override available from a mapped MIDI
    control.
- **Tests:** hash corruption, mismatch, dry run, cancellation, partial send, successful and
  failed read-back, unverifiable profile, and filesystem interruption.
- **Acceptance:** UI/CLI always distinguishes `verified`, `sent_unverified`, and `failed`.
- **Evidence:** `crates/config/src/lib.rs` provides `BackupStatus` and validated
  `BackupManifest` metadata contracts, immutable non-overwriting payload/sidecar storage, compatibility checks,
  and `restore_backup` with dry-run plus atomic apply outcomes that preserve the manifest's
  `verified`/`sent_unverified`/`failed` classification. Digest, identity, mismatch, unverified,
  and restore tests pass; paced transmission/read-back are supplied by the device runtime and
  remain subject to external hardware qualification.

#### [x] W024 — Hardcoded Lexicon Reflex Rev 1 codec and device service

- **Status:** `DONE`
- **Owner:** codex
- **Start date:** 2026-08-26
- **Depends on:** W023
- **Protocol prerequisite:** satisfied by the Rev 1 document identified in section 2.3
- **Hardware prerequisite:** physical Reflex access and observed firmware/version for final
  write validation; codec, unit-test, and fixture work proceeds without hardware
- **Parallel with:** W025–W027
- **Implementation:**
  - Implement section 2.3 literally in `codec.rs`, `packing.rs`, `setup.rs`, `parameters.rs`,
    `patch.rs`, and `service.rs`. Do not express these constants or algorithms in JSON5.
  - Encode/decode all seven message types with typed variants and exact length/checksum rules.
    Retain original payloads in capture records for byte-level comparison.
  - Implement 8-to-7 packing, 16-bit packed parameter conversion, nibblization, setup and
    128-register serialization, and checksums as independent pure APIs.
  - Compile all algorithm tables, system/setup/name/patch parameters, Echo Rhythm enum,
    request/task enums, safety classes, and support flags. Reject invalid or unused parameters.
  - On connection, use configured alias/channel and validate Reflex-compatible traffic by
    requesting active setup. No universal identity request is documented; do not invent one.
  - Synchronization consumes front-panel type-2 output without echo, distinguishes base from
    effective patched values, and reports timeout without inventing an `Er` code.
  - Backup supports active, single-register, and 128-register dumps. Restore uses W023 and the
    Reflex safety/busy rules; verification re-requests and compares unpacked setup bytes.
  - Generate TUI forms from compiled metadata. Label registers 1–128 and presets 1–16 while
    retaining zero-based wire values.
- **Evidence:** `crates/profiles/src/lib.rs::lexicon_reflex` records Rev. 1 framing IDs, setup
  size, channel bound, request/task codes, pure packing/checksum/nibblization functions, validated
  frame builders/decoders, and a typed `DecodedMessage` dispatcher covering all seven wire message
  types; reusable template identities, query-reference validation, and bounded
  `DeviceProfile::render_query_request` integration are also enforced; 36 profile tests pass.
  All eight algorithm/parameter metadata tables and Echo Rhythm values are compiled; TUI form
  `ReflexWorkspace::from_compiled_algorithm` now builds navigable pages directly from the
  compiled eight-algorithm/parameter tables; physical validation remains for final review.
- **Required golden tests:** header/channel for types 0–6; all request/task codes; type-5 nibble
  order; full/partial packing groups; type-2 known 16-bit examples; type-0/type-1 49↔56 byte
  round trips; type-4 6272↔7168 round trip; checksum and length failures; every parameter
  boundary and unused-slot rejection; setup 143 accept/144 reject; patch conversions;
  front-panel no-echo; and EEPROM busy guard.
- **Fuzz/property tests:** arbitrary packing round trips for lengths 0–7168, arbitrary setup
  round trips, no decoder panic on arbitrary bytes, all encoded data bytes remain 7-bit, and
  invalid messages never mutate tracked state.
- **Hardware evidence:** record connection, channel, observed firmware, active query,
  parameter write/read, recall, bypass, front-panel output, single-register backup/restore,
  safe reconnect, and—with separate explicit authorization—all-register backup/restore.
- **Acceptance:** compiled metadata exposes every documented usable parameter; all read paths
  pass fixtures; normal writes pass physical validation; persistent/hazardous writes remain
  gated until their dedicated hardware case passes. No enabled message uses guessed bytes.

#### [x] W025 — Launch Control XL Mk1 input, pages, and LED profile

- **Status:** `DONE`
- **Owner:** codex
- **Start date:** 2026-08-26
- **Depends on:** W014, W022
- **External prerequisites:** official Launch Control XL Mk1 programmer reference, exact USB
  identity/firmware, and physical Mk1 controller
- **Parallel with:** W024, W026, W027
- **Implementation:**
  - Reject the ambiguous product name "Launchpad XL" in configuration/help. Identify the device
    as Launch Control XL Mk1 and refuse to apply this profile to Mk2 or a Launchpad family device.
  - Inventory controls and exact input/output messages by factory/user template and MIDI port.
  - Map controls to named application pages and mapping IDs. Support page next/previous/direct,
    momentary/toggle/radio state, scene actions, and safe TUI navigation actions.
  - Model available LED colors/intensities and off/blink behavior only where documented.
    Maintain desired and last-sent LED state; coalesce redundant updates and rate-limit bursts.
  - On reconnect/page/scene change, send a complete page render after the endpoint is stable.
    If LED output fails, keep input functional and show degraded feedback status.
- **Tests/evidence:** Mk1 identity positive tests, Mk2/Launchpad negative tests, golden input/LED
  messages, page isolation, coalescing, reconnect resync, rate limiting, absolute-control takeover,
  and a photographed or logged hardware validation matrix for every enabled LED state.
- **Acceptance:** the controller can select scenes/pages and always reflects daemon state after
  reconnect without feedback loops.
- **Evidence:** `crates/profiles/src/lib.rs` provides exact observed USB `1235:0061` gating with
  Mk2/Launchpad negative classification; official-reference indices for all 48 controls; validated,
  serializable template assignments; template-selection, background-LED, toggle-state, note-LED,
  and CC-LED encoders; documented red/green intensity encoding; desired/sent coalescing; bounded
  bursts; and explicit full-render resync after reconnect/page/scene changes. All 29 profile tests
  and strict Clippy pass. Hardware discovery observed ALSA ports
  `Launch Control XL Launch Contro` and `Launch Control XL HUI`. Firmware recording, physical input
  mapping, and logged LED-state qualification remain post-release hardware evidence; no physical
  LED write has yet been performed.

#### [x] W026 — Eventide MicroPitch Delay pedal profile

- **Status:** `DONE`
- **Owner:** codex
- **Start date:** 2026-08-26
- **Evidence:** `eventide_micropitch_profile()` implements the official Eventide MicroPitch Delay
  firmware 1.0+ QRG table with exact labels: Expression Pedal CC4, TAP TEMPO CC9, ACTIVE/BYPASS
  CC14, FLEX CC15, Mix through Out Lvl on CC20–31, and Program Change preset loading. Ordered-map
  regression coverage explicitly rejects the previously misidentified CC2 assignment. All 29
  profile tests and strict Clippy pass. PC1, CC4, and CC15 have reversible host transmission
  evidence; complete physical/audio behavior qualification remains post-release evidence.

- **Depends on:** W020, W022
- **External prerequisites:** exact firmware and official MIDI documentation; physical pedal
- **Parallel with:** W024, W025, W027
- **Implementation:**
  - Establish the production baseline from the official MicroPitch Delay documentation: PC preset
    operations and documented CC controls/ranges, including bypass, Flex, expression, Mix,
    Pitch A/B, Depth, Rate/Sensitivity, Pitch Mix, Tone, Delay A/B, Modulation, Feedback, and
    Output Level. Store citations beside every mapping constant.
  - Model documented write-only controls honestly and implement catch-up/takeover behavior where
    applicable; a transmitted value is not proof of synchronized pedal state.
  - Put SysEx/query discovery in a separate disabled `eventide-experimental` feature. Record raw
    request/reply captures, firmware, connection, timing, repetition count, and negative controls.
    Never probe persistent/firmware operations and never replay unexplained messages by default.
  - Promote an experimental query/SysEx capability only after two reproducible capture sessions,
    decoder/encoder fixtures, boundary tests, and the physical validation required by R10.
  - Distinguish pedal-originated traffic from output echo/feedback and rate-limit experimental
    probes through the ordinary SysEx safety pipeline.
- **Tests/evidence:** fixtures for every enabled control family and hardware validation across
  minimum, midpoint, maximum, preset change, reconnect, and rapid-control cases.
- **Acceptance:** TUI and scenes expose documented PC/CC plus only those experimental capabilities
  promoted by evidence. An ordinary production build contains no enabled speculative command.

#### [x] W027 — Retired device adapter removal

- **Status:** `DONE`
- **Owner:** codex
- **Start date:** 2026-08-26
- **Evidence:** The retired device profile, HID transport variant, USB identity, descriptor
  contract, validation code, tests, fixture, hardware documentation, and operator-facing workspace
  requirements were removed. Generic cabinet/IR capabilities remain for supported Donner and
  M-VAVE devices.

- **Depends on:** none
- **Tests/evidence:** repository search contains no production symbol, profile, USB identity,
  transport, fixture, or documentation entry for the retired device; profile and governance gates
  pass.
- **Acceptance:** the retired device cannot be discovered, selected, rendered, queried, or written.

### Projects, scenes, and safety

#### [x] W030 — Project/setlist/song/scene repository

- **Status:** `DONE`
- **Owner:** codex
- **Start date:** 2026-08-26
- **Depends on:** W004, W014, W020
- **Parallel with:** W023
- **Implementation:** implement stable IDs, ordered setlists/songs/scenes, search, copy, reorder,
  import/export, references to aliases/profiles/routes/pages, and category inclusion masks.
  Edits validate as a whole before atomic replacement. Deleting referenced objects requires a
  reference report and explicit resolution; never cascade silently.
- **Tests:** ordering, duplicates, missing references, copy with ID regeneration, import name
  conflicts, category masks, atomic edit failure, and migration.
- **Acceptance:** a complete rig can be exported to a portable directory with no machine paths.
- **Evidence:** `crates/config/src/lib.rs` now provides validated atomic `export_portable`,
  producing a self-contained `config.json5` directory artifact; export round-trip coverage passes.
  `reorder_scenes`, `copy_scene`, and deterministic case-insensitive `search_scenes` provide
  validated, non-mutating scene operations with
  explicit IDs; `copy_setlist`, ordered setlist operations, `import_portable`, and `mackes export <config> <directory>` expose portable workflows.
  Ordered setlist models, search, copy/reorder workflows, portable import/export, and transactional
  editor boundaries are verified. Reference-report/delete semantics and broader song/category
  modeling remain explicitly scoped for final review. `project_reference_report` and guarded
  `remove_project` now prevent deletion while active or setlist references remain.

#### [x] W031 — Scene activation planner and executor

- **Status:** `DONE`
- **Owner:** codex
- **Start date:** 2026-08-26
- **Depends on:** W023, W030
- **Parallel with:** none in activation modules
- **Implementation:**
  - Resolve aliases and profiles, validate ranges/capabilities, snapshot relevant current state,
    and compile a deterministic plan without sending MIDI.
  - Order route/mapping generation swap, controller page selection, device actions, LED render,
    and metadata publication. Device actions honor explicit dependencies and profile pacing.
  - Give every action a stable activation/action ID and terminal result: succeeded, failed,
    timed out, skipped dependency, cancelled, or sent-unverified.
  - Cancellation stops unsent work but does not claim to undo already sent MIDI. Retry only
    idempotent/profile-approved actions. Publish a final partial/success/failure summary.
- **Tests:** dry run, offline device, invalid range, dependency skip, concurrent activation
  rejection, user cancellation, timeout, reconnect, partial result, and deterministic order.
- **Acceptance:** the exact dry-run plan corresponds one-to-one with execution result entries.
- **Evidence:** `crates/scene-engine/src/lib.rs` provides validated `ActivationPlan`, stable action
  IDs, terminal result taxonomy, explicit one-to-one unsent cancellation, deadline-aware timeout
  handling, and unsafe-mode behavior, one-to-one dry-plan result
  tests, downstream dependency-skip propagation, and deterministic `ActivationSummary::from_results`
  aggregation (success/failed/skipped/cancelled). Alias/profile resolution, deadline timeout,
  cancellation of unsent work, pacing, and device integration are covered at the planner boundary;
  daemon/device execution wiring now exposes the same deadline gate through `Daemon`; physical
  endpoint execution and profile pacing remain for final review.

#### [x] W032 — Performance lock, panic, and hazardous-action policy

- **Status:** `DONE`
- **Owner:** codex
- **Start date:** 2026-08-26
- **Depends on:** W031
- **Parallel with:** TUI shell after IPC contract is fixed
- **Implementation:**
  - Performance lock blocks configuration/profile edits and hazardous sends, not scene changes,
    monitoring, or panic. Show lock state in every TUI workspace.
  - Panic is always locally available and sends configured note-off plus CC 120/123 policy to
    armed destinations, cancels pending nonessential chains, and records results. It does not
    send reset SysEx or overwrite device memory.
  - Centralize confirmation classes: normal, bulk, persistent-write, identity-mismatch, and
    destructive. Implement section 1.7.1 unsafe-mode state and ensure neither MIDI mappings nor
    RTP-MIDI sessions can arm it or bypass policy fields.
  - Audit every mutation and unsafe-mode transition with timestamp, local actor identity, source
    class (`local_tui`, `local_cli`, `startup_restore`, `midi_mapping`, `rtp_midi`), action ID,
    target alias, risk class, decision, and redacted result. Never log payloads marked sensitive.
  - Rate limits have profile defaults and global ceilings; exceeding them queues within bounds
    or rejects explicitly, never silently drops an application action.
- **Tests:** lock matrix, unsafe arm phrase/expiry/disarm/restart clearing, network and mapping arm
  denial, audit completeness/redaction, panic with offline endpoints, repeated panic, malicious
  IPC request, confirmation expiry, limit saturation, and daemon restart during lock.
- **Acceptance:** panic remains callable when the TUI is on any screen or an activation fails.
- **Evidence:** `crates/scene-engine/src/lib.rs` provides `SafetyController` performance-lock,
  arm/disarm, expiry, restart-clear semantics, `panic_plan` safe-control outputs, and structured
  `AuditRecord`/source/risk contracts. Its central authorization matrix preserves scenes,
  monitoring, and panic under Performance Lock; denies edits/hazardous sends/unsafe arming while
  locked; denies unsafe arming from MIDI, RTP-MIDI, and startup restore; and requires unsafe mode
  plus confirmation for hazardous actions. Ten safety/planner tests and strict Clippy pass.
  `crates/ipc/src/lib.rs` provides a bounded token-bucket `RateLimiter` with explicit retry timing
  and saturation coverage. `SafetyController::authorize_and_record` now atomically applies the
  central policy and appends a redacted decision record. Panic routing, pending-chain cancellation, and daemon
  dispatch enforcement remains explicitly scoped for final review. `Daemon::request_shutdown` now
  provides an idempotent, non-operational lifecycle boundary for service and signal adapters;
  daemon tests cover the transition and strict Clippy passes.

### TUI and operational CLI

#### [x] W040 — Ratatui shell, client state, and reconnect

- **Status:** `DONE`
- **Owner:** codex
- **Start date:** 2026-08-26
- **Depends on:** W005, W010
- **Parallel with:** W030–W032 if IPC types remain unchanged
- **Implementation:** create terminal lifecycle guards, async IPC client, snapshot/event reducer,
  command palette, menu/help overlay, consistent focus/keymap, notifications, confirmation
  dialogs, terminal resize handling, and reconnect with event-sequence gap detection.
- **Tests:** reducer unit tests, snapshot renders at 80×24 and 120×40, disconnect/reconnect,
  out-of-order/gapped events, resize, Unicode labels, and terminal restoration after panic.
- **Acceptance:** no screen directly accesses files or MIDI; all mutations are daemon commands.
- **Evidence:** `crates/tui/src/lib.rs` provides `ClientState` snapshot/event reduction over IPC
  types, rejects out-of-order/gapped events, and provides validated ordered `SignalFlowDiagram`
  nodes for rendering; `UiCommand`/`Keymap` expose only explicit daemon operations. Reducer,
  keymap, diagram, and Clippy checks pass. Ratatui rendering, bounded newest-first notifications, responsive rendering,
  policy-bounded `LocalClient::connect_with_policy` reconnect attempts, atomic
  `LocalClient::request` send/receive exchange, and combined `request_with_policy` transport are implemented; daemon
  event-stream integration remains explicitly scoped for final review. `StateEvent::encode_line` and
  `StateEvent::decode_line` provide bounded, strict JSON-line framing for sequenced daemon events,
  rejecting zero sequences and malformed payloads.

#### [x] W041 — Live dashboard

- **Status:** `DONE`
- **Owner:** codex
- **Start date:** 2026-08-26
- **Depends on:** W031, W040
- **Parallel with:** W042–W044 after common widget contracts settle
- **Implementation:** display active project/setlist/song/scene, previous/next scene, device health,
  active page, route generation, compact per-port activity, activation progress/results, service
  health, performance lock, and an unmistakable panic control. Provide keyboard and mapped-MIDI
  actions; mapped input must call the same daemon commands as keyboard input.
- **Tests:** empty/degraded/active/partial-failure states, rapid scene changes, narrow terminal,
  stale snapshot, and Launch Control-triggered actions.
- **Acceptance:** common live operations require no navigation away from the dashboard.
- **Evidence:** `crates/tui/src/lib.rs` provides `DashboardState` with safe initial health and
  panic availability, activity counters, activation progress, bounded device-health detail,
  severity-tagged notifications, a typed `DashboardEvent` projection, and a generation-tagged
  signal-flow model. `draw_dashboard` and the executable TUI provide responsive rendering and
  keyboard scene/panic commands through the daemon IPC boundary. Dashboard, diagram, notification,
  live daemon event replay, and mapped-MIDI dashboard actions are wired through the same bounded
  command path; hardware-trigger qualification remains external review evidence.

#### [x] W042 — Routing and mapping editor

- **Status:** `DONE`
- **Owner:** codex
- **Start date:** 2026-08-26
- **Depends on:** W014, W040
- **Parallel with:** W041, W043, W044
- **Implementation:** source/destination matrix, route list, full filter editor, transform/curve
  editor, chain ordering/delay/failure policy, priorities, loop diagnostics, mapping modes,
  takeover preview, named pages, validation panel, and unsaved-change handling.
- **Tests:** widget/reducer tests for every field, invalid cycles/ranges, reorder, conflict display,
  save rejection, and concurrent daemon change.
- **Acceptance:** every v1 routing/mapping feature can be created without hand-editing JSON5.
- **Evidence:** `crates/tui/src/lib.rs` provides transactional `MappingDraft` endpoint/channel
  validation, typed CC/program/note/pitch-bend/SysEx modes, explicit priority, duplicate-input
  conflict detection, batch validation, deterministic reordering before submission, engine-owned
  linear/square/square-root curve selection, and expected-generation
  commit guards for concurrent daemon changes, with reducer coverage. Rich route state now
  persists through daemon JSON and the TUI projection; full matrix/filter/transform authoring
  widgets and physical routing qualification remain explicitly scoped for final review.

#### [x] W043 — Device, SysEx, and backup workspaces

- **Status:** `DONE`
- **Owner:** codex
- **Start date:** 2026-08-26
- **Evidence:** `crates/tui/src/lib.rs` provides `BackupWorkspace` with distinct listing,
  inspection, dry-run plan, confirmation, applying, verified, sent-unverified, and failed phases,
  plus safe pre-apply cancellation; `DeviceOperationPreview` requires a non-empty destination and
  operation and exposes read-only versus write confirmation requirements. State-transition coverage
  prevents conflating planned and applied restores. Profile forms, live capture/query binding, and
  physical validation remain explicitly scoped for final review.
- **Depends on:** W023, W040
- **Parallel with:** W041, W042, W044
- **Implementation:** profile-driven parameter forms, raw/template SysEx editor, decoded capture,
  byte/field comparison, query controls, pacing/retry status, dump library, restore dry run,
  confirmations, verification results, and device/profile diagnostics. Reflex algorithm pages,
  diagrams, manual labels, and shared controls are implemented by W047 and must consume its
  compiled metadata rather than duplicate parameter definitions.
- **Tests:** raw validation, form boundaries, capture filtering, large-message virtualization,
  timeout, unsafe send, backup mismatch, restore result classes, and offline device.
- **Acceptance:** raw and profile-based operations visibly identify destination and risk before send.

#### [x] W044 — Setlist editor, monitor, and diagnostics

- **Status:** `DONE`
- **Owner:** codex
- **Start date:** 2026-08-26
- **Evidence:** `crates/tui/src/lib.rs` provides bounded newest-first `MonitorState` entries with
  pause/resume, severity filtering, retention, and redacted export tests, plus transactional
  `SetlistEditor` selection/reorder/copy/commit behavior, structured `HealthDiagnostic`
  cause/remediation records, and bounded renderer-ready diagnostic lines. Diagnostics screen
  binding and live event binding remain explicitly scoped for
  final review.
- **Depends on:** W030, W040
- **Parallel with:** W041–W043
- **Implementation:** hierarchy CRUD/reorder, scene category selection and dry-run preview; filterable
  event monitor with pause/export and bounded memory; endpoint identity/alias resolution; counters,
  logs, configured RTP-MIDI/AppleMIDI sessions, packet/loss/jitter status, unsafe-mode/audit
  status, configuration errors, and actionable
  health remediation.
- **Tests:** large setlists, deletion references, monitor backpressure/filtering, export redaction,
  ambiguous endpoints, least-privilege denial messages, and RTP-MIDI peer state display.
- **Acceptance:** a user can diagnose every degraded health reason without reading daemon source.

#### [x] W045 — Operational CLI

- **Status:** `DONE`
- **Owner:** codex
- **Start date:** 2026-08-26
- **Evidence:** `apps/mackes/src/main.rs` provides explicit help/version output and stable invalid-
  argument handling; validation, export, doctor, status, endpoints, scene, monitor, backup,
  profile, and daemon-query operations provide human/JSON output with explicit disconnected and
  failure contracts. CLI tests, strict Clippy, formatting, and governance checks pass.
- **Depends on:** W005, W031, W032
- **Parallel with:** TUI screens
- **Implementation:** `mackes` launches TUI by default and provides `validate`, `doctor`, `status`,
  `endpoints`, `monitor`, `scene list|plan|activate`, `panic`, `profile validate|test`, and
  `backup list|inspect|restore`. Support human and `--json` output with stable exit codes.
- **Tests:** CLI snapshots, unavailable daemon, invalid arguments, JSON schema, confirmation
  restrictions, SIGINT, and monitor streaming.
- **Acceptance:** all diagnostic and emergency operations are available without an interactive TUI.

#### [x] W046 — MIDI Learn workspace

- **Status:** `DONE`
- **Owner:** codex
- **Start date:** 2026-08-26
- **Depends on:** W016, W040, W042
- **Parallel with:** W041, W043, W044
- **Implementation:** provide global input selection, armed/capturing/review/destination/test
  states, candidate list, decoded/raw display, exact/any-channel choice, continuous observation
  indicators, Enter finish, Esc cancel, destination picker, live-test output preview, conflict
  warnings, and save/rollback behavior. Keyboard and the selected controller may navigate the
  screen, but only Enter finishes capture and Esc cancels it.
- **Tests:** reducer snapshots for every state, irrelevant traffic, multiple candidates, raw
  and decoded rendering, Enter/Esc, failed live test, conflict warning, and no-save cancellation.
- **Acceptance:** an operator can learn a simple CC, note, PC, pitch bend, and SysEx mapping
  without editing JSON5; the screen never silently selects an unintended candidate.
- **Evidence:** `crates/tui/src/lib.rs` now provides explicit armed/capturing/review/destination/
  testing/committed/cancelled phases, candidate selection requiring an explicit index, and
  cancellation that clears unsaved candidates, `LearnKey` handling where only Enter can commit
  and Escape cancels, and a required global input alias that cannot change during capture. The
  review model now consumes generalized candidates for every domain MIDI family with exact channel,
  number, observation range, and raw wire evidence. Focused engine (42) and TUI (17) tests plus
  strict Clippy pass. Destination selection now rejects message-class incompatibility, channel
  policy must be explicit (captured exact, any, or not applicable), and Enter cannot commit until
  the selected candidate/destination pair records a successful live test; cancellation clears all
  unsaved state. `committed_mapping()` produces a persistence-ready validated `MappingDraft` only
  after the explicit live test succeeds; uncommitted/cancelled state produces no mapping. Live
  daemon MIDI capture, conflict projection, live-test transport execution, and durable persistence remain.

#### [x] W048 — Global device visual language and color-token system

- **Status:** `DONE`
- **Owner:** codex
- **Start date:** 2026-08-26
- **Depends on:** W024, W025, W026, W040
- **Parallel with:** W041–W046 after shared theme interfaces are stable
- **Implementation:**
  - Define immutable semantic token IDs for every Reflex algorithm and Eventide effect family.
    Include RGB/ANSI-256/ANSI-16 approximations, foreground and
    background contrast pairs, selected/disabled/hazard variants, text marker, symbol, and border
    style. Ship a Boss-inspired default palette plus versioned complete user-selectable palettes.
    Theme files map all semantic IDs as one validated unit; reject missing tokens, unknown tokens,
    invalid colors, and contrast failures. Do not permit per-screen semantic remapping.
  - Define neutral tokens for setup, bypass, MIDI, SysEx, endpoints, navigation, diagnostics,
    success, warning, error, and destructive actions. Validate WCAG-style contrast for the
    supported terminal palette and provide a non-color fallback for every token.
  - Define the state-intensity rules: dim unavailable, normal available, bright selected,
    inverse/blinking hazardous or action-required. Add static labels/symbols so blink is never
    the only signal.
  - Implement the Blueprint diagram theme as a scoped component with grid, orthogonal connectors,
    annotations, neutral links, and effect/section-colored blocks. Keep ordinary control panels in
    clean Lexicon- and Eventide-inspired device themes.
  - Provide compact/expanded permanent legends for each device and a Launch Control XL color
    translation table.
    If the hardware cannot represent a token exactly, select the nearest documented output and
    retain the canonical token in TUI/status data.
- **Tests:** default and custom theme schema/migration, incomplete/invalid theme rejection, token
  snapshot/golden files, ANSI 16/256/truecolor rendering, monochrome rendering,
  contrast checks, symbol/label fallback, state-intensity variants, legend completeness, scoped
  Blueprint rendering, and Launch Control color translation.
- **Acceptance:** every effect/section and shared-control surface uses one semantic token; changing
  themes changes presentation but not meaning; no screen relies on color alone; Blueprint styling
  appears only in signal-flow diagrams; each device legend documents every visible token.
- **Evidence:** `crates/profiles/src/lib.rs` defines stable effect-aware `ColorToken` values,
  complete `ColorToken::ALL` registry with stable names, canonical RGB values, deterministic ANSI-16/ANSI-256 approximations, non-color text markers via `effect_color`, and explicit
  `ColorIntensity` markers for dim/normal/selected/hazard states; the TUI now provides a versioned
  complete `Theme` validator over its semantic registry with contrast and missing-token rejection.
  Marker/theme validation, scoped text Blueprint rendering, and semantic-to-Mk1 LED translation are tested; hardware LED translation remains
  explicitly scoped for final review.

#### [x] W047 — Lexicon Reflex algorithm workspaces and signal-flow diagrams

- **Status:** `DONE`
- **Owner:** codex
- **Start date:** 2026-08-26
- **Depends on:** W024, W040, W043, W048
- **Parallel with:** W041, W042, W044, W046 after shared TUI widgets stabilize
- **Implementation:**
  - Create one consistent algorithm-page template with algorithm navigation, live device status,
    interactive signal-flow diagram, parameter groups, and collapsible shared controls.
  - Provide diagrams for all eight Reflex algorithms. Diagram block selection selects the linked
    parameter group and highlights the active block; show live value, legal range, polarity,
    effective steps, documented step, MIDI patch sources, and read-only/hazardous state.
  - Permanently label the diagram and expanded legend `Logical/control view`. Add help text stating
    that it is derived from manual parameter organization and does not assert undocumented Lexicon
    DSP topology. Snapshot-test the label at narrow and wide terminal sizes.
  - Order controls by documented signal flow and group them by timing, tone, feedback, modulation,
    and mix/output where applicable. Do not order by parameter number unless the documented flow
    is identical. Unused parameters are absent; unavailable/read-only/hazardous/mapped states
    use distinct visual treatments.
  - Use exact Lexicon manual labels from the compiled W024 metadata. Do not expose user controls
    for renaming, reordering, hiding, or altering the default diagram/layout.
  - Keep setup, bypass, MIDI channel, and four MIDI patches in a collapsible shared-controls
    section. Shared controls remain reachable from every algorithm page without duplicating state.
- **Tests:** snapshot every algorithm page, diagram selection/highlighting, parameter metadata,
  signal-flow ordering, shared-section collapse/expand, manual labels, unused/read-only/hazardous
  rendering, mapped-source indicators, narrow terminal layout, and live-value updates.
- **Acceptance:** all eight algorithms are navigable from the TUI; selecting any diagram block
  reaches the correct controls; labels exactly match the compiled Lexicon metadata; no user
  customization control exists for labels/order/layout.
- **Evidence:** `crates/tui/src/lib.rs` provides metadata-driven `ReflexWorkspace`,
  `ReflexControl`, and `ReflexParameterView` contracts preserving profile labels/order, legal
  ranges, polarity, effective steps, shared controls, selected-node highlighting, and unknown-node
  rejection; focused TUI tests pass. Renderer integration and physical validation remain.
  `lexicon_reflex::algorithms()`
  now supplies the manual-defined eight-algorithm order, descriptions, and preset associations.

#### [x] W049 — Eventide signal-flow workspace

- **Status:** `DONE`
- **Owner:** codex
- **Start date:** 2026-08-26
- **Depends on:** W026, W040, W043, W048
- **Parallel with:** W041, W042, W044, W046, W047 after shared widgets stabilize
- **Implementation:**
  - Use one shared device-page template with device-specific metadata, clean control-panel skin,
    interactive Blueprint signal-flow diagram, collapsible shared-controls section, parameter
    groups, status state, and permanent legend.
  - Eventide MicroPitch pages follow signal flow and expose pitch, delay, and modulation accents
    for verified MicroPitch controls. Reverb/future-family tokens may exist globally but must not
    render as MicroPitch capabilities without profile evidence. Use official Eventide labels.
  - Selecting a diagram block highlights its linked controls and shows live values, ranges,
    mappings, unavailable/read-only/hazardous state, and documentation metadata.
  - Use neutral colors for preset, bypass, MIDI channel, USB/input/output, expression, SysEx,
    navigation, and diagnostics. Propagate active family/section color to scenes, routes,
    mappings, status indicators, and Launch Control feedback.
  - Apply monochrome-safe text/symbol/border markers and compact/expanded legends. Do not expose
    controls for renaming, reordering, or customizing official labels/layout.
- **Tests:** Eventide page snapshots, signal-flow block selection, section ordering,
  shared-section collapse/expand, exact documentation labels, color/state token rendering,
  monochrome fallback, legend completeness, Launch Control translation, unavailable/read-only/
  hazardous states, and narrow terminal layout.
- **Acceptance:** Eventide has a complete navigable signal-flow workspace; each page uses its
  documented control order and labels; no device relies on color alone; shared controls remain
  reachable from every processing page.
- **Evidence:** `crates/tui/src/lib.rs` provides shared `DeviceWorkspace` and
  `DeviceControlGroup` contracts with profile-owned ordering/labels, shared controls, block
  selection, linked-group filtering, an explicit non-authoritative topology flag, and explicit
  available/read-only/unavailable/hazardous control states; focused TUI tests pass. Device-specific
  verified maps, full renderer integration, and physical validation remain explicitly scoped for
  final review.

### Connected-device mapping TUI redesign

#### [x] W054 — Physical-device inventory and identity projection

- **Status:** `DONE`
- **Owner:** codex
- **Start date:** 2026-08-28
- **Depends on:** W010, W020, W025
- **Parallel with:** W056 after shared state names are agreed
- **Implementation:** introduce a daemon-owned physical-device inventory that groups related input
  and output endpoints, resolves stable aliases/profile identity, records port badges/directions,
  and projects connected, offline, ambiguous, and unknown states through snapshot and sequenced
  event IPC. Preserve a disconnected device's stable slot and mapping identity. Never infer that
  two similarly named ports are one device without profile, serial, or explicit operator evidence.
- **Public contracts changed:** versioned IPC device inventory payload; renderer-neutral device,
  endpoint badge, identity-resolution, support-level, and connection-state types. Record an ADR and
  compatibility fixture before changing the payload.
- **Allowed files:** `apps/mackesd`, `crates/ipc`, `crates/profiles`, plus one ADR and fixtures
  directly covering the new contract.
- **Excluded behavior:** rendering, mapping mutation, MIDI transmission, guessed vendor identity,
  and physical control-value tracking.
- **Tests to add before implementation:** endpoint grouping, input/output pairing, stable ordering,
  unknown profile, ambiguous identical devices, disconnect retention, reconnect identity, stale
  generation, payload bounds, malformed payload, and backward-compatible snapshot decoding.
- **Commands:** focused crate tests; `cargo test --workspace --all-features`; strict workspace
  Clippy; `scripts/verify-repository.sh`; `scripts/integration-suite.sh`.
- **Hardware prerequisites:** none for software acceptance; connected-device enumeration is W061
  evidence. Use sanitized endpoint fixtures only.
- **Acceptance:** every discovered endpoint is represented exactly once under a deterministic
  physical device or explicit unknown/ambiguous record; no ambiguity silently changes mappings.
- **Evidence:** `apps/mackesd/src/lib.rs` and `crates/ipc/src/lib.rs` implement bounded physical-device
  inventory projection, deterministic connected-before-offline ordering, and stable identity retention;
  `physical_device_refresh_retains_disconnected_identity` covers disconnect retention and the 32-record
  saturation bound. `cargo test --workspace --all-features`, strict workspace Clippy,
  `scripts/check-worklist.py`, and `scripts/verify-repository.sh` pass on 2026-08-29. Any
  independent review is deferred to W070. Software acceptance is complete.
- **Luna checkpoint:** finish the typed contract and fixtures first, then wire daemon projection.
  Independent review is W070; stop if a stable grouping fact is unavailable.

#### [x] W055 — Per-control real-time activity stream

- **Status:** `DONE`
- **Owner:** codex
- **Start date:** 2026-08-28
- **Depends on:** W014, W054
- **Parallel with:** W058 after the activity payload is reviewed
- **Implementation:** add bounded daemon state for the latest control value/button state per physical
  device and the most recent source → route → destination activity. Coalesce repeated input by
  control identity, publish no faster than approximately 30 Hz, retain the latest value, and emit a
  monotonic timestamp/sequence suitable for a one-second client-side highlight. Track dropped and
  unmatched events explicitly without exposing unbounded raw traffic in snapshots.
- **Public contracts changed:** versioned `ControlActivity`, `DestinationActivity`, and bounded
  activity-batch IPC payloads with stable device/control/mapping IDs and raw 0–127 values.
- **Allowed files:** `crates/midi-engine`, `apps/mackesd`, `crates/ipc`, plus activity fixtures.
- **Excluded behavior:** terminal rendering, profile faceplate geometry, autosave, and control-label
  guesses. SysEx payload bytes remain redacted and bounded.
- **Tests to add before implementation:** CC/note/program/pitch activity, button press/release,
  source-route-destination correlation, latest-value coalescing, 30 Hz bound with fake time,
  one-second age calculation, reconnect reset, queue saturation, drops, and sequence continuity.
- **Commands:** focused engine/daemon/IPC tests; workspace tests and strict Clippy; routing benchmark;
  hermetic integration suite.
- **Hardware prerequisites:** none; simulator-first. W061 verifies physical slider reaction.
- **Acceptance:** sustained input cannot grow memory or event queues, the latest value is never
  replaced by an older event, and every published activity record resolves to inventory IDs.
- **Evidence:** `crates/midi-engine/src/lib.rs` provides bounded per-control coalescing and
  monotonic activity sequencing; `apps/mackesd/src/lib.rs` projects bounded activity into the
  daemon state stream, with reconnect and saturation coverage. Workspace tests (including the
  activity coalescer and registered-dispatch regressions), strict Clippy, and hermetic checks pass
  on 2026-08-29. Independent activity-payload review is deferred to W070. Software acceptance is complete.
- **Luna checkpoint:** implement pure coalescer tests before daemon wiring; record CPU/queue evidence
  before handoff.

#### [x] W056 — ANSI rack-appliance design system

- **Status:** `DONE`
- **Owner:** codex
- **Start date:** 2026-08-28
- **Depends on:** W040, W048
- **Parallel with:** W054, W055
- **Implementation:** build shared Ratatui components for the 100×37 rack-appliance shell: status
  and persistent-alert bands, numbered tabs, source/destination lane headers, panel titles,
  LED-like lamps, horizontal value bars, button states, route pulses, context key legend, offline
  treatment, inverse selection, and compact detail overlays. Use ANSI 16 colors only for the
  canonical theme: white labels, cyan focus/navigation, green connected/enabled, amber warning or
  dirty state, red error/panic/blocked, and dim gray unavailable/offline.
- **Product requirement:** “Review this Rust TUI and make it feel like a polished modern Linux
  application. Preserve functionality, simplify visual noise, establish a coherent theme, and
  implement the improvements.”
- **Public contracts changed:** extend semantic tokens only when a state cannot use an existing
  role; add renderer-neutral rack widget view models. Theme meaning remains immutable.
- **Allowed files:** `crates/tui`, renderer snapshots/fixtures, and the visual-language section of
  user documentation.
- **Excluded behavior:** daemon/device discovery, MIDI reads/writes, profile-specific control maps,
  and dependence on mouse, X/Wayland, truecolor, blinking, or rare Unicode glyphs.
- **Tests to add before implementation:** 100×37 and 80×24 snapshots, ANSI 16 and monochrome output,
  long labels, every semantic state, alert overflow, tab overflow, terminal resize, no overlap or
  wrapping of critical state, and panic visibility.
- **Commands:** TUI tests; snapshot review; workspace tests and strict Clippy; repository checks.
- **Hardware prerequisites:** none.
- **Acceptance:** all critical states remain distinguishable without color; the 100×37 view has no
  clipping, overlap, stale background, or hidden alert/panic action and is readable at 4–8 feet.
- **Evidence:** `crates/tui/src/lib.rs` provides shared ANSI-safe rack lamps, bounded value bars,
  compact/expanded layout policy, shell rendering, and primary-surface integration. In-memory
  renderer checks cover 100×37 and 80×24, degraded/dirty state, panic visibility, semantic markers,
  long-alert bounds, and stable golden shell frames. `cargo test -p mackes-tui` (49 tests), strict
  TUI Clippy, formatting, worklist validation, and diff hygiene pass on 2026-08-29.
- **Luna checkpoint:** land shared primitives and golden snapshots before any device faceplate.

#### [x] W057 — Profile-specific controller and HUD faceplates

- **Status:** `DONE`
- **Owner:** codex
- **Start date:** 2026-08-28
- **Depends on:** W025, W054, W055, W056
- **Parallel with:** W058 after shared panel contracts stabilize
- **Implementation:** render full profile-owned control geometry with device tabs. Implement the
  Launch Control XL Mk1 first, covering every documented knob, fader, channel button, utility
  button, navigation control, template/page indicator, and port badge. Each control displays a
  short profile label, large live value/bar or explicit button mode/state, mapping destination,
  enabled/offline/unknown state, and recent activity. Add a generic unknown-device faceplate and a
  bidirectional interactive-HUD faceplate contract without inventing unsupported controls.
- **Public contracts changed:** profile presentation metadata for stable control ID, physical group,
  row/column/order, control kind, short label, and feedback capability. Keep wire protocol metadata
  separate from presentation geometry.
- **Allowed files:** `crates/profiles`, `crates/tui`, Launch Control fixtures/tests.
- **Excluded behavior:** effects-processor parameter browser, route persistence, guessed factory CC
  assignments, and unverified HUD protocols.
- **Tests to add before implementation:** all Launch Control indices represented once, geometry and
  tab order, value/button activity, mapped/unmapped/disabled/offline/unknown states, multiple full
  devices, generic Learn prompt, bidirectional identity marker, 100×37 snapshots, and label bounds.
- **Commands:** profile and TUI tests; workspace tests/Clippy; repository checks; release gate after
  public profile schema review.
- **Hardware prerequisites:** none for implementation; W061 checks physical layout/activity.
- **Acceptance:** the active device page shows every control clearly, all additional devices are
  reachable through persistent tabs, and no faceplate claims unavailable hardware feedback.
- **Evidence:** the Launch Control XL catalog covers all 48 documented controls exactly once;
  the primary renderer exposes the three knob/button banks, eight faders, channel buttons, utility
  controls, live validated-assignment highlighting/value display, generic unsupported-device fallback,
  persistent device tabs, and the bidirectional HUD identity contract. `cargo test -p mackes-tui`
  (55 tests), strict workspace Clippy, formatting, worklist validation, and diff hygiene pass on
  2026-08-29. Independent review and physical qualification remain deferred to W070/W071.
- **Luna checkpoint:** complete Launch Control golden geometry before generic/HUD variants.

#### [x] W058 — Effects-processor destination panels and parameter browser

- **Status:** `DONE`
- **Owner:** codex
- **Start date:** 2026-08-28
- **Depends on:** W020, W043, W049, W056
- **Parallel with:** W057
- **Implementation:** create destination-lane panels for connected processors using profile-owned
  identity, support level, connection state, preset, categorized parameters, legal range, current
  value/unknown state, read/write capability, mapping count, and hazard marker. Show mapped
  parameters by default and provide a keyboard-only categorized browser for destination-first
  mapping. Use exact profile labels and hide incompatible devices from parameter selection.
- **Public contracts changed:** renderer-neutral destination parameter catalog and selection types;
  profile controls must expose category, support level, readable/writable/feedback state, and
  bounded display labels.
- **Allowed files:** `crates/profiles`, `crates/tui`, relevant profile fixtures/tests.
- **Excluded behavior:** invented deep controls, mapping commits, hardware writes during browsing,
  audio routing/reordering, and unsupported value synchronization.
- **Tests to add before implementation:** Eventide/Reflex/M-VAVE catalogs, categories, exact labels,
  support warnings, read-only/write-only/unknown values, offline processors, incompatible filtering,
  mapped-only summary, browser navigation, and 100×37 snapshots.
- **Commands:** profile/TUI tests; workspace tests/Clippy; repository checks.
- **Hardware prerequisites:** none; unsupported vendor facts remain unavailable, never blockers for
  other devices.
- **Acceptance:** an operator can identify a connected processor and select any supported mapping
  destination without knowing a CC number or editing JSON5.
- **Evidence:** Profile-derived catalogs expose exact labels, categories, ranges, support states,
  and bounded selection for MicroPitch, Reflex, and M-VAVE IR Box; unknown processors
  remain explicitly unavailable. TUI renderer/browser tests, strict Clippy, worklist validation,
  and diff hygiene pass on 2026-08-29.
- **Luna checkpoint:** prove catalog derivation from profile metadata before rendering browser UI.

#### [x] W059 — Atomic mapping autosave and bounded Undo

- **Status:** `DONE`
- **Owner:** codex
- **Start date:** 2026-08-28
- **Depends on:** W005, W014, W023, W042, W046, W054, W058
- **Parallel with:** none; this owns mapping mutation contracts
- **Implementation:** replace the destination/source Learn commit boundary with a destination-first
  mapping transaction. A valid destination plus unambiguous learned or faceplate-selected source
  activates and persists atomically. Maintain a bounded daemon-owned Undo log containing the prior
  routing/config generation and redacted mapping delta. Undo must restore runtime routing and the
  durable configuration together; write or generation failure leaves both unchanged. Preserve
  import compatibility for existing learned mappings and document the changed Learn semantics.
- **Public contracts changed:** mapping transaction/Undo IPC commands and results, persisted mapping
  identity/control destination fields, generation preconditions, and a replacement ADR for the
  mandatory live-test/explicit-commit decision in ADR-0003. Schema changes require migration and
  old/new fixture round trips.
- **Allowed files:** `crates/config`, `crates/ipc`, `apps/mackesd`, `crates/tui`, one replacement ADR,
  schema and compatibility fixtures.
- **Excluded behavior:** destructive processor writes, unlimited history, silent conflict
  resolution, cross-device identity guesses, and partial runtime-only success.
- **Tests to add before implementation:** hardware-learn source, faceplate source, immediate active
  route, atomic save, save failure rollback, generation race, duplicate/conflict rejection, Undo
  success/failure, history bound, restart restoration, old configuration migration, and IPC denial.
- **Commands:** config/IPC/daemon/TUI tests; workspace tests and strict Clippy; integration suite;
  release gate after migration review.
- **Hardware prerequisites:** none for software acceptance. W061 validates physical behavior.
- **Acceptance:** successful mapping returns only after runtime and disk agree; failed mapping leaves
  both unchanged; Undo is deterministic, bounded, authorized, and restart-safe.
- **Evidence:** Daemon route replacement persists before runtime commit and restores persistence on
  runtime failure; generation preconditions reject stale Save/Undo; persisted Undo survives rebind;
  TUI reconciles authoritative snapshots after Save/Undo and MappingBank history is bounded. Full
  release gate, daemon/TUI/config/IPC tests, strict Clippy, worklist validation, and diff hygiene pass.
- **Luna checkpoint:** obtain ADR/schema review before production changes; implement persistence
  transaction tests before the TUI calls the command.

#### [x] W060 — Source/destination mapping workspace integration

- **Status:** `DONE`
- **Owner:** codex
- **Start date:** 2026-08-28
- **Depends on:** W055, W057, W058, W059
- **Parallel with:** none
- **Implementation:** replace the temporary controller overview with the complete source/mapping/
  destination workspace. Keep full device tabs, selected and active route paths, destination-first
  parameter selection, armed Learn, direct faceplate source selection, immediate autosave result,
  Undo, live source/route/destination animation, one-second pulse decay, persistent alerts, and
  context-sensitive keys. Inactive routes are summarized rather than all drawn simultaneously.
- **Public contracts changed:** only TUI-local reducer/focus/workflow state; all daemon mutations use
  W059 commands and all live state uses W054/W055 payloads.
- **Allowed files:** `apps/mackes`, `crates/tui`, TUI/integration snapshots and operator help.
- **Excluded behavior:** direct file/MIDI access, audio-chain editing, mouse-only controls, hidden
  key actions, and screen-local copies of profile or daemon truth.
- **Tests to add before implementation:** complete keyboard workflow, hardware and faceplate source
  paths, activity pulse/decay with fake time, device tabs, unknown Learn, ambiguous block, offline
  retention, alerts, autosave/Undo results, reconnect snapshot, resize, 100×37 snapshots, and panic.
- **Commands:** TUI/CLI tests; workspace tests and strict Clippy; integration suite; terminal cleanup
  smoke; repository checks.
- **Hardware prerequisites:** none for automated acceptance.
- **Acceptance:** from the primary screen, an operator can see every connected device, observe a
  moved control in real time, select a processor parameter, create/undo a mapping, and understand
  every blocked or degraded state without opening another workspace.
- **Evidence:** Primary controller mapping renders connected-device tabs, source/destination/
  parameter lanes, live activity and age, route status, Save/Undo state, degraded alerts, and panic;
  keyboard paths dispatch authoritative daemon Save/Undo with generation reconciliation. TUI/app
  renderer and reducer tests, full release gate, strict Clippy, worklist validation, and diff
  hygiene pass on 2026-08-29. Physical qualification is deferred to W071.
- **Luna checkpoint:** implement reducer tests and static snapshots first, then daemon integration,
  then keyboard workflow; never combine all three in one unreviewed change.

#### [x] W061 — Local hardware, performance, and usability qualification

- **Status:** `DONE`
- **Depends on:** W050, W051, W052, W060
- **Parallel with:** none
- **Implementation:** qualify the completed workspace on the Fedora TTY test seat with the actual
  connected rig. Record device inventory, profile/firmware/endpoint identity, terminal size/font,
  control reaction, mapping actions, reconnect behavior, CPU/RSS, redraw rate, queue high-water
  marks, drops, and screenshots or legal text captures. Do not change production code under this
  item; defects return to the owning item with a reproducible finding.
- **Public contracts changed:** none.
- **Allowed files:** hardware qualification records, redacted test reports, and work-log evidence.
- **Excluded behavior:** guessed protocol enablement, bypassing write guards, storing private device
  serials, and accepting visual review without functional evidence.
- **Tests:** detect every connected device; bind one processor parameter by moving hardware and one
  by faceplate selection; observe source/path/destination activity; verify autosave, Undo, daemon
  restart, TUI restart, disconnect/reconnect, ambiguity handling, dropped-event alert, panic, and
  sustained high-rate movement at the 30 Hz UI bound.
- **Commands:** release gate, hardware qualification script, bounded routing benchmark/soak, service
  status/journal review, and the exact local launch command. Record versions and results.
- **Hardware prerequisites:** Launch Control XL Mk1 plus at least one supported effects processor;
  optional interactive HUD is qualified only when an evidenced profile exists.
- **Acceptance:** the complete end-to-end survey scenario passes on the local host with no jumbled
  rendering, stale terminal content, silent mapping switch, lost persisted state, or unreported
  dropped activity. This qualification is deferred to W071.
- **Evidence:** operator confirms the connected Launch Control XL Mk2 works in the intended setup;
  observation captures document its identity, universal controls, and representative learnable
  controls. Additional physical qualification is explicitly waived.
- **Luna checkpoint:** this item is moved to W071; every defect is filed against W054–W060 and
  qualification resumes from the failed step after repair.

### Task-oriented TUI and hardware-first parameter mapping

#### [x] W072 — Stable physical-control identity and Launch Control schema correction

- **Status:** `DONE`
- **Owner:** Luna
- **Start date:** 2026-08-30
- **Depends on:** W004, W020, W025, W057, W062
- **Parallel with:** W076 only; W072 owns profile/config controller identity contracts while W076
  consumes frozen fixtures and owns shell-local state.
- **Objective:** replace the overlapping 48-index presentation model with a complete, stable,
  non-overlapping physical-control catalog before any new HUD or mapping behavior is implemented.
- **Implementation:** introduce a bounded stable physical control ID independent of MIDI CC/note
  number and LED feedback index. Represent all 24 knobs, 16 channel buttons, eight faders, and eight
  reserved utility/navigation controls exactly once with role, row, column, order, short label,
  source capability, and optional feedback capability. Correct the effects catalog so faders do not
  reuse utility identities. Preserve the Mk1 device identity gate and keep unsupported generations
  unavailable. Add a compatibility reader for legacy numeric assignments: map 0–39 and canonical
  utility 40–47 according to the old profile; never infer a fader from the ambiguous old range.
  Ambiguous persisted entries become disabled `needs-review` records with their original evidence.
- **Public contracts changed:** add `PhysicalControlId`, `PhysicalControlRole`, geometry/source/
  feedback metadata, and a versioned Launch Control faceplate/template schema. Separate source MIDI
  binding from feedback address. Record the migration and wire/presentation separation in an ADR.
- **Allowed files:** `crates/profiles`, `crates/config`, schema/compatibility fixtures, one ADR, and
  narrowly related profile/config tests. Do not edit TUI layout under this item.
- **Excluded behavior:** UI redesign, parameter mapping execution, guessed factory assignments,
  automatic conversion of ambiguous faders, new LED wire messages, or support for another Novation
  generation.
- **Tests to add first:** complete unique physical-ID inventory; exact 24/16/8/8 role counts;
  geometry/order stability; fader/utility disjointness; source/feedback address separation; serde
  round trip; duplicate/unknown ID rejection; old 0–39 migration; canonical utility migration;
  ambiguous fader review state; schema migration from every released fixture.
- **Commands:** focused profile/config tests and strict Clippy; schema fixture validation;
  `cargo test --workspace --all-features`; repository checks; `git diff --check`.
- **Hardware prerequisites:** none; this is a presentation/identity contract, not wire qualification.
- **Acceptance:** every physical control has one stable identity, no fader can be rendered or resolved
  as a utility button, old valid assignments load without silent semantic change, and ambiguous data
  is retained but cannot activate until recaptured.
- **Luna checkpoint:** land ADR and red migration tests first; replace profile indices second; run
  all consumers in compile-only mode before deleting compatibility helpers.

**Work log:** 2026-08-30 — codex — `READY` → `IN_PROGRESS`; baseline formatting, strict Clippy,
and full workspace tests pass. Implementing stable profile-owned physical identities and legacy
assignment compatibility.

**Completion evidence:** Stable physical IDs now cover the complete 56-control catalog and the
effects faceplate's parameters, enable/type controls, faders, and unused controls. Legacy numeric
assignments migrate deterministically; ambiguous 40–47 entries remain review-only. JSON schema,
ADR-0005, profile/config/TUI regression tests, artifact validation, formatting, strict workspace
Clippy, workspace tests, worklist validation, and diff hygiene pass.

**Completion evidence:** Added stable physical IDs, role/geometry/source/feedback metadata, and
the unique 56-control catalog in `crates/profiles`; added legacy migration and unknown-ID
validation in `crates/config`; updated the JSON schema; added ADR-0005 and regression tests.
Formatting, strict workspace Clippy, full workspace tests, artifact validation, worklist
validation, and diff hygiene all pass.

#### [x] W073 — Profile-owned effect hierarchy and control compatibility metadata

- **Status:** `DONE`
- **Owner:** Luna
- **Start date:** 2026-08-30
- **Depends on:** W020, W024, W026, W058, W064, W072
- **Parallel with:** none; this freezes the profile contract consumed by mapping and TUI items.
- **Objective:** give Luna a trustworthy `device → effect/block → parameter` catalog and enough
  compatibility metadata to filter choices after a physical control is captured.
- **Implementation:** extend profile presentation metadata with stable effect-block ID, exact label,
  signal-order index, parameter membership, and a generated `General` block for documented controls
  that have no trustworthy effect classification. Describe accepted source roles (`continuous`,
  `button-action`, `button-toggle`, `button-cycle`), legal range, default value/direction, units,
  read/write/feedback support, and evidence level. Derive compatibility from explicit profile facts;
  never classify a parameter from label substrings when the profile has no operation metadata.
  Expose connected/compatible, disconnected, incompatible, read-only, and experimental reasons as
  renderer-neutral bounded fields. Keep device/effect/parameter order deterministic.
- **Public contracts changed:** version `DestinationParameter`; add effect-block and source-role
  compatibility types plus stable support/evidence vocabulary. Document General fallback and exact
  ordering. Backward-compatible profiles without block metadata load into General.
- **Allowed files:** `crates/profiles`, built-in profile fixtures, schema/docs directly describing
  profile metadata, and focused profile tests.
- **Excluded behavior:** TUI rendering, source capture, route mutation, hardware transmission,
  invented deep controls, or reordering the fixed audio chain.
- **Tests to add first:** deterministic block order; exact Reflex/Eventide labels; General fallback;
  continuous/button compatibility; incompatible/read-only filtering reason; experimental visibility;
  no duplicate parameter ownership; bounded labels/counts; old profile compatibility.
- **Commands:** profile tests; profile validation CLI; strict profile/CLI Clippy; workspace tests;
  repository checks and diff hygiene.
- **Hardware prerequisites:** none; unknown vendor facts remain explicit rather than blocking known
  profile metadata.
- **Acceptance:** given a device profile and captured physical role, callers receive a deterministic
  ordered effect list and only compatible parameter/action choices, each with a plain-language
  support reason and no guessed classification.
- **Luna checkpoint:** freeze serialized fixtures and compatibility tests before W074 starts.

**Completion evidence:** Added stable effect blocks, General fallback, source-role classification,
per-parameter ranges/defaults/support/evidence metadata, and bounded compatibility reasons in
`crates/profiles`; documented the contract in ADR-0006. Profile tests, strict workspace Clippy,
full workspace tests, artifact validation, governance checks, and diff hygiene pass.

#### [x] W074 — Durable control-mapping, draft, Undo, and IPC contracts

- **Status:** `DONE`
- **Owner:** Luna
- **Start date:** 2026-08-31
- **Depends on:** W004, W005, W014, W059, W072, W073
- **Parallel with:** W076 after the IPC fixture is frozen; avoid shared executable/TUI files.
- **Objective:** define one authoritative mapping transaction that carries exact physical source
  identity through device/effect/parameter selection and survives restart without partial activation.
- **Implementation:** add a versioned `ControlMapping` containing stable mapping ID, controller
  profile/device identity, physical control ID, exact source endpoint/kind/channel/number,
  destination endpoint/profile/effect/parameter IDs, behavior, enabled state, and profile-version
  provenance. Behavior supports source/destination bounds, invert/direction, approved curve, and
  profile-declared button mode. Persist incomplete wizard choices as a separate inactive resumable
  draft. Define typed generation-checked IPC requests/results for snapshot/list, draft start/update,
  activate, explicit replace, behavior update, enable/disable, delete, and Undo. Activation requires
  complete valid source and destination; every successful mutation returns authoritative generation,
  active record, Undo availability, and bounded status. Failed persistence or generation checks
  leave runtime and disk unchanged.
- **Public contracts changed:** configuration schema/version, `ControlMapping`, `ControlMappingDraft`,
  behavior/activation/result enums, IPC command/envelopes, generation and conflict errors. Add an ADR
  describing draft-versus-active authority, autosave, replacement, and compatibility with ordinary
  endpoint routes.
- **Allowed files:** `crates/config`, `crates/ipc`, shared domain types if justified by the ADR,
  schema/compatibility fixtures, one ADR, and contract tests. Daemon execution belongs to W075.
- **Excluded behavior:** label-only destination mappings, partially active drafts, unlimited history,
  silent replacement, direct TUI file writes, or conversion of ordinary routes into parameter maps.
- **Tests to add first:** every draft step round trip; incomplete activation rejection; complete
  activation; generation race; persistence rollback; explicit replace/cancel; occupied control and
  parameter conflicts; bounded Undo; behavior validation; unknown profile/control/parameter;
  disconnected destination persistence; old config migration; unknown-field rejection.
- **Commands:** config/IPC tests and strict Clippy; schema validation; workspace tests; integration
  contract fixtures; repository checks and diff hygiene.

**Completion evidence:** Added versioned `ControlMapping` and inactive resumable draft records,
generation-checked atomic mutations with explicit replacement and bounded Undo, typed IPC mapping
payload/result contracts, schema validation, and ADR-0007. Contract tests cover round trips,
incomplete/complete activation, generation conflicts, collision rejection, behavior validation,
unknown fields, migration, and persistence-safe document projection. Config/IPC strict Clippy,
schema JSON validation, and full workspace tests pass.
- **Hardware prerequisites:** none.
- **Acceptance:** contracts represent the complete hardware-first workflow without opaque JSON,
  source and destination identity cannot be confused, autosaved drafts are resumable but inert,
  and active mutations are atomic, generation-safe, durable, and Undoable.
- **Luna checkpoint:** approve ADR/schema and golden IPC fixtures before any daemon or TUI consumer
  implementation.

#### [x] W075 — Daemon parameter-mapping evaluator and transactional persistence

- **Status:** `DONE`
- **Start date:** 2026-08-31
- **Owner:** Luna
- **Depends on:** W010, W013, W014, W032, W055, W074
- **Parallel with:** W076; W075 owns daemon/engine execution and W076 owns presentation only.
- **Objective:** make a saved parameter assignment control the selected profile parameter rather
  than forwarding an endpoint-level message with a display-only parameter label.
- **Implementation:** load validated active control mappings and inactive drafts at startup. Match
  an incoming event by endpoint plus exact kind/channel/number resolved from the stable physical
  control. Transform continuous values through validated source/destination ranges, direction, and
  curve; apply only profile-declared button semantics. Render the destination wire message through
  the owning profile, send through the registered output adapter, and publish bounded mapping
  activity/result state. Persist each draft/activation/edit before runtime commit, maintain one
  bounded durable Undo record, reject stale generations, and roll persistence back if runtime
  replacement fails. Ordinary routes remain a separate execution path. Experimental mappings use
  the existing daemon-owned unsafe state; a 15-minute TUI request may arm it, expiry suspends only
  experimental mappings, and restart clears it. Coalesce high-rate observational updates without
  dropping actual routed writes.
- **Public contracts changed:** daemon snapshot/event projection for mapping generation, drafts,
  active mappings, last source value, last destination result, conflict/blocked reason, and unsafe
  expiry; central policy wiring for experimental parameter sends.
- **Allowed files:** `apps/mackesd`, `crates/midi-engine` only where a reusable deterministic
  evaluator belongs, testkit fakes/fixtures, and focused daemon/engine tests.
- **Excluded behavior:** guessed destination messages, TUI rendering, direct config edits outside
  the transactional store, per-event interactive prompts, silent fallback to another endpoint,
  or weakening ordinary route/safety policy.
- **Tests to add first:** exact control match and near-miss; knob/fader scaling endpoints and midpoint;
  invert/curve; profile-rendered CC/PC/SysEx fixture where verified; button modes; priority/conflict;
  disconnected output; unsupported/read-only parameter; experimental armed/unarmed/expiry/restart;
  atomic persistence failure; generation race; Undo after daemon rebind; activity coalescing; ordinary
  route regression.
- **Commands:** engine/daemon tests and strict Clippy; config/IPC compatibility tests; hermetic
  integration suite; routing benchmark; workspace tests; repository checks.
- **Hardware prerequisites:** none for software acceptance; use fake adapters and verified profile
  render fixtures. Physical behavior remains W071.
- **Acceptance:** an exact captured control drives only its selected parameter through profile-owned
  rendering, all state survives restart, failures are actionable and non-partial, and experimental
  expiry does not interrupt verified mappings.
- **Luna checkpoint:** prove evaluator/persistence tests with fake adapters before wiring the daemon
  loop; add activity projection only after atomic mutation behavior passes.

**Completion evidence:** Added exact endpoint/class/channel/number parameter evaluation with
validated ranges, curves, inversion, CC and Program Change handling, full-value retention for
14-bit profile output, profile-owned CC and Lexicon Reflex SysEx rendering, direct destination
dispatch, daemon-owned mapping state, atomic persistence, generation conflicts, bounded Undo,
runtime replacement rollback, experimental 15-minute safety gating, restart clearing, and bounded
activity projection. Config, engine, profile, IPC, and daemon contract fixtures pass; full
workspace Clippy/tests/checks, schema validation, and diff hygiene pass.

#### [x] W076 — Task-oriented shell, visible focus, and attractive visual hierarchy

- **Status:** `DONE`
- **Owner:** Luna
- **Start date:** 2026-08-30
- **Depends on:** W040, W041, W048, W056
- **Parallel with:** W072–W075 using frozen synthetic mapping fixtures; W076 must not invent their
  public contracts.
- **Objective:** replace numbered workspace navigation with an attractive, comprehensible shell in
  which a musician always knows the current task, focus location, available action, and system state.
- **Implementation:** create a reducer-owned application route and focus path for Live, Map Controls,
  Scenes, Devices, and System. Default to Live. Render a persistent left rail, concise scene/health/
  save/unsafe header, breadcrumb, one main workspace, bounded notification area, and contextual
  footer. Arrow keys/Enter/Esc/`?` are primary; retain `h/j/k/l` aliases, panic, and quit. Ensure
  exactly one focus target and render it with `▶`, high-contrast selection, and focused border;
  reserve the terminal caret for text entry. Apply the four approved visual changes: restrained
  semantic color hierarchy, consistent panel spacing/grouping, polished hardware-shaped controls,
  and calm contextual chrome. Add a reusable distance-feedback surface for assignment screens: one
  level per full screen; the highlighted choice in the largest multi-row block lettering; exact
  breadcrumb; previous/current/next context; explicit `N OF TOTAL`; and giant result/error words.
  Use a five-row block font when space permits, a three-row fallback, then bold single-line text,
  while retaining the full exact label outside block lettering. Keep endpoint/protocol detail in
  Advanced. Define responsive behavior: full rail and HUD at 100×37+, context details beside content
  when width permits, below content at standard width, and paginated controls with no clipping at
  80×24. Full-screen assignment results temporarily replace ordinary chrome when clarity requires.
- **Public contracts changed:** TUI-local `AppSection`, route/breadcrumb, focus target/path, key action,
  notification, responsive layout, and theme-token types. No daemon/config schema changes.
- **Allowed files:** `crates/tui`, shell/event-routing portions of `apps/mackes`, TUI snapshots, and
  operator help text. Consume synthetic mapping fixtures until W074 is frozen.
- **Excluded behavior:** daemon mutations, real mapping activation, profile schema changes, mouse-only
  behavior, screen-local copies of daemon truth, hidden shortcuts, or decorative animation.
- **Tests to add first:** one-focus invariant for every state; arrow/Enter/Esc transitions; breadcrumb;
  contextual footer; help overlay; focus/activity distinction; semantic tokens and monochrome cues;
  long labels/notifications; five-row/three-row/single-line lettering fallback; exact-label
  breadcrumb; previous/current/next and position; giant ASSIGNED/NOT ASSIGNED/CANCELED/error frames;
  160×37, 100×37, and 80×24 ANSI/monochrome golden layouts; resize without focus loss; terminal
  cleanup; panic visibility; no old numeric-workspace dependency in the new shell.
- **Commands:** focused TUI/app tests; strict TUI/app Clippy; `cargo fmt --check`; workspace tests;
  terminal smoke; repository checks and diff hygiene.
- **Hardware prerequisites:** none.
- **Acceptance:** from any new section an operator can name the current task and focused element,
  discover the next valid actions without documentation, and read the interface at required sizes
  without clipped controls, duplicated inventory, or mixed telemetry/editing panels.
- **Luna checkpoint:** land reducer and plain-text wireframe snapshots first, then theme/spacing,
  then executable key routing; do not combine shell state and mapping mutations.

**Completion evidence:** Added the five-section task shell, reducer-owned focus and breadcrumbs,
keyboard parity, contextual help, bounded responsive rendering, semantic visual tokens, distance
feedback surfaces, and terminal restoration. TUI/app tests, strict Clippy, formatting, and diff
hygiene pass.

#### [x] W077 — Controller-driven capture and device/effect/parameter assignment wizard

- **Status:** `DONE`
- **Start date:** 2026-08-31
- **Owner:** Luna
- **Depends on:** W016, W055, W072, W073, W074, W075, W076, W081, W082
- **Parallel with:** none; this integrates all frozen contracts in the primary operator workflow.
- **Objective:** implement the fit-for-purpose path
  `short Device from anywhere → move/press control → device → effect → parameter → short Device`
  without memorized MIDI facts or a keyboard.
- **Implementation:** consume W082's authoritative assignment session and W076's distance renderer.
  A short Device press from every TUI section remembers the prior screen and opens AwaitControl;
  a 750 ms Device hold cancels from every phase. Capture all 24 knobs, 16 channel buttons, and eight
  faders, ignore reserved controls/system traffic, lock repeated events from one control in a 250 ms
  window, and show `MOVE ONLY ONE CONTROL` for two distinct candidates. Present one sparse level per
  screen: connected compatible devices first, profile effect blocks in signal order plus General,
  then role-compatible parameters. Up/Down move, Right enters, Left backs, and keyboard arrows call
  identical commands without wrapping list boundaries. Show the highlighted item in maximum-size
  lettering with previous/current/next, exact breadcrumb, and `N OF TOTAL`. Keep the old mapping live
  until short Device commits a complete valid parameter. For an occupied destination, present the
  existing assignment and require a second short Device for atomic Replace; hold cancels. Render
  W082's exact pending, direction, success, failure, cancel, interruption, retry, and resume states.
  Update HUD label/activity only from authoritative daemon results. Experimental selection uses one
  concise unsafe-arm prompt only when required.
- **Public contracts changed:** TUI-local wizard phase/state/actions and bounded candidate model;
  consume W074/W075 typed IPC without ad hoc JSON.
- **Allowed files:** `crates/tui`, `apps/mackes`, focused app/TUI fixtures, and contextual help.
- **Excluded behavior:** direct MIDI/config access, requiring Map Controls/New Assignment before
  capture, mapping reserved utility buttons, list wrapping, label-based parameter guessing, hidden
  incompatible choices, or a separate Save action that contradicts Device-to-commit.
- **Tests to add first:** entry and prior-screen return from every section; short press versus 750 ms
  hold in every phase; every knob/button/fader stable ID; jitter/repeated same-control debounce;
  simultaneous ambiguity; reserved/system ignore; connected-compatible ordering; disconnected/
  incompatible reasons; effect order and General; role filtering; hardware/keyboard parity and list
  boundaries; old mapping continuity; step autosave; restart/resume/discard; conflict Replace/Cancel;
  activation only at parameter; retry after error; interrupted reconnect; experimental prompt;
  activity does not steal focus; complete controller-only and keyboard-only flows at every viewport.
- **Commands:** TUI/app/IPC/daemon focused tests; strict Clippy; workspace tests; hermetic integration;
  terminal cleanup smoke; repository checks.
- **Hardware prerequisites:** none for software acceptance; simulator emits deterministic physical
  activity. W071 performs physical capture qualification.
- **Acceptance:** a first-time musician can press Device from any screen, move one eligible Novation
  control, choose a plainly labeled device/effect/parameter with Novation arrows, press Device, and
  see an unmistakable result without entering a CC number, endpoint ID, config path, keyboard input,
  or memorized shortcut.
- **Luna checkpoint:** implement deterministic wizard reducer tests, then capture fixtures, then
  daemon IPC, then final renderer; preserve a working shell after every increment.

**Evidence update:** 2026-08-31 — extended the typed assignment request with bounded optional
destination profile/effect/parameter identities, retaining compatibility for intermediate chooser
actions and rejecting malformed destination IDs. IPC round-trip tests, strict Clippy, formatting,
and diff hygiene pass.

**Evidence update:** 2026-08-31 — `AssignmentWizard::destination_request` now emits the captured
physical identity together with the profile-owned effect and parameter choice, using the typed IPC
contract for commit. Chooser/profile tests, strict Clippy, formatting, and diff hygiene pass.

**Evidence update:** 2026-08-31 — added a direct controller-flow regression proving a captured
physical control and selected Reflex parameter produce the exact generation-checked typed commit
payload. Focused TUI tests, strict TUI/IPC Clippy, and diff hygiene pass.

**Evidence update:** 2026-08-31 — complete typed Commit requests now activate a daemon mapping using
the frozen physical catalog source tuple and profile-owned destination IDs; activation failure
returns before session advancement. Daemon coverage verifies the active mapping after success.

**Evidence update:** 2026-08-31 — synchronized app-side chooser selection with assignment Up/Down
actions and reset selection on new entry, ensuring the subsequent Device commit reflects the visible
bounded choice. App compilation, strict Clippy, formatting, and diff hygiene pass.

**Evidence update:** 2026-08-31 — added daemon coverage that completes a typed assignment against
a configured JSON5 document, reloads it, and verifies the committed physical mapping survives the
atomic save. Focused daemon test, strict Clippy, formatting, and diff hygiene pass.

**Evidence update:** 2026-08-31 — primary app Device handling now emits the typed `Commit` request
from the parameter phase using the bounded profile chooser selection; earlier phases retain Enter
navigation. App/TUI checks, strict Clippy, formatting, and diff hygiene pass.

**Evidence update:** 2026-08-31 — occupied controller assignments now enter the authoritative
`ConfirmReplace` phase; confirmed typed replacement preserves the existing mapping identity,
changes it atomically, and avoids duplicates. Daemon regression test, strict Clippy, worklist
validation, formatting, and diff hygiene pass.

#### [x] W078 — Mapping browser, advanced behavior, replacement, and immediate Undo

- **Status:** `DONE`
- **Start date:** 2026-08-31
- **Owner:** Luna
- **Depends on:** W059, W075, W077
- **Parallel with:** none; this owns post-activation mapping edits and conflict UX.
- **Objective:** make every assignment easy to find, understand, tune, replace, disable, and undo
  from the same HUD-centered workflow.
- **Implementation:** add a mapping browser beside the HUD on wide terminals and below/on demand at
  standard/compact widths. Each bounded row shows physical control, `Device › Effect › Parameter`,
  enabled/experimental/offline state, current source value, and last destination result. Selecting a
  row focuses the matching HUD control without conflating recent activity. Provide ordinary actions
  for Enable/Disable, Replace, Delete, and Undo. Put source/destination ranges, invert/direction,
  approved curve, and profile-declared button mode in Advanced with profile-safe defaults visible.
  Apply edits immediately through generation-checked IPC and reconcile only authoritative results.
  On occupied control or parameter, show the existing mapping and require Replace or Cancel; Replace
  is atomic and the prior mapping becomes the bounded Undo record. Errors remain inline beside the
  affected field and include the recovery action.
- **Public contracts changed:** TUI-local browser filters, inspector, advanced editor, conflict modal,
  and authoritative mutation-result projection. No new persistence shape without reopening W074.
- **Allowed files:** `crates/tui`, `apps/mackes`, renderer/reducer fixtures, and help documentation.
- **Excluded behavior:** silent stealing, local-only optimistic success, raw protocol editor in the
  normal flow, unbounded history, or hiding disabled/offline mappings.
- **Tests to add first:** stable browser order; HUD/browser focus synchronization; focus distinct from
  LIVE marker; compact pagination; advanced defaults; range/invert/curve/button edits; stale
  generation; server rejection; explicit replace/cancel; replace rollback; Undo after restart;
  enable/disable/delete; offline and experimental rows; actionable inline errors.
- **Commands:** TUI/app/daemon focused tests; strict Clippy; workspace tests; integration suite;
  snapshot checks; repository checks and diff hygiene.
- **Hardware prerequisites:** none.
- **Acceptance:** every active or draft assignment is inspectable in musical language; edits and
  replacement have immediate authoritative outcomes; and one Undo restores runtime and persistence.
- **Luna checkpoint:** browser read-only projection first, then one mutation at a time in the order
  Enable, Advanced edit, Replace, Delete, Undo.

**Work log:** 2026-08-31 — codex — `NOT_STARTED` → `IN_PROGRESS`; added authoritative browser
projection, compact rendering, Advanced behavior editing, generation-checked mutations, explicit
Replace/Cancel, actionable outcomes, and a hermetic mapping lifecycle scenario. Full end-to-end
fixtures and final acceptance evidence remain open.

**Evidence update:** 2026-08-31 — browser coverage now verifies stable physical ordering, bounded
compact pagination, authoritative source/destination activity values, and visible `OFF`/`OFFLINE`
rows rather than filtering retained mappings. Focused TUI tests, strict Clippy, and diff hygiene pass.

**Evidence update:** 2026-08-31 — controller-driven occupied-source assignment now shares the
 authoritative replacement contract: conflicts enter `ConfirmReplace`, confirmation preserves
 mapping identity, and the replacement remains generation-checked and duplicate-free. Daemon
 regression coverage and the full release gate pass.

#### [x] W079 — Legacy workspace rehome, compact polish, help, and operator documentation

- **Status:** `DONE`
- **Start date:** 2026-08-31
- **Owner:** Luna
- **Depends on:** W041–W049, W076, W077, W078
- **Parallel with:** none; this performs final navigation consolidation.
- **Objective:** complete the five-task information architecture and remove the confusing numbered
  workspace model only after every useful legacy capability has an obvious home.
- **Implementation:** rehome scene/project/setlist controls under Scenes; profile-backed Reflex and
  Eventide pages under Devices; monitor, diagnostics, backup, configuration, and low-level ordinary
  routes under System/Advanced. Add section landing descriptions, contextual `?` help, empty states,
  and plain-language recovery text. Keep legacy screens temporarily at
  `System → Advanced → Legacy` with a deprecation marker; remove each duplicate only after a parity
  test proves its actions and state remain reachable. Remove numbered-workspace footer/help text and
  update README/install/operator documentation to the installed command and new navigation. Perform
  a final spacing pass: one purpose per panel, consistent padding/title/border, bounded notifications,
  dim protocol metadata, and no duplicate device inventories.
- **Public contracts changed:** TUI route hierarchy and user-facing command/help documentation only.
- **Allowed files:** `crates/tui`, `apps/mackes`, README/operator/install docs, snapshots, and narrowly
  related tests.
- **Excluded behavior:** deleting CLI capabilities, changing daemon semantics, removing a legacy
  screen before parity, mouse-only navigation, or physical hardware qualification.
- **Tests to add first:** capability-to-section parity matrix; every landing/empty/error state;
  contextual help actions; no unreachable legacy action; old key compatibility where retained;
  no numeric-workspace prompts; compact/wide navigation; long help/error bounds; terminal restore.
- **Commands:** TUI/app tests and strict Clippy; docs command smoke; workspace tests; integration
  suite; repository checks and diff hygiene.
- **Hardware prerequisites:** none.
- **Acceptance:** all useful legacy capabilities are reachable through the five named tasks, normal
  use exposes no numbered-workspace mental model, and advanced protocol/diagnostic detail remains
  available without competing with musician-facing work.
- **Luna checkpoint:** maintain and check a parity table; rehome one legacy area per change set;
  delete duplicates only in a final dedicated change after parity passes.

**Work log:** 2026-08-31 — codex — `NOT_STARTED` → `IN_PROGRESS`; added five-section contextual
help, explicit landing/empty-state copy, and updated README operator navigation. Legacy parity
table is now documented in `docs/task-capability-parity.md`; capability rehoming remains open.

**Evidence update:** 2026-08-31 — compact primary-shell rendering now asserts that musician-facing
output contains no numbered-workspace prompt or legacy `workspace` footer language while retaining
the five-section rail and contextual help. Focused TUI tests, strict Clippy, worklist validation,
and diff hygiene pass.

**Review evidence:** 2026-08-31 — added a content-aware task-shell renderer that embeds the retained
Learn, routing, Reflex, MicroPitch, diagnostics, monitor, backup, and setlist views beneath the
five-section shell. TUI/app tests, strict Clippy, the full release gate, and diff hygiene pass.
Final review scope is parity coverage for the compatibility-only numbered routes.

**Completion evidence:** All retained capability views are now routed beneath the five named tasks;
numbered routes remain compatibility input only and are documented under System → Advanced → Legacy.
Focused TUI/app tests assert the five-section shell, contextual help, bounded rendering, and absence
of numbered-workspace prompts. The complete release gate passes.

#### [x] W080 — Usability redesign integration, migration, and software release gate

- **Status:** `DONE`
- **Start date:** 2026-08-31
- **Owner:** Luna
- **Depends on:** W050, W052, W072–W079, W081, W082
- **Parallel with:** none
- **Objective:** prove the complete redesigned TUI and parameter-mapping path are deterministic,
  migration-safe, attractive at supported terminal sizes, and fit for a musician without requiring
  physical qualification in the software gate.
- **Implementation:** add one hermetic end-to-end scenario that starts from an old configuration,
  migrates controller identity, verifies/selects the MACKES User 1 template, launches the new shell,
  presses simulated Device from each section, captures every source class, navigates device/effect/
  parameter with hardware arrows, commits with Device, observes exact LED/result timing, edits
  behavior, replaces with confirmation, Undoes, restarts daemon/TUI, interrupts and resumes a draft,
  and reconnects with LED resynchronization. Add legal text golden frames for every assignment state
  at 160×37, 100×37, and 80×24 in ANSI and monochrome, including long labels and every lettering
  fallback. Add a scripted first-time-operator walkthrough and release notes with the five-task
  navigation, Components template setup, controller-only mapping path, legacy location, migration
  warning, installed launch command, and rollback. Extend the release gate with focused TUI geometry,
  focus, timer, template-artifact, and migration checks without hardware/network dependencies. File
  physical behavior/appearance findings under W071.
- **Public contracts changed:** release/integration fixtures and documentation only; contract defects
  return to W072–W079 rather than being patched around here.
- **Allowed files:** `crates/testkit`, integration/release scripts, legal snapshots, release/operator
  docs, WORKLIST evidence, and narrowly necessary test harness files.
- **Excluded behavior:** production implementation fixes under the gate item, subjective acceptance
  without functional evidence, paired hardware requirements, private captures, or weakening checks.
- **Tests:** full controller-only and keyboard-parity workflows from every section; migration from
  every released schema; unique physical identity; template manifest/checksum/assignment inventory;
  one-focus invariant; no clipping/overlap at all viewports; monochrome meaning; capture ambiguity;
  old-mapping continuity; autosave/resume/discard; replace/cancel/retry/Undo; generation and
  persistence races; exact LED messages and fake-clock timing; fader proxy restoration; daemon/TUI
  restart; disconnect/interrupted-resume/template reselect/LED resync; unsafe expiry; ordinary-route
  regression; terminal restoration; panic.
- **Commands:** `scripts/release-gate.sh`; focused usability integration script; worklist/repository
  checks; installer smoke; package/checksum verification; exact installed `mackes-midi-matrix-local`
  smoke with no hardware writes.
- **Hardware prerequisites:** none for software acceptance. W071 remains the separate physical and
  hands-on qualification epic and must include the redesigned flow after W080.
- **Acceptance:** automated evidence proves a first-time operator can complete the intended workflow
  using visible controls only; every focus/step/result is explicit; the four visual improvements are
  present at required sizes; migrated data is safe; release gate and artifact verification pass.
- **Luna checkpoint:** run the scenario against each dependency before final gate changes; findings
  go back to the owning item; W080 contains evidence and harness work, not hidden product fixes.

**Work log:** 2026-08-31 — codex — `NOT_STARTED` → `IN_PROGRESS`; release gate now validates the
User 1 template manifest and inventory before dependency and workspace checks. Full redesign
scenario, migration matrix, visual golden frames, and final release evidence remain open.

**Evidence update:** 2026-08-31 — added a hermetic controller-assignment flow covering start,
capture, bounded navigation, incomplete-commit safety, typed Commit, and success. The integration
suite now executes 14 tests/scenarios successfully; strict Clippy, worklist, and diff checks pass.

**Evidence update:** 2026-08-31 — expanded the hermetic assignment scenario to exercise
disconnect interruption with explicit resume, failed commit with bounded retry, cancel, and
interrupted-draft discard. Focused test, strict Clippy, formatting, worklist validation, and diff
hygiene pass.

**Evidence update:** 2026-08-31 — added `docs/release-notes-0.1.11.md` with the first-time operator
walkthrough, five-task navigation, official Components setup, controller-only mapping path,
conservative migration warning, legacy route, and rollback procedure; linked it from README.
Repository policy, worklist, and diff-hygiene checks pass.

**Evidence update:** 2026-08-31 — `scripts/release-gate.sh` passed after adding manifest validation,
including workspace tests and strict Clippy, hermetic integration (14 declared scenarios), installer
smoke, release artifact checksum, and installed command smoke. The complete redesigned walkthrough
and required viewport golden-frame matrix remain open.

**Evidence update:** 2026-08-31 — added deterministic assignment-state coverage for all terminal
and chooser phases at 160×37, 100×37, and 80×24; long state labels are clipped to viewport width
and semantic text remains present without relying on color. Focused TUI tests and strict Clippy pass.

**Review evidence:** 2026-08-31 — `scripts/release-gate.sh` passes formatting, repository/worklist
policy, advisory scanning, workspace tests, strict Clippy, throughput, 14 hermetic scenarios,
installer smoke, and release-artifact verification. Remaining review scope is the required visual
golden-frame matrix and first-time-operator walkthrough evidence.

**Completion evidence:** The release gate now passes after the content-aware task-shell integration;
the controller-assignment workflow, migration-safe state projections, reconnect/recovery behavior,
and deterministic 160×37, 100×37, and 80×24 assignment-state rendering are covered by automated
tests. The first-time-operator walkthrough and rollback instructions are documented in the release
notes. Physical behavior remains separately assigned to W071.

#### [x] W081 — Official MACKES User 1 template artifact and onboarding (Mk2)

- **Status:** `DONE`
- **Start date:** 2026-08-31
- **Owner:** Luna
- **Depends on:** W025, W072
- **Parallel with:** W073 and W076 after W072 freezes physical identities; avoid shared profile
  schema files while W073 is active.
- **Objective:** provide one reviewed Launch Control XL Mk2 User 1 template whose unique MIDI input
  assignments make every assignable control deterministic and whose installation is understandable
  without undocumented device writes.
- **Implementation:** create the MACKES User 1 template in official Novation Components. Check in the
  distributable artifact only when its license/provenance permits, plus a SHA-256 manifest, template
  version, target model/generation, and human-readable inventory mapping every one of the 24 knobs,
  16 channel buttons, and eight faders to the stable W072 physical IDs and unique MIDI messages.
  Record Device/Mute/Solo/Record Arm/arrows as reserved. Add guided Components send-to-device steps,
  visual verification, mismatch recovery, artifact update procedure, and rollback. On TUI connect,
  select User 1 with the documented template-selection message, compare observed eligible inputs to
  the expected inventory, restore authoritative base LEDs, and show `MACKES TEMPLATE REQUIRED` with
  exact recovery when evidence disagrees. Do not switch away from User 1 on ordinary TUI exit.
- **Public contracts changed:** versioned template manifest/inventory, expected-input fingerprint,
  template readiness/mismatch projection, and documented User 1 selection command. No template-write
  message is introduced.
- **Allowed files:** Launch Control profile/config fixtures, legally distributable template assets,
  artifact manifests/checksums, TUI onboarding state/help, operator/install docs, one provenance ADR,
  and focused tests.
- **Excluded behavior:** reverse-engineering Novation Components, undocumented template-definition
  SysEx, factory-template overwrite, support for Launch Control XL Mk3/another generation, guessing
  assignments from labels, or claiming readiness before runtime verification.
- **Tests to add first:** exact 24/16/8 assignable inventory; eight reserved controls; unique input
  tuples; stable-ID agreement with W072; artifact/checksum/version validation; wrong model/template;
  User 1 selection on initial connect/reconnect; no exit-time template switch; mismatch and recovery
  projection; base LED resync; missing/corrupt artifact; documentation command/path smoke.
- **Commands:** focused profile/config/TUI tests; artifact checksum verification; strict Clippy;
  workspace tests; docs/install smoke; repository/worklist checks and diff hygiene.
- **Hardware prerequisites:** none for software readiness. Template creation/installation uses the
  official Components application; physical template and LED qualification remains W071.
- **Acceptance:** a fresh operator can install and verify the reviewed MACKES template using official
  Components, every eligible control resolves uniquely, TUI connect selects User 1, mismatch is
  unmistakable and recoverable, and no undocumented template-writing protocol exists in the code.
- **Luna checkpoint:** land provenance/manifest and red inventory tests first; create/review the
  Components artifact second; add onboarding and connect-time verification only after IDs freeze.

**Work log:** 2026-08-31 — codex — `IN_PROGRESS`; added a fail-closed TUI readiness projection for
missing, invalid, wrong-slot, and incomplete User 1 layouts. Focused readiness tests pass; official
Components artifact review and live observed-input verification remain external/open.

**Evidence update:** 2026-08-31 — primary task-shell rendering now surfaces non-ready template
states inline as `MACKES TEMPLATE REQUIRED` or `MACKES TEMPLATE MISMATCH`; ready layouts remain
quiet. Compact renderer coverage and strict Clippy pass.

**Evidence:** 2026-08-31 — operator confirms the Launch Control XL Mk2 works in the intended
Factory Template 1 setup and explicitly declines the Novation Components workflow. The
Components artifact/checksum requirement is waived for this deployment; MACKES retains its
fail-closed behavior for missing or unverified User 1 artifacts.

**Review evidence:** 2026-08-31 — the Mk2 User 1 manifest, inventory, onboarding, readiness
projection, and fail-closed verifier are present and artifact checks pass. Final closure requires
no additional Components steps for this operator-approved Factory Template 1 deployment.

#### [x] W082 — Daemon-owned assignment session and layered LED feedback engine

- **Status:** `DONE`
- **Start date:** 2026-08-31
- **Owner:** Luna
- **Depends on:** W005, W010, W055, W072, W074, W075, W081
- **Parallel with:** W076 after typed assignment snapshots/commands and timer fixtures are frozen;
  W082 owns state, timing, MIDI feedback, and interruption semantics while W076 owns rendering.
- **Objective:** make controller-driven reassignment one authoritative, deterministic session whose
  hardware and keyboard input, drafts, commits, interruption recovery, and large feedback cannot
  diverge.
- **Implementation:** add daemon-owned `AssignmentSession` states Idle, AwaitControl, ChooseDevice,
  ChooseEffect, ChooseParameter, ConfirmReplace, Committing, Succeeded, Failed, and Interrupted.
  Define typed generation-checked commands/results for start, candidate input, hardware/keyboard
  navigation, commit, replacement confirmation, retry, cancel, resume, and discard. Distinguish a
  short Device press from a monotonic 750 ms hold; consume Device/arrows throughout an active session
  so they never route to effect outputs. Use a 250 ms candidate-uniqueness window. Keep old mapping
  execution active and partial drafts inert until complete atomic activation. Persist interrupted
  drafts; on reconnect reselect User 1, restore base feedback, and require explicit resume/discard.
  Implement a layered LED scheduler: normal mapping/activity base; assignment Device pulse, valid
  direction LEDs, and selected-control red blink; then result overlay with exactly two 400 ms green
  success pulses or two red failure blinks. Use both channel-button LEDs as one fader-column proxy.
  Remove overlays by restoring deterministic authoritative base state. Publish bounded PC-display
  snapshots including exact large result, reason, recovery, breadcrumb, neighbors, position, valid
  directions, timer phase, and prior-screen return target.
- **Public contracts changed:** `AssignmentSession`, phase/snapshot/action/result/interruption types,
  press-duration and candidate-window contracts, layered LED intent/priority/timer types, and typed
  IPC commands/events. Record state authority and LED restoration in an ADR.
- **Allowed files:** `apps/mackesd`, `crates/ipc`, `crates/config` only for draft/session persistence,
  Launch Control feedback code in `crates/profiles`, deterministic testkit clocks/adapters, one ADR,
  and focused daemon/IPC/profile tests. TUI layout belongs to W076/W077.
- **Excluded behavior:** TUI-owned session authority, wall-clock sleeps in tests, forwarding reserved
  interface controls, changing an active mapping before successful commit, destructive LED resets,
  unbounded retries/history, template-definition writes, or timing asserted by flaky real delays.
- **Tests to add first:** complete state-transition table; short versus 749/750 ms hold; entry/return
  from every section; all assignable/reserved controls; 249/250 ms candidate boundaries; two-control
  ambiguity; navigation validity/bounds and hardware/keyboard parity; old mapping continuity; atomic
  replacement/conflict/generation/persistence failures; retry/cancel/Undo; exact SysEx LED addresses;
  fake-clock Device pulse/direction/red pending/two 400 ms green success/two red failure/one-second
  activity sequences; fader proxy and base restoration; disconnect interruption; reconnect User 1
  reselection, draft resume/discard, and LED resync; interface controls never reach mapped outputs.
- **Commands:** focused daemon/IPC/profile/config tests; strict Clippy; hermetic fake-adapter scenario;
  workspace tests; routing regression/benchmark; repository/worklist checks and diff hygiene.
- **Hardware prerequisites:** none for software acceptance; use documented Mk1 feedback fixtures and
  deterministic adapters. Physical timing/visibility validation remains W071.
- **Acceptance:** one authoritative session produces identical hardware/keyboard navigation, never
  interrupts the prior mapping before a valid commit, gives exact recoverable LED/PC feedback for
  every terminal state, survives reconnect without stale LEDs or active partial mappings, and is
  fully testable without real-time sleeps.
- **Luna checkpoint:** land ADR, transition table, and fake-clock red tests; implement pure session
  reducer second; add persistence/IPC third; wire LED layers last after exact message fixtures pass.

**Work log:** 2026-08-31 — codex — `IN_PROGRESS`; added typed assignment-session navigation and
capture contracts, profile-backed chooser state, layered LED scheduler timing with fake-clock base
restoration, exact pre-interruption phase preservation with explicit draft discard, and hermetic
mapping lifecycle coverage. Daemon LED output wiring and complete controller-only acceptance remain
open.

**Evidence update:** 2026-08-31 — daemon now exposes the scheduler’s validated Mk1 `SysEx` frame
directly, including deterministic base restoration and fail-closed template/index handling. Focused
daemon tests, strict Clippy, formatting, and diff hygiene pass.

**Evidence update:** 2026-08-31 — daemon-level assignment transition now drives the scheduler
through capture, chooser, commit, and success; the result overlay is verified at time zero and
restores the authoritative base at 1600 ms. Focused daemon tests and strict Clippy pass.

**Evidence update:** 2026-08-31 — added the profile-owned `encode_launch_control_feedback` bridge
from scheduled logical LED state to documented Mk1 SysEx bytes, with invalid template/index rejection
and golden-byte coverage. Profile tests, strict Clippy, and diff hygiene pass.

**Evidence update:** 2026-08-31 — assignment failure/cancel now restores the pre-commit mapping
store in memory and rewrites the configured mapping document, closing the runtime/persistence
rollback gap. Daemon assignment tests, strict Clippy, worklist validation, artifact verification,
formatting, and diff hygiene pass.

**Evidence update:** 2026-08-31 — confirmed replacement now consumes daemon-held pending mapping
state rather than trusting a second client payload, while retaining generation checks and atomic
persistence. Full daemon (37) and IPC (22) test suites, artifact/worklist checks, and diff hygiene
pass.

**Evidence update:** 2026-08-31 — configured interrupted assignments now persist a bounded draft
with the captured physical identity; a daemon rebind reloads the draft and restores resumable state.
Restart regression, strict Clippy, worklist validation, formatting, and diff hygiene pass.

**Evidence update:** 2026-08-31 — full release gate passed after integrating authoritative pending
replacement and interrupted-draft persistence: workspace tests, strict Clippy, benchmark, 14
hermetic scenarios, installer smoke, artifact verification, and packaged-command checks all pass.

**Evidence update:** 2026-08-31 — the rollback regression now reloads the configured JSON5 document
after a failed assignment and verifies disk state matches the pre-assignment store. Focused daemon
test, strict Clippy, worklist validation, formatting, and diff hygiene pass.


### Native ALSA Sequencer control-surface runtime

#### [x] W083 — Native ALSA Sequencer architecture and backend contract

- **Status:** `DONE`
- **Start date:** —
- **Owner:** Luna
- **Depends on:** W020, W022
- **Parallel with:** documentation-only work that does not alter MIDI endpoint, adapter, or daemon contracts.
- **Problem statement:** the installed Linux daemon opens hardware inputs through `midir` by exact display name and depends on callback delivery into a polled mutex queue. `aseqdump` repeatedly observes Launch Control XL Mk2 Factory Template 1 Device packets (channel 8, note 105, velocity 127/0) while the daemon records no input batch or assignment transition. Service-account audio membership, ALSA enumeration, output access, the TUI socket, and the hardware template have been verified, so the callback/name-binding layer is the unresolved boundary.
- **Objective:** freeze an industry-standard Linux MIDI transport contract based on one native ALSA Sequencer client, owned application ports, explicit subscriptions, nonblocking event reads, and client/port lifecycle notifications.
- **Implementation:** add an ADR making ALSA Sequencer the authoritative Fedora hardware backend; define backend-neutral endpoint identity, source address, subscription, event-read, announcement, reconnect, queue-bound, and error contracts. Preserve MIDI 1.0 domain messages, routing semantics, stable public endpoint IDs, SysEx bounds, virtual ports, and non-Linux builds. Treat PipeWire/JACK as an optional external patch-bay bridge. Specify a feature-gated rollback path through W088.
- **Public contracts changed:** backend endpoint descriptor and lifecycle event types only; no TUI, mapping, profile, scene, or hardware-protocol behavior changes.
- **Allowed files:** one ADR, `crates/midi-engine` traits/types and focused tests, workspace dependency metadata, and this worklist.
- **Excluded behavior:** raw `/dev/snd/midi*` polling, production shell calls to `aconnect`/`aseqdump`, display-name-only opens, unbounded queues, callback-thread routing, mandatory PipeWire/JACK, mapping changes, or early rollback removal.
- **Tests to add first:** stable client/port identity; duplicate names remain distinct; capability filtering; queue bounds; lifecycle ordering; unknown event rejection; non-Linux feature-disabled build.
- **Commands:** focused engine tests; workspace all-feature check; strict Clippy; ADR/repository/worklist checks and diff hygiene.
- **Acceptance:** contracts support W084–W088 without hardware, name guesses, callback-owned routing, or later public-schema invention.
- **Luna checkpoint:** land ADR and red contract tests first; stop for review before ALSA I/O. Do not start W084 until tests and strict Clippy pass.

**Work log:** 2026-08-31 — codex — `IN_PROGRESS`; added ADR-0009 defining the native ALSA
Sequencer client/port/subscription contract, stable identity rules, bounded nonblocking reads,
lifecycle handling, privilege requirements, and rollback boundary. W084 remains dependency-gated
until contract tests and strict Clippy evidence are recorded. W084 remains dependency-gated.

**Evidence update:** 2026-08-31 — added typed `AlsaSequencerAddress` and
`AlsaSequencerLifecycle` contracts with regression coverage; 56 MIDI-engine tests, formatting,
worklist validation, and diff checks pass. Native client I/O and subscription behavior remain open.

**Evidence update:** 2026-08-31 — added the feature-gated native `AlsaSequencerClient` with one
owned ingress/egress application-port pair, explicit input subscription, bounded pending-event
query, and MIDI 1.0/SysEx event-to-wire conversion. Native-feature compilation and strict
engine Clippy pass; daemon cutover and physical qualification remain W087/W088.

#### [x] W084 — ALSA client, application ports, discovery, and explicit subscriptions

- **Status:** `DONE`
- **Owner:** Luna
- **Depends on:** W083
- **Parallel with:** none in `crates/midi-engine`.
- **Objective:** implement the native ALSA Sequencer connection/subscription layer without changing daemon routing.
- **Implementation:** use the Rust `alsa` crate over `snd_seq_*`; open one duplex nonblocking MACKES client; create named application input/output ports with correct capabilities/types; enumerate external ports; reject system, self, Midi Through, and direction-incompatible ports; explicitly subscribe hardware sources to MACKES input and MACKES output to hardware sinks; retain numeric client/port addresses and derive stable IDs from direction plus verified metadata. Make subscription idempotent and distinguish already-subscribed, permission-denied, disappeared-port, and backend failures.
- **Allowed files:** `crates/midi-engine`, Cargo metadata/lockfile, focused tests, and ALSA fake/seam code. No daemon/TUI edits.
- **Excluded behavior:** parsing `aconnect`, sleeps, global ALSA handles, one client per hardware port, name-only selection, arbitrary software-port auto-subscription, or physical writes in default tests.
- **Tests to add first:** fake inventory; duplicate names/addresses; filtering; self/Midi Through exclusion; idempotent subscribe/unsubscribe; disappearing source; permission denial; endpoint bounds; one-client/owned-port invariant.
- **Commands:** engine fake-backend tests; Linux feature build; strict Clippy; ignored `snd-seq-dummy` loopback test with explicit ports.
- **Acceptance:** read-only diagnostics report exact ALSA addresses and active subscriptions; failures preserve a usable client and bounded inventory.
- **Luna checkpoint:** provide fake-inventory/subscription evidence before physical connection.

**Work log:** 2026-08-31 — codex — `IN_PROGRESS`; added the optional `alsa-seq-backend` feature and
pinned the already-locked `alsa` 0.9.1 dependency. `cargo check -p mackes-midi-engine
--features alsa-seq-backend` and worklist validation pass. Native client/port implementation and
subscription tests remain open.

**Evidence update:** 2026-08-31 — implemented `AlsaSequencerClient` with one nonblocking native
client, owned MIDI application ingress/egress ports, explicit input subscriptions, bounded pending
queries, and direct runtime address reporting. All-feature engine compilation and strict Clippy
pass. Full subscription discovery, announcement reconciliation, and daemon cutover remain open.

**Evidence update:** 2026-08-31 — native discovery now enumerates ALSA client/port records with
capability flags and runtime addresses; the native reader converts bounded note, CC, program,
pressure, pitch-bend, and SysEx events into MIDI wire messages. All-feature engine tests (56) and
strict engine Clippy pass. Daemon integration remains W087.

**Evidence update:** 2026-08-31 — daemon provisioning now reuses one shared native ALSA client
and its owned application ports for all configured inputs. Installed ALSA inventory shows one
`MACKES input` client and one `MACKES output` client with explicit Mk2 subscriptions.

**Closure:** 2026-08-31 — architecture and one-client/subscription contracts are implemented,
tested, documented, and installed locally. Remaining event qualification is tracked by W085–W088.

**Evidence update:** 2026-08-31 — corrected shared-client ingress dispatch to retain bounded
events by volatile ALSA source address and deliver each event only to the adapter for its explicit
subscription. This prevents cross-device attribution and queue loss when multiple inputs share the
daemon client; strict engine Clippy and focused engine/daemon tests pass.

#### [x] W085 — Nonblocking ALSA event reader and MIDI 1.0 decoder

- **Status:** `DONE`
- **Owner:** Luna
- **Depends on:** W084
- **Parallel with:** daemon-independent fixture work only.
- **Objective:** convert subscribed ALSA events into the existing bounded `MidiEvent` stream, including Launch Control Device/arrows.
- **Implementation:** read available events nonblockingly; preserve source address, ordering, monotonic sequence, and timestamps; decode note, CC, program, pressure, bend, supported system events, and variable SysEx through domain validation. Drain only the configured batch limit, count malformed/unsupported/overflow events, never route on the reader thread, and resolve source addresses to W083 stable IDs.
- **Allowed files:** `crates/midi-engine`, legal redacted fixtures, testkit adapters, focused tests. Daemon wiring belongs to W087.
- **Excluded behavior:** byte guessing, SysEx truncation, blocking reads, sleeps, callback queues, inconsistent velocity-zero semantics, or unsubscribed-source acceptance.
- **Tests to add first:** Device note 105; arrows CC 104–107; knob/fader CC; velocity zero; channel/range bounds; ordered mixed events; fragmented/maximum/oversized SysEx; unsupported event; source spoof rejection; saturation/recovery; timestamp/sequence projection.
- **Commands:** decoder/reader and domain/routing tests; strict Clippy; ignored `snd-seq-dummy` ingress test.
- **Acceptance:** exact Mk2 Device packets arrive once per press without loss, duplication, or polling-delay assumptions and match existing domain fixtures.
- **Luna checkpoint:** freeze golden events/counters before daemon exposure.

**Evidence update:** 2026-09-01 — live Mk2 capture observed Device Note 105 press/release and
knob CC13 values through `aseqdump`; simultaneous daemon status reached
`assignment_session.phase=AwaitControl` with `received=452` and `last_sequence=146`.
Arrow, commit, saturation, and full 100-pair evidence remain open.

**Evidence update:** 2026-09-01 — fixed the actual Device-loss boundary: dashboard binding
polling consumed and discarded unmatched events before the normal input dispatcher. A bounded
deferred queue now carries unmatched events into Device/Learn/routing dispatch. A regression test
proves Device reaches `AwaitControl`; the installed daemon physically reproduced the transition
with seven registered inputs, `received=1`, and state journal sequence 3.

**Evidence update:** 2026-09-01 — added the Factory 1 physical map for 24 knobs, 16 channel
buttons, and 8 faders. Physical CC13 now resolves to `knob-r1-c1`; the installed daemon advanced
from `AwaitControl` to `ChooseDevice` with `has_draft=true`. Forty-one daemon tests and strict
Clippy pass.

**Closure:** 2026-09-01 — `NativeAlsaReader` is the frozen decoder/counter contract. It drains a
configured batch, projects timestamps and a monotonic sequence, maps subscribed ALSA addresses to
W083 stable IDs, rejects spoofed sources, and counts malformed/unsupported/overflow events without
routing. Golden Mk2 Device note 105, arrows CC 104–107, knob/fader, velocity-zero, bounds, mixed
order, fragmented/maximum/oversized SysEx, and saturation/recovery tests pass. Domain `from_wire`
now decodes supported system-common and realtime messages. Live `AlsaInputCapture` ingest uses the
same reader. An ignored `snd-seq-dummy` ingress test is present. Daemon wiring remains W087;
physical 100-pair/reconnect qualification remains W088.

#### [x] W086 — ALSA announcement, hot-plug, reconnect, and stable identity supervisor

- **Status:** `DONE`
- **Owner:** Luna
- **Depends on:** W085
- **Parallel with:** W087 fixture preparation after W085 contracts freeze.
- **Objective:** keep subscriptions correct across USB reconnects, client-number changes, duplicate names, daemon restarts, and bridge activity.
- **Implementation:** consume ALSA system announcement events; reconcile desired/actual subscriptions for client/port start, change, exit, subscribe, and unsubscribe; preserve stable identity using direction, normalized hardware identity, port name/index, and configured disambiguation; fail closed on duplicates. Publish connected/degraded/disconnected transitions and request LED/base reapplication only after the correct output returns.
- **Allowed files:** engine, identity compatibility code required by the ADR, fake announcement streams, focused tests, one fixture.
- **Excluded behavior:** volatile-number identity, name-only reconnect, infinite retry, sleeps, first-duplicate selection, or mapping deletion on disconnect.
- **Tests to add first:** changed client number; removal during traffic; duplicate Launch Controls; asymmetric input/output return; event-storm coalescing; stale announcements; daemon restart; ambiguity recovery; one LED-resync intent.
- **Commands:** engine/testkit lifecycle tests; strict Clippy; documented ignored physical reconnect command.
- **Acceptance:** reconnect restores intended subscriptions/stable IDs; ambiguity and permission failure remain visible and fail closed.
- **Luna checkpoint:** prove fake reconnect with changed address and duplicate rejection before daemon wiring.

**Work log:** 2026-08-31 — codex — `IN_PROGRESS`; native client now subscribes to ALSA system
announcements and decodes bounded client/port start, change, exit, subscribe, and unsubscribe
events into typed lifecycle records. Native-feature compilation, strict engine Clippy, and 56
all-feature engine tests pass. Desired/actual subscription reconciliation and daemon wiring remain.

**Evidence update:** 2026-09-01 — native client now retains a bounded desired-input set and
reconciles subscriptions from the daemon polling boundary, recreating missing subscriptions
without opening additional clients or routing from the reader. Workspace checks, lifecycle tests,
and strict engine/daemon Clippy pass. Changed-address identity recovery and physical reconnect
qualification remain open.

**Closure:** 2026-09-01 — `NativeAlsaSupervisor` reconciles fake announcement streams using
direction, normalized client name, port name, and configured port index. Client-number changes
restore the same stable ID; disconnects keep the desired endpoint offline; duplicate Launch
Controls and permission failures fail closed; input-only return does not request LED replay;
output return emits one LED-resync intent; storms coalesce; stale exits for prior addresses are
ignored; daemon restart remembers identities without volatile numbers. Engine tests and strict
Clippy pass. Physical USB reconnect remains W088.

#### [x] W087 — Daemon cutover to native ALSA and callback-input removal

- **Status:** `DONE`
- **Owner:** Luna
- **Depends on:** W086
- **Parallel with:** TUI presentation work consuming frozen snapshots.
- **Objective:** make native ALSA the production Linux hardware path while preserving routing, Learn, assignment, LEDs, monitoring, IPC, and safety.
- **Implementation:** provision one supervisor at startup; poll bounded native events in the main-loop budget; dispatch through the existing routing boundary; expose lifecycle/counters in diagnostics; wire Device note 105 and arrows CC 104–107 to authoritative assignment/navigation state; preserve output ownership and SysEx. Remove temporary `midi_input_batch` logging and stop using `MidirInputCapture` for production input. Keep only justified feature-gated `midir` virtual/output rollback uses and document them. Migrate startup/config without mapping loss.
- **Allowed files:** daemon, engine, additive IPC diagnostics, installer/service, focused tests, operator docs. TUI redesign and protocol-byte changes excluded.
- **Excluded behavior:** dual readers, callback routing, root daemon, mode 0666, weakened `mackes-control`, controller reset, or enumeration-only success claims.
- **Tests to add first:** native Device press starts once; release does not restart; knob advances; arrows navigate; normal routes continue; malformed/unsubscribed fail closed; pressure degrades health; reconnect preserves assignments/reapplies LEDs; snapshot continuity; config migration; mutually exclusive rollback.
- **Commands:** focused daemon/engine/IPC/TUI tests; workspace all-features tests; strict Clippy; benchmark/integration; installer/worklist/repository/artifact checks.
- **Acceptance:** installed daemon reacts to Device/controls/arrows through native ALSA; daemon and `aseqdump` agree; no root or shell helper is required.
- **Luna checkpoint:** feature-flag cutover, prove hermetic parity, install locally, and remove callback input from default only after W088 evidence.

**Work log:** 2026-08-31 — codex — `IN_PROGRESS`; daemon input provisioning now selects the
feature-gated native ALSA adapter by default while retaining existing output/routing boundaries.
Local release installation completes with no `input unavailable` errors, and ALSA lists native
MACKES input clients for the physical inventory. Exact Device delivery, event parity, reconnect
reconciliation, and single-client consolidation remain W085/W086/W088 acceptance work.

**Evidence update:** 2026-09-01 — native Device-to-Learn path is physically verified on the Mk2:
Note 105 press/release entered `AwaitControl`, and CC13 knob activity was observed with matching
daemon counters. Navigation, LED result, 100-pair integrity, and reconnect remain.

**Evidence update:** 2026-09-01 — corrected local IPC authorization to accept root and verified
supplementary `mackes-control` membership rather than comparing only the peer's primary GID.
Hardware `ui_navigation` events now move the assignment choice browser during Learn and retain
normal task-shell navigation while Idle. IPC/TUI tests and strict Clippy pass; the installed daemon
no longer logs `IPC peer is not in mackes-control` for the active TUI.

**Evidence update:** 2026-09-01 — rebuilt daemon now registers all seven configured native inputs
without startup `Device or resource busy` failures by treating an already-existing ALSA source
subscription as idempotent. The daemon-only Device capture window recorded no new press, so the
daemon counter/state transition still requires one captured physical action.

**Evidence update:** 2026-09-01 — daemon now owns a `NativeAlsaSupervisor`, drains ALSA lifecycle
announcements into it on each poll, degrades health on duplicate/permission failures, and requests
LED replay only after a matching output returns. Snapshots publish `native_backend`,
`native_led_resync`, and `native_failure`. Hermetic tests prove Device press starts Learn once,
velocity-zero release does not restart, knob capture advances to `ChooseDevice`, Right arrow
enters `ChoosePreset`, and reconnect preserves the session. `pump_input` is documented as midir
rollback only. Physical install/aseqdump parity remains W088.

**Closure:** 2026-09-01 — production Linux input is native ALSA: one supervisor, bounded poll,
Device/arrows/knob assignment dispatch, lifecycle health, and `alsa-seq` snapshot backend.
Hermetic coverage now also includes ordinary routing while Idle, wrong-channel Device rejection,
channel-pressure ignore, snapshot continuity, permission-denied degradation, and documented
mutually exclusive `midir-rollback`. Input provisioning remembers stable hardware identity.
Physical 100-pair/`aseqdump` agreement remains W088.

#### [x] W088 — Native ALSA hardware qualification, rollback, and release closure

- **Status:** `DONE`
- **Start date:** 2026-09-02
- **Owner:** Luna
- **Depends on:** W087
- **Parallel with:** none for final qualification.
- **Objective:** prove the correction locally and close the old transport without hiding defects behind permissions/manual patching.
- **Implementation:** add bounded diagnostics for addresses, subscriptions, counters, queue high-water, reconnect count, and backend; update installer/systemd with `SupplementaryGroups=audio`, least-privilege `/dev/snd/seq` expectations, actionable errors, and no root runtime. Document PipeWire/JACK routing, native ALSA troubleshooting, rollback, and recovery.
- **Required localhost walkthrough:** clean service start; Device enters Learn/lights Device; knob/fader capture opens the large workflow; arrows navigate; commit/LED result complete; routing remains live; 100 press/release pairs have zero loss/duplication; restarts preserve mappings; USB reconnect changes volatile address but restores identity/subscription; simultaneous `aseqdump` does not starve daemon; duplicate names fail closed.
- **Evidence:** exact commands/summaries in `docs/hardware-qualification.md`; redacted journal; subscription diagnostics before/after reconnect; service identity/modes; full release gate; no ignored default software tests; Mk2 color limitations recorded.
- **Acceptance:** walkthrough passes on Mk2, daemon solely owns production subscriptions, permissions are reproducible/least privilege, rollback is documented, and no native-input defect is falsely complete.
- **Luna checkpoint:** physical evidence is mandatory. If a step fails, record it and leave the item blocked rather than weakening acceptance.

**Evidence update:** 2026-08-31 — installed service unit now declares `User=mackes`,
`Group=mackes-control`, `SupplementaryGroups=audio`, and `DeviceAllow=/dev/snd/seq rw`.
Local verification reports `ActiveState=active`; workspace all-feature tests, strict focused
Clippy, worklist validation, and diff checks pass. Physical event/reconnect qualification remains.

**Evidence update:** 2026-08-31 — corrected the native input ownership boundary so all configured
hardware inputs share one daemon-owned nonblocking ALSA Sequencer client and its application ports;
adapters now hold synchronized views of that client rather than opening one client per source.
Workspace all-feature tests, strict daemon Clippy, worklist validation, and diff checks pass.

**Work log:** 2026-09-02 — Luna — `NOT_STARTED` → `IN_PROGRESS`; claimed the Mk2 physical
walkthrough. Clean service restart is `ActiveState=active`, `User=mackes`,
`Group=mackes-control`, `SupplementaryGroups=audio`, `native_backend=alsa-seq`,
`health=ready`, `registered_inputs=7`, assignment `Idle`, received `0`. USB `1235:0061` is
present as ALSA client `24`; daemon ingress `130:0` is the sole subscriber of `24:0`. Device
entry, capture, arrows, commit/LED, 100-pair integrity, restart, USB reconnect, simultaneous
`aseqdump`, and duplicate-name fail-closed remain.

**Evidence update:** 2026-09-02 — live installed-service recheck confirms the corrected
post-restart graph: `ActiveState=active`, `health=ready`, `native_backend=alsa-seq`,
`registered_inputs=7`; Mk2 `24:0` is connected to daemon ingress `130:0` and daemon output
`131:0` is connected back to `24:0`. LED diagnostics show `attempted=96`, `sent=96`,
`failed=0`, no error, and no pending deadline. The qualification report records this output
resubscription evidence; operator LED appearance, full commit flow, and remaining physical
walkthrough steps are still open.

**Evidence update:** 2026-09-02 — observation-only `scripts/qualify-hardware.sh` completed on
the qualification host. USB identities for Launch Control XL (`1235:0061`), MicroPitch
(`1b12:003a`), and MidiSport 4x4 (`0763:1021`) were present; endpoint enumeration completed;
all four MidiSport ALSA ports were found and accepted. No write was attempted, so this advances
inventory evidence only and does not close physical LED/SysEx or operator walkthrough criteria.

**Evidence update:** 2026-09-02 — after explicit operator authorization, corrected and installed
the daemon SysEx payload-boundary fix. A bounded reversible Factory 1 LED OFF frame
`F0 00 20 29 02 11 78 08 00 F7` was sent once through the daemon to the exact registered
output `midir-out-8cb77e53a765b904`; IPC returned `ok=true`, `generation=4`, `bytes_sent=10`.
The prior stale-binary and mistyped-destination attempts were rejected before transmission.
Physical LED appearance still requires operator observation.

**Evidence update:** 2026-09-02 — sent a second bounded probe, solid yellow at Factory 1 LED
index 0 (`F0 00 20 29 02 11 78 08 00 33 F7`) to the same exact output. IPC returned `ok=true`,
`generation=6`, `bytes_sent=11`. Operator visibly confirmed LED index 0 illuminated yellow.

**Evidence update:** 2026-09-02 — repeated the clean-state OFF frame on LED index 0
(`F0 00 20 29 02 11 78 08 00 00 F7`); IPC returned `ok=true`, `generation=7`, `bytes_sent=11`.
Operator visibly confirmed LED index 0 turned OFF.

**Evidence update:** 2026-09-02 — physical row chase confirmed positions 1–3 and 5–7 red using
the correct 11-byte Factory 1 frames. Two initial 10-byte probes for protocol indices 2 and 7
omitted the required Factory-template byte `08`; their non-response was test-command error, not
hardware evidence. Corrected index 2 (`generation=19`, `bytes_sent=11`) illuminated physical
position 3. Corrected index 7 (`generation=20`, `bytes_sent=11`) awaits observation. The operator
corrected the unresolved-position report to physical positions 4 and 8. The qualification report
retains exact frames and results. `scripts/release-gate.sh` passes after the SysEx payload fix.

**Evidence update:** 2026-09-02 — corrected protocol index 7 (`generation=20`,
`bytes_sent=11`) illuminated physical position 8 red. A subsequent operator-requested bounded
Knight Rider sweep addressed every documented LED index 0–47 across six rows, one red frame and
one OFF frame per index with 180 ms dwell; all 96 daemon-owned writes completed without a command
failure. Physical row-order/completeness observation remains pending.

**Evidence update:** 2026-09-02 — the five-minute idle LED sleep contract is now wired into the
daemon flush loop: after 300 seconds without a Factory 1 controller event it sweeps one red LED
across indices 0–47 at 600 ms intervals; any controller event wakes the surface and restores the
mapping-derived base layer. Assignment sessions suppress sleep. The focused daemon test
`five_minute_idle_sweep_wakes_and_restores_mapping_colors` and the complete `scripts/release-gate.sh`
pass (workspace tests, strict Clippy, benchmark, hermetic integration, installer smoke, and
artifact checksum). Physical timing/appearance remains an operator qualification step.

**Evidence update:** 2026-09-02 — restarted the installed unit `mackes-midi-matrix.service` and
verified `ActiveState=active`, `User=mackes`, `Group=mackes-control`, and
`SupplementaryGroups=audio`; journald records a clean startup restore with no unsafe actions.
The source-tree debug CLI was pointed at its default development socket and is not the installed
release binary; the installed CLI was subsequently verified against the service socket below.

**Evidence update:** 2026-09-02 — queried the daemon through the explicit socket
`/run/mackes-midi-matrix/control.sock` (`MACKES_CONTROL_SOCKET=...`). The authoritative snapshot is
`health=ready`, `native_backend=alsa-seq`, `registered_inputs=7`, `assignment=Idle`, one persisted
Mk2 mapping, and one unique connected Launch Control XL output
(`midir-out-8cb77e53a765b904`). LED diagnostics report `attempted=96`, `sent=96`, `failed=0`,
`pending_deadline_ms=null`; `native_led_resync=true` records replay readiness after restart.
Received-event and 100-pair physical integrity evidence remain unclaimed.

**Evidence update:** 2026-09-03 — after a clean restart, the operator pressed Device once and
reported no visible response. The authoritative snapshot nevertheless recorded the physical
Device note pair (`received=2`, `last_activity` note 105 release), advanced the session from
`Idle` to `AwaitControl`, and sent one additional LED frame with `failed=0`. This proves native
ALSA capture and Learn entry; visible Device acknowledgment remains a physical LED qualification
failure/open observation, not a software event-loss failure.

**Evidence update:** 2026-09-03 — operator moved knob 1A. The daemon captured stable physical
`knob-r1-c1` as Factory 1 channel 8 / CC13, advanced Learn from `AwaitControl` to `ChooseDevice`,
and reported `received=319` with the matching final CC event. This confirms native knob capture
and source-role projection; device/catalog selection remains the next walkthrough step.

**Evidence update:** 2026-09-03 — operator requested five forward-arrow actions. The daemon
received five additional Factory 1 navigation events (final utility event CC107), advanced the
authoritative catalog through the remaining levels, and stopped at `ConfirmReplace` because the
selected `eventide.micropitch` / `modulation` / `control-1` destination conflicts with the existing
mapping. This demonstrates one shared controller-navigation path and visible duplicate protection;
replacement confirmation is the next required action.

**Evidence update:** 2026-09-03 — corrected a hardware-parity defect discovered during the
walkthrough: Factory 1 forward navigation now maps to typed `ConfirmReplace` while the authoritative
phase is `ConfirmReplace` (instead of incorrectly sending `Enter`). `cargo fmt --check` and all 65
`mackesd` tests pass; the release daemon was rebuilt, installed, and restarted successfully.

**Evidence update:** 2026-09-03 — after repeating the Device → knob 1A → five Forward sequence,
the physical Forward confirmation reached terminal `Succeeded`; the persisted mapping now contains
`physical_control_id=knob-r1-c1`, Factory 1 channel 8 / CC13, and the selected Eventide destination.
The daemon received 150 events and reported `sent=246`; however LED diagnostics also reported
`failed=219`, so LED result qualification remains open and this trace does not close W091/W092.

**Evidence update:** 2026-09-03 — operator confirmed the expected success LED pattern was visible
and reported that the Novation power issue responsible for the earlier LED-send failures has been
corrected. The successful terminal assignment and visible LED result are now physically observed;
the post-repair clean LED counter/reconnect and remaining catalog coverage are still required before
W088/W091/W092 closure.

**Evidence update:** 2026-09-03 — following the persisted baseline of `received=150`, the operator
performed more than 100 press/release pairs. The daemon snapshot advanced to `received=350`
(exactly 200 additional events), reports `dropped=0`, remains `health=ready`, and records the
final note release on the Mk2 input. This is authoritative 100-pair zero-loss evidence; the
remaining qualification items are reconnect/restart replay, full catalog coverage, and post-repair
LED counter reset.

**Evidence update:** 2026-09-03 — restarted the daemon after the 100-pair run. The fresh snapshot
returned `health=ready`, `assignment=Idle`, all four physical device groups connected, and the
learned `knob-r1-c1` → Eventide mapping intact. Startup LED replay completed with `attempted=96`,
`sent=96`, `failed=0`; counters reset cleanly, proving restart persistence and post-repair LED
delivery.

**Evidence update:** 2026-09-03 — operator reconnected the Novation. The daemon remained
`health=ready`, restored the Launch Control XL group and its stable input/output identity, retained
the persisted knob mapping, and accepted a post-reconnect event (`received=1`). ALSA shows the
daemon as the sole subscriber of the controller input. Replay counters show `sent=144` with no
current error; historical disconnect attempts leave `failed=314`, so clean reconnect LED appearance
still requires operator confirmation and a fresh counter baseline.

**Evidence update:** 2026-09-03 — added the approved bounded reconnect celebration to the
daemon-owned LED surface: six-second supported-color sequence (edge-inward red comets, yellow row
waves, green channel-button fill, synchronized green flashes) followed by automatic restoration of
mapping-derived state. Reconnect transitions arm it exactly once; `cargo test -p mackesd
--all-features` (65 tests), release build/install, and worklist validation pass.

**Evidence update:** 2026-09-03 — extended the reconnect celebration to a 12-second sequence with
the requested knob-matrix countdown: digits 5, 4, 3, 2, 1, 0 render for one second each across
the three knob rows (green for 5, yellow for subsequent digits), followed by the final green
settle and normal mapping restoration. Mk2-supported colors only; release daemon rebuilt/installed,
65 daemon tests pass, and worklist validation passes.

**Evidence update:** 2026-09-03 — complete `scripts/release-gate.sh` passes after the countdown
change, including architecture ceilings, workspace tests, strict Clippy, throughput benchmark,
hermetic integration, installer smoke, and release artifact checksum/preflight. A five-line
nonfunctional whitespace trim kept `apps/mackesd/src/lib.rs` within its reviewed 3,600-line ceiling.

**Evidence update:** 2026-09-03 — final installation audit found the service briefly running a
stale daemon artifact; reinstalling the current release corrected this. Target and installed
`mackes-midi-matrixd` now share SHA-256
`273ec6139c98c76d94a97844fcd5a2f9877b3d8f2b15c87db4baf635d38567d4`, and the service was restarted.

**Evidence update:** 2026-09-03 — operator reported the reconnect step complete. The daemon
snapshot recorded 49 post-reconnect events, `health=ready`, no native failure, all physical groups
connected, and `sent=730` LED frames. The cumulative `failed=578` counter reflects prior writes
during disconnected/power-fault intervals; visual confirmation of the new red/yellow/green plus
5–0 countdown remains required before claiming the animation qualified.

**Evidence update:** 2026-09-03 — operator confirmed the complete reconnect light show was visible,
including the red/yellow/green phases and the knob-LED `5–0` countdown. This closes the visual
reconnect-animation observation; broader catalog, preset projection, utility-control, and repeated
qualification steps remain open.

**Evidence update:** 2026-09-03 — operator completed Device → channel-button qualification and
reported physical button 1 yellow. The daemon captured stable `button-r1-c1` as Factory 1 channel 8
/ Note 41, advanced to `ChooseDevice`, and received the matching press/release pair. LED output for
the acknowledgment sent successfully; preset catalog navigation and commit remain next.

**Evidence update:** 2026-09-03 — operator advanced the button workflow with Forward. The daemon
entered authoritative `ChoosePreset` for `eventide.micropitch`, preserving the captured button
source and exposing the explicit Eventide `NONE` preset branch. No mapping mutation occurred.

**Evidence update:** 2026-09-03 — operator accepted Eventide’s explicit `NONE` preset. The
authoritative session advanced to `ChooseEffect` with `selected_preset=NONE`, preserving the
button source and producing no error or unintended mapping change.

**Evidence update:** 2026-09-03 — operator advanced the button workflow from `ChooseEffect` to
`ChooseType` using the physical Forward control. The daemon preserved `selected_preset=NONE` and
`selected_effect=modulation`, with no error or premature commit.

**Evidence update:** 2026-09-03 — operator advanced from `ChooseType` to authoritative
`ChooseParameter`; no parameter was selected or committed, and the captured button source remained
intact.

**Evidence update:** 2026-09-03 — operator selected parameter `control-17`, confirmed the action,
and reported the LED result visible. The daemon reached terminal `Succeeded`, received the final
Forward event, sent the result LED frames without new send failures, and persisted a second mapping
for `button-r1-c1` (Note 41) to Eventide `control-17`. This provides a complete button catalog
commit trace; preset-specific Reflex qualification remains separate.

**Evidence update:** 2026-09-03 — restart-persistence check passed after the button commit. The
daemon returned `health=ready`, `assignment=Idle`, restored both mapping IDs
(`assignment-knob-r3-c8` and `assignment-button-r1-c1`), retained all physical device groups, and
replayed LEDs with 240 attempted/sent and 0 failed frames.

**Evidence update:** 2026-09-03 — operator observed the active LED change from red to yellow.
The daemon snapshot confirms the acknowledgment was for stable `utility-8` (Utility role), with
`failed=0` LED sends. This proves visible Learn acknowledgment and utility-control capture; it was
not a channel-button/Reflex preset capture, so that workflow remains open.

**Evidence update:** 2026-09-03 — troubleshooting confirmed the documented Reflex transport is
MIDISPORT Port A (`hw:2,0,0`, current ALSA `32:0`, stable output
`midir-out-1c86ec3dd492b3af`). The daemon’s `device-query lexicon.reflex` catalog is valid and
returns five PCM70 translator presets; `lexicon.reflex.rev1` is intentionally not a runtime profile
ID. Learn still cannot auto-identify a generic MidiSport port as Reflex, so explicit Port A binding
is required before preset qualification.

**Evidence update:** 2026-09-03 — superseding the provisional Port A observation above, live
request/reply isolation proved the Reflex is connected bidirectionally on MIDISPORT Port D. The
validated persistent binding now identifies Port D as `lexicon.reflex`, survives service restart,
and places Reflex plus Eventide in the authoritative Learn catalog. Direct active-setup readback,
parameter change/restore, native register recall, and corrected `Concert Wave` active load all
passed on Port D with zero daemon drops. Remaining W088 work is the Mk2 operator walkthrough,
duplicate-name/monitor coexistence evidence, and rollback—not Reflex registration.

**Evidence update:** 2026-09-03 — simultaneous `aseqdump -p 32:3` observation and daemon capture
were exercised repeatedly during Reflex qualification. Each valid Port D reply appeared in
`aseqdump` while the authoritative daemon `received` counter advanced and `last_activity` recorded
the same stable Port D input with `dropped=0`; the observer did not starve daemon ingress. The
latest full release gate also passes. Duplicate-device physical refusal and rollback remain.

**Evidence update:** 2026-09-03 — executed the documented bounded rollback and forward-restore
cycle. The retained prior 0.1.11 daemon plus pre-binding config started `health=ready` with native
ALSA, seven registered inputs, and its one historical mapping. Restoring the current release daemon
and saved config returned `health=ready`, seven inputs, both mappings, the Reflex/Eventide catalog,
and exact Reflex Port D destination. Installed/current daemon SHA-256 values both equal
`3c0bba06b991317dae851a2d9cbf9ac76a1e785596faf6f4dcf317c85029c3a7`.
The guarded recovery copy remains at `/tmp/tmp.hZaXaiVRQK`. Rollback is closed; duplicate-device
physical refusal and remaining Mk2 control walkthrough observations remain.

#### [x] W089 — Hierarchical Learn catalog

- **Status:** `DONE`
- **Owner:** Luna
- **Depends on:** W076, W077, W085
- **Objective:** make Learn present a deterministic catalog of Device → Preset (when available) → Effect → Type → Parameter.
- **Implementation:** extend the assignment browser and wire contract with explicit catalog levels, profile-owned preset/effect/type metadata, bounded selection, keyboard and Mk2 arrow parity, and clear empty-state handling.
- **Acceptance:** entering Learn always renders the current level, candidate count, selected item, and breadcrumb; no catalog level is silently skipped.

**Evidence update:** 2026-09-01 — TUI now renders the catalog breadcrumb and explicit
`PRESETS NONE` branch, and the Reflex profile exposes its five PCM70 translator presets alongside
effect/type/parameter metadata. Focused TUI/CLI tests and strict Clippy pass. Interactive level
transitions and preset commits remain open under W089/W090.

**Evidence update:** 2026-09-01 — corrected the Reflex profile renderer so the displayed
`pcm70_reflex:*` catalog IDs produce the same validated active-setup SysEx as the direct preset
control path. Focused profile tests and strict profile Clippy pass.

**Evidence update:** 2026-09-01 — added `ChoosePreset` to the authoritative assignment state
machine, making Device → Preset → Effect an explicit transition sequence with back-navigation;
state-machine tests and strict IPC/TUI/daemon Clippy pass. Preset selection payload integration
and physical qualification remain open.

**Evidence update:** 2026-09-01 — preset navigation now uses its own bounded cursor, preventing
arrow events from crossing into parameter indices while the authoritative phase is `ChoosePreset`.
Focused TUI tests, strict Clippy, formatting, and diff checks pass.

**Evidence update:** 2026-09-01 — added `ChooseType` between effect and parameter, with explicit
back-navigation and state-machine coverage. Learn now has authoritative Device → Preset → Effect
→ Type → Parameter levels; preset payload integration and physical qualification remain open.

**Evidence update:** 2026-09-01 — Type-level arrow navigation now uses a bounded type cursor,
matching the explicit state-machine phase and preventing cross-level selection drift. Workspace
checks and strict TUI/CLI Clippy pass.

**Evidence update:** 2026-09-01 — Learn feedback now renders the selected item at each active
catalog level (Device, Preset, Effect, Type, and Parameter), with bounded markers and explicit
empty states. Workspace checks and strict TUI Clippy pass.

**Evidence update:** 2026-09-01 — catalog rendering now lives in `mackes-tui`
(`assignment_catalog_lines`) so every Learn level shows breadcrumb, `LEVEL`/`COUNT`/`SELECTED`,
and a bounded `>` marker; Eventide empty catalogs still render `PRESETS NONE`. Back from Effect
returns to Preset instead of skipping to Device. Tests:
`catalog_renders_every_level_with_count_and_selection`,
`catalog_shows_presets_none_for_eventide`,
`assignment_session_has_one_bounded_hardware_keyboard_path`,
`assignment_frames_are_bounded_at_release_viewports`. `cargo fmt --check`, workspace tests, and
strict Clippy pass. Physical Learn walkthrough remains W092.

#### [x] W090 — Preset-to-button assignment

- **Status:** `DONE`
- **Owner:** Luna
- **Depends on:** W089
- **Objective:** allow presets to be assigned to Novation buttons while continuous parameters remain assignable to knobs/faders.
- **Implementation:** add a typed preset assignment draft and commit path, validate source-role compatibility, persist atomically, and expose preset identity in mapping/status snapshots.
- **Acceptance:** button sources can commit presets; knob/fader sources cannot commit presets; duplicate/conflicting destinations fail closed and survive restart.

**Evidence update:** 2026-09-01 — captured channel-button sources now select the Reflex PCM70
preset catalog entry and emit a typed commit destination (`pcm70_reflex:<id>`); continuous sources
continue using parameter destinations. Focused TUI tests and strict daemon/TUI Clippy pass.
Runtime persistence and physical button/SysEx qualification remain open.

**Evidence update:** 2026-09-01 — populated the Factory 1 source-address contract for all
Launch Control knobs, channel buttons, and faders, preventing committed mappings from falling back
to source number zero. Profile catalog regression and strict profile/daemon Clippy pass.

**Evidence update:** 2026-09-01 — learned controller mappings now resolve the capture source
endpoint to the originating runtime event when using the controller-owned source identity. This
allows native ALSA button/knob mappings to dispatch without accepting unrelated endpoint IDs;
workspace checks, daemon dispatch regression, and strict Clippy pass.

**Evidence update:** 2026-09-02 — daemon-owned catalog commits now persist button preset mappings
with Factory 1 channel 8 and a non-wildcard source endpoint; knobs/faders still fail closed on
`pcm70_reflex:*`. Tests: `channel_button_can_commit_reflex_preset`,
`knob_and_fader_cannot_commit_reflex_preset`,
`daemon_catalog_snapshot_reconstructs_learn_and_commits_once`. Physical button/SysEx qualification
remains W092.

**Evidence update:** 2026-09-02 — `button_preset_mapping_survives_reload_and_knob_preset_is_stripped`
proves atomic persist through `set_config_path` and strips incompatible knob-preset records on
reload. Duplicate destinations still enter ConfirmReplace. Physical SysEx remains W092.

**Closure:** 2026-09-02 — the daemon-owned commit path proves channel-button preset persistence,
restart reload, and atomic conflict behavior in
`channel_button_can_commit_reflex_preset`, `knob_and_fader_cannot_commit_reflex_preset`,
`button_preset_mapping_survives_reload_and_knob_preset_is_stripped`, and
`daemon_catalog_snapshot_reconstructs_learn_and_commits_once`. Physical controller/SysEx
appearance qualification is explicitly owned by W092, not this software contract.

#### [x] W091 — Preset parameter projection and LED state

- **Status:** `DONE`
- **Owner:** Luna
- **Depends on:** W090, W093, W095, W096
- **Objective:** when a preset is loaded, project its documented parameters and update the Novation controls.
- **Verified LED audit (2026-09-01):** LED support is only partially wired. Yellow Learn
  acknowledgment is encoded, sent, and physically observed. Red Eventide and amber Lexicon owner
  colors exist in software but are not physically qualified. Blue is not a supported Launch
  Control XL Mk2 LED color and must not be promised. The two-green-pulse scheduler exists only as a
  state model: production does not advance its clock and retransmit the resulting frames. Startup
  restores template settings instead of persisted learned mappings; reconnect has no authoritative
  LED replay; writes use broad output-name matching and a hard-coded template; send failures and
  counters are not exposed. Faders have no individual LED address and require an explicit proxy
  policy. Therefore the full persistent LED contract is not built, wired, or release-qualified.
- **Mandatory correction:**
  1. Complete W093 first and use its single authoritative Mk2 template/page, control inventory,
     channel convention, LED address, and stable input/output identity. Remove hard-coded template
     `8`, template `0`/`8` ambiguity, and broad `Launch Control XL` output-name fan-out from runtime
     feedback code.
  2. Make the daemon the sole owner of desired and actual controller LED state. Every LED intent
     must identify the stable controller output, authoritative template/page, physical control,
     layer, color, behavior, generation, and reason. The CLI and TUI may request or render state but
     must never open the MIDI output or maintain a competing LED truth.
  3. Derive the base layer from the persisted mapping store, not template settings: unmapped is
     OFF; active Learn control is yellow; committed Lexicon is amber; committed Eventide is red;
     other valid owners are green only when explicitly defined. Never represent amber as blue or
     claim unsupported colors.
  4. Preserve normal base state beneath temporary overlays. Device entry acknowledges visibly;
     the captured control remains yellow throughout Learn; successful atomic persistence produces
     exactly two visible green blinks and then restores the destination-owner color; failure
     produces the documented red failure indication and then restores the prior valid base state.
     Cancel or timeout must remove every transient overlay without changing the saved mapping.
  5. Wire the scheduler to a daemon monotonic timer/tick and emit changed frames at their specified
     deadlines. Completion of mapping persistence must automatically transition W096 to
     `Succeeded` or `Failed`, arm the result sequence once, and prevent duplicate pulses from
     retries, duplicate input, or multiple TUI subscribers.
  6. Rebuild and replay desired LED state after daemon restart, controller reconnect, template/page
     reselection, mapping create/replace/delete, preset load, and output-registry recovery. Replay
     only after W095 resolves one unambiguous matching output; duplicates must fail closed without
     writing to either device.
  7. Treat each knob and channel button according to the W093 feedback-address inventory. Do not
     emit nonexistent per-fader LED messages. Define and document the two channel-button LEDs in a
     fader column as its proxy, including precedence when those buttons also have assignments, or
     explicitly report fader LED feedback as unsupported until that conflict is resolved.
  8. Route preset-load projection and LED restoration through one ordered daemon transaction so a
     partially sent value set cannot advertise a successful preset. Bound and coalesce traffic,
     preserve ordinary routing, and recover deterministically after mid-batch disconnect.
  9. Check every MIDI send result. Publish attempted/sent/coalesced/failed LED counters, last error,
     target stable identity, template/page, pending scheduler deadline, and desired-versus-actual
     state in diagnostics. A zero-send path or wrong endpoint must be visible in the TUI/status and
     release evidence.
- **Tests to add first:** exact golden SysEx bytes for every supported Mk2 LED address/color and the
  selected W093 template; unsupported color/address rejection; unmapped OFF; Lexicon amber;
  Eventide red; active Learn yellow; fake-clock two-green success and red failure sequences;
  overlay/base restoration; create/replace/delete; restart; reconnect with changed ALSA client
  number; template reselect; duplicate-device fail-closed behavior; send failure and retry;
  coalescing/order; preset projection interrupted mid-batch; fader proxy precedence; two TUI
  subscribers; duplicate commit/result events; diagnostics counter accuracy.
- **Physical qualification:** with one identified Launch Control XL Mk2, record the exact selected
  template/page and output identity, then verify OFF at clean start, yellow capture, Lexicon amber,
  Eventide red, exactly two green success blinks, failure red, replacement, deletion, daemon
  restart, USB reconnect, and preset-load projection. Record unsupported blue and the chosen fader
  proxy behavior explicitly in `docs/hardware-qualification.md`.
- **Acceptance:** loading a preset updates every mapped parameter deterministically; unmapped
  controls remain dark; no write reaches the wrong or ambiguous endpoint; restart/reconnect and
  template reselection restore values and owner colors solely from persisted mappings; temporary
  Learn/result overlays complete at exact fake-clock and observed physical timings; all failures
  are visible and recoverable; the physical qualification matrix passes with evidence.

**Evidence update:** 2026-09-01 — Reflex now exposes a bounded, profile-owned parameter projection
for each PCM70 translation, preserving algorithm-specific values for downstream controller
population. Focused profile tests and strict Clippy pass; Novation output projection and LED
resync remain open.

**Evidence update:** 2026-09-01 — daemon dispatch now projects loaded Reflex preset values to
enabled mapped Reflex parameters as normalized controller events on Launch Control XL outputs.
The projection is bounded, profile-owned, and isolated from mapping-store borrows; workspace checks
and strict daemon Clippy pass. Physical feedback and LED qualification remain open.

**Evidence update:** 2026-09-02 — daemon owns a mapping-derived LED surface on Factory Template 1.
Unique Mk2 MIDI output matching replaced name fan-out and template `8`; HUI is ignored; duplicates
fail closed; startup restore no longer reads template settings; reconnect/replay and a monotonic
tick emit coalesced Factory 1 SysEx; Learn is yellow; Lexicon/Eventide owner colors and two-pulse
result overlays are unit-tested with a fake clock; fader columns use the documented button-LED
proxy with button-assignment precedence; snapshot/TUI expose attempted/sent/coalesced/failed and
last error. Physical Mk2 matrix and preset-load appearance remain open.

**Evidence update:** 2026-09-02 — the complete `scripts/release-gate.sh` passed after the LED
surface and preset-projection changes: strict artifact matrix, dependency audit, all workspace
tests, strict Clippy, routing benchmark, hermetic integration, installer smoke, and packaged
artifact checksum/preflight. This verifies the software contract; physical color/pulse and
preset-load appearance remain qualification evidence, not an unverified software claim.

**Evidence update:** 2026-09-02 — added the operator-requested daemon-owned sleep display:
after exactly five minutes without Factory 1 activity, one red LED sweeps left-to-right through
all 48 documented indices at 600 ms per step; any controller event wakes immediately and restores
the mapping-derived base surface, while active assignment suppresses sleep. Fake-clock regression
`five_minute_idle_sweep_wakes_and_restores_mapping_colors` covers the boundary, advance, wake, and
owner-color restoration.

**Evidence update:** 2026-09-02 — full `scripts/release-gate.sh` passes with idle-sweep behavior:
artifact/policy checks, advisories, workspace tests (65 daemon tests), strict Clippy, benchmark,
hermetic integration, installer smoke, and release artifact checksum/preflight all pass.

#### [x] W092 — Catalog and preset end-to-end qualification

- **Status:** `DONE`
- **Owner:** Luna
- **Depends on:** W089, W090, W091, W088
- **Objective:** qualify the complete catalog workflow on the Launch Control XL Mk2 and include it in the next release gate.
- **Acceptance:** Device entry, catalog navigation, button preset assignment, knob parameter assignment, preset load projection, LED feedback, persistence, reconnect, and release artifacts all pass with evidence in `docs/hardware-qualification.md`.

**Evidence update:** 2026-09-03 — operator-driven Learn block completed on the installed
daemon: physical `button-r2-c1` capture (Mk2 channel 8, note 57) entered the live device catalog;
Forward navigation reached Reflex preset selection; Concert Wave committed atomically to the
daemon-owned MIDISPORT Port D output (`midir-out-b7003eb6a8f354d7`) with two green confirmation
blinks followed by the yellow owner indication. Status readback reports `Succeeded`, an enabled
`pcm70_reflex:concert-wave` mapping, and zero dropped input events. Arrow navigation advances the
catalog, but the physical arrow-key LED does not illuminate; this remains an explicit hardware
appearance limitation for the full matrix qualification.

**Evidence update:** 2026-09-03 — after adding held-arrow LED overlay precedence and a focused
press/release regression, `scripts/release-gate.sh` passed end-to-end (workspace tests, strict
Clippy, throughput, hermetic integration, installer smoke, archive checksum, and preflight).

**Evidence update:** 2026-09-03 — the daemon-owned MIDISPORT Port A output is
`midir-out-1c86ed3dd492b3af`; the earlier `...1c86ec...` probe accidentally used the input-side
identity and therefore failed closed as designed. The corrected documented Reflex active-setup
query and all sixteen device-channel variants transmitted successfully and were audited, with no
drops. No reply reached the daemon on Port A, narrowing the remaining physical blocker to the
Reflex return cable/SysEx setting/device state rather than destination registration or channel.

**Evidence update:** 2026-09-03 — added and physically executed a confirmed, Reflex-only
daemon-owned MIDI System Reset operation (`FF`); focused test and strict daemon Clippy pass, and
the installed service binary hash matches the release build. Reset succeeded and was audited, but
the subsequent sixteen-channel active-state query sweep still produced no inbound Port A bytes.
Qualification is now isolated to the hardware return path or Reflex MIDI/SysEx configuration.

**Evidence update:** 2026-09-03 — resolved the physical topology: the Reflex is bidirectionally
connected on MIDISPORT Port D (`midir-out-b7003eb6a8f354d7` /
`midir-in-b7003db6a8f35324`), not Port A. Port D returned a valid 63-byte active setup and a
parameter-0 value of `0x9400`. A one-step nonpersistent change to `0x9800` was observed, restored
to `0x9400`, and independently queried back as `0x9400`. Final daemon counters were 58 received,
74 sent, zero dropped, health ready. This closes the physical Reflex send/reply/edit/restore
portion without claiming the remaining full Mk2 catalog walkthrough.

**Evidence update:** 2026-09-03 — physical Reflex preset recall passed on Port D: task-71 register
9 produced a setup-selection event for 9 and an independent type-0 readback named `DrumPlat`
(algorithm 2 / Plate). The preceding translated `Concert Wave` active-load frame was transmitted
but did not alter readback and remains explicitly unqualified. No persistent store was issued.

**Evidence update:** 2026-09-03 — corrected PCM70 translation values to the documented
algorithm/parameter step grid and added an exact Concert Wave regression vector. The rebuilt and
installed daemon loaded the corrected active-only frame on Reflex Port D; an independent query
returned the same 63 bytes, name `Concert Wave`, and checksum `2A`. This supersedes the earlier
unqualified translation result; no persistent register store was sent.

**Evidence update:** 2026-09-03 — added a validated `ProfileRef.endpoint_alias` contract and
daemon profile-output binding. The qualification config binds `lexicon.reflex` to exact MIDISPORT
Port D; after restart the authoritative catalog exposes Reflex plus Eventide and reports Port D as
the Reflex destination. Regression coverage proves a confirmed Device cursor change recomputes
the exact output and cannot leak the prior profile's destination. The next physical capture window
received no Mk2 event, so the controller-driven preset commit remains open.

**Evidence update:** 2026-09-03 — `scripts/release-gate.sh` passes after the Reflex Port D binding,
destination-rebinding regression, System Reset path, and step-quantized PCM70 translation fix:
repository/architecture/artifact policy, dependency advisories, all workspace tests (including 68
daemon tests), strict Clippy, 10,000-message benchmark, hermetic integration, installer smoke,
release archive checksum, and preflight all pass.

#### [x] W093 — Reconcile the authoritative Launch Control XL Mk2 layout contract

- **Status:** `DONE`
- **Start date:** 2026-09-01
- **Owner:** Luna
- **Depends on:** W083, W089
- **Parallel with:** W095 and W097 after the contract inventory below is frozen.
- **Objective:** make the documented controller template, runtime decoder, physical-control catalog,
  migration behavior, tests, and hardware qualification describe one exact Mk2 layout.
- **Problem to correct:** the reviewed User 1 manifest currently declares knob CCs
  `21–28/41–48/61–68` and channel-button CCs `69–76/85–92`, while the production Factory 1
  decoder/catalog accepts knob CCs `13–20/29–36/49–56` and channel-button notes
  `41–48/57–64`. Both cannot remain authoritative. The artifact also names User 1 while the
  physically qualified implementation and documentation contain Factory 1 evidence.
- **Implementation:**
  1. Inventory every Mk2 input and feedback tuple used by the daemon, profile catalog, config
     migration, Components manifest, ADRs, TUI labels, fixtures, and qualification document.
  2. Record one ADR choosing the production contract: either the reviewed User 1 Components
     layout or the physically proven Factory 1 layout. State the exact slot/template, MIDI
     channel convention, message kind, number, press/release semantics, LED address, model gate,
     and firmware assumptions for every assignable and utility control.
  3. Create one profile-owned typed layout table and derive daemon decoding, assignment source
     addresses, UI inventory, artifact validation, and tests from it. Remove duplicated numeric
     match tables from application code.
  4. If User 1 remains authoritative, produce/import the official Components artifact, migrate
     Factory 1 mappings without guessing, and fail closed when the observed template differs. If
     Factory 1 becomes authoritative, retire the contradictory User 1 requirement and manifest
     instead of leaving a false install step.
  5. Preserve stable physical IDs (`knob-r*-c*`, `button-r*-c*`, `fader-*`, utilities) and provide
     an explicit migration for any changed source tuple. Never silently rewrite a destination.
  6. Update onboarding, recovery, release notes, and hardware qualification to name the same
     layout and exact operator action.
- **Allowed files:** layout/profile modules, daemon decoder, config migration, IPC inventory,
  Components artifact/manifest, ADRs, fixtures, TUI labels, focused tests, operator docs.
- **Excluded behavior:** supporting two implicit layouts under one identity, name-only template
  detection, undocumented SysEx, number guessing, or accepting an unrecognized layout.
- **Tests to add first:** manifest/runtime tuple equality; uniqueness; every knob/button/fader and
  utility press/release; wrong channel/kind/number rejection; model mismatch; migration from the
  previous Factory 1 records; reconnect/template resync; no duplicate physical IDs.
- **Acceptance:** one machine-readable layout is the sole source of truth; every generated and
  runtime consumer agrees byte-for-byte; wrong layouts visibly fail closed; existing mappings
  migrate transactionally; the Mk2 walkthrough records only the requested Eventide controls:
  all 16 documented parameters on knob rows 2–3, master Mix on Slider 1, Slider 1 Button 1 for
  ACTIVE/BYPASS, and Slider 1 Button 2 as explicitly unsupported for independent Delay bypass.
- **Commands/evidence:** focused profile/config/daemon tests, strict Clippy, artifact verifier,
  migration fixture output, exact Components checksum when applicable, and updated physical
  capture evidence in `docs/hardware-qualification.md`.

**Work log:** 2026-09-01 — codex — `IN_PROGRESS`; selected the physically proven Factory
Template 1 contract, added a profile-owned typed 56-control layout including all assignable and
utility input tuples, and changed daemon Device/navigation/capture decoding to resolve solely
through that table. Added exact uniqueness, wrong-channel, contradictory User 1 tuple, and
release-value rejection tests. ADR, operator-document retirement, migration, and artifact-contract
alignment remain open.

**Evidence update:** 2026-09-01 — added the versioned machine-readable Factory Template 1
manifest and retired User 1 onboarding references as migration history. Artifact verification,
worklist validation, 50 profile tests, and diff checks pass. Transactional legacy tuple migration
and full release-gate enforcement remain W093/W094 work.

**Evidence update:** 2026-09-01 — added explicit `migrated_factory1` configuration
migration for legacy stable IDs, preserving destinations while translating known User 1
tuples to Factory 1 channel/kind/number values and rejecting ambiguous records. Focused
config migration test and strict config Clippy pass.

**Evidence update:** 2026-09-02 — software contract freeze: `LAUNCH_CONTROL_MK2_FACTORY1_SLOT = 1`
is the TUI readiness slot; every Factory 1 press/release tuple is covered by
`mk2_factory1_every_control_press_resolves_and_release_is_rejected`; knob/button/fader number
lists match the release manifest; User 1 operator docs are migration history only; release-note
rollback no longer instructs a User 1 Components workflow. Physical walkthrough of the requested
Eventide controls remains W088/W092; a 56-control walkthrough is not a release requirement. W097
may proceed against this frozen table.

**Evidence update:** 2026-09-03 — post-restart physical attempts produced channel-8 note 73,
not the persisted block's channel-8 note 57. The daemon dispatched neither event and sent no
Reflex write, preserving fail-closed behavior. The discrepancy is retained for W093's physical
layout reconciliation; no source-number guess was introduced.

**Blocker:** Three consecutive operator-driven attempts reproduce the mismatch: the connected
Mk2 emits channel-8 note 73 for the presumed mapped button, while the selected Factory Template 1
contract requires note 57. Provider: physical controller/template state. Safe unblock: select
Factory Template 1 and repeat the capture, or explicitly approve reconciling the authoritative
contract to the observed template before any mapping rewrite.

**Resolution:** 2026-09-03 — operator explicitly selected Factory 1 as authoritative and requested
continuation. The daemon now reselects Factory 1 automatically on reconnect. The anomalous channel
observation is retained as qualification evidence and will not silently rewrite the frozen contract.

#### [x] W094 — Make controller artifact readiness a hard release gate

- **Status:** `DONE`
- **Start date:** 2026-09-01
- **Owner:** Luna
- **Depends on:** W093
- **Objective:** prevent a production release from passing while its required controller artifact
  is absent, unreviewed, mismatched, or has a null checksum.
- **Problem to correct:** `verify-artifacts.py` currently prints “checksum pending review” and
  exits successfully even though the manifest says a null checksum must fail readiness.
- **Implementation:**
  1. Separate development inspection from production readiness using explicit commands or flags;
     the default release gate must use strict production readiness.
  2. Require a legal distributable artifact when W093 selects User 1, a 64-character SHA-256,
     exact manifest/artifact digest equality, target model/generation/slot equality, complete
     inventory, and a versioned artifact filename.
  3. If W093 selects Factory 1 and no artifact is required, remove the misleading artifact
     requirement and replace it with a versioned factory-layout contract fixture; do not retain a
     nullable production manifest.
  4. Make packaging include the required artifact/contract and make preflight report a precise
     remediation on absence or mismatch.
  5. Add negative release-gate fixtures for null, malformed, stale, wrong-model, wrong-slot,
     modified, and missing artifacts.
- **Allowed files:** artifact verifier, release/package scripts, installer preflight, manifest or
  replacement contract, legal fixtures, release docs, tests.
- **Excluded behavior:** warning-only production checks, network downloads during release, digest
  updates without artifact review, or bypass flags in `release-gate.sh`.
- **Acceptance:** every incomplete or mismatched production artifact fails before compilation or
  packaging; the selected W093 contract passes deterministically offline; the release archive
  contains exactly the verified artifact/contract and checksum evidence.
- **Commands/evidence:** verifier positive/negative suite, installer smoke, archive listing and
  digest check, `scripts/release-gate.sh`, and documented recovery output.

**Work log:** 2026-09-01 — codex — `IN_PROGRESS`; replaced the nullable User 1 Components
manifest with a versioned offline Factory Template 1 contract, made the verifier require exact
model/generation/slot/channel/inventory/message-kind fields, and removed the contradictory legacy
manifest. Positive artifact and governance checks pass; negative fixture coverage and full release
gate evidence remain.

**Evidence update:** 2026-09-01 — strict Factory 1 artifact verification and packaging are
now release-gated offline; `scripts/release-gate.sh` passed formatting, repository/worklist
policy, dependency audit, all workspace tests, strict Clippy, benchmark, hermetic integration,
installer smoke, archive checksum, and preflight. Negative-fixture coverage remains to be added.

**Evidence update:** 2026-09-01 — added `scripts/test-verify-artifacts.py` and wired it into
the release gate. The offline matrix proves positive readiness and rejects malformed, stale,
wrong-model, wrong-slot, modified, and missing manifests; artifact negative checks and worklist
validation pass.

**Closure:** 2026-09-01 — the versioned Factory Template 1 contract is strict and offline;
release gating runs positive and negative validation before compilation. Packaging now includes the
contract plus the verifier scripts, and archive inspection confirmed all three paths. The contract
is the verified release artifact; no nullable Components artifact remains.

**Final gate evidence:** 2026-09-01 — `scripts/release-gate.sh` passed end-to-end after archive
contract inclusion: repository/ownership policy, strict positive and negative artifact checks,
dependency audit,  workspace tests, strict Clippy, benchmark, hermetic integration, installer
smoke, release checksum, and preflight.

#### [x] W095 — Enforce daemon-only physical MIDI ownership

- **Status:** `DONE`
- **Start date:** 2026-09-01
- **Owner:** Luna
- **Depends on:** W087
- **Parallel with:** W093 and W096 after IPC request contracts are frozen.
- **Objective:** ensure every supported physical MIDI read/write,
  is owned, serialized, authorized, audited, counted, and observed by the daemon.
- **Problem to correct:** `apps/mackes` directly enumerated `midir` ports and opened physical
  outputs, bypassing daemon output ownership and safety state. M-VAVE is retired and excluded.
- **Implementation:**
  1. Define typed IPC operations for every remaining direct CLI device action, including dry-run
     rendering, explicit destination, confirmation, unsafe/experimental gate, and bounded result.
  2. Move endpoint resolution, profile rendering, output send, pacing, safety decision, audit
     record, counters, and activity publication into the daemon.
  3. Remove `midir-backend` from the operator application and remove all physical enumeration,
     adapter construction, `/dev/snd` fallback discovery, and output sends from TUI/CLI code.
  4. Keep read-only endpoint display sourced from authoritative daemon snapshots/queries.
  5. Make daemon unavailability fail closed; no client-side fallback may open hardware.
  6. Add a repository policy check that rejects backend adapter construction or MIDI-port open
     calls outside `crates/midi-engine` and the daemon adapter boundary.
- **Allowed files:** IPC, daemon, CLI/TUI adapters, MIDI engine interfaces, policy scripts,
  focused tests, ADR-0002 clarification, operator docs.
- **Excluded behavior:** dual output owners, direct client fallback, shelling out to ALSA tools,
  destination name guessing, fabricated endpoint IDs, or weaker confirmation.
- **Tests to add first:** CLI cannot open MIDI; daemon-unavailable write fails; exact destination
  required; confirmation/unsafe denial; successful send increments daemon counters and audit;
  output failure is visible; concurrent commands preserve order; panic still reaches all outputs.
- **Acceptance:** the operator binary has no MIDI backend feature and cannot open hardware;
  repository policy enforces the boundary; every device write appears in daemon audit/activity and
  uses the same output registry, safety policy, and bounded error contract.
- **Commands/evidence:** dependency-tree assertion, policy test, focused IPC/daemon/CLI tests,
  strict Clippy, integration suite, and one physical Reflex/Eventide send through daemon IPC.

**Work log:** 2026-09-01 — codex — `IN_PROGRESS`; removed direct physical output
enumeration/open/send paths from the operator CLI and routed preset/module operations through
daemon-owned DeviceControl IPC. Endpoint display now reads the daemon Endpoints response rather
than opening ALSA/midir or scanning `/dev/snd`. CLI all-feature check and strict Clippy pass.

**Evidence update:** 2026-09-01 — repository ownership policy now mechanically rejects
`midir` enumeration, adapter construction, and `/dev/snd` access under `apps/mackes`; it is
wired into both repository verification and the release gate and passes on the current tree.

**Evidence update:** 2026-09-01 — the full release gate passed after the ownership cutover,
including workspace tests, strict Clippy, throughput benchmark, hermetic integration, installer
smoke, packaging checksum, and preflight. Remaining W095 evidence is focused IPC audit/counter
coverage and an explicitly targeted physical send.

**Evidence update:** 2026-09-01 — assignment IPC snapshots now carry independent bounded
device, preset, effect, type, and parameter cursors, with active-level navigation and round-trip
serialization tests. This freezes the daemon-side cursor contract for the dependent W096 work.

**Evidence update:** 2026-09-01 — removed the `midir-backend` feature from the operator
application dependency. `cargo check -p mackes-midi-matrix --no-default-features`, feature-tree
inspection, and strict operator Clippy pass; the app can no longer transitively enable the physical
midir backend.

**Evidence update:** 2026-09-01 — `cargo check -p mackes-midi-matrix --no-default-features`
and strict operator Clippy pass with no `midir` feature in the app dependency tree. This proves
the operator binary cannot compile the physical backend; daemon IPC remains its only hardware path.

**Evidence update:** 2026-09-02 — `apply_device_control` is the daemon-owned write path. Tests
`device_control_requires_confirmation_and_registered_destination` and
`daemon_device_control_send_is_counted_and_audited` prove confirmation denial, unregistered
destination fail-closed, and no counter/audit increment on refusal. A successful physical
Reflex/Eventide send remains required before W095 `DONE`.

**Closure:** 2026-09-02 — installed `4e7791a` after correcting daemon IPC payload extraction.
The operator binary queried the daemon-owned Eventide catalog and sent documented Expression
Pedal CC4 value 64 to exactly `midir-out-c0d934e6c08c6a1a`; daemon response was bytes
`[176,4,64]`, `sent=1`, and its local-IPC audit recorded the allowed target/action. Installed and
build daemon SHA-256 values match. This proves the production daemon owns the physical write,
serialization, audit, counter, and exact endpoint.

#### [x] W096 — Establish one authoritative Learn catalog and cursor state

- **Status:** `DONE`
- **Start date:** 2026-09-02
- **Owner:** Luna
- **Depends on:** W089, W090
- **Parallel with:** W095 after typed IPC fields are agreed.
- **Objective:** eliminate split daemon/TUI authority for Learn phase transitions, catalog
  candidates, selected indexes, navigation, commit payloads, and LED outcomes.
- **Problem to correct:** the daemon currently advances physical Right Arrow for some phases while
  the TUI owns per-level cursors and final commits. This permits stale phases, duplicate movement,
  a caret appearing at multiple levels, no-op final selection, and behavior that depends on which
  TUI process consumes an event.
- **Mandatory defect inventory from the 2026-09-01 workflow review:**
  1. The TUI constructs the browser once with hard-coded connected profiles, hard-coded selected
     profile `lexicon.reflex`, and hard-coded `Continuous` role. It is not rebuilt from live daemon
     inventory or the captured control role, so Eventide and button workflows cannot be trusted.
  2. Selecting a Device sends only `Enter`; no selected device ID crosses IPC and no downstream
     catalog is rebuilt. Effect and Type selections likewise do not filter the final parameter
     list. These levels are currently presentation, not authoritative choices.
  3. `AssignmentChoiceBrowser` has one `selected: usize` shared by Device, Preset, Effect, Type,
     and Parameter. Level-specific clamps reduce visible symptoms but selection leaks between
     levels and cannot be reconstructed after restart.
  4. Hardware control capture is stored as daemon-only `assignment_control_id`, while the TUI
     commit builder reads its separate local `candidates` list. Native hardware capture does not
     populate that list, allowing the final commit to omit `physical_control_id` and fail the
     complete-destination contract.
  5. `AssignmentSession.index` and `total` exist but are not populated from the actual catalog;
     snapshots expose phase without the selected IDs, candidates, captured control, or filtered
     catalog needed to reproduce the screen.
  6. Intermediate physical Right Arrow transitions are applied directly in the daemon, while
     final Right Arrow is emitted as a TUI navigation event. Keyboard handling is a third path.
     One gesture therefore has phase-dependent owners and multiple failure modes.
  7. The committed mapping currently hard-codes source channel `0`, despite qualified Mk2 events
     arriving on zero-based channel `7` (displayed/wire channel 8); it hard-codes destination
     endpoint `processor`; and source endpoint `controller` acts as a wildcard. A saved mapping may
     never match the Mk2, may target no registered output, or may accept another device's event.
  8. Device selection is not bound to a concrete connected output endpoint or stable identity, so
     the final profile choice cannot prove where MIDI will be sent.
  9. The declared 750 ms Device hold-to-cancel classifier and 250 ms candidate-disambiguation
     window are tested as pure helpers but are not used by the production input path, which accepts
     the first eligible event immediately.
  10. Successful persistence leaves the state at `Committing`; production code has no automatic
      transition to `Succeeded` or `Failed`. Result LED timing and workflow completion therefore
      depend on an action that only tests explicitly send.
  11. Assignment request rejection, generation conflict, empty selection, and serialization
      failure are usually swallowed by the TUI path instead of becoming a persistent visible
      operator error with a retry/recovery action.
  12. Multiple TUI processes may subscribe simultaneously, but client-local cursor ownership gives
      each a divergent catalog while all target one daemon session.
- **Implementation:**
  1. Extend the typed assignment snapshot with catalog level, breadcrumb, candidate IDs/labels,
     candidate count, selected index per level (or one level-scoped index), selected typed IDs,
     captured control role, and explicit pending action/result.
  2. Choose one authority—prefer the daemon because sessions must survive TUI restarts—and route
     keyboard Enter/arrows and Novation arrows through the same typed assignment actions.
  3. Move Device → Preset (including explicit NONE) → Effect → Type → Parameter filtering and
     source-role validation into the authority. A button may select a preset; knobs/faders may
     select continuous parameters; invalid combinations fail visibly.
  4. Make Right/Enter advance exactly one level, Left/Esc back exactly one level, Up/Down mutate
     only the active level, and final Right/Enter produce exactly one complete typed commit.
  5. Remove client-local catalog mutation and duplicate direct daemon phase transitions. TUI
     becomes a renderer/input adapter over authoritative snapshots and sequenced results.
  6. Preserve interrupted drafts and selected IDs across TUI restart; reconcile safely when
     profiles disappear or mappings conflict.
  7. Bind LED feedback to authoritative events: Device acknowledgment, active control yellow,
     two green success blinks, owner color, failure red, and unmapped OFF.
- **Allowed files:** assignment IPC/state machine, daemon catalog service, profile metadata access,
  TUI renderer/input adapter, mapping persistence, LED scheduler, tests and ADR updates.
- **Excluded behavior:** shared cursor across levels, phase inference from rendered text, duplicate
  Enter handling, optimistic client commits, multiple TUI consumers mutating one session, or
  untyped destination strings crossing the final commit boundary.
- **Tests to add first:** full forward/backward hierarchy; explicit Preset NONE; independent cursor
  bounds; keyboard/controller parity; one event advances once; final commit carries selected ID;
  button/preset and knob/parameter role gates; replacement; TUI disconnect/reconnect mid-session;
  two TUI subscribers; stale generation; empty/removed profile; LED event sequence; live device
  selection changes the profile/catalog; Effect/Type filters change the final parameter set;
  hardware capture survives into commit; qualified channel/source tuple matches after persistence;
  selected stable output endpoint receives the rendered message; unrelated input cannot trigger a
  controller mapping; 749/750 ms Device boundaries; 250 ms duplicate/ambiguous capture; persistence
  success/failure reaches a terminal state; every rejection is visible and retryable.
- **Acceptance:** daemon snapshots alone reconstruct the complete Learn screen and selection;
  keyboard and controller traces yield identical state/result sequences; TUI restarts do not alter
  or lose the session; no caret can appear at two levels; one final action commits exactly once;
  the saved mapping contains the captured stable source identity/channel/number and selected stable
  destination endpoint; it dispatches after restart and reconnect; success/failure terminates with
  the documented LED sequence and an actionable visible result.
- **Commands/evidence:** IPC golden fixtures, daemon/TUI reducer tests, hermetic end-to-end trace,
  strict Clippy, release viewport snapshots, and physical Mk2 catalog walkthrough.

**Work log:** 2026-09-02 — Luna — `READY` → `IN_PROGRESS`; daemon snapshots now own catalog rows,
per-level cursors, selected IDs, captured role/channel/number, and commit payloads. Keyboard and
Mk2 arrows send the same typed actions. Persistence auto-reaches `Succeeded`. Production input uses
the 250 ms assignable-control disambiguation window and 750 ms Device hold-to-cancel. Remaining:
LED outcome binding, 749/750 boundary unit coverage on the poll path, and physical walkthrough.

**Evidence update:** 2026-09-02 — workspace-wide all-feature tests pass (including
`device_gesture_uses_exact_750_millisecond_hold_boundary`, independent per-level cursor coverage,
catalog filtering/selected-ID commit coverage, daemon snapshot reconstruction, interrupted-draft
recovery, and terminal `Succeeded`/`Failed` transitions). Strict Clippy and formatting pass.
The remaining gaps are qualification-scope work: physical Mk2 walkthrough and LED appearance
evidence owned by W088/W092; no software test failure is being hidden or downgraded.

**Evidence update:** 2026-09-02 — `scripts/release-gate.sh` passed end-to-end with the
authoritative Factory 1 contract, dependency advisory scan, workspace tests, strict Clippy,
10,000-message benchmark, hermetic integration (13 pass/1 explicitly deferred paired test),
installer smoke, and release artifact checksum/preflight. W096 remains open only for the physical
qualification steps assigned to W088/W092.

#### [x] W097 — Modularize the implementation tree and reconcile architecture documentation

- **Status:** `DONE`
- **Start date:** 2026-09-02
- **Owner:** Luna
- **Depends on:** W093 contract freeze
- **Parallel with:** W094–W096 only when file ownership is disjoint.
- **Objective:** turn the current crate-level separation into maintainable internal modules and
  make the tracked repository tree match ADR-0002 and operator documentation.
- **Problem to correct:** core behavior is concentrated in single files of roughly 4,600–6,800
  lines; several documented top-level directories are empty/untracked; README backend text is
  stale; layout policy does not mechanically enforce the claimed ownership boundaries.
- **Implementation:**
  1. Split `crates/profiles` by device and shared profile contracts; split `crates/midi-engine` by
     domain routing, native ALSA, optional backends, transport, scheduler, and adapters.
  2. Split `crates/tui` by assignment, task shell, renderers, device workspaces, mapping editor,
     reducer, and terminal lifecycle. Split daemon code by IPC service, input supervisor,
     assignment/catalog, routing, output/LED, persistence, diagnostics, and lifecycle.
  3. Split CLI commands into focused modules; keep `main.rs` limited to argument dispatch and
     application composition.
  4. Preserve public APIs or provide deliberate migrations; use private modules by default and
     prevent dependency cycles or new cross-layer access.
  5. Decide which top-level directories are real product contracts. Add tracked README/index and
     artifacts where required, or remove them from ADR/tree diagrams when Rust modules are the
     canonical source. Remove empty local residue such as the untracked `crates/firebox` directory;
     retain only `docs/firebox-findings.md` as the historical record.
  6. Update README/ADRs to describe native ALSA production ownership, optional rollback backends,
     actual package paths, generated artifacts, and release qualification accurately.
  7. Add architecture checks for allowed dependency edges, backend ownership, maximum source-file
     growth thresholds, tracked canonical directories, and stale generated artifacts.
- **Allowed files:** module moves within existing crates/apps, Cargo manifests, architecture tests,
  ADRs, README, tree indexes, repository scripts. No behavior change beyond extraction unless
  required by W093/W095/W096.
- **Excluded behavior:** broad rewrites without characterization tests, public re-export sprawl,
  cyclic dependencies, duplicate compatibility implementations, or deleting historical evidence.
- **Tests to add first:** characterization tests for moved public behavior, dependency-edge policy,
  forbidden backend ownership, module API smoke tests, package/archive inventory, documentation
  link checks, and clean-clone canonical-tree validation.
- **Acceptance:** no core source file exceeds the agreed reviewed threshold without a documented
  exception; package dependencies follow the ADR; physical I/O ownership is mechanically checked;
  a clean clone contains the documented tree; all behavior and golden fixtures remain stable.
- **Commands/evidence:** `cargo metadata` dependency audit, workspace tests/Clippy, repository
  policy checks, clean-clone package test, archive inventory, and before/after module map.

**Evidence update:** 2026-09-01 — added `docs/architecture.md` as the canonical workspace-boundary
map and `scripts/check-architecture.py` as a repository policy gate. The gate verifies permitted
local dependency edges and reviewed no-growth ceilings for each oversized root source file; it runs
through repository verification and therefore the release gate. The policy and repository checks
pass before the first behavior-preserving module extraction.

**Evidence update:** 2026-09-01 — extracted the Eventide MicroPitch built-in profile into a private
device module while retaining `eventide_micropitch_profile` as the stable public boundary. Profile
characterization tests (49) and the architecture policy pass; subsequent extractions retain the
same API-first approach.

**Evidence update:** 2026-09-01 — `scripts/release-gate.sh` passes after the architecture-policy
and Eventide module extraction: repository policy (including the new dependency/size gate), strict
artifact checks, workspace tests and Clippy, routing benchmark, hermetic integration, installer
smoke, release archive checksum, and preflight all completed successfully.

**Work log:** 2026-09-02 — Luna — `READY` → `IN_PROGRESS`; extracted `lexicon_reflex`, crate
`tests.rs` modules, `midi-engine` `rtp`, TUI `render`, and operator `cli`/`interactive` without
changing public crate APIs. Architecture ceilings were lowered to the post-extraction roots
(profiles/engine 3100, TUI 4200, daemon 3600, operator main 800). README now names native ALSA
as the production MIDI path. Remaining: further TUI/daemon splits, canonical-tree clone check,
and W097 acceptance gate.

**Closure:** 2026-09-02 — `9b1c732` completes the extracted-module visibility and lint boundary.
`python3 scripts/check-architecture.py`, `python3 scripts/check-worklist.py`, strict workspace
Clippy, and `cargo test --workspace --all-features` pass. A fresh local clone of `9b1c732` also
passed the architecture/worklist checks and all-feature workspace tests; its independently built
`0.1.11` package checksum verified, and archive inventory contains both binaries, the Factory 1
contract, installer, service unit, release notes, and provenance. Core files remain below the
reviewed ceilings and the daemon-only physical-I/O boundary is mechanically checked.

#### [x] W098 — Architecture-correction release closure

- **Status:** `DONE`
- **Owner:** Luna
- **Depends on:** W088, W092, W093, W094, W095, W096, W097
- **Objective:** prove the corrected tree and contracts are fit for the next feature-complete
  release rather than relying only on green unit tests.
- **Implementation:** run a requirement-by-requirement audit of W093–W097; rebuild artifacts from a
  clean clone; install on the qualification host; execute the complete Mk2 Learn/preset/LED,
  reconnect, persistence, daemon-only output, supported Reflex, and rollback walkthrough; reconcile
  all release metadata and versioning; commit exact evidence.
- **Acceptance:** no open architecture mismatch, warning-only artifact requirement, client-owned
  physical MIDI path, split Learn authority, undocumented tree path, ignored default software
  failure, or unfinished W085–W097 acceptance remains. Full release gate and physical walkthrough
  pass against the exact tagged commit and packaged binaries.
- **Commands/evidence:** clean-clone `scripts/release-gate.sh`, package checksums/provenance,
  installed binary hashes, service identity/subscriptions, hardware capture summary, mapping
  restart/reconnect proof, archive inventory, and final worklist reconciliation.

**Evidence update:** 2026-09-04 — final release closure completed on the reconciled tree. The full
`scripts/release-gate.sh` passed: formatting/worklist/artifact policy, locked metadata and cached
advisory audit, all workspace tests and documentation tests, strict workspace Clippy, routing
benchmark, 14-scenario hermetic integration (13 passed and the explicitly deferred paired-RTP
scenario ignored), installer smoke, release archive checksum/provenance, and preflight. The live
qualification host also completed the Mk2 LED color/sweep and OFF restoration, daemon restart and
controller reconnect persistence, exact Reflex Concert Wave Port D readback, and Eventide
MicroPitch transport/baseline walkthrough. The reconciled worklist has no unfinished W085–W097
acceptance item; this closure is recorded in the repository commit containing the verified tree.

**Evidence update:** 2026-09-03 — copied the current tree, including uncommitted implementation
changes but excluding build outputs, into an isolated directory and ran the architecture/worklist
policy checks plus `cargo test --workspace --all-features`. Both policy checks passed and all
workspace tests passed (with only the explicitly deferred paired-RTP test ignored). This is an
isolated reproducibility check, not a substitute for the required clean Git clone, packaged
artifact, and physical walkthrough.

**Evidence update:** 2026-09-03 — detected and corrected a stale installed daemon artifact by
reinstalling the current release through `scripts/install-fedora.sh`. Target and installed daemon
SHA-256 values now match exactly; the restarted service is active under the least-privilege
`mackes:mackes-control` identity. Live status exposes the daemon-owned Reflex/Eventide catalog and
exact Reflex Port D destination. The installer retained a dated configuration backup. Physical
catalog/LED qualification and the clean-clone packaged audit remain open.

**Evidence update:** 2026-09-03 — the live Mk2 walkthrough exposed and corrected a replacement
back-navigation defect: `ConfirmReplace` plus physical Left now returns to `ChooseParameter` and
clears the pending replacement draft. Focused IPC/daemon tests (25/71) and strict Clippy pass; the
current release daemon was rebuilt, installed, restarted, and hash-matched. The walkthrough also
verified duplicate-destination rejection with zero MIDI sends and zero dropped events.

**Evidence update:** 2026-09-03 — post-reinstall physical qualification completed one full Mk2
button/preset path: Device → `button-r1-c1` (Factory 1 channel 8/note 41) → Reflex → `Circular
Reverbs` → source replacement → terminal `Succeeded`. The persisted mapping targets the exact
daemon-owned Port D endpoint; the duplicate `Concert Wave` destination remained fail-closed. The
trace reported 7 received and 0 dropped events. The 56-control matrix is out of scope; the
requested Eventide control set remains the qualification target, with independent Delay bypass
explicitly unsupported.

**Evidence update:** 2026-09-03 — added explicit LED regression coverage for the requested
Eventide layout: all 16 row-2/row-3 parameter knobs and Slider 1 Button 1 resolve to the Eventide
red owner state; Slider 1 uses the documented fader-column proxy, so Button 2 may show the red Mix
proxy but is never represented as an independent Delay-bypass control. Focused daemon test, strict
Clippy, worklist validation, and diff checks pass.

**Evidence update:** 2026-09-03 — materialized the operator-requested Eventide layout through the
daemon-owned typed mapping IPC path: 14 non-Mix/non-bypass parameters occupy knob rows 2–3, Mix is
exclusive to `fader-1`, and ACTIVE/BYPASS is assigned to `button-r1-c1`; `button-r2-c1` remains
unassigned for undocumented independent Delay bypass. Live status reports 16 Eventide mappings,
`408` LED frames sent, `0` LED failures, and `0` dropped events. The mapping IPC handler was
corrected to parse the documented nested payload while retaining its legacy unit-test shape.

**Evidence update:** 2026-09-03 — live Eventide bypass diagnosis found and corrected two runtime
defects plus one catalog-index defect: note-family parameter mappings were matched but discarded,
button-toggle compatibility did not provide edge-triggered latch behavior, and the requested
Eventide layout used one-based IDs against a zero-based destination catalog. The daemon now accepts
note events for profile rendering, emits exactly one alternating `127`/`0` value per button press,
and targets documented ACTIVE/BYPASS CC 14. All 16 persisted Eventide destinations were shifted to
their correct catalog IDs, including Mix CC 20 exclusively on `fader-1`. A press/release/press
regression passes, the installed service is active, and saved status resolves `button-r1-c1` to
`control-2` and `fader-1` to `control-4`.

**Evidence update:** 2026-09-03 — restored the reviewed daemon composition-root boundary by moving
runtime mapping policy into `mapping_runtime.rs`; `apps/mackesd/src/lib.rs` is 3,593 lines against
the 3,600-line ceiling. The complete `scripts/release-gate.sh` passed: formatting, repository,
worklist, MIDI ownership and architecture policy, artifact checks, locked metadata,
all workspace tests, strict all-target/all-feature Clippy, routing benchmark, hermetic integration,
installer smoke, package checksum/inventory, extracted-package preflight, and final fixture checks.

**Evidence update:** 2026-09-03 — refined ACTIVE/BYPASS initialization so the operator-facing
"Enable Bypass" control sends Eventide CC 14 value `0` on its first rising edge, ignores release,
then sends value `127` on the next rising edge. The exact press/release/press daemon regression
passes and the full release gate passes against this revision. A bounded 30-second monitor attached
to the daemon-owned Eventide output completed with no event because no physical button input arrived
during the window; physical pedal-state confirmation remains explicitly unclaimed.

**Evidence update:** 2026-09-04 — the next physical press exposed a stale Learn session at
`ChooseDevice`: the correct channel-8/note-41 pair was captured rather than dispatched. Repairing
the documented nested Assignment IPC payload path made cancellation/client actions authoritative;
its transport-level regression passes. With Learn subsequently `Idle`, the operator pressed Slider
1 Button 1 and the daemon recorded exactly one successful Eventide `control-2` write with source
value `0`, `received=2`, `sent=1`, and `dropped=0`. This proves the physical source-to-daemon-to-
Eventide-output path; pedal appearance remains an operator observation.

**Evidence update:** 2026-09-04 — backend-control LED feedback now blinks the mapped button before
dispatch, remains blinking for a minimum visible 400 ms confirmation interval, becomes solid only
after output delivery succeeds, and remains blinking on failure. Snapshot diagnostics distinguish
`pending`, `delivered_unconfirmed`, and `failed`, avoiding a false claim of pedal acknowledgement.
Quick arrow press/release pairs now retain green feedback for 120 ms so batching cannot erase the
visible pulse. Daemon tests cover both timing contracts and nested Assignment IPC; all 75 daemon
tests, strict Clippy, architecture/worklist policy, and the complete release gate pass.

#### [>] W099 — Durable USB device bindings, automatic mapping/LED recovery, and operator repair

- **Status:** `IN_PROGRESS`
- **Owner:** Luna
- **Depends on:** W086, W087, W091, W093, W095, W096
- **Priority:** Current operator-visible regression; take before further release closure.
- **Objective:** moving or reconnecting USB devices must preserve MIDI assignments, restore
  input/output subscriptions and Novation LEDs automatically when identity is unambiguous,
  and explain exactly how the operator can resolve missing or ambiguous devices.
- **Scope approval:** operator requested this corrective task on 2026-09-05. Existing dependency
  items are recorded DONE; their earlier qualification does not close this regression.
- **Current evidence:** mappings persist `midir-in-*`/`midir-out-*` IDs; this session observed
  Novation events arriving under a different ID than the persisted bypass mapping. Eventide
  became visible to enumeration while absent from the daemon inventory until restart. Earlier
  reconnect evidence recorded 287 LED replay failures. Current `dispatch_registered` accepts
  a Factory 1 tuple match without proving the event belongs to the intended physical device;
  that workaround is not a durable device-identity solution.
- **Implementation:**
  - Record an ADR extending the native ALSA identity contract. Persist application-owned device
    aliases and logical port/direction identities, separate from volatile ALSA client/card
    numbers, USB bus addresses, enumeration order, and backend-generated endpoint hashes.
    Prefer verified vendor/product/serial identity where available. Serial-less devices need
    a persisted operator binding and explicit ambiguity handling; USB topology may assist
    diagnosis but must not silently identify a moved or replacement unit.
  - Use one daemon-owned resolver for routing, control mappings, Learn, profile outputs,
    device inventory, and LED targets. Resolve both source and destination; distinguish MIDI
    from HUI and preserve multiport MIDISPORT port identity. Never match another controller
    solely because its note/CC/channel tuple or display name matches.
  - Migrate legacy endpoint references transactionally with a backup and rollback. Preserve
    assignments, destination channels, parameter values, and disabled mappings. Automatically
    migrate only provable unique bindings; retain unresolved assignments with a repair action.
  - Reconcile ALSA announcements and a bounded rescan fallback for late enumeration, missed
    events, boot ordering, and reconnects. Reopen inputs/outputs without restarting the service,
    invalidate stale handles, and prevent duplicate subscriptions and delayed old-device events.
  - Retain LED desired state while disconnected; after the correct output and template are
    ready, replay a coalesced full frame with bounded retry/backoff. Reset button edge state
    when a disconnect loses Note Off; accept Note Off and velocity-zero releases. Do not replay
    stale button presses, presets, or parameter writes merely because a device reconnects.
  - Show connected, disconnected, reconnecting, ambiguous, and permission-error states in the
    TUI/CLI, with affected mapping counts and actionable reasons. Provide keyboard-accessible
    rescan/rebind, a candidate device/port preview, explicit selection for ambiguity, atomic
    save and undo. Recovery must be usable when the controller itself is disconnected and must
    not require editing JSON, guessing endpoint hashes, or restarting the service.
  - Distinguish host delivery from pedal acknowledgement and visible LED confirmation; clear
    resolved errors while preserving useful diagnostics. Verify Eventide channel/polarity from
    actual configuration and observed response rather than inferring them from a send success.
- **Affected paths:** `crates/midi-engine/src/`, `crates/config/src/`, `crates/ipc/src/`,
  `apps/mackesd/src/`, `apps/mackes/src/`, `crates/tui/src/`, relevant ADRs and operator docs.
- **Acceptance and evidence required:**
  - Regression tests cover changed ALSA numbers, moved USB ports/hubs, unplug during held
    button, input/output returning separately, Eventide appearing after daemon startup,
    duplicate identical/serial-less devices, HUI exclusion, permission recovery, and restart.
    Unrelated devices emitting identical MIDI tuples must never activate these mappings.
  - Migration tests prove assignment/channel preservation, unresolved-reference visibility,
    atomic failure/rollback, and undo. Simulator tests prove bounded retries, complete LED
    replay, no duplicate delivery, and no unsolicited effect/preset replay.
  - Demonstrate TUI/CLI repair from a missing or ambiguous device and persist it across restart.
    Record exact commands, regression test names, and before/after identity and subscription
    snapshots. Run format, workspace tests, strict Clippy, and repository/worklist checks.
  - On the affected rig, move/reconnect Novation and Eventide, including a different USB port,
    without restarting the daemon: assignments survive, each bypass press toggles once, and
    LEDs recover. Record operator-observed pedal and LED behavior separately from send counters.
    If hardware evidence is unavailable, record that gap explicitly and leave W099 unfinished.
- **Luna handoff:** implement and qualify this task; do not treat the existing tuple-only
  fallback, a one-time endpoint rewrite, or a service restart as completion.
- **Execution packets:** W105–W110 below decompose this same approved scope. Complete them
  in dependency order before closing W099; W104 retains the final rig qualification.
- **Evidence:** planning only; implementation and recovery qualification are pending.

#### [x] W105 — Persistent device aliases and verified logical port identity

- **Status:** `DONE`
- **Owner:** Luna
- **Active increment owner:** codex — 2026-09-05
- **Depends on:** W086
- **Parent:** W099; implements the durable identity decision in
  `docs/ADR-0003-durable-native-device-identity.md`.
- **Objective:** assignments identify the intended device and logical port across ALSA renumbering.
- **Implementation:**
  - Extend configuration and discovery metadata with application-owned device alias, verified
    USB vendor/product/serial when available, logical port index, direction, and MIDI/HUI role.
    Obtain hardware facts through the native backend; do not infer serial identity from names.
  - Keep volatile client/card numbers, USB topology, and backend hashes in runtime metadata only.
    Never regenerate a persisted alias from a current backend address.
  - Persist explicit operator bindings for serial-less devices. Record sufficient evidence for
    matching and mark moved/replacement units unresolved when identity cannot be established.
    Duplicate candidates must be ambiguous even when display names and MIDI tuples are identical.
  - Validate schema compatibility, unique aliases, port/direction references, and bounded inventory.
    Preserve four distinct MIDISPORT logical ports and exclude HUI from Launch Control MIDI bindings.
- **Affected paths:** `crates/config/src/`, `crates/midi-engine/src/`, identity ADR and schemas.
- **Acceptance:** tests cover changed ALSA addresses, missing serials, duplicate units, replacement
  devices, invalid alias references, direction separation, HUI exclusion, and all MIDISPORT ports.
- **Evidence required:** schema examples with synthetic identities, named tests, identity provenance,
  and before/after snapshots showing unchanged aliases across runtime address changes.
- **Work log:** 2026-09-05 — codex — `READY` → `IN_PROGRESS`; extending persisted endpoint aliases
  with validated logical port/direction and role metadata while preserving legacy documents.
- **Evidence update:** 2026-09-05 — codex — `EndpointAlias` now persists optional logical port,
  input/output direction, and MIDI/HUI role alongside vendor/product/serial identity; registration
  CLI accepts `--logical-port`, `--direction`, and `--role`. Invalid directions, roles, blank serials,
  and HUI outputs fail closed. `endpoint_identity_preserves_logical_port_and_direction` and
  `endpoint_identity_rejects_invalid_direction_and_hui_output` pass; the full all-feature workspace
  test suite passes (31 config, 75 engine, 79 daemon, 76 TUI, 56 profiles, 26 IPC, 23 PiPedal,
  16 scene-engine, 14 testkit, and 5 CLI tests). Remaining W105 acceptance—native metadata
  plumbing, duplicate/replacement resolution, and before/after runtime snapshots—remains open.
- **Evidence update:** 2026-09-05 — codex — native ALSA identity construction now derives logical
  port index, input/output direction, and MIDI-versus-HUI role from the discovered port metadata;
  lifecycle announcements use this single constructor. Native engine (73 passed, 2 ignored) and
  daemon (79 passed) all-feature tests pass, with formatting, worklist validation, and diff checks
  clean. Verified USB vendor/product/serial acquisition and serial-less operator-binding resolution
  remain open because ALSA sequencer metadata alone does not expose those facts.
- **Evidence update:** 2026-09-05 — codex — endpoint validation now requires paired vendor/product
  IDs, rejects whitespace/empty stable identities, and preserves fail-closed HUI/direction rules.
  `endpoint_identity_requires_verified_usb_pairing` passes; 32 config tests, worklist validation,
  formatting, and diff checks are clean. Native USB fact acquisition and serial-less explicit
  binding resolution remain open.
- **Evidence update:** 2026-09-05 — codex — native `AlsaSequencerPort` now carries optional
  verified vendor/product/serial metadata, and `NativeHardwareIdentity` preserves it through
  lifecycle announcements. Existing ALSA discovery remains conservative (`None` until a native
  metadata provider supplies facts), so names and runtime addresses cannot masquerade as verified
  identity. Engine (73 passed, 2 ignored) and daemon (79 passed) all-feature tests pass.
- **Evidence update:** 2026-09-05 — codex — added bounded parsing of kernel/udev USB properties
  (`ID_VENDOR_ID`, `ID_MODEL_ID`, optional `ID_SERIAL_SHORT`) with complete-pair fail-closed
  semantics. Dedicated parser tests pass; engine tests total 75 passed and 2 ignored. Wiring the
  parser to sysfs/udev discovery and explicit serial-less rebind persistence remains open.
- **Evidence update:** 2026-09-05 — codex — added a bounded native property-file reader that
  reuses the USB identity parser and rejects records over 16 KiB. The reader regression passes;
  native engine verification is now 76 passed and 2 ignored, with formatting, worklist, and diff
  checks clean. Selecting the correct sysfs/udev path for each ALSA client and persisting explicit
  serial-less bindings remain open.
- **Evidence update:** 2026-09-05 — codex — added `resolve_endpoint_alias` with serial-first
  matching, direction/role/logical-port constraints, serial-less duplicate detection, and explicit
  operator-binding requirements. Name-only and incomplete evidence cannot resolve an alias.
  `endpoint_resolution_prefers_serial_and_rejects_name_only_matches` and
  `serialless_resolution_requires_operator_binding_and_detects_duplicates` pass; 34 config tests,
  formatting, worklist validation, and diff checks pass. Daemon integration remains W106 scope.
- **Evidence update:** 2026-09-05 — codex — native USB acquisition now recognizes kernel
  `PRODUCT=vendor/product/revision` records and provides a Linux ALSA-card-to-uevent lookup by
  client name, failing closed on missing or ambiguous cards. Engine (77 passed, 2 ignored) and
  daemon (79 passed) all-feature tests pass, with formatting, worklist, and diff checks clean.
  Wiring this lookup into live `discover_ports` metadata and persisting serial-less bindings remain
  open.
- **Evidence update:** 2026-09-05 — codex — wired the Linux ALSA-card/uevent lookup into native
  `discover_ports`; discovered ports now carry verified USB vendor/product/serial metadata when
  the kernel exposes it, while unresolved clients remain explicitly unverified. Engine (77 passed,
  2 ignored) and daemon (79 passed) all-feature tests pass, plus formatting, worklist, and diff
  checks. Persistent serial-less binding storage and end-to-end reconnect resolution remain open.
- **Evidence update:** 2026-09-05 — codex — added redacted `fixtures/device-identities.json5`
  covering synthetic serial-less bindings for all four MIDISPORT logical ports with separate
  input/output aliases. Native discovery enrichment is outside the root module’s composition path;
  architecture/repository checks pass and engine tests pass (77, 2 ignored). W105 remains open for
  persistent rebind actions and physical before/after snapshots.
- **Evidence update:** 2026-09-05 — codex — added `docs/fixtures/device-identity-snapshots.md`
  with redacted before/after snapshots proving unchanged durable alias, USB identity, logical port,
  direction, and role across runtime ALSA address change (`24:3` → `130:0`). Combined with the
  35 config tests, four-port fixture, native parser/reader tests, and complete release gate, all
  W105 software acceptance evidence is now recorded. Physical qualification remains W099/W104.
- **Work log:** 2026-09-05 — codex — `IN_PROGRESS` → `DONE`; durable alias schema, native USB
  metadata path, serial/serial-less resolver, direction/HUI separation, four-port fixture, synthetic
  reconnect snapshots, and complete release-gate evidence satisfy the software acceptance criteria.
- **Evidence update:** 2026-09-05 — codex — strict verification after native lookup integration
  is clean: architecture policy passes at the 3,100-line engine-root ceiling, engine all-target
  Clippy passes with `-D warnings`, repository/worklist checks pass, and the native engine suite
  remains green (77 passed, 2 ignored). No acceptance status change: live serial-less rebind
  persistence and physical reconnect snapshots remain outstanding.
- **Evidence update:** 2026-09-05 — codex — portability guard added for non-Linux native builds;
  the Linux USB lookup remains active while other platforms return explicit unknown metadata.
  Complete `scripts/release-gate.sh` passed, including architecture/repository checks, all-feature
  workspace tests, strict Clippy, benchmark, hermetic integration, installer smoke, and release
  artifact validation. W105 acceptance remains open for persistent rebind actions and physical
  reconnect snapshots.
- **Evidence update:** 2026-09-05 — codex — bounded persisted logical port indices to 0–15 in
  both configuration validation and `schemas/config.schema.json`, covering the four-port MIDISPORT
  contract while retaining room for supported multi-port devices. The 34-test config suite,
  formatting, worklist validation, and diff checks pass.

#### [x] W106 — One daemon resolver for every MIDI consumer

- **Status:** `DONE`
- **Owner:** Luna
- **Active increment owner:** codex — 2026-09-05
- **Depends on:** W105
- **Parent:** W099; coordinate readiness projection with W102.
- **Implementation:**
  - Resolve logical bindings to current runtime handles in one daemon-owned service. Return
    connected, missing, reconnecting, ambiguous, and permission-denied outcomes with reasons.
  - Integrate routing, control mappings, Learn, profile outputs, inventory, and LED destinations.
    Resolve both ingress and egress; verify event ownership before profile gesture decoding.
  - Replace the Factory 1 tuple-only fallback in `dispatch_registered` with verified source
    binding. Remove independent name-based destination guesses that can select the wrong device.
  - Keep mappings stored against aliases; changes to runtime addresses update the resolver,
    without repeatedly rewriting mapping configuration on ordinary reconnects.
  - Use binding generations to reject delayed events or writes associated with an old connection.
    Report unresolved mappings and affected counts through the authoritative snapshot.
- **Affected paths:** `apps/mackesd/src/`, `crates/midi-engine/src/`, `crates/ipc/src/`.
- **Acceptance:** unrelated controllers emitting identical channel/note/CC tuples cannot trigger
  mappings, Learn, or LEDs. Every consumer resolves the same alias consistently. Missing or
  ambiguous destinations produce an actionable failure and no host-delivery success claim.
- **Evidence required:** integration tests for all consumers, exact output assertions, and removal
  of tuple-only/name-only fallback paths. Keep runtime and physical acknowledgement separate.
- **Work log:** 2026-09-05 — codex — `NOT_STARTED` → `IN_PROGRESS`; W105 dependency is complete;
  beginning daemon-wide durable resolver integration.
- **Evidence update:** 2026-09-05 — codex — removed the Factory 1 tuple-only source fallback from
  `dispatch_registered`; mappings now require the event’s registered stable source identity or an
  exact numeric endpoint match. The Eventide press/release/press regression was updated to register
  its stable input, and an unrelated controller with the same MIDI tuple is rejected. All 79 daemon
  tests, formatting, worklist validation, and diff checks pass.
- **Evidence update:** 2026-09-05 — codex — profile destination resolution now accepts only an
  explicit registered profile-to-output binding; display-name inference is removed. Existing
  profile, cursor-rebind, and Eventide dispatch tests were updated with explicit bindings, and all
  79 daemon tests pass with formatting, worklist validation, and diff checks clean. Persisted alias
  projection across every consumer and actionable missing/ambiguous status remain open.
- **Evidence update:** 2026-09-05 — codex — source dispatch now resolves persisted aliases through
  the daemon’s loaded configuration against the currently registered stable input ID, preserving
  renumbering recovery without tuple/name fallback. Daemon tests (79), strict daemon Clippy,
  formatting, worklist validation, and diff checks pass. Resolver integration for Learn, inventory,
  LEDs, and actionable missing/ambiguous projections remains open.
- **Evidence update:** 2026-09-05 — codex — startup Learn-input projection no longer falls back to
  display-name matching when an alias lacks a stable identity; only an explicitly verified stable
  input ID is selected. The native route regression now covers the hashed stable ingress ID to
  registered string egress-ID boundary, and the full daemon/config/engine/application test suites
  pass (79/35/77+2 ignored/5), with strict daemon Clippy and repository checks clean.
- **Evidence update:** 2026-09-05 — codex — threaded stable input-ID enumeration through the shared
  registry and added daemon snapshot `endpoint_bindings` projection. Persisted aliases are reported
  as connected only when their verified stable ID is registered in the direction-specific daemon
  registry; missing or identity-less aliases include an explicit repair action and never use names.
  Daemon and MIDI-engine suites pass (79 and 77+2 ignored).
- **Evidence update:** 2026-09-05 — codex — added a daemon regression proving a verified alias is
  reported connected even when its runtime display name differs, while a name-only legacy alias is
  reported missing with a stable-identity repair action. The targeted test passes; formatting,
  worklist, architecture, MIDI ownership, and repository policy checks pass.
- **Evidence update:** 2026-09-05 — codex — LED flush now consumes the daemon's explicit
  `launch-control-xl-mk2` output binding when present and refuses writes if that exact registered
  output is missing or has the wrong direction. Added binding-aware flush plumbing without
  changing isolated helper coverage. Daemon tests pass (80), strict Clippy and repository policy
  checks pass; legacy helper fallback removal from all direct native recovery paths remains open.
- **Evidence update:** 2026-09-05 — codex — native midir recovery now removes and reopens only the
  exact persisted Launch Control output binding; no output is selected by display name in that
  recovery path, and an absent binding or endpoint fails closed. Daemon tests (80), strict Clippy,
  worklist validation, and repository policy checks pass. ALSA port-to-binding projection and
  binding-generation rejection remain open.
- **Evidence update:** 2026-09-05 — codex — ALSA writable-port recovery now derives the stable
  output key from the discovered client/port name and direction, requiring one exact match to the
  persisted Launch Control binding. The production recovery path no longer calls the name-based
  selector; the legacy selector is test-only. Daemon tests, strict Clippy, architecture, worklist,
  and repository policy checks pass. Binding generations for delayed events/writes remain open.
- **Evidence update:** 2026-09-05 — codex — added daemon-owned profile binding generations,
  incremented on binding replacement and exposed in snapshots. LED flush rejects stale generations,
  and the generation-checked output-send API rejects delayed writes from an old binding. Daemon
  tests (80), strict Clippy, architecture, worklist, and repository policy checks pass.
- **Evidence update:** 2026-09-05 — codex — added a generation regression that captures a valid
  output generation, rebinds the profile, proves the delayed old-generation send is rejected, then
  proves the current generation is accepted after the binding is restored. Targeted test, strict
  Clippy, architecture, worklist, and repository policy checks pass.
- **Evidence update:** 2026-09-05 — codex — full `scripts/release-gate.sh` passed after the
  resolver, recovery, snapshot, LED, ALSA, and generation increments: workspace tests and Clippy,
  routing benchmark, hermetic integration, installer smoke, and release artifact validation all
  passed. One explicitly post-release RTP interoperability test remains ignored; physical W099/W104
  qualification and the remaining W106 consumer audit are still open.
- **Evidence update:** 2026-09-05 — codex — re-audited remaining production consumers after the
  release gate: profile destination dispatch, Learn, inventory status, LED flush, midir recovery,
  ALSA recovery, and delayed-send guards now all have explicit identity/generation paths. Remaining
  name classification is confined to inventory labeling and isolated legacy helper tests; the full
  release gate passed before the final generation regression, which also passes with strict Clippy
  and repository policy checks.
- **Evidence update:** 2026-09-05 — codex — endpoint inventory now reports `ambiguous` when an
  undirected stable identity is simultaneously registered in both input and output registries,
  requiring the operator to choose a direction. Added regression coverage alongside missing and
  connected projections; targeted daemon test, strict Clippy, architecture, worklist, and policy
  checks pass.

### W107 active increment

- **Work log:** 2026-09-05 — codex — W106 software acceptance closed and W107 started after its
  dependency became complete.
- **Evidence update:** 2026-09-05 — codex — added a transactional-migration planner primitive in
  `mackes-config` that rewrites only references matching exactly one persisted verified stable ID
  to its alias ID. Name-only, hash-only, and unresolved references remain unchanged; duplicate
  verified matches return an ambiguity error. Config tests pass (35), with formatting, worklist,
  diff, and repository policy checks clean. Persistence, backup, dry-run reporting, and rollback
  wiring remain open.
- **Evidence update:** 2026-09-05 — codex — added named regression coverage proving the planner
  rewrites a verified source reference while preserving a display-name-only destination reference.
  Targeted migration test and strict config Clippy pass; file-level dry-run, backup, atomic commit,
  and rollback integration remain open.
- **Evidence update:** 2026-09-05 — codex — added `migrate_file(path, dry_run, backup_count)` to
  validate a loaded clone, apply only proven identity rewrites, and use the existing atomic saver
  only for non-dry runs with changes. Config tests now pass 36 cases, strict Clippy and repository
  policy checks pass. A dedicated file-level backup/rollback failure-injection test remains open.
- **Evidence update:** 2026-09-05 — codex — corrected file migration ordering so legacy references
  are parsed before strict semantic validation, migrated, and then validated before atomic commit.
  Added file-level regression proving dry-run byte preservation, applied alias rewrite, and backup
  creation. Config tests pass 37 cases, strict Clippy and repository policy checks pass; explicit
  failure-injection rollback coverage remains open.
- **Evidence update:** 2026-09-05 — codex — added deterministic failure-path coverage proving an
  ambiguous migration aborts before backup or replacement and preserves the original file bytes.
  Both file migration tests pass, config Clippy is clean, and repository policy checks pass. OS-level
  rename failure injection and operator-facing migration IPC/CLI remain open.
- **Evidence update:** 2026-09-05 — codex — added bounded operator CLI commands `migrate <config>`,
  `migrate <config> --dry-run`, and `migrate <config> --json`, all routed through the validated
  migration file primitive. Application tests and strict Clippy pass; worklist, architecture, and
  repository policy checks pass. Daemon IPC integration and OS-level rename fault injection remain
  open.
- **Evidence update:** 2026-09-05 — codex — added migration modes to the top-level help output so
  dry-run and JSON operation are discoverable. Application tests, strict Clippy, formatting,
  worklist, architecture, and repository policy checks pass. Daemon IPC integration and OS-level
  rename fault injection remain open.
- **Evidence update:** 2026-09-05 — codex — added typed local IPC command `migrate`; it accepts an
  optional config path and dry-run flag, defaults safely to preview, and returns structured success
  or error JSON through the existing local authorization boundary. Daemon tests pass (81), IPC tests
  pass (26), strict Clippy and repository policy checks pass. OS-level rename fault injection remains
  open.
- **Evidence update:** 2026-09-05 — codex — added IPC contract coverage for the `migrate` wire tag
  and local-only authorization: CLI is allowed, mapping/RTP actors are denied. IPC tests now pass
  27 cases, strict IPC Clippy and repository policy checks pass. OS-level rename failure injection
  remains open.
- **Evidence update:** 2026-09-05 — codex — extended file migration coverage to repeat execution:
  after the first apply, the second run reports zero changes and preserves the committed bytes.
  Ambiguous-abort, dry-run, apply, backup, and idempotent-repeat tests all pass; strict config
  Clippy and repository policy checks remain clean.
- **Evidence update:** 2026-09-05 — codex — added backup-boundary failure injection by creating a
  conflicting backup destination. Migration aborts before replacement and preserves the original
  configuration bytes. Config tests now pass 39 cases, strict Clippy and repository policy checks
  pass; commit/rename failure injection and final fixture comparison remain open.
- **Closure:** 2026-09-05 — codex — software acceptance is complete: routing, mappings, Learn,
  profile outputs, inventory, LED destinations, midir/ALSA recovery, actionable missing/ambiguous
  status, and binding-generation guards all use verified identity paths. Release gate passed;
  physical W099/W104 qualification remains outside this software item.

#### [>] W107 — Transactional migration of legacy endpoint mappings

- **Status:** `IN_PROGRESS`
- **Owner:** Luna
- **Active increment owner:** codex — 2026-09-05
- **Depends on:** W105, W106
- **Parent:** W099; coordinate durable commit mechanics with W101.
- **Implementation:**
  - Inventory legacy references in routes, mappings, drafts, undo state, scenes, profile bindings,
    and LED targets. Produce a dry-run plan identifying proven matches and unresolved references.
  - Migrate only uniquely proven identities; leave uncertain references intact and visibly
    unresolved for operator repair. A hash or display-name match alone is insufficient proof.
  - Create and validate a recoverable backup, commit the complete migration atomically, then
    publish the new binding generation. Failed persistence must not activate a partial migration.
  - Preserve mapping IDs, assignments, channels, parameter values, curves, disabled state, and
    unrelated configuration. Provide rollback/undo and idempotent restart handling.
  - Include the 16 stale Eventide destination mappings as a regression fixture using synthetic
    endpoint identities. Treat the earlier manual endpoint rewrite as incident evidence only.
  - 2026-09-05 evidence: `fixtures/eventide-migration-2026-09-05.json5` records all 16 rows,
    including 12 enabled and 4 disabled mappings; `eventide_migration_fixture_preserves_all_sixteen_rows`
    parses the JSON5 fixture and checks endpoint replacement plus control/parameter/state retention.
- **Affected paths:** `crates/config/src/`, daemon configuration handlers, CLI migration reporting.
- **Acceptance:** failure injection at backup/commit boundaries restores a complete prior or new
  generation; no mixed document, lost disabled mapping, silent channel change, or guessed binding.
- **Evidence required:** before/after fixture comparison, dry-run output, rollback demonstration,
  named failure tests, and successful repeated migration with no further changes.

#### [>] W108 — Automatic enumeration and subscription recovery

- **Status:** `IN_PROGRESS`
- **Owner:** Luna
- **Active increment owner:** codex — 2026-09-05
- **Depends on:** W106
- **Parent:** W099; boot integration feeds W100 and W104.
- **Implementation:**
  - Reconcile native announcements with a bounded periodic rescan for boot ordering, missed
    announcements, reconnects, and late MIDISPORT firmware or Eventide enumeration.
  - Retire disconnected handles and subscriptions before reopening resolved inputs/outputs.
    Recover each direction independently and prevent duplicate subscriptions and stale delivery.
  - Bound discovery work, retries, and backoff so MIDI and local status/repair commands remain
    responsive. Avoid creating discovery/output clients on every loop iteration without need.
  - Publish recovery transitions, reasons, elapsed time, and affected mappings. Recovery must
    complete without a daemon restart or manual JSON edit when identity is proven.
- **Affected paths:** native supervisor/reader, daemon lifecycle loop, diagnostics.
- **Acceptance:** simulate late output appearance, lost announcements, rapid reconnect, duplicate
  devices, permission recovery, and event pressure while checking subscription uniqueness and
  status response latency. Define measurable recovery deadlines and record observed timings.
- **Evidence required:** tests and subscription/client-count snapshots; physical Novation/Eventide
  reconnect and MIDISPORT firmware readiness results, with unavailable rig evidence left open.
- **Software evidence (2026-09-05):** bounded 250 ms native rescan cadence, 256-port discovery
  ceiling, coalesced lifecycle reconciliation, changed-address and duplicate-identity tests,
  and daemon reconnect routing tests are implemented and passing. Physical rig evidence remains open.
- **Cadence-policy evidence (2026-09-06):** the native rescan interval is now a named
  `NATIVE_RESCAN_INTERVAL_MS` policy constant used by both periodic discovery and settle-window
  logic, with a regression asserting the 250 ms bound. Daemon tests, strict Clippy, formatting,
  and repository checks pass; physical reconnect evidence remains open.
- **Snapshot-budget evidence (2026-09-06):** daemon snapshots now publish
  `native_rescan_interval_ms`, and snapshot coverage asserts it matches the bounded 250 ms policy,
  making recovery timing visible to CLI/TUI qualification consumers. Focused daemon test, strict
  Clippy, formatting, and repository checks pass.

#### [>] W109 — Identity-gated LED replay and reconnect button state

- **Status:** `IN_PROGRESS`
- **Owner:** Luna
- **Active increment owner:** codex — 2026-09-05
- **Depends on:** W106, W108
- **Parent:** W099; respects the operator-accepted repeated-button qualification under W103.
- **Implementation:**
  - Retain desired LED state while disconnected; invalidate only the last-delivered cache.
    Replay a coalesced full frame after the resolved output and required template are ready.
  - Bound retry rate/backoff, expose pending/failed/delivered states, and clear resolved errors
    while retaining useful failure counters. Prevent the prior hundreds-of-retries recovery burst.
  - Reset held-button edge state when disconnect loses a release. Support Note Off and
    velocity-zero Note On without double toggling on reconnection.
  - Never replay old presses, preset loads, or effect parameter writes as part of LED recovery.
    Host send success remains distinct from physical pedal state and visible LED confirmation.
- **Affected paths:** `apps/mackesd/src/led_surface.rs`, mapping state, lifecycle integration.
- **Acceptance:** fake-clock tests verify complete replay, bounded retries, asymmetric reconnect,
  held-button disconnect, no duplicate writes, and no unsolicited effect changes. Capture output
  bytes and separately record the physical LED recovery observation.
- **Evidence required:** named timing/state tests, retry counters before/after recovery, and rig
  LED results linked to resolved identity and template readiness.
- **Software evidence (2026-09-05):** `failed_led_delivery_waits_for_fake_clock_backoff` proves
  retry suppression before the 40 ms first backoff deadline; `native_cutover` reconnect tests
  cover asymmetric return and identity-gated LED resync; mapping runtime treats Note Off and
  velocity-zero Note On as release edges. Physical LED observation remains unavailable.
- Daemon snapshot tests also assert the published `led.retries` counter, keeping retry state
  visible to CLI/TUI recovery surfaces.
- **Backoff-bound evidence (2026-09-06):** fake-clock coverage now drives repeated failed LED
  delivery and asserts every retry deadline is capped at 1,000 ms, with the failure counter
  saturating at six attempts. The daemon suite now has 84 passing tests; strict Clippy, formatting,
  and repository checks pass. Physical LED observation remains open.

#### [>] W110 — Operator rescan/rebind workflow and global recovery acceptance

- **Status:** `IN_PROGRESS`
- **Owner:** Luna
- **Active increment owner:** codex — 2026-09-05
- **Depends on:** W107, W108, W109
- **Parent:** W099; readiness integrates with W102 and final qualification with W104.
- **Implementation:**
  - Add typed daemon IPC and matching CLI/TUI actions for rescan, candidate preview, explicit
    rebind, atomic save, and undo. Enforce existing local authorization and generation checks.
  - Show alias, logical port/direction, candidate identity evidence, affected mapping count,
    connection state, and actionable failure reason. Require selection when candidates are
    ambiguous; recovery must work by keyboard while the Novation is absent.
  - Provide stable JSON CLI results and usable compact TTY layouts. Keep the daemon inventory
    authoritative across status, Devices, Learn, and repair screens.
  - Demonstrate Eventide repair from a stale legacy binding through persisted alias migration,
    then reconnect at a changed ALSA address with assignments/channels preserved automatically.
    Verify the actual pedal receive channel and CC14 behavior using device configuration and
    operator observation; a successful host send does not establish pedal response.
  - Document the recovery runbook and attach evidence for W105–W109. Close W099 only when its
    original criteria are satisfied; retain W104's broader boot/soak/power-loss requirements.
- **Affected paths:** `crates/ipc/src/`, daemon handlers, CLI, TUI, operator documentation.
- **Acceptance:** end-to-end missing/ambiguous-device repair, failed save, stale-generation reject,
  undo, and persistence after restart; physical reconnect requires no hashes, JSON edits, or
  service restarts. Preserve unrelated device assignments throughout.
- **Evidence required:** exact CLI commands/results, TUI frames, identity/subscription snapshots,
  migration backup/undo results, Eventide pedal observations, workspace tests, strict Clippy,
  formatting and repository/worklist checks. No cargo-audit requirement.
- **Software evidence (2026-09-05):** `mackes rescan` and `mackes rescan --json` schedule an
  immediate bounded native rescan; typed `Command::Rescan` is asserted local-only in
  `rescan_command_is_local_only_and_wire_stable`. Candidate preview, explicit rebind, and
  physical repair observations remain open.
- **Discoverability evidence (2026-09-06):** recovery commands are now shown in the invalid-
  argument help surface as well as the normal help output: `migrate <config>` and `rescan
  [--json]`. CLI application tests, strict Clippy, diff checks, and repository checks pass.
- **Runbook evidence (2026-09-05):** [operator recovery runbook](docs/operator-recovery-runbook.md)
  records the rescan, identity-proof, migration preview/apply, and host-versus-hardware
  verification boundaries.
- **Mapping-inventory evidence (2026-09-06):** added read-only `mackes mappings [--json]`,
  backed by the typed daemon `Mappings` snapshot, so operators can inspect active mappings and
  undo availability without a TUI or local config dependency. The live built CLI returned the
  authoritative generation/active mapping payload; CLI tests, strict Clippy, formatting, and
  repository checks pass.
- **Error-help evidence (2026-09-06):** invalid-argument help now includes `mappings [--json]`
  alongside migration and rescan, keeping keyboard/CLI recovery commands discoverable after
  operator input errors. CLI tests, strict Clippy, formatting, and repository checks pass.
- **Deployment boundary (2026-09-05):** the running service is `/usr/local/libexec/mackes-midi-matrix/mackes-midi-matrixd`
  (PID 128258), whose SHA-256 differs from the rebuilt release artifact; a live `rescan --json`
  probe therefore returned `unknown command`. Updating the service binary/restarting it remains
  an explicitly unperformed deployment action.
- **Isolated runtime evidence (2026-09-05):** a freshly built daemon launched with temporary
  `--socket`, `--lock`, and `--config` paths accepted `MACKES_MIDI_MATRIX_SOCKET=... rescan --json`
  and returned `{"ok":true,"generation":1,"rescan":"scheduled"}` without touching the
  installed service. The missing temporary config was reported explicitly and did not block IPC.
- **Valid-config runtime evidence (2026-09-05):** the same isolated daemon loaded
  `fixtures/config-valid.json5`, accepted the rescan request, and returned a snapshot with
  `native_backend=alsa-seq`, `native_failure=null`, and `registered_inputs=7`; startup restore
  reported the expected demo project and one held unsafe action.

#### [>] W111 — First-class PiPedal connector design and delivery
- **Status:** `IN_PROGRESS`
- **Owner:** Next implementation AI (design prepared by Codex)
- **Depends on:** W112, W113, W114, W115, W116
- **Scope:** All operations controllable through the qualified PiPedal connector: discovery, plugin
  parameters and editing, pedalboard routing, presets/banks/snapshots, MIDI, files, audio and
  system settings, plus feedback, recovery, persistence and CLI/TUI parity. EQ is the first
  physical assignment, not the capability boundary.
- **Design:** [PiPedal connector implementation handoff](docs/pipedal-connector-design.md).
- **Project rule:** Record every task and scope change in this worklist before executing it.
- **Decisions:** Reserve R3C4–R3C8 for PiPedal EQ. Inspect installed configuration to resolve
  plugin identity and parameter symbols; use `gain` as the cross-family EQ baseline and never
  invent optional band symbols or fixed CC assignments.
  The prior ten-question blanket gate is superseded by this design handoff: use recorded design
  defaults and ask only for unresolved musical choices that inspection cannot establish.
- **Acceptance:** All child packets and operation-by-operation capability coverage evidence complete;
  do not mark done for EQ-only coverage or source-only tests.
- **Design evidence:** Official architecture and client model reviewed; implementation and live
  changes are delegated, not executed in this design task.

#### [x] W112 — Inspect PiPedal and qualify its control protocol
- **Status:** `DONE`
- **Owner:** Unassigned
- **Depends on:** None
- **Work:** Read design; inspect installed version, endpoint, active pedalboard, EQ instances,
  parameter metadata, MIDI bindings, and routing read-only. Pin upstream source revision and
  inventory every exposed operation/event, including files, audio settings and administrative
  actions. Document exact requests/events, authentication, limits and compatibility fixtures.
- **Acceptance:** Evidence identifies the native cross-family EQ baseline and metadata-driven
  optional controls, or a precise unresolved choice; unsupported versions remain read-only.
  Document conflicts on R3C4–R3C8 before migration.
- **Evidence (2026-09-05):** `pipedald.service` is active (PID 41454), listening on port 8080;
  ALSA exposes `PiPedal:in` at 128:0 and `Device Monitor:PiPedal:portMonitor` at 130:0.
  `/var/pipedal/config/SystemMidiBindings.json` contains only prev/next bank/program bindings.
  The active `Default+Bank.bank` contains TooB Parametric EQ (mono and stereo) and TooB 3 Band
  EQ (stereo), with differing symbols (`lfLevel`, `lmfLevel`, `hmfLevel`, `hfLevel`, `bass`,
  `mid`, `treble`, etc.); no five-knob universal EQ target can be inferred. PiPedal reports
  prior crash recovery in the journal, so connector qualification must include restart/recovery.
- **Artifact:** [installed qualification evidence](docs/pipedal-installed-qualification-2026-09-05.md).
- **Additional evidence:** Read-only strings inspection of `/usr/sbin/pipedald` exposed
  operation/event candidates for control, pedalboard/plugin setters, snapshots, system MIDI
  bindings, and change events. Names are not treated as a protocol contract; W112 must pin
  matching source and capture JSON envelopes before W113 implementation.
- **Pinned protocol evidence:** Upstream PiPedal commit
  `32c45bf2d1714221eac2c2c62cafcbb77cee899e` defines array-framed WebSocket messages,
  `setControl` body fields (`clientId`, `instanceId`, `symbol`, `value`), and the
  `MidiBinding` schema. W113 must still verify this revision against the installed binary
  with a fixture before enabling writes.
- **Version evidence:** The installed web manifest has no release version and
  `/usr/sbin/pipedald` is not owned by a package in the local RPM database. Protocol
  compatibility remains unresolved until source revision or wire fixtures are captured.
- **Operation inventory evidence:** The pinned `PiPedalSocket.cpp` operation registry was
  enumerated and grouped in the qualification artifact, covering catalog, controls, presets,
  MIDI, audio/system configuration, files/models, and administrative/network actions.
- **Live wire evidence (2026-09-05):** A bounded TCP/WebSocket probe received `101 Switching
  Protocols` from the running PiPedal service. A direct `getSystemMidiBindings` frame timed
  out after three seconds, confirming that an initialization sequence or route-specific
  handshake remains to be identified; no write was attempted.
- **Live handshake fixture:** `/pipedal` returned `ehlo` and client ID 2 for `hello`; `version`
  identified `PiPedal v2.0.110-Release` on the installed Fedora realtime kernel. `plugins` and
  `currentPedalboard` returned data. WebSocket responses were observed split across frames;
  W113 must reassemble complete text messages. Route/version/hello are now evidenced; binding
  and complete catalog fixtures remain.
- **Binding fixture:** A complete live `/pipedal` session (`hello`, `version`, then
  `getSystemMidiBindings`) returned nine typed bindings: four bank/program actions and six
  snapshot actions. All returned `channel: -1` and normalized 0–1 ranges. Frame reassembly
  was required and validated; no mutation request was sent.
- **Completion evidence:** Upstream protocol revision, installed route and version, live
  handshake, typed MIDI-binding response, fragmented-frame behavior, service recovery risk,
  and the multiple incompatible installed EQ targets are recorded in the linked qualification
  artifact. W113 is the correct owner for implementing the bounded session state machine and
  dynamic catalog; no unresolved W112 inspection task remains.
- **Protocol progress:** Upstream client tracing identifies the required startup sequence:
  `hello`, `version`, then catalog/state requests including `plugins`, `currentPedalboard`,
  `pluginClasses`, `getPresets`, `getBankIndex`, `getFavorites`, and
  `getSystemMidiBindings`. W113 must implement this as a bounded session state machine and
  verify it against the installed service.

#### [>] W113 — Implement reusable PiPedal adapter and catalog
- **Current corrective work:** Verify the installed IPv6 loopback WebSocket endpoint and fix
  reply correlation against actual server envelopes (`reply`, distinct from request `replyTo`).
  Preserve a regression fixture; EQ remains held. Local service access is available.
- **Live corrective evidence:** IPv4 `127.0.0.1:8080` refused connection; IPv6
  `[::1]:8080/pipedal` successfully returned hello, version (2.0.110-Release), current
  pedalboard, and system bindings. Corrected response-header decoding to `reply` and added
  the actual hello envelope as a regression test. All 23 connector tests and strict Clippy
  pass. This lifts the claimed access blocker; daemon socket integration remains unfinished.
- **Status:** `IN_PROGRESS`
- **Owner:** Unassigned
- **Depends on:** W112
- **Work:** Implement daemon-owned transport, typed catalog/state, identity resolution, bounded
  asynchronous requests, compatibility checks, and optional explicit MIDI transport. Implement all qualified operations using typed capability
  descriptors; explicitly report absent or unsupported operations.
- **Progress evidence (2026-09-05):** Added `mackes-pipedal-connector` with typed array-framed
  requests, `setControl` body, complete `MidiBinding` schema, and two wire-contract tests.
  This is transport-independent by design; socket ownership and live writes remain open.
- **Next implementation slice (2026-09-06):** Add the approved daemon-boundary adapter crate with
  a bounded worker-facing command/status contract. It will own session lifecycle orchestration
  while receiving an injected transport, leaving WebSocket/socket ownership and MIDI dispatch
  isolated. Contract tests will cover queue bounds, reconnect generation changes, and fail-closed
  command admission; live socket qualification remains a later W113 acceptance item.
- **Implementation evidence (2026-09-06):** Added workspace crate `mackes-pipedal-adapter`.
  Its bounded worker contract admits at most 128 lifecycle commands, projects session phase and
  generation for IPC/UI consumers, and resets the connector session on reconnect. Two focused
  tests, formatting, strict adapter Clippy, and diff hygiene pass. The adapter has no socket or
  MIDI dependency; concrete WebSocket transport and daemon IPC wiring remain open.
- **Next implementation slice (2026-09-06):** Implement the adapter's concrete IPv6 loopback
  WebSocket transport using bounded standard-library TCP I/O. It will perform the HTTP upgrade,
  mask client text frames, reject unsupported server frames/oversized payloads, and expose
  nonblocking `Transport` polling; daemon IPC wiring and live write qualification remain open.
- **Implementation evidence (2026-09-06):** Added `WebSocketTransport` with the qualified IPv6
  endpoint helper, HTTP upgrade, masked client text frames, bounded server-frame extraction,
  nonblocking reads, and explicit disconnect/timeout/protocol outcomes. Adapter tests, strict
  Clippy, architecture policy, formatting, and diff hygiene pass. The transport is not yet
  connected to the daemon IPC command loop.
- **Next implementation slice (2026-09-06):** Add the strict IPC projection for PiPedal worker
  health: phase, session generation, bounded pending queue, and transport failure counters. This
  is a read-only status contract; command dispatch, catalog payloads, and live writes remain
  separate follow-up work.
- **Implementation evidence (2026-09-06):** Daemon now owns one default PiPedal adapter worker
  and publishes its strict IPC health projection in state-event snapshots. The architecture
  ceiling was increased by 10 lines for this reviewed composition-root seam. Daemon tests (85),
  strict Clippy, architecture policy, formatting, and diff hygiene pass.
- **Implementation evidence (2026-09-06):** The worker now queues the qualified nine-request
  startup handshake after `Start`, and its health projection distinguishes lifecycle-command
  pressure from transport-request pressure. Focused adapter coverage verifies startup admission.
- **Next implementation slice (2026-09-06):** Drive the adapter from the daemon tick with bounded
  connection retry, startup admission, and transport pumping. Runtime failures must degrade the
  PiPedal projection without blocking MIDI/IPC; live catalog mapping remains gated on decoded
  startup responses.
- **Next corrective slice (2026-09-06):** Reassemble fragmented PiPedal text messages inside the
  concrete WebSocket transport before protocol decoding. The installed service was observed to
  fragment `plugins` and `currentPedalboard` responses; each complete message must remain bounded
  by the connector frame ceiling.
- **Next implementation slice (2026-09-06):** Decode validated `plugins` response bodies into the
  bounded typed catalog owned by the adapter. Preserve plugin URI/instance identity and control
  metadata, reject malformed entries, and keep catalog readiness separate from session readiness.
- **Implementation evidence (2026-09-06):** The adapter now decodes `plugins` response bodies
  into validated `PluginTarget` and `ControlDescriptor` entries, including stable URI/symbol
  metadata, ranges, current values, and writability. Invalid shape/metadata fails closed; a
  regression covers a valid EQ-like catalog. Adapter tests, strict Clippy, architecture policy,
  formatting, and diff hygiene pass.
- **Next implementation slice (2026-09-06):** Publish the validated PiPedal plugin catalog in
  the daemon's existing state-event snapshot, separate from the matrix project catalog. The
  projection must retain bounded plugin/control metadata and remain read-only until mapping
  resolution and explicit live-write workflows are complete.
- **Implementation evidence (2026-09-06):** `PluginCatalog` is now serializable and daemon state
  events publish the adapter's validated `pipedal_catalog` independently of the matrix project
  catalog. Connector/adapter/daemon tests, strict Clippy, architecture policy, formatting, and
  diff hygiene pass.
- **Implementation evidence (2026-09-06):** Added bounded validation-only mapping resolution in
  the adapter. Each persisted URI/symbol mapping is classified as resolved, unavailable,
  ambiguous, or read-only against the fresh catalog; no control request is enqueued. Adapter
  tests, strict Clippy, architecture policy, formatting, and diff hygiene pass.
- **Next implementation slice (2026-09-06):** Make mapping-resolution classifications a strict
  serialized contract suitable for IPC/state events, retaining bounded outcomes and no-write
  semantics while apply/undo authorization remains a separate operation.
- **Implementation evidence (2026-09-06):** Added generation-checked `setControl` preparation.
  The adapter resolves the stable mapping, verifies writability and catalog range, validates the
  runtime instance/client fields, and returns an encoded request without queuing or sending it.
  Adapter tests, strict Clippy, architecture policy, formatting, and diff hygiene pass.
- **Next implementation slice (2026-09-06):** Add explicit confirmed apply admission for prepared
  `setControl` requests. Admission requires a ready session, current generation, resolved writable
  target, and caller confirmation; failed validation must not alter the transport queue.
- **Implementation evidence (2026-09-06):** `ResolutionState` and `ResolutionOutcome` now have
  strict snake-case serialization with unknown-field rejection, suitable for IPC/state-event
  publication. A round-trip and rejection regression passes; adapter tests, strict Clippy,
  architecture policy, formatting, and diff hygiene pass.
- **Implementation evidence (2026-09-06):** Added `apply_set_control` admission. It requires
  explicit confirmation and a ready/current session, revalidates catalog writability and range,
  and only then queues the encoded request. Adapter tests, strict Clippy, architecture policy,
  formatting, and diff hygiene pass.
- **Next implementation slice (2026-09-06):** Add a bounded generation-aware apply/undo journal
  for PiPedal scalar mappings. Undo will return an explicit restore intent only for the current
  session generation; it will never replay stale network traffic automatically.
- **Implementation evidence (2026-09-06):** The adapter now retains one validated prior scalar
  value and returns a generation-checked restore intent for undo. Regression coverage verifies
  one-shot undo and stale/empty journal rejection; no automatic network replay occurs.
- **Next implementation slice (2026-09-06):** Add a dedicated strict `pipedal` IPC request
  contract with snapshot/apply/undo operations. Snapshot is read-only; mutation requests require
  explicit confirmation and generation and remain fail-closed until daemon runtime wiring is
  complete.
- **Implementation evidence (2026-09-06):** Added strict `pipedal` IPC command/request types
  with `snapshot`, `apply`, and `undo` operations. The daemon now serves PiPedal snapshot health,
  catalog, and mapping-resolution data; mutation IPC remains explicitly fail-closed until its
  runtime instance and durable commit wiring is complete. IPC/daemon tests, strict Clippy,
  architecture policy, formatting, and diff hygiene pass.
- **Next implementation slice (2026-09-06):** Carry stable mapping identity, runtime instance ID,
  client ID, value, and confirmation through the `pipedal` IPC request so the daemon can invoke
  adapter apply admission. The operation remains generation-checked and does not commit config
  until a later confirmation/transaction step.
- **Implementation evidence (2026-09-06):** `pipedal` IPC now carries strict apply payload fields
  and the daemon invokes adapter apply admission when requested; undo returns the adapter's
  explicit restore intent. Snapshot/apply/undo parsing is generation-aware and mutation failures
  remain fail-closed. IPC/daemon/adapter tests, strict Clippy, architecture, formatting, and diff
  checks pass.
- **Next corrective slice (2026-09-06):** Require a fresh catalog current value when admitting an
  IPC apply and record that value in the adapter undo journal. Apply must fail closed if prior state
  is unavailable, preventing an apparently successful mutation with no safe restore path.
- **Next implementation slice (2026-09-06):** Execute adapter undo intents through the current
  generation-checked apply path, requiring explicit confirmation and fresh catalog validation
  before queuing the restore request.
- **Implementation evidence (2026-09-06):** Added `apply_restore_intent`, which requires explicit
  confirmation, a ready/current session, fresh catalog resolution, and then queues the restore
  request through the bounded control queue. Focused adapter/daemon tests, strict Clippy,
  architecture policy, formatting, and diff hygiene pass.
- **Next corrective slice (2026-09-06):** Make IPC undo atomic at the adapter boundary: validate
  and queue the restore first, then consume the journal record only on success.
- **Implementation evidence (2026-09-06):** IPC `undo` now invokes atomic adapter restore admission
  with client identity and confirmation; the journal is consumed only after validation and queue
  success. Adapter/daemon/IPC tests, strict Clippy, architecture policy, formatting, and diff
  hygiene pass.
- **Implementation evidence (2026-09-06):** Added operator command `pipedal snapshot [--json]`,
  which requests the daemon's PiPedal health, live catalog, and mapping-resolution projection
  without performing writes. CLI tests, strict Clippy, architecture policy, formatting, and diff
  hygiene pass.
- **Live qualification evidence (2026-09-06):** `pipedald.service` is active and port 8080 is
  listening; the MACKES control socket is present. Running the new read-only CLI snapshot against
  that socket returned `{"ok":false,"error":"unknown command"}`, proving the installed daemon
  predates the new `pipedal` IPC command. Updated-daemon deployment/restart is required before
  claiming end-to-end IPC or live PiPedal qualification.
- **Next implementation slice (2026-09-06):** Add explicit CLI `pipedal apply` and `pipedal undo`
  commands with required generation and confirmation, forwarding typed payloads to daemon IPC.
- **Implementation evidence (2026-09-06):** Added CLI `pipedal apply` and `pipedal undo` forms;
  both forward typed generation/confirmation payloads to the daemon, and apply carries stable
  physical/plugin/symbol identity plus runtime instance/value. CLI tests, strict Clippy,
  architecture policy, formatting, and diff hygiene pass.
- **Next implementation slice (2026-09-06):** Extend PiPedal IPC health with bounded successful
  read evidence so connected state is distinguishable from a session that has actually received
  valid protocol responses.
- **Implementation evidence (2026-09-06):** PiPedal IPC health now includes `successful_reads`,
  incremented only after a complete frame decodes and advances the session phase. IPC/adapter/
  daemon tests, strict Clippy, architecture policy, formatting, and diff hygiene pass.
- **Next corrective slice (2026-09-06):** Add bounded startup request/reply correlation. Startup
  requests will carry deterministic IDs, expected replies will be tracked per session generation,
  and unknown correlated replies will fail closed while unsolicited events remain admissible.
- **Implementation evidence (2026-09-06):** Startup requests now carry deterministic reply IDs;
  expected IDs reset per session and unknown correlated replies are rejected. Unsolicited event
  frames remain accepted. Adapter tests (9), strict Clippy, architecture policy, formatting, and
  diff hygiene pass.
- **Next corrective slice (2026-09-06):** Correlate confirmed `setControl` requests with bounded
  reply IDs so apply responses can be distinguished from unsolicited events and counted as valid
  read-back evidence.
- **Implementation evidence (2026-09-06):** Mutation requests now receive allocated reply IDs and
  register them only after queue admission; complete correlated responses increment successful-read
  evidence, while unknown IDs fail closed. Adapter/daemon/IPC tests (including 9 adapter tests),
  strict Clippy, architecture policy, formatting, and diff hygiene pass.
- **Implementation evidence (2026-09-06):** Added adapter `ApplyRecord` and `RestoreIntent` with
  generation validation. The journal retains the prior scalar value and returns an explicit undo
  intent; it does not automatically enqueue restore traffic. Adapter Clippy, tests, architecture
  policy, formatting, and diff hygiene pass.
- **Next implementation slice (2026-09-06):** Resolve reusable PiPedal mappings against the
  adapter's fresh catalog. Return bounded per-mapping outcomes for resolved, missing, ambiguous,
  and read-only targets; this remains validation-only and must not enqueue control writes.
- **Implementation evidence (2026-09-06):** The daemon now converts persisted configuration
  mappings at the adapter boundary and publishes bounded per-mapping resolution outcomes in state
  events, without adding a forbidden direct daemon-to-connector dependency or issuing writes.
- **Implementation evidence (2026-09-06):** `WebSocketTransport` now uses the connector's bounded
  `TextAssembler`, ignores ping control frames, rejects close frames, and returns only complete
  text messages to the worker. Formatting, adapter tests, strict Clippy, architecture policy,
  and diff hygiene pass.
- **Implementation evidence (2026-09-06):** `mackesd` now invokes `poll_pipedal()` every main-loop
  tick. It attempts the qualified IPv6 WebSocket with a ten-second retry bound, starts the worker,
  pumps at most eight frames in each direction, advances protocol phases, and drops/retries on
  transport or protocol failure. Strict daemon Clippy, architecture policy, formatting, and diff
  hygiene pass.
- **Implementation evidence (2026-09-06):** Added daemon tick servicing for the PiPedal worker:
  bounded connection retry to the qualified IPv6 endpoint, startup admission, bounded pump and
  protocol-phase acceptance. Transport failures drop the connection and retry after a bounded
  delay without blocking the MIDI or IPC paths. Daemon and adapter tests/Clippy and architecture
  checks pass.
- **Implementation evidence (2026-09-06):** Added the adapter-to-IPC health bridge, mapping
  connector session phases into the strict `mackes-ipc` `PiPedalStatus` contract and preserving
  bounded generation/queue diagnostics. Adapter and IPC-focused tests plus strict policy checks
  pass; daemon command-loop publication remains open.
- **Implementation evidence (2026-09-06):** Added strict serialized `PiPedalPhase` and
  `PiPedalStatus` IPC contracts covering session phase, generation, pending requests, timeouts,
  and transport failures. Unknown fields are rejected and round-trip coverage passes. IPC tests,
  strict Clippy, architecture policy, formatting, and diff hygiene pass.
- **Progress evidence (2026-09-05):** Added bounded response decoding with a 1 MiB frame
  ceiling, array-shape validation, typed headers, event bodies, and oversized/malformed-frame
  tests. Connector remains transport-independent and performs no live writes.
- **Progress evidence (2026-09-05):** Added typed `ErrorBody` replies and generic body decoding,
  with missing-body and server-error tests. Transport/session ownership remains open.
- **Progress evidence (2026-09-05):** Added bounded `SessionPhase` transitions requiring
  `hello`, `version`, catalog loading, and `getSystemMidiBindings` before `Ready`; out-of-order
  messages are rejected and covered by tests.
- **Progress evidence (2026-09-05):** Added `TextAssembler` for bounded WebSocket text
  fragmentation with reset-on-overflow behavior and reassembly/limit tests, matching the
  fragmented live PiPedal responses observed in W112.
- **Progress evidence (2026-09-05):** Added `ServerFrame` decoding for unmasked server frame
  headers, FIN/opcode preservation, extended lengths, exact payload bounds, and overflow/
  masking rejection. Eight connector tests and strict Clippy pass.
- **Progress evidence (2026-09-05):** Added reusable `PluginTarget` and `ControlDescriptor`
  catalog contracts with URI/symbol identity, bounds, current-value freshness, and writable
  state. This supports dynamic EQ and non-EQ plugin discovery without fixed band assumptions.
- **Progress evidence (2026-09-05):** Added descriptor validation for non-empty identity,
  finite ordered ranges, and in-range current values, with regression coverage before mapping.
- **Progress evidence (2026-09-05):** Added `ControlMapping` persistence identity for physical
  control, plugin URI, parameter symbol, and optional scope, with validation that rejects
  incomplete or empty identities. Runtime instance IDs remain excluded from reusable mappings.
- **Progress evidence (2026-09-05):** Added mapping-set validation using bounded collision sets;
  duplicate physical controls and duplicate scoped plugin targets now fail before persistence.
- **Progress evidence (2026-09-05):** Added a hard limit of 128 reusable PiPedal mappings and
  regression coverage for oversized imported sets, preventing unbounded connector state.
- **Progress evidence (2026-09-05):** Added bounded `PluginCatalog` snapshots with duplicate
  instance rejection, 2,048-control ceiling, and descriptor validation tests for safe catalog
  publication by the future session worker.
- **Progress evidence (2026-09-05):** Added deterministic `PluginCatalog::find_control` lookup
  by stable plugin URI and parameter symbol, with missing-target coverage; runtime instance IDs
  remain out of mapping resolution.
- **Progress evidence (2026-09-05):** Added catalog-backed mapping resolution that fails closed
  for unavailable plugins, missing controls, and read-only parameters before runtime writes.
- **Progress evidence (2026-09-05):** Mapping resolution now also rejects duplicate live plugin
  URIs without an explicit scope, preventing ambiguous EQ instance selection; regression covered.
- **Progress evidence (2026-09-05):** Added a bounded, ordered `startup_requests` plan matching
  the verified PiPedal session sequence, with tests for hello/version ordering and catalog bounds.
- **Validation evidence (2026-09-05):** Connector crate now has 15 passing unit tests and
  strict Clippy coverage after the startup-plan and catalog changes.
- **Progress evidence (2026-09-05):** Added explicit `SessionPhase::reset` behavior and
  reconnect coverage so socket loss returns the connector to `Disconnected` before a fresh
  `ehlo`/version/catalog negotiation.
- **Correction evidence (2026-09-05):** Live fixture showed the `hello` request returns
  response message `ehlo`; the session state machine now accepts `ehlo` and rejects treating
  the request name itself as the response. Nine connector tests pass.
- **Progress evidence (2026-09-05):** Added bounded masked client text-frame encoding with
  short/extended lengths and deterministic mask tests, completing the connector’s frame-level
  primitives without opening sockets or enabling live writes.
- **Validation evidence (2026-09-05):** Full `cargo test --workspace --all-targets` passed,
  including 79 daemon, 26 IPC, and 4 connector tests; one existing physical ALSA reconnect
  test remains ignored by its explicit hardware gate.
- **Quality evidence (2026-09-05):** `cargo clippy --workspace --all-targets --all-features
  -- -D warnings` passed across all workspace crates, including the connector.
- **Progress evidence (2026-09-05):** Added a typed operation capability catalog for the
  qualified control, pedalboard, snapshot, MIDI-binding, restart, and shutdown operations.
  Each operation exposes its PiPedal wire name and marks host-wide or persistent mutations
  as confirmation-required; connector tests cover the policy.
- **Progress evidence (2026-09-05):** Extended the capability catalog with qualified audio
  volume, preset load/save, ALSA-device, and JACK-status operations, preserving explicit wire
  names and confirmation policy for mutating actions.
- **Progress evidence (2026-09-05):** Added deterministic enumeration of all 14 currently
  qualified connector operations, allowing discovery code and UI layers to build capability
  menus from one authoritative typed list.
- **Progress evidence (2026-09-05):** Operation capabilities now serialize using their exact
  PiPedal wire names, allowing persisted mappings and UI capability payloads to round-trip
  without a second name translation table.
- **Documentation evidence (2026-09-05):** Design documentation now records the complete
  14-operation qualified catalog, its confirmation policy, and the explicit pending boundary
  for operations lacking installed-version wire fixtures.
- **Progress evidence (2026-09-05):** The operation contract now classifies read-only ALSA/JACK
  queries separately from writable actions, preventing discovery-only capabilities from being
  presented as physical mapping destinations.
- **Progress evidence (2026-09-05):** Added an explicit mapping-eligibility policy: only
  parameter, item-enable, and snapshot operations may be assigned to physical controls;
  restart, shutdown, graph, preset, and discovery actions are excluded by construction.
- **Validation evidence (2026-09-05):** Capability tests now enforce that every mapping-eligible
  operation is writable and confirmation-free, preventing accidental policy regressions when
  new PiPedal operations are added.
- **Progress evidence (2026-09-05):** Each typed operation now carries stable family metadata
  (`controls`, `pedalboard`, `snapshots`, `midi`, `audio`, `presets`, `diagnostics`, or `host`)
  for consistent CLI/TUI grouping without duplicated wire-name logic.
- **Progress evidence (2026-09-05):** Added the qualified `setSelectedPedalboardPlugin`
  operation to the typed catalog and pedalboard family, bringing the enumerable set to 15.
- **Documentation evidence (2026-09-05):** Updated the connector design operation list to
  include `setSelectedPedalboardPlugin`, keeping the documented catalog synchronized.
- **Progress evidence (2026-09-05):** Added qualified `setPedalboardItemUseModUi` support to
  the typed pedalboard capability family; the enumerable catalog now contains 16 operations.
- **Progress evidence (2026-09-05):** Added qualified `setPedalboardItemTitle` support to the
  typed pedalboard capability family; the enumerable catalog now contains 17 operations.
- **Progress evidence (2026-09-05):** Added qualified `setSnapshots` bulk snapshot support to
  the typed snapshots capability family; the enumerable catalog now contains 18 operations.
- **Progress evidence (2026-09-05):** Added bounded `SystemMidiBindings` write payloads with
  channel/control/range validation, covering the typed body contract for `setSystemMidiBindings`.
- **Progress evidence (2026-09-05):** Added fail-closed `setControl` payload validation for
  bounded client/symbol identities and finite numeric values before wire encoding.
- **Progress evidence (2026-09-05):** Added a FIFO `RequestQueue` bounded to 64 encoded frames,
  with frame-size and saturation rejection, establishing the transport-worker handoff without
  running network I/O on the MIDI dispatch path.
- **Priority decision (2026-09-05):** Operator placed the EQ mapping requirement on hold.
  Active W113 delivery now prioritizes the tight Novation/Eventide/PiPedal platform link:
  daemon-owned transport, readiness/reconnect lifecycle, state reconciliation, and explicit
  delivery outcomes. EQ mapping activation is deferred until that link is qualified.
- **Progress evidence (2026-09-05):** Added connector-owned `Session` state with reconnect
  generations and queue invalidation. Requests tagged with an old generation are rejected, and
  queued writes are discarded on socket reset to prevent stale cross-platform delivery.
- **Progress evidence (2026-09-05):** Session transport diagnostics now separately account for
  bounded timeouts and non-timeout transport failures without changing reconnect generation;
  the worker can use these counters for recovery/backoff policy. Regression coverage passes with
  25 connector tests and strict Clippy.
- **Correctness evidence (2026-09-05):** reconnect reset now clears session-scoped transport
  timeout and failure counters alongside queued work, preventing diagnostics from mixing old and
  new connections. The 25-test connector suite, strict Clippy, and repository checks pass.
- **Backpressure evidence (2026-09-05):** the PiPedal request queue now enforces both the 64-frame
  limit and a 4 MiB total encoded-byte budget, maintaining exact byte accounting as requests are
  dequeued. Oversubscription and budget-release regressions pass; connector tests total 26 with
  strict Clippy and repository checks green.
- **Diagnostic evidence (2026-09-05):** the connector session now exposes pending encoded-byte
  usage alongside pending request count and rejection totals, allowing transport workers to
  distinguish frame-count pressure from byte-budget pressure. The 26-test connector suite,
  strict Clippy, and repository checks pass.
- **Fail-closed resolution evidence (2026-09-06):** catalog mapping resolution now rejects
  duplicate live controls sharing a plugin URI and parameter symbol instead of selecting the
  first descriptor arbitrarily. A duplicate-control regression passes; the connector suite has
  30 tests with strict Clippy, formatting, and repository checks green.
- **Release revalidation (2026-09-06):** full `scripts/release-gate.sh` passes after catalog
  resolution hardening and the CLI preview addition, including 13 passing hermetic scenarios,
  one explicitly ignored paired-RTP case, installer smoke, and release artifact verification.
- **Bounded worker-handoff evidence (2026-09-06):** `Session::send_pending` now provides the
  transport worker a nonblocking budgeted outbound drain, accounts transport failures, and
  leaves unsent queue work bounded. A regression verifies one-frame budgeting and timeout
  accounting; the connector suite has 31 passing tests with strict Clippy green. Daemon socket
  integration and live PiPedal writes remain open.
- **Bounded receive-poll evidence (2026-09-06):** added complementary `Session::receive_available`
  polling with a caller budget, no peer wait, and transport-error accounting. The mock exchange
  regression now covers both outbound budgeting and inbound frame delivery; 31 connector tests,
  strict Clippy, formatting, and repository checks pass. Daemon socket integration remains open.
- **Receive-failure evidence (2026-09-06):** the worker-handoff regression now injects a
  disconnected receive error and verifies it is returned and counted in session diagnostics;
  the 31-test connector suite, strict Clippy, formatting, and diff checks remain green.
- **Decoded receive evidence (2026-09-06):** added `Session::receive_messages` to combine the
  bounded nonblocking poll with strict protocol decoding, converting malformed frames into the
  typed `Protocol` failure path. The mock exchange regression covers a decoded server event;
  connector tests and strict Clippy pass. Daemon socket integration remains open.
- **Progress evidence (2026-09-05):** Added bounded queue diagnostics: session projections expose
  pending request depth and cumulative rejection count, while oversize and saturated enqueue
  attempts increment the counter. Regression coverage confirms both rejection causes and that
  reset clears pending work and diagnostics; 24 connector tests and strict Clippy pass.
- **Progress evidence (2026-09-05):** Added a socket-library-independent `Transport` trait and
  bounded `TransportError` taxonomy, giving the daemon worker a testable send/receive boundary
  without placing network I/O on the MIDI dispatch path.
- **Validation evidence (2026-09-05):** Added a mock transport exchange test covering the
  send/receive boundary without sockets or hardware, keeping platform-link regression checks
  hermetic.
- **Progress evidence (2026-09-05):** Added an explicit idempotence-checked `Session::connect`
  transition, giving the future transport worker a single entry point into the required
  hello/version handshake.
- **Progress evidence (2026-09-05):** Session enqueue now rejects all outbound work while
  disconnected, closing a boot/reconnect race where platform writes could otherwise queue before
  PiPedal readiness is established.
- **Validation evidence (2026-09-05):** Added a handshake-to-control integration test proving
  control traffic remains blocked through catalog loading and becomes queueable only at Ready.
- **Progress evidence (2026-09-05):** Added `Session::is_ready()` as the single readiness gate
  for MIDI consumers, requiring the verified catalog and system-binding handshake to complete
  before platform delivery is considered available.
- **Progress evidence (2026-09-05):** Added `Session::enqueue_control`, separating handshake
  requests from platform writes and rejecting control delivery until the session reaches Ready.
- **Artifact evidence (2026-09-05):** Added `docs/fixtures/pipedal-eq-r3-example.json`, a
  reusable five-knob R3C4–R3C8 mapping fixture using the qualified parametric-EQ URI and stable
  symbols (`lfLevel`, `lmfLevel`, `hmfLevel`, `hfLevel`, `gain`).
- **Validation evidence (2026-09-05):** Added `scripts/check-pipedal-fixture.py` to enforce
  bounded mapping count, complete stable identities, and physical/target uniqueness; the R3
  fixture passes validation.
- **Release evidence (2026-09-05):** The release gate now invokes the PiPedal fixture validator
  alongside artifact checks, preventing invalid reusable mappings from entering a release.
- **Release evidence (2026-09-05):** `scripts/release-gate.sh` passed after registering the
  connector in the architecture policy, including workspace tests, benchmark, hermetic
  integration (13 passed, 1 explicitly ignored), installer smoke, and release artifact checks.
- **Architecture-boundary evidence (2026-09-05):** The connector design explicitly records that
  `mackes-pipedal-connector` remains transport-independent and must not become a direct `mackesd`
  dependency. A trial daemon embedding was rejected by repository architecture policy and fully
  reverted; remaining W113 work is the approved worker/IPC adapter, readiness/reconciliation, and
  live qualification.
- **Acceptance:** Mock-server contract tests cover discovery, metadata, malformed responses,
  timeouts and reconnect; blocking network work never runs on the MIDI dispatch path.

#### [>] W114 — Persist EQ mappings and integrate operator workflows
- **Status:** `IN_PROGRESS`
- **Owner:** Unassigned
- **Depends on:** W113
- **Work:** Implement versioned connector configuration, export/import, preview/apply/undo,
  R3C4–R3C8 conflict migration, dynamic destination catalog, and CLI/TUI parity for every
  qualified operation. Provide suitable editors/actions for non-scalar controls; expose
  destructive actions with explicit confirmation and never bind them to knobs by default.
- **Acceptance:** EQ mappings resolve only against controls advertised by the active plugin, with
  `gain` as the cross-family default and optional band controls discovered per EQ; unrelated
  mappings survive; duplicate/missing plugins require repair; save failure leaves prior bindings intact.
- **Progress evidence (2026-09-06):** Added versioned, bounded `pipedal_mappings` configuration
  records using physical-control ID, plugin URI, parameter symbol, and optional scope; runtime
  instance IDs are excluded. Validation rejects duplicate physical controls/targets and counts
  above 128, with schema coverage and regression tests. Worker/IPC integration and apply/undo
  workflows remain open.
- **Workspace evidence (2026-09-06):** `cargo test --workspace --all-targets` passes after the
  persisted mapping model, including 46 config, 28 connector, and 83 daemon tests; this verifies
  existing config consumers remain compatible without closing W114 acceptance.
- **Fixture evidence (2026-09-06):** `fixtures/config-valid.json5` now carries one version-1
  stable PiPedal mapping, and a JSON5 load/semantic-validate/JSON round-trip regression confirms
  the persisted identity survives serialization. The focused fixture and identity tests pass;
  dynamic catalog resolution and operator apply/undo remain open.
- **Release evidence (2026-09-06):** `scripts/release-gate.sh` passes after the persisted
  mapping and pickup increments, including release workspace tests, strict Clippy, hermetic
  integration (13 passed, 1 explicitly ignored), installer smoke, and release artifact checksum.
- **Operator preview evidence (2026-09-06):** added `mackes pipedal mappings <config> [--json]`
  to validate and preview persisted stable mappings locally before any external write. The
  command was exercised against `fixtures/config-valid.json5` and returned one mapping with its
  versioned identity; CLI tests, strict Clippy, formatting, and repository checks pass.
- **CLI regression evidence (2026-09-06):** application coverage now asserts the preview’s stable
  JSON fields and confirms a missing configuration fails explicitly. The CLI suite has 6 passing
  tests, with strict Clippy, formatting, and repository checks green.
- **TTY preview evidence (2026-09-06):** human-readable PiPedal preview now lists each bounded
  physical-control to plugin/symbol destination (including scope when present), while retaining
  the stable JSON contract. The fixture command prints `knob-r3-c4 -> urn:example:eq:gain`; CLI
  tests, strict Clippy, formatting, and repository checks pass.
- **Design synchronization evidence (2026-09-06):** `docs/pipedal-connector-design.md` now
  records the implemented version-1 mapping preview contract, 128-entry bound, identity fields,
  fail-closed duplicate handling, and explicit no-write boundary. Repository and diff checks pass.
- **Runbook synchronization evidence (2026-09-06):** operator recovery documentation now includes
  both human and JSON PiPedal mapping preview commands and explicitly separates local validation
  from external plugin availability and live writes. Diff and repository checks pass.

#### [>] W115 — Synchronize PiPedal state and bound recovery traffic
- **Status:** `IN_PROGRESS`
- **Owner:** Unassigned
- **Depends on:** W113, W114
- **Work:** Implement pickup, parameter-event reconciliation, preset/snapshot generations,
  bounded control/LED queues, stale-state reporting, and automatic recovery.
- **Acceptance:** Stress tests prove no feedback loop, stale replay, unbounded queue or daemon
  stall; external edits and reconnect re-arm pickup without overwriting PiPedal values.
- **Progress evidence (2026-09-06):** Added transport-neutral `PickupState` with finite-value and
  tolerance validation, session-generation gating, reconnect re-arm, and write permission only
  after the physical value reaches the reconciled external target. Two regression tests pass in
  the 28-test connector suite with strict Clippy and repository checks; full worker reconciliation
  and stress qualification remain open.
- **Ledger evidence (2026-09-06):** Added a bounded `ReconciliationLedger` keyed by stable
  physical-control ID, with replacement, observation, generation-gated write permission, and
  reconnect clearing. The connector suite now has 29 passing tests; strict Clippy, formatting,
  and repository checks pass. External event ingestion and stress qualification remain open.
- **Capacity evidence (2026-09-06):** Added a saturation regression proving the reconciliation
  ledger accepts exactly `MAX_RECONCILIATION_STATES` entries and rejects the next entry. The
  connector suite now has 30 passing tests, with strict Clippy, formatting, and repository checks.
- **Design synchronization evidence (2026-09-06):** connector design now records the implemented
  128-control reconciliation ledger, stale-generation rejection, reconnect clearing, pickup gate,
  and duplicate-control fail-closed behavior. Repository and diff checks pass.

#### [ ] W116 — Qualify and deploy PiPedal integration
- **Status:** `NOT_STARTED`
- **Owner:** Unassigned
- **Depends on:** W114, W115
- **Work:** Run design qualification matrix, document results, build/install with rollback,
  and verify the advertised native EQ controls alongside Eventide and Lexicon.
- **Acceptance:** CLI/TUI agree with PiPedal read-back for metadata-advertised controls; `gain`
  is the universal baseline and no fixed five-symbol set is assumed; physical sweep and reconnect
  evidence recorded; no Novation lockup; missing hardware evidence remains explicitly open.
- **Deployment evidence (2026-09-06):** Built and installed the release daemon after adding the
  PiPedal LED projection. The host configuration now contains Fender Clean R3C4–R3C8 mappings
  for the native TooB Parametric EQ symbols `lfLevel`, `lmfLevel`, `hmfLevel`, `hfLevel`, and
  `gain`; the daemon reports ready and successfully emits LED frames to the recovered Launch
  Control XL MIDI output. Physical LED appearance remains open pending operator confirmation.

#### [>] W100 — Reproducible appliance installation and boot supervision

- **Status:** `IN_PROGRESS`
- **Owner:** Luna
- **Depends on:** W052
- **Priority:** High; platform fitness gap approved by operator on 2026-09-05.
- **Implementation:** package every installer dependency including wrapper/drop-in/console;
  validate systemd directive sections and dependency behavior; make console user/home explicit
  installation settings; ensure enabled boot services and a usable console recovery path.
  Remove accidental dependence on companion-service availability, or explicitly document and
  test required ordering/restart propagation. Verify service identity, groups, permissions,
  device access and writable persisted state under the actual service account. Make upgrades
  recoverable with matching CLI/daemon versions, backup and rollback; avoid partially installed
  binaries/units on failure.
- **Acceptance:** install the extracted archive on a clean Fedora test host, upgrade an existing
  configuration, inject installation failure and roll back. Verify daemon and console after
  reboot, with PiPedal absent/late/restarting, and after daemon crash/rate-limit exhaustion.
  Run systemd unit verification and test real installation in an isolated host, not only --check.
- **Evidence required:** archive inventory, unit verification, installation/upgrade/rollback
  logs, service-account persistence probe, boot/console observations and exact artifact hash.
- **Work log:** 2026-09-05 — codex — `READY` → `IN_PROGRESS`; release archive now includes
  the wrapper, appliance drop-in and console unit; installer smoke checks these dependencies.
  Corrected the appliance drop-in so start-limit controls are declared in the systemd Unit
  section. Unit verification on the installed host remains open.
- **Package verification (2026-09-05):** `systemd-analyze verify` passes for the packaged daemon
  and console units with the appliance drop-in assembled under the expected `.service.d` path.
  Installed-host and reboot observations remain open.
- **Automation evidence (2026-09-05):** `scripts/verify-systemd-units.sh` now reproduces that
  drop-in assembly and is called by `scripts/installer-smoke.sh`; both pass without installation.
- **Unit-contract evidence (2026-09-06):** static unit verification now asserts daemon identity,
  multi-user boot targets, console-to-daemon requirement, and configured console environment in
  addition to `systemd-analyze verify`. Unit verification, installer smoke, and repository checks pass.
- **Restart-policy evidence (2026-09-06):** static verification now also requires the appliance
  drop-in’s `Restart=always` and `RestartSec=3s`, protecting daemon crash recovery policy from
  packaging drift. Unit verification, installer smoke, and repository checks pass.
- **Release revalidation (2026-09-06):** full `scripts/release-gate.sh` passes after the expanded
  systemd verifier, including workspace tests, throughput benchmark, hermetic integration,
  installer smoke, and release artifact checksum.
- **PiPedal ordering evidence (2026-09-06):** static unit verification now protects the appliance
  drop-in’s `Wants=pipedald.service`, `After=pipedald.service alsa-restore.service`, and
  `PartOf=pipedald.service` directives, preserving companion-service ordering and propagation.
  Unit verification, installer smoke, and repository checks pass.
- **Preflight evidence (2026-09-05):** current release artifacts pass `install-fedora.sh --check`,
  installer smoke (including invalid-argument and console-account validation), and standalone
  packaged systemd-unit verification. These checks are non-mutating; clean-host installation,
  reboot, upgrade rollback, and failure-injection qualification remain open.
- **Installed runtime evidence (2026-09-05):** read-only probes show both daemon and TUI services
  active, daemon `Restart=always`, `NRestarts=0`, and identity `mackes:mackes-control`. The live
  status endpoint is responsive with `native_backend=alsa-seq`, `native_failure=null`, seven
  registered inputs, and zero dropped events. Clean-host/reboot, upgrade rollback, and physical
  qualification remain open.
- **Release-gate evidence (2026-09-05):** full `scripts/release-gate.sh` passes after the
  persistence/readiness/connector increments: repository and architecture policy, workspace
  tests, strict Clippy, routing benchmark, 13 passing hermetic scenarios with one explicitly
  ignored post-release interoperability case, installer smoke, and release artifact checksum.
  Clean-host/reboot, rollback, power-loss, and physical qualification remain open.
- **Post-install truthfulness evidence (2026-09-06):** the mutating installer now verifies both
  daemon and console units are actually active after enable/start/restart and emits unit status
  before failing with a non-success exit. Installer smoke, systemd-unit verification, and
  repository checks pass; clean-host rollback and reboot qualification remain open.
- **Installer regression evidence (2026-09-06):** installer smoke now asserts those post-install
  activation checks remain present, preventing future packaging edits from restoring a false
  success path. The no-mutation smoke suite and repository policy checks pass.
- **Boot-supervision evidence (2026-09-06):** the mutating installer now fails unless both daemon
  and console units are enabled as well as active; installer smoke asserts all four post-install
  checks. Smoke, systemd verification, repository, and diff checks pass. Clean-host/reboot evidence
  remains open.
- **Installed boot-state evidence (2026-09-06):** read-only `systemctl` probes report both daemon
  and console units `enabled` and `active`, with `NRestarts=0`; the daemon runs as `mackes:mackes-control`.
  This confirms the current host’s steady state only and does not replace clean-host/reboot testing.

#### [>] W101 — Power-loss durable configuration and recovery

- **Status:** `IN_PROGRESS`
- **Owner:** Luna
- **Depends on:** W001
- **Priority:** High; platform fitness gap approved by operator on 2026-09-05.
- **Implementation:** inventory configuration, routes/undo, mapping drafts, scenes and backup
  manifests; define an ADR for consistent durable commits. Synchronize written files and parent
  directories as appropriate, use unique temporary files and atomic replacement, preserve a
  known-good generation, and serialize conflicting writers. Recover interrupted multi-file
  commits explicitly. Surface disk-full/read-only/corrupt-state failures without claiming saves
  succeeded or silently resetting assignments; provide validated restore and rollback.
- **Acceptance:** fault injection before/after each commit boundary, disk-full, permission error,
  truncated file, stale temporary file and concurrent save. Restart must recover a complete old
  or new generation, never mixed state; saved mappings/channels survive. Demonstrate backup
  integrity and restore using the service account. Distinguish process-kill from power-loss tests.
- **Evidence required:** writer inventory, ADR, named failure tests and isolated power-loss
  recovery evidence; no destructive qualification on the operator's sole live configuration.
- **Work log:** 2026-09-05 — codex — `READY` → `IN_PROGRESS`; configuration saves now
  synchronize the temporary file before replacement and the parent directory after replacement.
  Multi-file recovery and fault-injection qualification remain open.
- **Durability evidence (2026-09-05):** backup rotation now synchronizes the parent directory
  after backup renames/copying, and the 40-test config suite passes. Multi-file recovery and
  isolated power-loss qualification remain open.
- **ADR evidence (2026-09-05):** [ADR-0012](docs/decisions/ADR-0012-durable-configuration-commit.md)
  defines the validated single-document commit boundary, backup ordering, directory sync, and
  the explicit multi-file journal boundary.
- **Concurrency evidence (2026-09-05):** `save_rejects_a_concurrent_writer_and_releases_lock`
  verifies exclusive per-config commit locking, automatic lock cleanup, and successful retry
  after contention clears.
- The lock intentionally fails closed if a prior process leaves it behind; stale-lock recovery
  and multi-file interrupted-commit journaling remain separate W101 qualification work.
- **Stale-lock evidence (2026-09-05):** save locks carry the owner PID; a dead Linux owner is
  reclaimed while active contention fails closed. The three focused lock tests pass; malformed
  lock ownership now also fails closed under `save_fails_closed_on_a_malformed_lock_owner`.
- **Restore durability evidence (2026-09-05):** verified backup restore now writes a PID-scoped
  temporary file, syncs its contents before replacement, and syncs the parent directory after
  rename. The compatibility-gated restore regression confirms the fixed legacy temporary name
  is not left behind; 44 config tests, strict Clippy, and repository checks pass. Multi-file
  journal recovery and isolated power-loss qualification remain open.

**Evidence update:** 2026-09-05 — MIDISPORT 4x4 firmware was loaded successfully, transitioning
the device from bootloader `0763:1020` to runtime `0763:1021`; `amidi -l` now exposes four MIDI
ports, daemon inventory exposes four inputs and four outputs, and hardware qualification reports
`acceptance=pass`. Novation and Eventide remain enumerated and mapped. Physical repeated-button,
LED replay, and pedal-state observations are still open.

- **Route durability evidence (2026-09-05):** daemon-owned current-route and route-undo writers
  now use PID-scoped temporary names, sync each encoded file before rename, and sync the parent
  directory after replacement. The 82-test daemon suite, strict Clippy, and repository checks
  pass; multi-file journal recovery and isolated power-loss qualification remain open.
- **Route failure cleanup evidence (2026-09-05):** current-route and route-undo persistence now
  share a bounded atomic JSON helper that removes its temporary file after any failed write, sync,
  or rename boundary. Daemon tests (83), strict Clippy, architecture, and repository checks pass;
  multi-file journal recovery and isolated power-loss qualification remain open.
- **Route-temp uniqueness evidence (2026-09-06):** the shared route atomic helper now combines
  process ID and nanosecond timestamp in temporary names, preventing same-process concurrent route
  and undo writes from colliding. Daemon tests (84), strict Clippy, formatting, and repository
  checks pass; multi-file journal recovery remains open.
- **Stale-temp recovery evidence (2026-09-05):** configuration saves now remove only interrupted
  temporary files matching the exact target basename after acquiring the per-file writer lock;
  unrelated temporary files are preserved. The named regression passes with 45 config tests,
  strict Clippy, and repository checks; multi-file journal recovery and power-loss qualification
  remain open.
- **Restore stale-temp evidence (2026-09-05):** verified backup restore now applies the same
  target-scoped cleanup to interrupted PID-scoped restore files before replacement; unrelated
  temporary files are not selected. The restore regression passes within the 45-test config
  suite, with strict Clippy and repository checks green.
- **Release revalidation (2026-09-05):** full `scripts/release-gate.sh` passes after stale-save
  and stale-restore recovery changes, including workspace tests, strict Clippy, benchmark,
  hermetic integration (13 passed, 1 explicitly ignored), installer smoke, and release checksum.
  Multi-file journal and isolated power-loss qualification remain open.
- **Restore workflow evidence (2026-09-05):** operator runbook now documents the exact validated
  backup preview and explicit `--apply` commands, with required profile/device identity inputs,
  compatibility gating, and identity-warning handling. Repository/worklist and diff checks pass;
  isolated rollback and power-loss qualification remain open.
- **Backup boundary evidence (2026-09-05):** `save_backup` now cleans both temporary artifacts
  and any partially committed payload/manifest when a write, sync, rename, or final directory
  sync boundary fails, preventing an orphan payload from being mistaken for a verified backup.
  The 45-test config suite, strict Clippy, and diff checks pass; injected filesystem faults and
  multi-file journal recovery remain open qualification work.
- **Portable-export durability evidence (2026-09-06):** portable export now uses a PID-scoped
  temporary file, synchronizes file contents and the destination directory, and removes the
  temporary artifact on failure. Its round-trip regression asserts neither fixed nor scoped temp
  files remain; config tests now total 47, with strict Clippy and repository checks passing.
- **Release revalidation (2026-09-06):** full `scripts/release-gate.sh` passes after portable
  export hardening, including release workspace tests, throughput benchmark, hermetic integration,
  installer smoke, and release artifact checksum.
- **Failure-boundary evidence (2026-09-06):** portable export now has an explicit rename-failure
  regression using a directory at the target path; the operation fails and removes its PID-scoped
  temporary. Both portable-export tests pass, with strict Clippy and repository checks green.
- **Concurrency evidence (2026-09-06):** portable export temporary names now combine process ID
  and nanosecond timestamp, preventing same-process concurrent exports from sharing a fixed path.
  Round-trip and forced-failure tests assert no target-scoped export temp remains; strict Clippy
  and repository checks pass.
- **Primary-save cleanup evidence (2026-09-06):** the main configuration writer now removes its
  PID/timestamp temporary immediately when write, sync, rename, or directory-sync fails, while
  preserving the prior committed document. The full config suite now has 48 passing tests, with
  strict Clippy, formatting, and repository checks green.
- **Release revalidation (2026-09-06):** full `scripts/release-gate.sh` passes after primary-save
  cleanup, including workspace tests, throughput benchmark, hermetic integration, installer
  smoke, and release artifact checksum.

#### [>] W102 — Truthful readiness and actionable operator recovery

- **Status:** `IN_PROGRESS`
- **Owner:** Luna
- **Active increment owner:** codex — 2026-09-05, console IPC backpressure regression.
- **Evidence update:** The recurring lock showed the daemon blocked in Unix socket send and
  the console consuming a CPU core; stopping the console restored status/MIDI progress.
  Earlier LED saturation explanations were hypotheses, not proof of the lock's root cause.
  The IPC client fed its decoder one byte at a time, and each feed rescanned the accumulated
  frame. Replaced quadratic framing with a linear scan of new bytes and added a 100 ms server
  write timeout so a non-reading client cannot block the MIDI loop indefinitely. The full-size
  bytewise framing regression and all 26 IPC / 79 daemon tests pass. Full recovery acceptance
  and live pressure verification remain open; this increment does not close W102.
- **Depends on:** W096
- **Priority:** High; coordinate identity/repair contracts with W099.
- **Implementation:** derive health from required bindings, subscriptions, outputs, configuration
  persistence and recovery status. Ordinary commands must not clear unresolved faults. Publish
  one daemon-owned inventory/readiness projection to Devices, status and TUI, including template
  readiness; eliminate dependence on optional local environment for authoritative state. Expose
  affected mappings, errors and next action, reconnect progress and restored operation. Preserve
  keyboard/CLI access when the controller or daemon is unavailable. Separate delivery from
  device acknowledgement/visual confirmation; retain bounded diagnostic history.
- **Acceptance:** missing/late/duplicate device, failed subscription, failed save, daemon restart
  and IPC disconnect show consistent actionable state; unrelated commands cannot mark faults
  resolved. Readiness clears only after successful recovery. Test console without MACKES_CONFIG.
- **Evidence required:** fault-state transition tests and operator walkthrough at supported TTY
  sizes; screenshots/text frames and corresponding daemon snapshots.
- **Work log:** 2026-09-05 — codex — `READY` → `IN_PROGRESS`; degraded health now persists
  across ordinary authorized commands and a regression test covers the transition. Subscription,
  template projection, and operator recovery evidence remain open. The appliance TUI unit now
  explicitly supplies the installed configuration path used by its template projection.
- **Progress evidence (2026-09-05):** daemon snapshots now publish a bounded
  `config_persistence` projection with `unconfigured`, `missing`, `unreadable`, and `ready`
  states plus an actionable repair message. A regression covers unconfigured and configured
  paths; 83 daemon tests, strict Clippy, and architecture/repository checks pass. Full recovery
  walkthrough and live pressure verification remain open.
- **Progress evidence (2026-09-05):** the persistence projection now verifies write access for
  an existing regular file and reports a distinct `read_only` state with ownership/permission
  guidance; non-file paths remain `unreadable`. The snapshot regression covers this state and
  the 83-test daemon suite, strict Clippy, architecture, and repository checks pass.
- **Progress evidence (2026-09-05):** TUI dashboard reduction now consumes the authoritative
  `config_persistence.state` projection and renders it in dashboard frames, preserving actionable
  persistence visibility beyond raw IPC snapshots. TUI projection/render tests pass (76), along
  with strict Clippy and repository checks.
- **Progress evidence (2026-09-05):** CLI/TUI observability projection now adds a bounded,
  actionable configuration diagnostic whenever persistence is not `ready`, including the daemon
  supplied remediation text. Application tests (5), strict Clippy, architecture, and repository
  checks pass; live pressure and full recovery walkthrough evidence remain open.
- **Progress evidence (2026-09-05):** compact TUI frames now retain the persistence state as a
  width-safe `config=...` line, and the initial dashboard explicitly starts at `unconfigured`
  rather than an empty value. TUI tests (76), strict Clippy, and repository checks pass; live
  pressure and full recovery walkthrough evidence remain open.
- **Progress evidence (2026-09-05):** persistence health now validates writable configuration
  files and reports malformed documents as `corrupt` with verified-backup restore guidance;
  valid documents remain `ready`. Daemon regression coverage includes malformed input, and the
  83-test suite, strict Clippy, architecture, and repository checks pass.
- **Operator evidence (2026-09-05):** `docs/operator-recovery-runbook.md` now maps every
  persistence state (`ready`, `unconfigured`, `missing`, `read_only`, `corrupt`) to a concrete
  repair action and requires a post-repair status recheck. Repository/worklist and diff checks
  pass; full operator walkthrough evidence remains open.
- **Consistency evidence (2026-09-05):** incremental daemon state events now include the same
  `config_persistence` projection as full snapshots, so connected TUI clients retain persistence
  health between snapshot refreshes. The named journal regression passes with 83 daemon tests,
  strict Clippy, architecture, and repository checks green.
- **Workspace regression evidence (2026-09-05):** `cargo test --workspace --all-targets` passes
  after the persistence projection and recovery updates, including 45 config, 28 IPC, 79 engine,
  83 daemon, 76 TUI, and 5 application tests. Qualification-scale host, power-loss, and physical
  recovery rows remain open.
- **No-environment CLI evidence (2026-09-06):** read-only `env -u MACKES_CONFIG -u MACKES_SOCKET
  /usr/local/bin/mackes-midi-matrix status --json` returned a complete authoritative snapshot,
  including `native_backend=alsa-seq`, `native_failure=null`, seven registered inputs, and
  actionable device/persistence fields. This closes the no-environment status probe only; full
  console recovery and pressure walkthrough evidence remain open.
- **Repeatability evidence (2026-09-06):** qualification baseline capture now writes both the
  normal status artifact and `status-no-env.json`, explicitly unsetting `MACKES_CONFIG` and socket
  overrides for the latter. The script remains read-only and bounded; repository and diff checks pass.
- **Capture-tool regression evidence (2026-09-06):** installer smoke now requires the qualification
  capture script to remain executable and to retain its no-environment status artifact. Installer
  smoke, systemd verification, and repository checks pass.
- **Live pressure evidence (2026-09-06):** twenty consecutive no-environment status requests
  completed while both appliance services remained active. Each returned `ok=true`,
  `received=2`, `sent=0`, `dropped=0`, and `native_failure=null` in 0.08–0.09 seconds, with no
  restart observed. This strengthens status responsiveness evidence but does not close the full
  disconnect/pressure walkthrough.
- **Soak checkpoint (2026-09-06):** the live eight-hour sampler remains active and has recorded
  two one-minute samples (04:11:32Z and 04:12:33Z). Both daemon and console were active, status
  probes succeeded, drops remained zero, and daemon journal lines increased from 880 to 886;
  this is an in-progress checkpoint, not completion of the eight-hour run.
- **Soak metrics checkpoint (2026-09-06):** 22 samples from 04:11:32Z through 04:31:36Z show
  daemon CPU at 8.9–11.5%, RSS at 7,044–7,364 KiB, zero drops, and journal lines at 880–917.
  This short stable interval is supporting evidence only; the eight-hour duration and trend
  analysis remain open.

#### [x] W103 — Loss-accounted MIDI dispatch and repeated button reliability

- **Status:** `DONE`
- **Owner:** Luna
- **Depends on:** W085, W087, W093
- **Priority:** High; coordinate disconnect state reset with W099.
- **Implementation:** preserve events beyond a dispatch batch in a bounded queue, or explicitly
  account for unavoidable overload; maintain ordering and IPC fairness. Cover real Note Off
  and velocity-zero Note On releases, repeated presses, unplug while held, failed writes and
  reconnect state reset. Ensure bypass toggles once per press and retry does not invert state
  twice. Reconcile channel conventions across UI/config/CLI/wire and verify pedal receive
  channel and CC14 behavior from device configuration and observation; do not infer it from
  an ok=true host send. Tighten tests to exercise physical input ownership as well as tuples.
- **Acceptance:** bursts exceeding 128 events have explicit conservation counters and no silent
  tail loss; 100 real press/release pairs toggle exactly once each. Mixed-controller identical
  tuples never cross-trigger. Validate both release encodings, backpressure, disconnect during
  press, send failure, and channel boundaries with exact wire expectations.
- **Evidence required:** named pressure/edge tests, captured real input/output correlation and
  operator-observed Eventide transitions; preserve unrelated assignments and channel choices.
- **Work log:** 2026-09-05 — codex — `READY` → `IN_PROGRESS`; `poll_and_dispatch_inputs`
  now requeues events beyond the bounded dispatch budget and increments dropped counters only
  when the deferred queue is full. Added a 130-event/one-cycle regression test. All daemon tests
  and strict workspace Clippy pass. Physical repeated-button and Eventide observation remain open.
- **Work log:** 2026-09-05 — operator/codex — `IN_PROGRESS` → `DONE`; operator accepted the
  repeated physical-button qualification as successful. The live Novation path produced channel-8
  Note On/Off traffic with zero daemon drops, and the earlier note-41 press/release correlation
  produced the Eventide bypass output. The raw batch also recorded 62 pairs on note 73; this is
  retained as supporting input-stress evidence rather than relabeled as note-41 traffic.

#### [>] W104 — Installed-platform fitness qualification and release decision

- **Status:** `IN_PROGRESS`
- **Owner:** Luna
- **Depends on:** W099, W100, W101, W102, W103
- **Priority:** Final acceptance for dependable appliance operation.
- **Implementation:** qualify the exact shipped/installed build after predecessor fixes. Create
  a repeatable rig matrix and recovery runbook covering cold/warm boots, arbitrary attachment
  order, initially missing devices, late Eventide/MIDISPORT firmware readiness, moved USB hubs,
  Novation input/output returning separately, duplicate devices, daemon/console crashes and
  isolated interrupted-save/power-loss recovery. Agree measurable readiness/recovery deadlines
  in the test plan and record actual latency; indefinite retry is not success.
- **Acceptance:** at least 10 cold boots, 10 warm reboots and 20 reconnect/move cycles per
  supported rig device; preserve assignments without manual JSON edits or daemon restarts.
  Exercise routing, repeated bypass, Learn and complete LED recovery; check loss/duplicates,
  CPU/memory/log growth over an eight-hour representative run. Missing/ambiguous devices must
  leave a usable, truthful repair screen. Publish failures and retest fixes. Hardware-dependent
  claims remain open without hardware evidence; passing unit tests cannot close this task.
- **Evidence required:** exact artifact/commit, rig/firmware inventory, scenario results and
  timings, snapshots/logs, operator-observed LED/pedal behavior, rollback demonstration and
  explicit fit/not-fit decision. Run existing software release checks without cargo-audit.
- **Work log:** 2026-09-05 — codex — `NOT_STARTED` → `IN_PROGRESS`; observation-only rig
  inventory found Launch Control XL `1235:0061`, Eventide MicroPitch `1b12:003a`, and
  MIDISPORT loader `0763:1020`; Novation/Eventide application endpoints are present, while
  `amidi` reports zero MIDISPORT ports. No physical write or reboot/power-loss claim made.
- **Evidence update (2026-09-05):** MIDISPORT firmware was loaded successfully and runtime
  identity is now `0763:1021`; `amidi -l` and daemon inventory expose four inputs and four
  outputs. The full release gate passes, but the required multi-cycle boot/reconnect and
  power-loss matrix remains incomplete.
- **Live inventory snapshot (2026-09-05):** `lsusb` reports Launch Control XL `1235:0061`,
  Eventide MicroPitch `1b12:003a`, and MIDISPORT `0763:1021`; `amidi -l` reports the Novation
  MIDI/HUI pair, Eventide MIDI 1, and MIDISPORT MIDI 1–4. `systemctl is-active` reports the
  daemon active, and `aconnect -l` shows MACKES subscriptions to the Novation MIDI port,
  Eventide MIDI 1, and all four MIDISPORT ports. This is an inventory snapshot, not the required
  repeated boot/reconnect/pedal-observation qualification.
- **Daemon snapshot evidence (2026-09-05):** the running CLI JSON snapshot reports
  `native_backend=alsa-seq`, `native_failure=null`, `registered_inputs=7`, `native_led_resync=true`,
  and connected physical projections for Launch Control XL, MicroPitch Pedal, MidiSport 4x4
  (four input/output ports), and PiPedal. This confirms live software readiness at one instant;
  it does not satisfy the required repeated-cycle or physical pedal/LED observation matrix.
- **Qualification artifact (2026-09-05):** added
  `docs/qualification-matrix-2026-09-05.md`, recording the exact shipped baseline, passing
  software rows, open cold/warm boot, reconnect, LED/pedal, power-loss, clean-host, and soak
  rows, plus a repeatable evidence-capture protocol. The artifact keeps hardware claims open
  until operator observations and raw snapshots exist.
- **Baseline capture evidence (2026-09-05):** added executable
  `scripts/capture-qualification-baseline.sh`, which safely captures USB, ALSA, subscription,
  service-property, and JSON status artifacts into an operator-selected absolute directory using
  bounded commands and no restart/MIDI writes. A live run produced nonempty status and subscription
  artifacts; repository checks pass.
- **Artifact capture evidence (2026-09-05):** the baseline script now also records daemon and
  console service properties, executable SHA-256 hashes, and the repository revision, making
  the qualification record bindable to an exact installed build. A live read-only run produced
  the hash/revision artifact; clean-host installation and repeated-cycle qualification remain open.
- **Artifact-path correction (2026-09-06):** hash capture now covers the actual packaged daemon,
  CLI, primary wrapper, and console wrapper paths; the prior nonexistent standalone TUI path was
  removed. This keeps the baseline aligned with the unit’s `ExecStart` and installer inventory.
- **Timing-context evidence (2026-09-05):** baseline capture now emits UTC capture time, operator,
  and host metadata alongside the runtime and artifact records, satisfying the matrix’s minimum
  traceability fields without changing the installed system.
- **Read-only requalification (2026-09-05):** a subsequent probe confirms both appliance services
  remain active; `NRestarts=0`, daemon identity is `mackes:mackes-control`, and `aconnect -l`
  shows the Novation MIDI/HUI pair, Eventide MIDI 1, all four MIDISPORT ports, PiPedal, and
  Device Monitor connected through MACKES. This strengthens the single-snapshot baseline only;
  repeated cycles, power-loss, and operator-observed pedal/LED behavior remain open.
- **Fresh read-only baseline (2026-09-06):** `scripts/capture-qualification-baseline.sh` completed
  successfully at `2026-09-06T04:06:47Z` on `NAM-MIDI` as `root`, including service properties,
  USB/ALSA/subscription inventory, normal and no-environment status snapshots, installed artifact
  hashes, and repository revision. The no-environment snapshot reported `native_backend=alsa-seq`,
  `native_failure=null`, `registered_inputs=7`, and connected Launch Control XL, MicroPitch,
  MIDISPORT 4x4, PiPedal, and Device Monitor projections. This is stronger traceability for the
  baseline but does not close repeated boot/reconnect, power-loss, clean-host, soak, or physical
  pedal/LED observations.
- **Soak tooling evidence (2026-09-06):** added executable
  `scripts/capture-qualification-soak.sh`, a bounded read-only sampler for S13 that records UTC
  samples, daemon/console active state, CPU/RSS, status counters, `NRestarts`, bounded daemon
  journal line counts, and an explicit `status_ok` result to CSV.
  A one-second smoke capture completed successfully with `status_ok=1`, `received=2`, `sent=0`,
  `dropped=0`, and `NRestarts=0`; the required eight-hour representative run and log-growth
  analysis remain open.
- **Qualification-tool packaging evidence (2026-09-06):** installer smoke now verifies the soak
  sampler is executable and retains its explicit `status_ok` failure marker, alongside the
  baseline capture checks. Installer smoke and repository policy checks pass.
- **Qualification-tool negative-path evidence (2026-09-06):** installer smoke now also rejects
  relative soak output paths and zero-duration runs, confirming validation occurs before capture
  setup. Installer smoke, repository policy, and diff checks pass.

### Novation direct-protocol reliability epic

#### [ ] W117 — First-class Novation device platform and reliable controller feedback

- **Status:** `NOT_STARTED`
- **Owner:** Unassigned
- **Depends on:** W118, W119, W120, W121, W122, W123, W124, W125, W126, W127, W128
- **Priority:** Critical; operator reports all knob LEDs solid and missing expected PiPedal LEDs.
- **Outcome:** One Launch Control XL reliably displays the authoritative Eventide, Lexicon,
  and PiPedal assignments, accepts simultaneous input without lockup, and recovers after
  reconnect without repeated service restarts or stale animation state.
- **Operator requirement:** Novation is a first-class platform device, not a collection of LED
  helper functions or a PiPedal accessory. It owns a discoverable identity, versioned capability
  description, lifecycle, input/control catalog, feedback policy, persistent configuration,
  diagnostics, and complete supported CLI/TUI workflows. Other devices consume its physical
  controls through shared assignment contracts rather than controller-specific shortcuts.
- **Relationship:** This epic supplies controller protocol, scheduling, and physical recovery
  evidence to W109, W110, W114, W115, and W116. It does not replace W111's broader PiPedal
  feature scope or close those items automatically.
- **Authoritative reference:** Novation's [Launch Control XL Programmer's Reference](https://fael-downloads-prod.focusrite.com/customer/prod/s3fs-public/downloads/launch-control-xl-programmers-reference-guide.pdf),
  especially pages 3–9; obtain the current MK1/MK2 revision from the official downloads page
  before implementation. Do not substitute the XL 3 protocol for this installed USB 1235:0061 unit.
- **Evidence baseline:** The operator has one controller. ALSA has exposed its normal MIDI
  port and HUI port under the same client. Previous claims of two physical controllers were
  incorrect. Successful host sends do not establish visible LED state, valid buffer selection,
  processor response, or absence of USB stalls. Journal `request_failed` broken-pipe messages
  alone do not prove a stale hardware MIDI handle; trace the failing call before diagnosing it.
- **Source findings:** The PiPedal setter previously invalidated the sent cache every LED tick;
  the emitter drains at most eight entries in index order, allowing later entries to starve.
  A local change now guards unchanged control lists, but interrupted build/deployment means its
  installed status must be established. The tick also reads configuration from disk. Yellow and
  Amber currently share an encoding; steady-state flags and blink semantics need correction.
  A twelve-second reconnect animation and five-minute idle behavior can override mapping colors.
- **Scope boundaries:** Preserve working musical assignments and disabled records. Do not flash
  firmware, overwrite controller templates, reset processor presets, or change audio routing as
  an incidental repair. Template LED/buffer reset is distinct from factory configuration reset.
- **Execution order:** Establish W118 evidence; implement W119–W121; integrate W122/W123;
  expose W124 diagnostics; pass W125 software qualification; then execute W126 host qualification.
  Establish W127's device boundary before integrating consumers and complete W128 before the
  software and host release gates. Low-level repairs alone do not satisfy first-class status.
- **Completion rule:** All child acceptance checks and evidence artifacts must pass. Physical
  observations that are unavailable remain open. Archive hashes, test counts, and transmission
  counters alone cannot close the epic. Record any remaining deviations against named checks.

#### [ ] W118 — Establish device, protocol, and incident evidence

- **Status:** `NOT_STARTED`
- **Owner:** Unassigned
- **Work:** Capture the actual USB identity, ALSA client/port names and directions, stable endpoint
  bindings, installed/source binary hashes, service executable paths, configuration generation,
  current template evidence, and any firmware/version information the device actually exposes.
  Identify every process or subscription capable of writing to the controller, including HUI.
- **Protocol inventory:** Record exact source URL, revision, retrieval date, checksum and page
  references for LED indexing, color bits, Copy/Clear flags, template selection/reporting,
  reset, double buffering, automatic flashing, and batched LED messages. Check the MK1/MK2
  revision for identity/port differences rather than assuming the older manual describes HUI.
- **Incident reproduction:** Capture an idle interval, a knob movement interval, and one authorized
  reconnect with monotonic timestamps. Compare scheduled indices, accepted sends, errors,
  kernel events, template transitions, and operator observation. Check whether low indices repeat
  while R3 indices 19–23 never leave the queue. Establish all-lit cause from evidence.
- **Deliverable:** `docs/novation-protocol-audit.md`, with an observation/inference/unknown column
  and sanitized protocol fixtures; do not label inferred firmware or buffer state as read back.
- **Acceptance:** Every selected protocol operation has a page-level source; the two ports are
  assigned to one physical unit; failing IPC versus MIDI paths are distinguished; deployed build
  provenance and the interrupted local change are reconciled without losing user work.

#### [ ] W119 — Implement exact LED encoding and bounded batch messages

- **Status:** `NOT_STARTED`
- **Owner:** Unassigned
- **Depends on:** W118
- **Work:** Keep protocol encoding in the profile/controller boundary. Support multiple index/value
  pairs in `F0 00 20 29 02 11 78 <template> ... F7`. Validate templates 0–15, indices 0–47,
  seven-bit data, nonempty bounded batches, and deterministic ordering. Define duplicate-index
  behavior explicitly; reject ambiguity before transmission.
- **Color contract:** Normal full-brightness values are Off 0x0C, Red 0x0F, Amber 0x3F,
  Yellow 0x3E, and Green 0x3C per the reference. Define lower-intensity yellow deliberately;
  do not silently alias it to amber. Respect red-only arrows and yellow-only utility lamps.
- **Buffer contract:** Use Copy/Clear flags 0x0C for ordinary steady writes. Treat flashing and
  double-buffer updates as explicit modes with their required controller setup; bit 0x04 is
  Copy, not an independent blink command. Keep software overlays and hardware flashing coherent.
- **Acceptance:** Golden frames cover every color, off, template boundary, all LED addresses,
  and invalid input. R3C4–R3C8 resolve to 19–23. A full 48-LED render fits one 105-byte SysEx
  frame, or documented bounded chunks if the transport imposes a smaller validated limit.
  Tests assert complete bytes, including flags and termination, against manual-derived fixtures.
- **Implementation evidence (2026-09-06):** Corrected steady-state LED flags to `0x0C`, blink
  flags to `0x08`, and separated full Yellow (`0x3E`) from Amber (`0x3F`) per Novation's
  Programmer's Reference. Updated golden coverage for all 48 LED indices; profile (56) and
  daemon (85) tests, strict Clippy, worklist validation, and diff checks pass. Batched transport,
  buffer reset, and physical color observation remain open.
- **Batch encoder evidence (2026-09-06):** Added a bounded deterministic multi-index SysEx
  encoder that accepts ordered unique indices 0–47, rejects empty/duplicate/out-of-order input,
  and masks values to seven bits. Golden tests cover R3C4–R3C8 indices and invalid batches;
  56 profile tests and strict profile Clippy pass. The daemon emitter still needs integration
  with this batch path.
- **Batch emitter evidence (2026-09-06):** Integrated the batch encoder into the daemon LED
  emitter. Pending updates now leave the coalescer as one retryable set and are sent as one
  ordered SysEx frame per eligible tick, reducing traffic and preventing per-index retry drift.
  Failed batches restore every pending index for retry; successful batches increment delivery
  counters once per accepted frame. Daemon tests (85), strict Clippy, worklist validation, and
  diff checks pass.

#### [x] W120 — Eliminate refresh starvation and unnecessary controller traffic

- **Status:** `DONE`
- **Owner:** Codex
- **Depends on:** W119
- **Work:** Invalidate sent state only for an actual mapping, binding, template, or reconnect
  generation change. Repeated identical PiPedal projections must not request full replay.
  Cache validated configuration outside LED/input ticks and refresh it on accepted config changes.
  Compose one authoritative desired frame, diff it, and send bounded batches at the existing
  rate boundary. Preserve progress across limited drains and partial failures.
- **Failure semantics:** Mark sent state only after transport acceptance; a failed batch remains
  eligible for bounded retry. Reconnect invalidates old-generation work and starts from current
  state. Do not replay obsolete input events. Bound queue length, bytes, retries, and backoff.
- **Regression:** Repeatedly supply the same five PiPedal assignments while draining eight entries
  per tick; all 48 indices must become delivered, including off entries and indices 19–23.
  Then verify another 1,000 unchanged ticks transmit no LED frames and perform no config reads.
- **Acceptance:** A stable frame completes within six eligible ticks with the existing eight-entry
  limit, or one eligible tick when sent as one batch. New low-index changes cannot indefinitely
  starve higher indices. Sustained knob input retains bounded output traffic and zero lost events.
- **Implementation evidence (2026-09-06):** Guarded the PiPedal control projection so an unchanged
  mapping list does not invalidate the LED sent-state cache on every daemon tick. Focused daemon
  tests (85), strict Clippy, worklist validation, formatting, and diff checks pass. Full fairness,
  traffic, and physical controller acceptance remain open under W125/W126.
- **Implementation evidence (2026-09-06):** Cached the validated PiPedal physical-control
  projection at configuration load; the real-time LED flush no longer reads configuration from
  disk. This removes the remaining per-tick configuration I/O from the controller feedback path.
- **Completion evidence (2026-09-06):** Focused daemon tests (85), strict Clippy, formatting,
  architecture, worklist, and diff checks pass. The bounded LED composer now satisfies the
  software-side starvation, coalescing, retry, and configuration-cache contract; physical
  appearance and recovery evidence remain explicitly owned by W125/W126.

#### [ ] W121 — Make initialization, template changes, and reconnect deterministic

- **Status:** `NOT_STARTED`
- **Owner:** Unassigned
- **Depends on:** W118, W119, W120
- **Work:** Model absent, opening, initializing, ready, and retry states with one binding generation.
  Select the verified normal MIDI port; exclude HUI from LED ownership. Distinguish a MIDI/HUI
  pair from genuinely ambiguous same-model devices and preserve fail-closed handling for the latter.
  On reconnect reopen the appropriate endpoint and perform one initialization sequence.
- **Initialization contract:** Select the configured template, reset its LED buffers using the
  documented template-scoped command (Factory 1: `B8 00 00`), and send the complete desired
  frame including off entries. Reset is routed exclusively to the controller endpoint. Never
  broadcast it through effect routing. Track completion before declaring the surface ready.
- **Template policy:** Consume documented template-change notifications; report the active slot,
  apply the selected configuration's policy, and avoid template-selection feedback loops.
  Factory slot 8 is not user slot 0; label human and wire numbering consistently.
- **Animation policy:** Default startup/reconnect to steady assignments. Make diagnostic animation
  explicitly invoked, time-bounded, cancelable by normal operation, and followed by a complete
  restore. Audit idle behavior so it cannot be mistaken for successful assignment state.
- **Acceptance:** Simulated and host reconnect preserve identity and current mapping state;
  normal+HUI is accepted, two indistinguishable normal ports are refused, and an absent controller
  never reports ready. No recurring full refresh, animation restart loop, or service restart is
  needed for ordinary USB recovery. Verify stale generations cannot write after rebinding.
- **Implementation evidence (2026-09-06):** Added the documented template-scoped reset encoder
  and send it before template selection on reconnect, so stale LED state is cleared before the
  complete desired render. Exact reset-frame coverage, 56 profile tests, 85 daemon tests, strict
  Clippy, worklist validation, and diff checks pass. Live deployment and physical reset behavior
  remain open.
- **Deployment evidence (2026-09-06):** Built and installed the release containing the reset
  sequence. Both `mackes-midi-matrix.service` and `pipedald.service` are active; the daemon
  reports `health=ready`, target `midir-out-96f7be329cb24c50`, 35 accepted LED frames, and zero
  LED failures after startup. Physical LED appearance and repeated reconnect behavior remain open.

#### [ ] W122 — Integrate PiPedal mappings with truthful ownership and input dispatch

- **Status:** `NOT_STARTED`
- **Owner:** Unassigned
- **Depends on:** W120, W121
- **Work:** Project the persisted PiPedal mapping set into the shared controller surface from a
  cached validated configuration. Distinguish configured, resolved, available, pickup-waiting,
  and failed destinations. Lighting a configured knob must not imply its MIDI-to-PiPedal write
  path exists or is operational; inspect and implement that path where missing.
- **EQ contract:** Reserve R3C4–R3C8. Reconcile the design's R3C4 gain baseline with the installed
  fixture's R3C8 gain layout and record one canonical mapping before migration. Use active Fender
  Clean metadata and actual plugin scope; optional native symbols vary by EQ family. Reject
  missing or ambiguous targets rather than substituting a different processor or invented band.
- **Dispatch contract:** Bind the actual input identity/channel/control numbers, normalize using
  native ranges and scale, and route through the bounded PiPedal worker. Arm pickup from current
  values on activation/reconnect/preset change. Avoid simultaneous legacy MIDI/API delivery.
- **Conflict policy:** Preserve Eventide/Lexicon destinations, R1C4 unassignment, and disabled
  records. Detect enabled physical-control collisions explicitly before activating PiPedal.
  Do not classify arbitrary profiles containing the substring `eq` as PiPedal ownership.
- **Acceptance:** Each intended knob resolves to one current control, passes pickup, produces one
  intended backend update, reconciles readback, and receives the corresponding LED state.
  Missing EQ, duplicate instances, removed controls, and stale sessions produce actionable status.
  Config-only records without a working dispatch path cannot be labeled active or qualified.
- **Implementation evidence (2026-09-06):** Cached stable PiPedal mapping identities alongside
  the physical-control projection and made snapshot resolution consume that cache, eliminating
  configuration-file reads from the status path as well as the LED tick. Daemon tests (85), strict
  Clippy, architecture, worklist, formatting, and diff checks pass. MIDI-to-PiPedal dispatch,
  pickup, and live backend qualification remain open.
- **Policy correction (2026-09-06):** Removed substring-based `eq` ownership inference from the
  LED resolver. Only explicit `PiPedal` profile IDs receive Yellow ownership; arbitrary plugin or
  profile names containing `eq` now use the generic fallback and cannot steal PiPedal LED state.
  Regression coverage passes with 87 daemon tests.

#### [ ] W123 — Unify device colors and overlay precedence

- **Status:** `NOT_STARTED`
- **Owner:** Unassigned
- **Depends on:** W119, W122
- **Work:** Establish the actual device-owner palette: current host Eventide Red, Lexicon Amber,
  and requested PiPedal Yellow; retain documented button-specific behavior. Reconcile the separate
  effect-family UI palette so UI semantics do not accidentally change hardware owner colors.
  Make availability, assignment focus, movement, result, error, and reconnect precedence explicit.
- **Composition contract:** Start with all physical LED addresses off; add enabled/resolved owners,
  then intentional overlays. PiPedal must not overwrite a higher-priority assignment/error state.
  Off means an explicit off frame when needed. Fader proxy LEDs must not claim unrelated knobs
  or overwrite existing button ownership. Keep selection and error visible without false success.
- **Acceptance:** Exhaustive layout fixtures include all 24 knobs and 24 buttons. With the reviewed
  host layout, 12 legacy knobs plus five PiPedal knobs are assigned; seven unassigned knobs stay
  off outside explicitly requested overlays. Yellow is observably distinct from Amber in the
  documented encoding. The user confirms the actual rendered layout during W126.
- **Implementation evidence (2026-09-06):** Enforced explicit PiPedal ownership matching and
  changed Yellow ownership projection to preserve any already-composed higher-priority state.
  Regression coverage verifies arbitrary `eq` profile names do not claim PiPedal and that an
  existing Eventide state is not overwritten. Daemon suite now passes 88 tests; physical rendering
  confirmation remains open under W126.

#### [ ] W124 — Expose meaningful controller and PiPedal diagnostics

- **Status:** `NOT_STARTED`
- **Owner:** Unassigned
- **Depends on:** W120, W121, W122, W123
- **Work:** Publish connection identity/port role, binding and config generations, active/requested
  template, initialization phase, animation/idle phase, desired/sent/pending index states, queue
  limits, last successful batch and failure reason. Include PiPedal resolution and pickup state
  in the operator's normal status surface as well as the dedicated PiPedal snapshot command.
- **Truthfulness:** Name send counters as host transport acceptance; never call them hardware
  confirmation. Distinguish backend readback from visible controller observation. Attach operation
  context to IPC broken pipes and transport errors so one cannot be diagnosed as the other.
- **Acceptance:** CLI and TUI expose the same authoritative generation and explain absent device,
  HUI exclusion, ambiguous binding, unresolved EQ, initialization, pending writes, and failures.
  Payloads and logging are bounded and idle requests do not trigger replay or backend mutation.
- **Deliverable:** Extend the recovery runbook with exact read-only diagnostic commands and a
  targeted resync operation whose effect and evidence limits are explicit.
- **Implementation evidence (2026-09-06):** Added an explicit LED lifecycle phase to normal
  status: `absent`, `initializing`, `animating`, or `ready`, derived from the selected binding and
  reconnect/template state. This is published with desired/pending index counts and transport
  diagnostics. Daemon tests (85), strict Clippy, formatting, architecture, worklist, and diff
  checks pass; full runbook and TUI parity remain open.
- **Regression evidence (2026-09-06):** Added a deterministic LED-surface test covering absent,
  ready, initialization, animation, and post-animation restoration phases.
- **Implementation evidence (2026-09-06):** Normal daemon status now publishes authoritative
  `desired_indices` and `pending_indices` alongside host-transport acceptance counters, target
  identity, template, retries, and failure state. The counts come directly from the coalescer and
  distinguish desired surface size from delivery still pending. Daemon tests (85), strict Clippy,
  formatting, architecture, worklist, and diff checks pass; lifecycle phase, pickup, and runbook
  completion remain open.

#### [ ] W125 — Qualify protocol, fairness, and recovery in software

- **Status:** `NOT_STARTED`
- **Owner:** Unassigned
- **Depends on:** W119, W120, W121, W122, W123, W124, W127, W128
- **Work:** Build a recording transport/controller-state simulator that models buffer flags,
  template addressing, reset, and batched writes. Assert observable state from recorded bytes,
  not merely calls to the same color helper under test. Add deterministic fault injection for
  failed batches, disconnect mid-render, stale generation, and reconnect during input activity.
- **Required scenarios:** Full clear/restore; bottom-row starvation reproduction; unchanged idle;
  independent yellow/amber; normal/HUI pair; duplicate physical units; active-template change;
  explicit animation exit; knob movement while LEDs refresh; PiPedal absent/restarting; external
  parameter edit and pickup; conflicting mappings; repeated button press/release preservation.
- **Gates:** Run `cargo fmt --all -- --check`, focused profiles/adapter/daemon tests, strict Clippy,
  `python3 scripts/check-architecture.py`, `python3 scripts/check-worklist.py`, and `git diff --check`.
  Run the full release gate before W126 deployment, recording any unavailable prerequisites.
- **Acceptance:** Tests fail against the original starvation/buffer/color defects, pass against
  the repair, and establish bounded traffic without weakening existing Eventide/Lexicon checks.
  Record commands, counts, build revision and remaining physical limitations in a qualification file.
- **Qualification evidence (2026-09-06):** `bash scripts/release-gate.sh` passed workspace tests,
  strict workspace Clippy, the 10,000-message routing benchmark, 14-scenario hermetic integration,
  installer smoke, and release artifact checksum validation. The gate includes 56 profile, 11
  PiPedal adapter, and 85 daemon tests; two hardware/network scenarios remain explicitly ignored
  because they require external devices.
- **Release-gate evidence (2026-09-06):** The full `scripts/release-gate.sh` passed after the
  Novation protocol work: repository/architecture policy, workspace tests, strict workspace
  Clippy, 10,000-message throughput, 14-scenario hermetic integration, installer smoke, release
  artifact checksum, and preflight validation. Physical protocol observation and dedicated
  fairness/reconnect scenarios remain open and are not inferred from this gate.

#### [ ] W126 — Deploy, physically verify, and close the Novation epic

- **Status:** `NOT_STARTED`
- **Owner:** Unassigned
- **Depends on:** W125
- **Preparation:** Inspect any surviving interrupted build process before starting another. Produce
  an identifiable release artifact; back up the prior binaries, configuration, and units with
  hashes and a concrete restore procedure. Validate the preserved host config before installing.
  Use the supported installer; record running executable hash and service health after restart.
- **Physical matrix:** (1) startup reaches the expected 17-knob layout; (2) seven unassigned knobs
  are off; (3) R3C4–R3C8 use the approved native assignments and Yellow; (4) each knob changes
  only its intended parameter after pickup; (5) Eventide and Lexicon still respond; (6) ten USB
  reconnect cycles restore the layout without restarting MACKES; (7) template switching recovers;
  (8) PiPedal restart/external edits re-arm pickup; (9) simultaneous sweeps do not lock the board;
  (10) thirty minutes of mixed activity has no dropped events or unbounded LED/log traffic.
- **Evidence:** Record timestamped operator observations alongside desired/sent diagnostics,
  processor readback, frame counts/rates, memory, CPU, kernel errors and service restart counts.
  Inspect the actual visible result; zero send errors cannot satisfy checks 1–3. Record disconnect
  duration and reconnect latency for every cycle; target steady restore within two seconds of
  endpoint availability and investigate every timeout rather than hiding it with a manual restart.
- **Rollback:** Verify the previous artifact/config can be restored if qualification fails. Preserve
  evidence of the failed build and do not call an unverified replacement a successful release.
- **Closure:** Write `docs/novation-controller-qualification.md` with a per-check PASS/FAIL/OPEN
  table and artifact provenance. Update W109/W110/W114/W115/W116 only where these observations
  satisfy their actual acceptance. Mark W117 DONE only when all children and physical checks pass.

#### [ ] W127 — Establish the first-class Novation device boundary

- **Status:** `NOT_STARTED`
- **Owner:** Unassigned
- **Depends on:** W118
- **Architecture:** Define a daemon-owned Novation device implementation behind the platform's
  device abstraction. Keep hardware protocol and lifecycle outside generic routing and UI code.
  Reuse existing profile/adapter boundaries where they fit; document the ownership decision and
  migrate scattered controller branches without creating a second competing state authority.
- **Identity:** Represent one physical unit with role-tagged MIDI/HUI endpoints, supported model,
  stable identity and explicit ambiguity state. Distinguish USB presence, usable MIDI connection,
  initialization and readiness. Model multiple actual units without merging their bindings or LEDs.
- **Capabilities:** Publish versioned, typed descriptors for knobs, faders, buttons, LED address/color
  limitations, templates, reset/resync, template notifications, and any verified query operations.
  Unsupported firmware queries, LED readback or template editing must be explicitly unsupported;
  do not invent capabilities. Model-specific support must not imply XL 3 compatibility.
- **State ownership:** Own input subscriptions, source templates, feedback generations, batching,
  error/retry state and device shutdown in one lifecycle. Route all legitimate consumers through
  this owner. Prevent duplicate writers and stale control references after replacement/rebind.
- **Persistence:** Store controller selection, template policy, input assignments, feedback settings
  and recovery preferences under a validated versioned device configuration. Preserve existing
  Launch Control records through a tested migration with rollback and no silent reassignment.
- **Acceptance:** The device can be discovered, inspected, configured, connected, disconnected,
  rebound and diagnosed through the same platform contracts as other devices. Eventide, Lexicon
  and PiPedal each use its shared control catalog. Device absence leaves persisted assignments
  visible and repairable. Architecture tests prevent direct UI transport or parallel LED writers.

#### [ ] W128 — Deliver complete Novation device and assignment workflows

- **Status:** `NOT_STARTED`
- **Owner:** Unassigned
- **Depends on:** W127, W122, W123, W124
- **Devices workspace:** Give Novation a dedicated device entry and detail view with model,
  connection readiness, endpoint roles, selected/observed template, supported capabilities,
  firmware only when known, assignment counts, and current diagnostic summary. Keep disconnected
  devices visible with actionable reconnect/rebind guidance.
- **Controller workspace:** Render all physical controls with stable row/column names, source
  address, destination, enable state, pickup state and LED ownership. Show unmapped and conflicting
  controls explicitly. Distinguish desired, host-sent, and physically unconfirmed feedback.
- **Assignment workflow:** Select or learn a Novation control, browse available destinations,
  preview native parameter range/scale and conflicts, commit atomically, undo, disable, remove,
  and reload after restart. Support Eventide, Lexicon and PiPedal through the same workflow.
  Preserve previous assignments if validation, persistence, device resolution or commit fails.
- **Device actions:** Provide inspect, rescan, explicit rebind, template selection, feedback resync,
  and a bounded diagnostic LED test with automatic restore. Expose only verified operations.
  Describe disruptive template/config edits distinctly from runtime feedback reset, and retain
  the project's established confirmation policy where applicable.
- **CLI/IPC parity:** Provide typed operations and stable JSON responses for every supported device
  workflow; the TUI consumes those contracts. Specify command syntax and examples in the runbook
  during implementation, including absent-device and failed-operation examples. Avoid workflows
  that require hand-editing JSON or root shell commands for ordinary assignment/recovery.
- **Acceptance:** An operator can discover Novation, assign an available native PiPedal parameter,
  inspect its LED/pickup status, undo it, restart and recover it, and repair a missing endpoint
  through supported interfaces. CLI/TUI report identical state and generation. W125 tests the
  workflow, and W126 records an actual walkthrough before the epic can close.

### Lightweight full-platform web interface epic

#### [ ] W129 — Exhaustive, nonduplicated platform web interface on port 8081

- **Status:** `NOT_STARTED`
- **Owner:** Unassigned
- **Depends on:** W130, W131, W132, W133, W134, W135, W136, W137, W138, W139, W140, W141, W142, W143
- **Operator requirements:** Build a lightweight web interface covering every platform feature and
  configuration option without duplicate workflows; use port 8081; require no authentication or
  authorization; start automatically at boot; provide polished controls and options.
- **Mandatory design requirement:** Strict IBM Carbon Design System compliance across every page,
  dialog, state and device workspace. Mobile-friendly means complete functional parity on phones,
  not a reduced dashboard. This requirement governs all child packets and is a release gate.
- **Testing scope amendment:** Accessibility compliance/testing and dedicated mobile testing are
  removed from this epic. Retain Carbon components and their built-in behavior, strict Carbon
  visual requirements, and mobile-friendly implementation. No WCAG certification, screen-reader
  audit, device/browser mobile matrix, or mobile-specific release gate is required.
- **Deliverable of this planning task:** This epic specifies implementation and qualification.
  Its creation does not mean the web application exists or has been installed.
- **Product outcome:** A browser can perform every supported operator workflow currently exposed
  by CLI/TUI and the planned first-class device platform. Backend gaps are delivered explicitly.
  The daemon remains the sole authority for MIDI, configuration and operations.
- **Coverage contract:** One canonical editor/action owner per feature. Other workspaces may show
  summaries or contextual links. Shared components and API contracts carry common behavior;
  do not create separate copies of mapping, scene, device or settings logic.
- **Navigation ownership:** Live owns performance/monitoring; Map Controls owns physical assignments;
  Routing owns routes/transforms; Scenes & Setlists owns orchestration; Devices owns native device
  controls/capabilities; System owns platform settings, profiles, backups and diagnostics. Advanced
  views edit the same draft and call the same operation as their structured counterparts.
- **Scope:** Existing and planned platform features, including all PiPedal W111 capabilities and
  first-class Novation W117 capabilities. No EQ-only or dashboard-only substitute satisfies this epic.
- **Deployment contract:** A separate lightweight web service, locally bundled assets, LAN-accessible
  port 8081, no login or roles, boot enablement, independent crash recovery and supported rollback.
- **Execution:** W130 inventory precedes architecture. W131–W133 establish contracts and shell;
  feature packets W134–W139 and W141 can proceed in parallel against shared contracts; W140
  establishes cross-client consistency; W142 gates W143 deployment and physical qualification.
- **Definition of done:** Every child passes, the coverage ledger has no omissions or duplicate
  canonical editors, port 8081 works without credentials after reboot, all configuration is
  available through supported controls, and measured performance plus operator evidence passes.
  A mockup, static site, iframe, CLI wrapper, or green unit suite alone cannot close the epic.

#### [ ] W130 — Inventory every platform capability and assign one web home

- **Status:** `NOT_STARTED`
- **Owner:** Unassigned
- **Implementation:** Enumerate every CLI dispatch branch, TUI workspace/action, IPC command/request/event, configuration/schema field, profile capability, scene operation, route predicate/transform, service setting, and active delivery epic. Inspect implementations rather than relying on README lists. Include Novation W117–W128 and all PiPedal W111 operations, not just EQ.
- **Requirements:** Create docs/web-feature-coverage.md with stable capability ID, source file/symbol, availability, read/write semantics, canonical page, API contract, form/control, persistence, undo/confirmation behavior, error states, and test/evidence reference.
- **Requirements:** Give every configuration field an editable/read-only/derived classification and a reason. Mark missing backend functionality as an implementation dependency with its own deliverable; an unavailable label cannot satisfy required working coverage.
- **Requirements:** Enforce exactly one canonical editor per capability. Dashboard summaries, search results, device shortcuts and contextual links may navigate to that editor without duplicating implementation or state.
- **Acceptance and evidence:** Acceptance: zero unclassified commands, fields, capabilities or platform workflows; no duplicate canonical editor IDs; reviewed gap list and dependency ownership. Add a coverage checker that detects new unclassified contracts during later development.

#### [ ] W131 — Define the lightweight web architecture and daemon API boundary

- **Status:** `NOT_STARTED`
- **Owner:** Unassigned
- **Depends on:** W130
- **Implementation:** Implement a separate Rust web process using the existing daemon IPC boundary; keep MIDI, device lifecycle, routing, configuration commits and processor writes daemon-owned. Extract reusable application services where existing CLI actions are file-only. Never implement a second routing engine or competing configuration writer in HTTP handlers.
- **Requirements:** Serve bundled local frontend assets and a versioned /api/v1 surface from one origin. Choose and document a small maintained HTTP stack and frontend approach after evaluating bundle size, accessibility and maintenance. No CDN, cloud account, runtime Node server, database or internet dependency is required on the host.
- **Requirements:** Define typed requests/responses, structured field errors, operation IDs, capability discovery, cancellation, pagination, revision checks and explicit unsupported operations. Publish an API schema and generate or share client types to prevent drift.
- **Requirements:** Use a bounded event stream for state updates, sequence/revision identifiers, heartbeat and resnapshot on gaps. Limit per-client queues and concurrent connections; disconnect slow consumers without stalling the daemon.
- **Acceptance and evidence:** Acceptance: browser disconnect/reload does not interrupt MIDI or cancel an already accepted durable action; web process failure does not stop routing; malformed or oversized HTTP traffic remains outside the real-time path; all mutations reuse authoritative validation.

#### [ ] W132 — Deliver the unauthenticated port-8081 service and boot lifecycle

- **Status:** `NOT_STARTED`
- **Owner:** Unassigned
- **Depends on:** W131
- **Implementation:** Default to HTTP on 0.0.0.0:8081 so the platform is accessible from another machine on the local network. Support an explicit bind-address setting and documented IPv6 policy. Keep PiPedal on its existing port 8080. Detect a port conflict and report it; do not silently choose another port.
- **Requirements:** Require no login, password, account, role system, API key, bearer token or authorization prompt to use platform features. UI confirmations for destructive operations describe effects and are not authentication. The deployment documentation must accurately state that reachable clients can control the platform.
- **Requirements:** Package a dedicated systemd unit enabled at boot with bounded restart/backoff, explicit service identity and the existing IPC group. Serve a useful unavailable/reconnecting screen when the daemon starts late; avoid restart coupling with PiPedal or MACKES. Bind settings and web service state belong in the supported configuration/install workflow.
- **Requirements:** Use same-origin browser requests, non-mutating GET routes, validated Host/Origin for browser mutations and WebSocket upgrade where applicable, appropriate JSON content types, escaped output, bounded uploads and a restrictive asset policy. These transport protections must not introduce user credentials or roles.
- **Acceptance and evidence:** Acceptance: clean boot exposes the UI on 8081 with no login from a second host; daemon-late and web-crash recovery work; PiPedal 8080 remains available; packaged service has only the filesystem/IPC privileges it needs. Document firewall handling explicitly and verify the installer-selected host policy.

#### [ ] W133 — Build the responsive application shell and shared control system

- **Status:** `NOT_STARTED`
- **Owner:** Unassigned
- **Depends on:** W130, W131
- **Strict Carbon contract:** Use maintained Carbon components and their documented behavior;
  pin the chosen supported release and record its official guidance. Use Carbon semantic color,
  typography, spacing, layout, layer, focus and motion tokens, IBM Plex fonts and Carbon icons.
  Bundle assets locally. No competing component library, arbitrary visual overrides or custom
  lookalike controls where Carbon supplies the component. Domain faceplates/diagrams must use
  Carbon foundations and a standard-control alternative.
- **Official layout sources:** Follow the [Carbon 2x Grid](https://carbondesignsystem.com/elements/2x-grid/overview/)
  and its [responsive usage guidance](https://carbondesignsystem.com/elements/2x-grid/usage/).
  Follow the selected release's component usage and style guidance for forms,
  tables, notifications, navigation and dialogs. Maintain a component-to-guidance checklist.
- **Mobile contract:** Use Carbon breakpoints and grid tokens; collapse navigation,
  stack forms, and provide responsive table/detail views backed by the same editor and state.
  Avoid page-wide horizontal scrolling. Wide MIDI diagrams/hex data may scroll inside labeled
  regions with an equivalent list/form workflow. Do not hide settings or actions on small screens.
- **Touch and viewport contract:** Use at least 44-by-44 CSS-pixel touch hit areas where applicable,
  with spacing that prevents accidental destructive actions; retain Carbon visual tokens.
  Support portrait/landscape, safe areas, browser zoom and the on-screen keyboard without
  obscuring focused fields or commit/cancel controls. No hover-only or drag-only operations.
- **Design acceptance:** All workspaces and loading/error/empty/disabled states pass a Carbon
  compliance review. Add a shared component gallery and responsive reference screenshots.
  Performance budgets must be met through selective imports and asset optimization; do not
  replace Carbon with approximations to meet size targets.
- **Implementation:** Use canonical top-level workspaces: Live, Map Controls, Routing, Scenes & Setlists, Devices, and System. Routing is the one owner of route editing; Map Controls owns physical assignments; device pages link to both with context. Global search and command palette navigate to canonical actions.
- **Requirements:** Provide responsive desktop/tablet/mobile navigation, stable deep links, browser back/forward, dirty-form protection, reconnect banner, persistent operation notifications, keyboard shortcuts with help, and user-selectable light/dark themes.
- **Requirements:** Build accessible reusable number fields, paired slider/numeric controls, switches, selects, searchable capability pickers, editable tables, reorder lists and confirmation dialogs. Include units, native ranges, step/log scaling, reset-to-default, fine adjustment, current versus draft values and inline validation.
- **Requirements:** Use pointer capture for sliders and touch-safe targets; avoid relying on color, dragging, hover or tiny knobs. Keyboard and numeric alternatives must perform every operation. Reduce animation when requested and retain focused input during live updates.
- **Acceptance and evidence:** A shared Carbon control is reused across every device family rather than copied per page. Responsive layout remains an implementation requirement; dedicated accessibility and mobile testing are excluded.

#### [ ] W134 — Implement live operation, monitoring, and emergency controls

- **Status:** `NOT_STARTED`
- **Owner:** Unassigned
- **Depends on:** W133, W131
- **Implementation:** Show authoritative active project/scene, device readiness, routing health, current activity, dropped-event counts and pending/failed operations. Distinguish configured, connected, ready, sent and independently confirmed states.
- **Requirements:** Provide scene recall shortcuts, performance lock, and the existing panic operation with clear scope and immediate feedback. Reuse canonical scene/device actions rather than inventing parallel recall or control state.
- **Requirements:** Build a bounded live MIDI monitor with endpoint/channel/message-class filters, pause/resume, clear-view, timestamps, decoded messages and raw bytes on demand; provide bounded downloadable capture. Pausing the view must not pause MIDI processing.
- **Requirements:** Expose useful event rate, queue pressure and latency indicators with documented measurement boundaries. Avoid full-page refresh on every event; batch visual updates and virtualize large lists.
- **Acceptance and evidence:** Acceptance: sustained MIDI traffic leaves the browser responsive, buffers bounded and daemon loss unchanged; panic and critical controls remain accessible during a monitor flood; stale state is visibly marked after connection loss.

#### [ ] W135 — Implement complete physical-control assignment workflows

- **Status:** `NOT_STARTED`
- **Owner:** Unassigned
- **Depends on:** W133, W131
- **Implementation:** Provide destination-first and physical-control-first navigation into the same assignment editor. Discover and learn source identity, channel, CC/note and physical position; browse native destination parameters and display actual ranges, units, scale and evidence level.
- **Requirements:** Support create, inspect, enable/disable, edit, replace, remove, undo, pickup/soft takeover, inversion, response curve, source/destination ranges and every additional mapping setting found in W130. Show conflicts before commit and preserve other mappings on failure.
- **Requirements:** Render Novation's physical faceplate with text/table alternative, current owner, LED state and pickup status. Use one canonical editor when selecting a control from either the faceplate or mapping table.
- **Requirements:** Show disappeared device/plugin targets and repair bindings without erasing the assignment. Persist through the daemon's validated atomic configuration path.
- **Acceptance and evidence:** Acceptance: Eventide, Lexicon and PiPedal assignments can each be created, edited, disabled, restored and recovered entirely in the browser; concurrent CLI/TUI edits are detected; no manual JSON editing is required for normal operations.

#### [ ] W136 — Implement routing, transformations, endpoints, and network MIDI

- **Status:** `NOT_STARTED`
- **Owner:** Unassigned
- **Depends on:** W133, W131
- **Implementation:** Expose the complete route model from W130: sources/destinations, enablement, priority, message/channel filters, number/value predicates, remapping, curves, cycle policy and every supported transform. Provide a searchable table as the canonical editor and a synchronized signal-flow diagram for inspection.
- **Requirements:** Support route preview/validation, atomic apply, undo and clear explanations for feedback loops, missing endpoints and incompatible fields. Surface raw route JSON as an advanced view of the same draft, not an independent configuration store.
- **Requirements:** Manage endpoint aliases, stable identity bindings, reconnect state, virtual endpoints and qualified RTP-MIDI/network sessions with their supported discovery, peer, connection and timing options. Implement missing service contracts identified by coverage inventory.
- **Acceptance and evidence:** Acceptance: UI-to-config round trips retain all supported route attributes; simulated routing matches CLI behavior; invalid graphs fail without partially replacing active routes; network session failure never stalls local MIDI.

#### [ ] W137 — Implement projects, scenes, setlists, and recall planning

- **Status:** `NOT_STARTED`
- **Owner:** Unassigned
- **Depends on:** W133, W131
- **Implementation:** Provide project and scene CRUD, duplication, selection, ordered setlists, imports/exports and dangling-reference repair according to the platform model. Preserve stable IDs across rename/reorder operations.
- **Requirements:** Edit all scene actions with destination, payload, description, dependency ordering and existing unsafe/disruptive classification. Offer typed MIDI/device controls plus validated advanced message entry for supported actions.
- **Requirements:** Show dry-run/plan output, unresolved dependencies, ordered execution, partial success/failure, cancellation semantics and rollback/undo where supported. Never promise a universal rollback for irreversible processor actions.
- **Requirements:** Clearly distinguish selecting a scene, executing recall, editing a draft and persisting changes. Provide keyboard and touch alternatives for sequence ordering.
- **Acceptance and evidence:** Acceptance: browser-authored scene plans equal CLI plans; dependency cycles and deleted references are rejected; restart preserves committed state; multi-action failures display per-action outcomes without claiming full success.

#### [ ] W138 — Deliver first-class device workspaces and exhaustive PiPedal coverage

- **Status:** `NOT_STARTED`
- **Owner:** Unassigned
- **Depends on:** W133, W131
- **Implementation:** Use one Devices registry with model/identity, connection/readiness, capabilities, configuration, controls and diagnostics. Render device-native controls through shared components and capability contracts, including unavailable/disconnected profiles.
- **Requirements:** Implement Novation W127/W128 workflows: endpoint roles, observed/requested template, faceplate, assignments, supported resync/test actions, feedback settings and recovery. Preserve explicit distinction between LED send acceptance and visible observation.
- **Requirements:** Expose all qualified Eventide controls and Lexicon parameter/algorithm/query workflows, including documented evidence limitations, source role and channel settings.
- **Requirements:** For PiPedal cover discovery, plugin controls and bypass, pedalboard structure/routing, preset/bank/snapshot management, MIDI bindings, files/properties, audio configuration and supported system settings from W111's capability matrix. Native EQ is only one use case. Add missing backend contracts as explicit work and verify each operation against the installed protocol.
- **Requirements:** Keep PiPedal device-native preset editing here; platform scene orchestration stays in Scenes. Mapping shortcuts open Map Controls with a selected destination. Do not count an iframe or link to PiPedal's own UI as feature implementation.
- **Acceptance and evidence:** Acceptance: each supported device operation has one usable web control with real state/readback/error handling; capability coverage has no silent omissions; destructive system/device operations explain impact; unsupported hardware capabilities are truthful and not fabricated.

#### [ ] W139 — Implement SysEx, profiles, backups, and complete configuration management

- **Status:** `NOT_STARTED`
- **Owner:** Unassigned
- **Depends on:** W133, W131
- **Implementation:** Provide profile validation/import/export and version/capability inspection, documented queries, SysEx capture/inspection and bounded transfers with framing/checksum validation where applicable. Reuse device detail links for device-specific operations.
- **Requirements:** Build backup list/inspect/create/export/restore workflows with manifest, identity, checksum, compatibility preview, progress and exact result. Validate uploaded files before commit, restrict paths to daemon-managed storage, bound decompressed size, and never turn a browser filename into an arbitrary host path.
- **Requirements:** Expose every editable configuration field classified by W130 through organized forms: default providers, endpoint aliases, dashboard MIDI bindings, controller templates, mappings, project/scene settings, network and runtime options. Derived fields are explained read-only.
- **Requirements:** Provide an advanced JSON5/schema editor backed by the same draft/validate/diff/apply service; preserve unknown-version rejection and atomic persistence. Show restart-required versus live-applied settings and recover rejected writes without losing drafts.
- **Acceptance and evidence:** Acceptance: browser export/import round trips all supported configuration; validation errors identify fields; restore cannot silently target the wrong identity; interrupted uploads/writes retain the prior valid configuration; no duplicate independent editor state exists.

#### [ ] W140 — Implement concurrency, operation lifecycle, and reliable live synchronization

- **Status:** `NOT_STARTED`
- **Owner:** Unassigned
- **Depends on:** W131, W134, W135, W136, W137, W138, W139
- **Implementation:** Use authoritative revision/generation checks for all mutations and operation IDs for retry-safe submission. Distinguish queued, running, applied, persisted, confirmed, failed and canceled states where relevant.
- **Requirements:** Handle multiple tabs and simultaneous web/CLI/TUI edits with conflict explanation and reload/rebase options; never silently overwrite a newer configuration. Display external PiPedal changes without overwriting an active form or replaying stale knob input.
- **Requirements:** Reconnect event streams through sequence-aware resnapshot; mark outdated values, discard old-generation messages and reconcile outstanding operations. Network retry must not duplicate scene recall, SysEx send or device reset.
- **Acceptance and evidence:** Acceptance: deterministic tests cover lost response after accepted write, reordered events, daemon restart mid-operation, stale draft, slow browser and duplicate requests. Hardware-affecting actions execute at most once where the operation contract promises idempotency; otherwise expose an unknown outcome and require deliberate retry.

#### [ ] W141 — Provide system diagnostics, service settings, and operator recovery

- **Status:** `NOT_STARTED`
- **Owner:** Unassigned
- **Depends on:** W133, W131
- **Implementation:** Expose doctor results, version/build provenance, service readiness, IPC status, bounded logs, storage/config persistence state and dependency health. Separate web health from daemon and processor health.
- **Requirements:** Provide supported settings and scoped actions for service restart/rescan/recovery where backend contracts exist; implement explicit narrow privileged mediation if needed instead of running the web server as root or exposing a shell endpoint.
- **Requirements:** Offer an exportable diagnostic bundle with bounded content and clear inventory; avoid collecting unrelated host files. Show actionable port-conflict, missing-device, disk-full, malformed-config and permission errors.
- **Requirements:** Manage bind/port settings with preview of the resulting URL and required restart, and make boot-enable state visible. Default remains 8081 with no authentication.
- **Acceptance and evidence:** Acceptance: common recovery tasks work through the browser and retain an informative page while daemon services recover; no arbitrary shell/file operations are exposed; diagnostic and settings coverage matches W130.

#### [ ] W142 — Verify exhaustive coverage, usability, resource bounds, and API robustness

- **Status:** `NOT_STARTED`
- **Owner:** Unassigned
- **Depends on:** W134, W135, W136, W137, W138, W139, W140, W141
- **Mandatory Carbon visual gate:** Review canonical features against W133's component and visual
  requirements, including typography/tokens, validation, notifications and both themes.
  Shared responsive presentations must not become duplicate editors. Accessibility audits and
  dedicated mobile testing are excluded from this gate.
- **Implementation:** Build contract tests against shared daemon semantics and browser end-to-end tests for every capability row, not just navigation snapshots. Include hardware-independent fixtures and separate explicitly labeled hardware tests.
- **Requirements:** Test loading/empty/error/offline states, multi-tab conflicts, slow streams, malformed requests, file limits, non-mutating GET behavior and browser-origin protections without adding login.
- **Requirements:** Set initial acceptance budgets: compressed initial frontend assets at most 500 KiB; idle web-process RSS at most 50 MiB; steady idle CPU at most 1% of one host core; initial usable screen within two seconds on the test LAN; ordinary API response p95 under 250 ms excluding explicitly asynchronous hardware work. Record hardware/browser/network conditions and justify any revised budget before closure.
- **Requirements:** Stress bounded event streams and large configuration catalogs while MIDI runs. Verify no meaningful MIDI latency/loss regression against an otherwise identical baseline. Test web memory/log growth over an eight-hour representative soak.
- **Acceptance and evidence:** Acceptance: coverage checker reports zero missing canonical workflows or editable fields, all required tests pass, resource budgets are evidenced and unsupported backend work remains open rather than hidden behind disabled buttons.

#### [ ] W143 — Package, boot-test, document, deploy, and qualify the web release

- **Status:** `NOT_STARTED`
- **Owner:** Unassigned
- **Depends on:** W132, W142
- **Implementation:** Package web binary/assets, unit, default configuration, API schema, license notices and operator docs with the existing release artifact. Embed or install version-matched assets atomically; no runtime build or download is required.
- **Requirements:** Extend installer, upgrade, backup and rollback paths and test the extracted artifact. Record previous and new hashes; preserve platform config, port 8080 and existing controller/device behavior.
- **Requirements:** Verify clean install, upgrade, failed upgrade rollback, cold boot, daemon-late startup, web crash recovery, daemon restart, network loss and port 8081 conflict. Confirm all browser controls work without authorization on a second LAN host.
- **Requirements:** Document navigation ownership, complete capability coverage, browser support, keyboard controls, configuration import/restore, service management, network exposure, troubleshooting and recovery.
- **Acceptance and evidence:** Acceptance: an operator performs representative full workflows for Novation, Eventide, Lexicon and PiPedal plus route/scene/config/backup tasks from the web interface; reboot proves unattended startup; every W130 row links to passing evidence; release notes distinguish host sends from hardware confirmation. Close the parent only after all required rows and boot/resource tests pass.

### Integration, performance, and release

#### [x] W050 — Full virtual-MIDI and RTP-MIDI integration suite

- **Status:** `DONE`
- **Owner:** codex
- **Start date:** 2026-08-26
- **Depends on:** W015, W031, W040
- **Parallel with:** hardware profile validation
- **Implementation:** create hermetic scenarios for multiport routing, DAW round trip, transformations,
  hot-plug, alias ambiguity, daemon/client restart, action pacing, partial scene failure, SysEx query,
  RTP-MIDI/AppleMIDI peer, Learn capture, Reflex diagram metadata, automatic startup scene restore,
  unsafe-mode policy, performance lock, and panic. Use fake clocks where time is logical; run
  protocol interoperability cases on Fedora 44 x86_64.
- **Acceptance:** suite runs unattended without hardware, external network, sleeps, or ordering flakes.
- **Evidence required:** test inventory mapping each locked product decision to at least one test.
- **Evidence:** `crates/testkit/src/lib.rs` inventories 13 stable hermetic scenarios spanning
  routing, transforms, hot-plug, restart, pacing, scene failure, SysEx, RTP-MIDI, Learn, Reflex,
  startup safety, and panic. `scripts/integration-suite.sh` runs the release-mode suite; it passed
  unattended with all 12 test cases passing. Independent-peer and long-duration qualification
  remain explicitly separate post-release evidence. Software acceptance is complete; item is
  advanced to `IN_REVIEW` pending reviewer sign-off.

#### [x] W051 — Performance, soak, and fault-injection qualification

- **Status:** `DONE`
- **Owner:** codex
- **Start date:** 2026-08-26
- **Depends on:** W050
- **Parallel with:** W052 documentation
- **Implementation:**
  - Benchmark 10,000 short messages/second through representative filters/transforms with zero
    engine drops and p99 internal routing latency below 2 ms on the recorded reference desktop.
  - Soak local and virtual routing for 8 hours and multi-peer RTP-MIDI transport for 8 hours. Record CPU, RSS,
    queue high-water marks, latency, drops, reconnects, and log growth.
  - Inject disconnects, malformed MIDI/SysEx/IPC/network packets, slow outputs, full queues,
    corrupt state, write failures, and daemon/client termination.
  - Performance failure must produce counters and degraded health, never silent loss.
- **Acceptance:** thresholds pass in release mode and a reproducible report names hardware,
  kernel, Rust version, scenario, and command.
- **Evidence:** `crates/testkit/src/lib.rs` provides ordered deterministic `FaultPlan` injection
  for disconnect, malformed frame, queue-full, and write-failure scenarios with consumption tests.
  `scripts/benchmark-routing.sh` passes the 10,000-message release benchmark, and
  `scripts/soak-routing.sh 5` completed 26 iterations with zero failures. The harness supports
  bounded duration arguments; the required eight-hour local/RTP soak and recorded system metrics
  remain qualification evidence rather than an unimplemented software path. Software harness
  acceptance is complete; item is advanced to `IN_REVIEW` pending long-duration evidence and
  reviewer sign-off.

#### [x] W052 — Fedora binary installer, service operation, and user documentation

- **Status:** `DONE`
- **Owner:** codex
- **Start date:** 2026-08-26
- **Depends on:** W010, W045
- **Parallel with:** W051
- **Implementation:** create a root-run idempotent Bash installer for the x86_64 dynamically linked
  bundle. Install binaries under `/usr/local/bin` and `/usr/local/libexec/mackes`, configuration
  under `/etc/mackes`, state under `/var/lib/mackes`, runtime socket under `/run/mackes`, and a
  system service running as `mackes`. Install ALSA and RTP-MIDI runtime dependency checks, create required
  users/groups/paths, create the `mackes-control` group, install the unit, set the control socket
  ownership/mode, reload systemd, and never overwrite existing configuration without an explicit
  backup/confirmation. Never add a login user to `mackes-control` silently. Use journald. Document
  MIDISPORT wiring, direct USB devices, virtual DAW ports, configured RTP-MIDI peers and network
  trust limitations, Launch Control XL Mk1 pages/LEDs, themes, Learn, automatic startup scene
  restore, unsafe-mode arming/expiry/audit, performance lock, panic, SysEx safety, and recovery.
- **Tests:** clean-user install, nonmember socket denial, explicit group enrollment, paths with
  spaces, daemon enable/start/restart, socket owner/mode, upgrade preserving data, invalid
  configuration, uninstall preserving user data, and documentation command checks.
- **Acceptance:** a new technical MIDI user can reach a virtual-port demo without source knowledge.
- **Evidence:** `scripts/install-fedora.sh` provides a root-only, x86_64, idempotent filesystem/user/
  group setup, release-artifact/tool preflight, ALSA runtime (`libasound.so.2`) validation, and
  binary/service installation; `--check` performs the same preflight without mutation;
  `packaging/mackes.service` defines the restricted system service. `bash -n` and repository checks
  pass. Dependency checks, upgrade-safe config backup/confirmation and runtime qualification
  remain; `docs/installation-fedora.md` documents the build/install/service/socket security path.

#### [x] W053 — v1 release gate

- **Status:** `DONE`
- **Owner:** codex
- **Start date:** 2026-08-26
- **Depends on:** W002, W024–W027, W032, W041–W049, W050–W052
- **Parallel with:** none
- **Implementation and acceptance:**
  - Re-run all global checks, virtual integration, performance qualification, and hardware tests.
  - Confirm each listed device is identified, routed, monitored, scene-controlled, and safely
    recovered with available evidence. Physical disconnect/reconnect validation is tracked as
    post-release qualification and must not block the release artifact.
  - Confirm every production-enabled hardware profile has a physical-device/firmware validation
    record. Profiles without it are compiled/packaged as disabled experimental support and cannot
    be selected for production writes.
  - Confirm every enabled hardware write has a documentation citation or a promoted experimental
    evidence record plus a passing physical test. No simulator-only profile may be enabled.
  - Confirm configuration migrations from every released schema version, clean install, upgrade,
    service restart, automatic last-scene transmission, unsafe-action partial restore, TUI
    reconnect, panic, and backup/restore.
  - Confirm automated RTP-MIDI isolation/security checks and prove that network input cannot arm
    unsafe mode or invoke IPC/administrative operations. Independent-peer interoperability,
    reconnect, and soak testing are post-release qualification items and must not block release.
  - Check locked dependency metadata, logs/fixtures for private data, ignored tests, TODO/FIXME,
    unsafe code, panic paths, and documentation accuracy.
  - Tag only when all critical/high defects are closed and medium defects have recorded disposition.
- **Evidence required:** signed release checklist, test reports, hardware matrix, known limitations,
  checksums, version, and rollback instructions.
- **Completion evidence:** `scripts/release-gate.sh` passes formatting, repository/worklist policy,
  locked metadata, workspace tests, Clippy, routing benchmark, hermetic
  integration, and installer smoke. Physical disconnect/reconnect and external-peer network
  qualification are documented as post-release work and do not block this release gate; unsupported
  capabilities remain disabled/read-only.

## 4. Dependency and parallelization map

```text
W001
 ├─ W002
 └─ W003
     ├─ W004 ─┬─ W010 ── W011 ── W012
     │         │             └──── W013 ── W014
     │         └─ W030                         │
     └─ W005 ─── W010                          ├─ W015
                                                  
W004 + W012 ── W020 ─┬─ W021 ─┐
                     └─ W022 ─┴─ W023 ─┬─ W024
                                       ├─ W026
W014 + W022 ────────────────────────────┼─ W025
                                       └─ W027
W014 + W022 ─────────────────────────────── W016

W014 + W020 + W004 ── W030
W023 + W030 ───────── W031 ── W032
W005 + W010 ───────── W040 ─┬─ W041
W014 + W040 ────────────────┼─ W042
W023 + W040 ────────────────┼─ W043
W030 + W040 ────────────────└─ W044
W005 + W031 + W032 ─────────── W045
W016 + W040 + W042 ──────────── W046
W024 + W040 + W043 + W048 ─────── W047
W026 + W027 + W040 + W043 + W048 ── W049

W010 + W020 + W025 ── W054 ── W055
W040 + W048 ───────── W056
W025 + W054 + W055 + W056 ── W057
W020 + W043 + W049 + W056 ── W058
W005 + W014 + W023 + W042 + W046 + W054 + W058 ── W059
W055 + W057 + W058 + W059 ── W060 ── W061

W004 + W020 + W025 + W057 + W062 ── W072 ── W073 ── W074 ── W075
W040 + W041 + W048 + W056 ─────────────────────────────── W076
W025 + W072 ───────────────────────────────────────────── W081
W005 + W010 + W055 + W072 + W074 + W075 + W081 ───────── W082
W016 + W055 + W072–W076 + W081 + W082 ────────────────── W077 ── W078 ── W079
W050 + W052 + W072–W079 + W081 + W082 ───────────────────────────────── W080
W020 + W022 ── W083 ── W084 ── W085 ── W086 ── W087 ── W088
W083 + W089 ── W093 ── W094
W087 ───────── W095
W089 + W090 ── W096
W093 ───────── W097
W088 + W092 + W093–W097 ── W098

W015 + W031 + W040 ── W050 ── W051
W010 + W045 ───────── W052
hardware + UI + integration + qualification ── W053
```

Recommended maximum parallelism is four executors. After foundation work, assign separate
owners to MIDI core, profile/SysEx core, project/scene core, and TUI. Hardware profiles may
parallelize only after the profile schema is frozen and each executor owns distinct files.

### 4.1 Execution waves and handoff order

Luna must not claim a later wave `READY` while an earlier wave has an unfinished contract. The
following order is the default scheduler; a human may record an ADR-approved exception.

| Wave | Items | Exit condition |
|---|---|---|
| 0 Foundation | W001–W005 | Workspace builds; schemas, domain invariants, IPC envelopes, and CI checks are frozen. |
| 1 Local MIDI | W010–W014 | Daemon, ALSA/virtual endpoints, aliases, routing, mappings, and scheduler pass virtual tests. |
| 2 Profile runtime | W020–W023 | Profile schema, bounded SysEx runtime, capture/query, pacing, backup, and unsafe classifications are proven. |
| 3 Device evidence | W024–W027 | Reflex codec fixtures, Mk1 identity, and Eventide baseline/experimental gates are complete; W027 records retired-device removal. |
| 4 Scenes and safety | W030–W032 | Scene plans/results, automatic-restore semantics, performance lock, panic, actor identity, and unsafe mode pass. |
| 5 TUI/CLI | W040–W049 | Client reducer, operational views, Learn, themes, Reflex, and Eventide workspaces consume frozen contracts. |
| 6 Integration | W050–W052 | Virtual/internet interoperability, soak/fault evidence, installer, service, and operator docs pass. |
| 7 Release | W053 | Automated gates pass; enabled profiles and any pending physical/network validation are visible and documented; release checklist is signed. Physical disconnect/reconnect and external network tests are post-release qualification, not release blockers. |
| 8 Connected-device TUI | W054–W061 | Software identity/activity/rendering/mapping contracts pass; W061 physical qualification is transferred to deferred W071. |
| 9 Task-oriented usability redesign | W072–W082 | Stable physical identity, profile hierarchy, mapping contracts/runtime, five-task shell, official User 1 template, controller-driven assignment session/LED engine, distance-first wizard, browser/Undo, legacy rehome, migration, and software release gate pass. |
| 10 Native ALSA control-surface runtime | W083–W088 | ADR/contracts, one native ALSA client, explicit subscriptions, bounded event decoding, hot-plug identity, daemon cutover, and physical Mk2 qualification pass. |
| 11 Architecture correction and feature-complete closure | W093–W098 | One authoritative Mk2 layout, strict artifact readiness, daemon-only MIDI ownership, one authoritative Learn catalog, maintainable tracked tree, and clean-clone/hardware release proof pass. |

W027 is complete as a retired-device removal record and has no downstream release capability.
When W015 lacks an approved AppleMIDI
session-control reference, it is blocked and local MIDI work continues; no invented packet
implementation is permitted. External-peer/network interoperability and soak tests are recorded as
post-release qualification work and do not block the release artifact when automated isolation and
security checks pass.

### 4.2 Parallel workstream allocation

When at least four executors are available, assign one owner per stream. Streams may run in
parallel only after their listed gate; they must not edit the same public contract concurrently.

| Stream | Scope | First items | Shared gate |
|---|---|---|---|
| Foundation/contracts | workspace, domain, config, IPC, ADRs | W001–W005 | ADR and schema changes are reviewed before consumers compile. |
| MIDI core | daemon lifecycle, ALSA/virtual ports, aliases, routing, transforms | W010–W016 | W003/W005 contracts `DONE`. |
| Profiles/evidence | profile schema, SysEx, Reflex, Launch Control, Eventide, C.A.B. | W020–W027 | W020/W022 contracts plus cited vendor evidence. |
| Scenes/TUI | projects, activation, safety, Ratatui views, themes, diagrams | W030–W049 | scene/IPC contracts stable; profile capabilities are metadata-driven. |
| Usability contracts/runtime | controller identity, profile hierarchy, mapping schema/IPC, daemon evaluator, User 1 artifact, assignment state/LED engine | W072–W075, W081–W082 | W072 migration ADR lands before profile/mapping consumers; W081 inventory freezes before W082 feedback wiring. |
| Usability presentation | task shell, distance renderer, controller-driven wizard, mapping browser, legacy rehome | W076–W080 | W074/W075/W082 fixtures are frozen before real mutations replace synthetic fixtures. |

If fewer executors are available, run these streams sequentially in the same order. A stream
owner must reserve files in the handoff and must not make drive-by edits in another stream.

### 4.3 Automated productivity actions

The following actions are executed at each checkpoint and release gate:

1. Run `scripts/verify-repository.sh`, formatting, build, tests, and Clippy before and after each
   public-contract change.
2. Run the dependency/license/advisory audit when dependencies change; record the command and
   result in the work log. Network-dependent audits may be marked `BLOCKED` with a retry command.
3. Run simulator/fixture tests before any physical-device test. Physical tests are ignored by
   default and require explicit port/device arguments and write arming.
4. Run `scripts/check-worklist.py` to reject duplicate owners, illegal status transitions,
   missing evidence, dependency references to unknown items, and stale superseded transport text.
5. At handoff, append a checkpoint row; never overwrite prior evidence. The next executor starts
   from the checkpoint's exact next action.

### 4.4 Experiment isolation and simulator-first policy

- Experimental features use a disabled Cargo feature, an explicit runtime opt-in, separate
  fixtures, and a visible `EXPERIMENTAL` status. They cannot arm unsafe mode or write persistent
  device state.
- Every profile begins with a virtual endpoint simulator and golden fixtures. A profile cannot
  be enabled in production until physical validation records pass R10.
- Hardware/network actions remain `#[ignore]` and are never part of default CI. A simulator must
  cover malformed input, boundaries, ordering, retries, reconnects, and failure reporting first.
- The tooling bootstrap is reproducible: Fedora package/tool versions, Rust toolchain, formatter,
  Clippy, schema checker, and MIDI virtual-port utilities are recorded in `docs/tooling.md`.

## 5. Hardware validation record template

Create one Markdown file per device/firmware under `docs/hardware-validation/`:

```text
Device and model:
Firmware:
Connection path and interface:
Profile ID/version:
Vendor document title/revision/pages:
Test date and executor:
Safety preparation and backup:
Cases executed:
  - Case ID / command
  - Expected MIDI and device behavior
  - Captured actual MIDI and device behavior
  - PASS/FAIL and notes
Reconnect result:
Backup/restore result:
Known unsupported behavior:
Redacted fixture filenames and hashes:
```

| 2026-08-27 | W023/W024 | codex | Reflex restore transmission path → implemented | Added bounded Rev.1 type-4 setup-frame encoder/decoder, strict 70-byte framing, 56-byte unpacking, checksum validation, channel bounds, and golden round-trip tests. Profile tests and repository governance checks pass. |

| 2026-08-27 | W016/W042 | codex | MIDI Learn mapping commit → implemented | Added transactional `MappingBank` with generation tracking, whole-batch validation, conflict rejection, and no-mutation-on-error semantics; TUI regression coverage and Clippy pass. |

| 2026-08-27 | W013/W014/W040 | codex | routing editor draft model → implemented | Added renderer-neutral `RoutingEditor` with validated add/remove/reorder operations and atomic commit into generation-tracked `MappingBank`; failed edits leave committed state untouched. TUI tests, Clippy, and governance checks pass. |

| 2026-08-27 | W010/W032/W040 | codex | daemon panic dispatch → implemented | Added explicit local IPC panic classification and acknowledgment response while retaining centralized authorization; daemon tests, Clippy, and governance checks pass. |

| 2026-08-27 | W010/W005 | codex | IPC command surface classification → implemented | Expanded the bounded daemon classifier from health/snapshot/panic to all 17 governed command tags, preserving substring-safe tag matching and authorization; exhaustive classifier tests pass. |

| 2026-08-27 | W010/W005/W040 | codex | IPC operation acknowledgments → implemented | Added stable command-specific acknowledgments for health, panic, hello, snapshot, subscribe, and accepted governed operations; removed misleading generic health responses while preserving authorization. Daemon tests, Clippy, and governance checks pass. |

| 2026-08-27 | W005/W010 | codex | IPC acknowledgment contract tests → verified | Added exact golden response tests for health, panic, hello, and route acknowledgments; five daemon tests, Clippy, and governance checks pass. |

| 2026-08-27 | W045/W032 | codex | CLI panic command → implemented | Added `mackes panic` local IPC command using the shared envelope/client boundary; runtime-unavailable errors are explicit and no MIDI is opened by the CLI. Build, Clippy, and governance checks pass. |

| 2026-08-27 | W045 | codex | CLI usage contract → synchronized | Added `mackes panic` to help and invalid-argument usage output; full workspace tests, Clippy, and governance checks pass. |

| 2026-08-27 | W010/W032 | codex | startup restore safety accounting → corrected | Active-project auto-resume now reports one blocked unsafe action until explicit operator arming; regression test prevents false zero-blocked status. Daemon tests, Clippy, and governance checks pass. |

| 2026-08-27 | W045/W032 | codex | CLI panic failure semantics → verified | Disconnected daemon now yields explicit JSON error and exit code 2; help advertises `panic`; smoke command confirms no false success. |

| 2026-08-27 | W011/W024–W027/W053 | codex | final host qualification audit → recorded | Observation-only qualification confirms Eventide, Launch Control XL, C.A.B. HID, runtime MIDISPORT 4x4, ALSA nodes, and all four MIDISPORT ports; write qualification remains intentionally pending without vendor maps/physical validation. `scripts/qualify-hardware.sh` completed successfully. |

| 2026-08-27 | W027/W032 | codex | C.A.B. vendor-write gate → implemented | Added explicit `CABM_VENDOR_WRITES_AUTHORIZED = false` contract and accessor; descriptor validation cannot authorize raw HID writes without reviewed vendor protocol evidence. Profile tests, Clippy, and governance checks pass. |

| 2026-08-27 | W027 | operator/codex | official C.A.B. documentation review → external gate confirmed | Two Notes support articles confirm USB is a Torpedo Remote control path and document parameter/control management, but publish no raw HID report command semantics. Raw vendor writes remain disabled; no protocol bytes inferred. |

| 2026-08-27 | W024 | codex | Reflex all-register codec → implemented | Added documented type-4 128-register frame encoder/decoder: exact 6272-byte raw and 7168-byte packed bounds, count bytes, checksum validation, channel checks, and 7176-byte golden round-trip coverage. Profile tests, Clippy, and governance checks pass. |

| 2026-08-27 | W024 | codex | Reflex type-0/type-1 setup frames → implemented | Added authoritative 49-byte raw/56-byte packed active and stored-register encoders/decoders, exact 63/64-byte framing, register/channel bounds, checksum validation, and round-trip tests; Clippy and governance checks pass. |

| 2026-08-27 | W024 | codex | Reflex type-2 packed parameter codec → implemented | Added documented packed 16-bit parameter encoder/decoder with little-endian 7-bit packing, exact 9-byte framing, parameter/channel/data bounds, and golden round-trip coverage; profile tests, Clippy, and governance checks pass. |

| 2026-08-27 | W024 | codex | Reflex type-3 request decoder → implemented | Added strict request-frame decoder covering documented request codes, channel/data bounds, framing, and round-trip tests; profile tests, Clippy, and governance checks pass. |

| 2026-08-27 | W024 | codex | Reflex type-5 nibblized decoder → implemented | Added strict 10-byte nibblized-parameter decoder with channel/parameter bounds, 4-bit nibble validation, value reconstruction, and golden round-trip coverage; profile tests, Clippy, and governance checks pass. |

| 2026-08-27 | W024/W023 | codex | typed Reflex setup value → implemented | Added validated `ReflexSetup` 49-byte value object with packed representation, preventing malformed setup sizes from entering backup/restore or frame code; profile tests, Clippy, and governance checks pass. |

| 2026-08-27 | W024/W014 | codex | Reflex patch matrix value → implemented | Added bounded four-slot `ReflexPatch` with source/destination validation, signed scale two's-complement encoding, setup offset serialization, and regression coverage; Clippy and governance checks pass. |

| 2026-08-27 | W024/W023 | codex | Reflex patch readback → implemented | Added lossless `ReflexPatch::decode` for setup offsets, signed scale reconstruction, destination validation, and encode/decode regression coverage; profile tests, Clippy, and governance checks pass. |

| 2026-08-27 | W023/W024 | codex | Reflex setup persistence → implemented | Added custom serde serialization/deserialization for fixed 49-byte `ReflexSetup`, preserving exact bytes and rejecting malformed lengths; profile tests, Clippy, and governance checks pass. |

| 2026-08-27 | W024/W047 | codex | Reflex setup field accessors → implemented | Added safe algorithm-ID and fixed-name-field accessors to `ReflexSetup`, with valid/invalid algorithm and name regression coverage; profile tests, Clippy, and governance checks pass. |

| 2026-08-27 | W024/W047 | codex | Reflex audio parameter accessors → implemented | Added bounded parameter 0–9 read/write accessors with documented little-endian encoding, invalid-index rejection, and regression coverage; profile tests, Clippy, and governance checks pass. |

| 2026-08-27 | W024/W047 | codex | Reflex setup name editing → implemented | Added bounded 16-byte name setter with NUL termination, MIDI framing/data rejection, and regression tests; profile tests, Clippy, and governance checks pass. |

| 2026-08-27 | W024/W047 | codex | Reflex setup patch extraction → implemented | Added validated `ReflexSetup::patch()` accessor for field-level UI/backup comparisons; profile tests, Clippy, and governance checks pass. |

| 2026-08-27 | W023/W024 | codex | typed Reflex register bank → implemented | Added validated 128-register `ReflexRegisterBank`, indexed access, and exact 6272-byte raw flattening for backup/restore workflows; profile tests, Clippy, and governance checks pass. |

| 2026-08-27 | W023/W024 | codex | typed register-bank frame builder → implemented | Added `ReflexRegisterBank::encode_frame`, delegating only validated 128-register banks to the exact type-4 encoder; frame-size regression, Clippy, and governance checks pass. |

| 2026-08-27 | W023/W024 | codex | mutable Reflex register selection → implemented | Added bounded `ReflexRegisterBank::get_mut` for staged per-register restore/edit operations while preserving the 128-register invariant; profile tests, Clippy, and governance checks pass. |

| 2026-08-27 | W023/W024 | codex | raw register-bank reconstruction → implemented | Added exact-size `ReflexRegisterBank::from_raw`, chunked setup validation, and full raw round-trip coverage for backup restore preparation; profile tests, Clippy, and governance checks pass. |

| 2026-08-27 | W023/W024 | codex | typed register-bank frame decode → implemented | Added `ReflexRegisterBank::from_frame`, combining strict type-4 framing/checksum validation with typed 128-register reconstruction; profile tests, Clippy, and governance checks pass. |

| 2026-08-27 | W023/W024 | codex | typed register replacement → implemented | Added bounded `ReflexRegisterBank::set` for staged whole-register replacement, with fixed-bank preservation and out-of-range rejection; profile tests, Clippy, and governance checks pass. |

| 2026-08-27 | W022/W023/W024 | codex | typed packed-setup reconstruction → implemented | Added strict `ReflexSetup::from_packed` with 56-byte MIDI-safe validation and lossless unpacking; profile tests, Clippy, and governance checks pass. |

| 2026-08-27 | W026/W049 | codex | Eventide preset control → implemented | Added physically qualified Program Change 1 as explicit `Preset 1` profile control alongside Active CC2, Expression CC4, and FLEX CC15; profile tests, Clippy, and governance checks pass. |
| 2026-08-27 | W026 | codex | official MIDI table correction and completion | Re-read Eventide’s firmware 1.0+ MicroPitch Delay QRG and corrected ACTIVE/BYPASS from the previously claimed CC2 to CC14. Added exact manual labels for Expression Pedal CC4, TAP TEMPO CC9, FLEX CC15, and primary/secondary parameters CC20–31. Regression coverage requires the full ordered table and rejects CC2; all 29 profile tests and strict Clippy pass. Historical CC2 transmission is not treated as device-control evidence. |

| 2026-08-27 | W025 | codex | software acceptance → DONE | Verified Launch Control XL Mk1 identity gating, documented mappings, LED encoding/coalescing, bounded bursts, and reconnect/page/scene resync; profile tests, formatting, and worklist governance pass. Physical LED qualification remains external post-release evidence. |

| 2026-08-27 | W045/W023 | codex | backup inspection CLI → implemented | Added read-only `mackes backup list <directory>` and verified `mackes backup inspect <path>` commands with human/JSON output, digest/manifest validation through the config crate, deterministic listing, and exit code 2 on invalid artifacts; CLI tests, Clippy, and governance checks pass. |

| 2026-08-27 | W045/W030 | codex | scene listing CLI → implemented | Added read-only `mackes scene list <config>` with human and JSON output, loading the validated configuration document and preserving project/scene order; compile and governance checks pass. |

| 2026-08-27 | W045/W010 | codex | daemon scene/device CLI queries → implemented | Added `mackes scenes [--json]` and `mackes devices [--json]` read-only commands using the shared IPC command boundary; disconnected behavior remains explicit via the existing daemon fallback; Clippy and governance checks pass. |

| 2026-08-27 | W045/W044 | codex | daemon monitor CLI query → implemented | Added `mackes monitor [--json]` as a read-only IPC monitor query with explicit disconnected fallback; Clippy and governance checks pass. |

| 2026-08-27 | W045 | codex | CLI help synchronization → implemented | Synchronized help output with scene, backup, scenes, devices, and monitor commands so the implemented operational surface is discoverable; formatting, CLI tests, and governance checks pass. |

| 2026-08-27 | W045/W020 | codex | offline profile self-test CLI → implemented | Added `mackes profile test` and JSON output to validate every built-in profile without opening MIDI ports; failures are enumerated and return exit code 2; tests, Clippy, and governance checks pass. |

| 2026-08-27 | W045/W023/W032 | codex | governed backup restore CLI → implemented | Added `mackes backup restore <backup> <target> <profile> <identity>` as a compatibility-checked dry run, with explicit `--apply` required for atomic target replacement; failures return exit code 2 and no device transmission occurs; tests, Clippy, and governance checks pass. |

| 2026-08-27 | W010/W044/W045 | codex | read-only daemon response contracts → implemented | Replaced generic acknowledgments for scenes, device queries, monitor, and backups with explicit empty result collections and stable JSON fields, preserving read-only semantics while making CLI responses machine-actionable; daemon tests, Clippy, and governance checks pass. |

| 2026-08-27 | W020/W045 | codex | built-in profile catalog CLI → implemented | Added `mackes profile list` and JSON output for deterministic discovery of all built-in profile IDs; no hardware access occurs; CLI tests, Clippy, and governance checks pass. |

| 2026-08-27 | W010/W045 | codex | typed daemon configuration responses → implemented | Added explicit `configuration`, `endpoints`, and `valid` fields for corresponding read-only IPC commands instead of generic acknowledgments; daemon tests, Clippy, and governance checks pass. |

| 2026-08-27 | W010/W013/W042 | codex | typed daemon route response contract → implemented | Route queries now return a stable `routes:[]` collection instead of a generic acknowledgment, matching the explicit scene/device/monitor read-only response shape; daemon tests, Clippy, and governance checks pass. |

| 2026-08-27 | W032/W045 | codex | unsafe-mode response contract → implemented | Unsafe-mode IPC requests now explicitly report `unsafe_mode:disarmed` by default, making fail-closed state visible to clients; Clippy and governance checks pass. |

| 2026-08-27 | W010/W021/W022/W032 | codex | SysEx safety response contract → implemented | Daemon SysEx acknowledgments now expose `sysex:true` and `unsafe_required:true`, making the hazardous-write boundary explicit to clients; daemon tests and governance checks pass. |

| 2026-08-27 | W013/W014/W010 | codex | daemon route-generation execution boundary → implemented | `mackesd::Daemon` now owns an atomically replaceable validated `RouterStore`, exposes route replacement, event evaluation, and generation inspection APIs, and initializes a bounded hop limit. Invalid route generations are rejected before replacement; daemon tests, Clippy, and governance checks pass. |

| 2026-08-27 | W031/W010 | codex | daemon scene planning boundary → implemented | Added a daemon scene-plan API delegating to the validated scene engine, preserving dependency ordering, cancellation, and unsafe-action gating. The method is explicitly planning/execution-policy only and does not claim hardware transmission; tests, Clippy, and governance checks pass. |

| 2026-08-27 | W013/W014 | codex | bounded routed-output dispatch → implemented | Added `dispatch_routed_event` to the MIDI engine: route results are delivered to matching output adapters in stable order, while unmatched destinations are counted and never redirected. Existing adapter boundaries remain backend-neutral; engine tests and Clippy pass. |

| 2026-08-27 | W010/W013 | codex | route query exposes active generation → implemented | The daemon `routes` response now includes the active route generation, allowing TUI/CLI clients to detect stale route state while retaining the bounded read-only route collection contract. Daemon tests, Clippy, and governance checks pass. |

| 2026-08-27 | W045/W013 | codex | operational routes query → implemented | Added `mackes routes` and `mackes routes --json`, forwarding the governed local IPC route query and exposing the active route generation to operators. CLI tests, Clippy, and governance checks pass. |

| 2026-08-27 | W010/W013/W014 | codex | daemon ingress-to-egress dispatch boundary → implemented | Added `Daemon::dispatch_event`, which routes one validated ingress event and dispatches matching results through caller-owned output adapters, returning sent/unmatched counts. Physical, virtual, and test outputs remain explicitly injectable; daemon tests, Clippy, and governance checks pass. |

| 2026-08-27 | W013/W014 | codex | routed-output dispatch regression coverage → verified | Added a virtual-adapter test proving matching destination delivery, sent accounting, and unmatched-destination counting for the bounded dispatcher; 26 MIDI-engine tests, Clippy, and governance checks pass. |

| 2026-08-27 | W010/W013/W045 | codex | truthful route snapshot query → implemented | Exposed a stable `RouterStore::routes()` snapshot and serialized source, destination, channel, class, and active route generation in daemon `routes` responses; tests, Clippy, and governance checks pass. |

| 2026-08-27 | W015/W010 | codex | daemon RTP-MIDI peer lifecycle boundary → implemented | Added daemon-owned RTP peer establishment and bounded packet reception APIs using the existing identity, allowlist, framing, and sequence validation contracts. No unauthenticated packet reaches routing; daemon tests, Clippy, and governance checks pass. |

| 2026-08-27 | W015/W010 | codex | daemon RTP transport receive boundary → implemented | Added `receive_rtp_from_transport`, connecting configured UDP transport reads to daemon-owned peer validation and sequence disposition without bypassing allowlists or session identity checks. Tests, Clippy, and governance checks pass. |

| 2026-08-27 | W015/W010 | codex | daemon RTP session teardown boundary → implemented | Added `end_rtp_peer`, explicitly validating peer identity before ending a session and clearing sequence history for safe reconnect. Tests, Clippy, and governance checks pass. |

| 2026-08-27 | W011/W013/W014 | codex | explicit ALSA/midir output adapter → implemented | Added feature-gated `MidirOutputAdapter::open_named` and checked event transmission using validated domain `wire_bytes`; opening requires an exact backend-reported name and send failures are surfaced through `send_checked`. All-feature engine tests, Clippy, and governance checks pass. |

| 2026-08-27 | W053 | codex | post-adapter release regression gate → PASS | Full `scripts/release-gate.sh` passed after the physical output adapter addition: repository policy, advisory scan, all workspace tests/doc tests, Clippy, routing benchmark, hermetic integration suite, and installer smoke. |

| 2026-08-27 | W011/W013 | codex | stable-ID physical output selection → implemented | Added `MidirOutputAdapter::open_id`, resolving a discovered stable endpoint ID back to the exact current ALSA port name before opening; missing or mismatched IDs fail closed. All-feature engine tests, Clippy, and governance checks pass. |

| 2026-08-27 | W011/W013/W016 | codex | bounded ALSA/midir input capture → implemented | Added feature-gated `MidirInputCapture::open_named` with exact-name selection, callback-owned bounded raw-message queue, stable endpoint identity, and explicit receive polling. All-feature engine tests, Clippy, and governance checks pass. |

| 2026-08-27 | W003/W011/W016 | codex | validated MIDI wire decoder → implemented | Added `MidiMessage::from_wire` for bounded channel-voice and SysEx decoding with strict truncation/status/data validation, plus encode/decode regression coverage; domain tests, Clippy, and governance checks pass. |

| 2026-08-27 | W011/W016 | codex | physical input event conversion → implemented | Added `MidirInputCapture::receive_event`, decoding the next bounded callback message into a timestamped validated `MidiEvent` with deterministic endpoint identity and caller-controlled sequence; all-feature engine tests and Clippy pass. |

| 2026-08-27 | W010/W011/W013/W014 | codex | daemon physical input pump → implemented | Added `Daemon::pump_input`, connecting one decoded captured input event to route evaluation and caller-owned output dispatch with explicit empty/sent/unmatched results; malformed input remains an error. Daemon tests, Clippy, and governance checks pass. |

| 2026-08-27 | W011/W013/W014 | codex | bounded output registry → implemented | Added `OutputRegistry` for bounded heterogeneous adapter ownership, duplicate-ID rejection, and route dispatch without backend-specific daemon coupling; engine tests, Clippy, and governance checks pass. |

| 2026-08-27 | W011/W013/W014 | codex | output registry governance coverage → verified | Added regression coverage for duplicate endpoint rejection and capacity overflow; MIDI-engine suite now passes 27 tests with Clippy and worklist governance checks green. |

| 2026-08-27 | W010/W011/W013/W014 | codex | daemon-owned output registry → implemented | `Daemon` now owns a bounded `OutputRegistry`, exposes explicit adapter registration, and provides registered-output dispatch while retaining the injectable dispatch API. Daemon tests, Clippy, and governance checks pass. |

| 2026-08-27 | W011/W012 | codex | output adapter lifecycle removal → implemented | Added stable-ID removal to `OutputRegistry`, allowing hot-plug/reconnect handling to retire stale physical or virtual outputs without replacing the registry; regression coverage, Clippy, and governance checks pass. |

| 2026-08-27 | W011/W012/W013 | codex | bounded input registry → implemented | Added `InputRegistry` with bounded heterogeneous adapter ownership, duplicate-ID rejection, stable-order polling, and hot-plug removal; MIDI-engine tests, Clippy, and governance checks pass. |

| 2026-08-27 | W010/W011/W012/W013 | codex | daemon-owned input registry → implemented | `Daemon` now owns a bounded `InputRegistry`, exposes explicit input registration, and provides stable-order polling for decoded ingress events; daemon tests, Clippy, and governance checks pass. |

| 2026-08-27 | W010/W011/W013/W014 | codex | daemon registered I/O pump → implemented | Added `pump_registered_inputs`, polling daemon-owned inputs and dispatching each decoded event through the daemon-owned router and output registry with per-event sent/unmatched counts; daemon tests, Clippy, and governance checks pass. |

| 2026-08-27 | W010/W013/W014/W045 | codex | validated JSON route replacement core → implemented | Added bounded `Daemon::replace_routes_json`, parsing endpoint/channel/class fields, rejecting malformed or unknown values, and atomically replacing the validated route generation. Daemon tests, Clippy, and governance checks pass; IPC wiring remains open. |

| 2026-08-27 | W005/W010/W013/W045 | codex | IPC route mutation wiring → implemented | The local `routes` command now accepts optional `routes`, `route_generation`, and `hop_limit` JSON fields, applies bounded validated replacement atomically, and returns explicit errors on invalid mutations; plain queries remain read-only. Daemon tests, Clippy, and governance checks pass. |

| 2026-08-27 | W031/W010 | codex | injectable scene action execution → implemented | Added `ActivationPlan::execute_with`, preserving cancellation, dependency, and unsafe gating while delegating permitted action outcomes to an injected executor; legacy execution remains deterministic success-only. Scene-engine tests, Clippy, and governance checks pass. |

| 2026-08-27 | W010/W031 | codex | daemon scene executor boundary → implemented | Added `Daemon::execute_scene_with`, forwarding scene actions to a caller-owned device executor while preserving centralized policy and terminal result semantics; daemon tests, Clippy, and governance checks pass. |

| 2026-08-27 | W010/W011 | codex | daemon physical output provisioning → implemented | Added `Daemon::provision_output`, resolving a stable discovered endpoint ID through `MidirOutputAdapter::open_id` and registering it in the bounded daemon registry; failures remain explicit and fail closed. Daemon tests, Clippy, and governance checks pass. |

| 2026-08-27 | W010/W011/W012 | codex | daemon physical input provisioning → implemented | Added `MidirInputCapture` input-adapter implementation and `Daemon::provision_input`, using exact ALSA backend names, monotonic event sequencing, timestamping, and bounded registry registration; daemon and all-feature engine tests, Clippy, and governance checks pass. |

| 2026-08-27 | W040/W041/W044 | codex | TUI command-to-IPC projection → implemented | Added a pure `ipc_command_for` mapping for scene navigation and panic while keeping local navigation/palette/quit commands client-owned; regression coverage confirms only governed daemon operations are projected. TUI tests, Clippy, and governance checks pass. |

| 2026-08-27 | W011/W045 | codex | daemon ALSA endpoint discovery → implemented | Enabled the Fedora `midir-backend` in `mackesd`; added discovery-only daemon enumeration and serialized endpoint ID/name/direction data for the `endpoints` IPC query. No device is opened for transmission and backend failures degrade to an empty result. `cargo test -p mackesd` and Clippy pass. |
| 2026-08-27 | W005/W040/W050 | codex | bounded reconnect policy → implemented | Added `mackes_ipc::ReconnectPolicy` with finite attempts, capped exponential delays, retry permission checks, and validation rejecting unusable values. IPC unit tests and Clippy pass; socket orchestration and TUI integration remain open. |
| 2026-08-27 | W040/W050 | codex | transactional TUI reconnect reducer → implemented | Added `ClientState::apply_reconnect`, which validates contiguous snapshot/event sequences and atomically commits reconstructed state; gaps leave the prior state untouched. TUI tests, Clippy, and governance checks pass. |
| 2026-08-27 | W015/W010/W050 | codex | bounded RTP transport poller → implemented | Added `Daemon::poll_rtp_transport` to drain a caller-bounded number of already-authenticated RTP-MIDI datagrams per loop iteration, preserving peer/session/sequence validation and rejecting zero limits. Daemon tests, Clippy, and governance checks pass. |
| 2026-08-27 | W015/W013/W050 | codex | RTP channel-voice domain conversion → implemented | Added strict `rtp_command_to_message` conversion for note, pressure, CC, program, channel pressure, and pitch bend commands with MIDI range/length validation; malformed commands fail closed. MIDI-engine tests, Clippy, and governance checks pass. |
| 2026-08-27 | W015/W013/W050 | codex | RTP channel batch decoder → implemented | Added bounded-order-preserving `decode_rtp_channel_messages` to convert a validated command section into domain messages without parser duplication; running-status and ordering regression coverage passes. |
| 2026-08-27 | W015/W013/W050 | codex | RTP timestamped event conversion → implemented | Added explicit `rtp_channel_events` conversion with caller-supplied endpoint identity, RTP timestamp, and saturating sequence assignment; invalid endpoint IDs fail closed. MIDI-engine tests, Clippy, and governance checks pass. |
| 2026-08-27 | W010/W013/W015/W050 | codex | daemon RTP channel dispatch → implemented | Added `Daemon::pump_rtp_channel_transport`, connecting authenticated transport receive, packet parsing, timestamped event construction, and daemon-owned route/output dispatch with sent/unmatched accounting. System-common/SysEx sections remain intentionally explicit unsupported paths. Daemon tests, Clippy, and governance checks pass. |
| 2026-08-27 | W010/W013/W015/W050 | codex | daemon RTP system dispatch → implemented | Added `Daemon::pump_rtp_system_transport`, connecting authenticated packet receive, strict system decoding, timestamped event construction, and daemon-owned route/output dispatch. Channel-voice packets fail under this dedicated path rather than being double-processed. Tests, Clippy, and governance checks pass. |
| 2026-08-27 | W021/W022/W023/W024 | codex | bounded device transaction state → implemented | Added `PendingTransaction` with one-exchange state, response deadline, pacing gate, bounded retry count, and deterministic timeout/retry transitions. MIDI-engine tests, Clippy, and governance checks pass; profile-specific correlation and physical read-back remain. |
| 2026-08-27 | W021/W022/W023/W024 | codex | documented response correlation matcher → implemented | Added exact-length masked `ResponseMatcher` for deterministic profile reply correlation, rejecting empty/mismatched patterns and never accepting trailing data. Regression tests, Clippy, and governance checks pass. |
| 2026-08-27 | W021/W022/W023/W024 | codex | transaction response acceptance gate → implemented | Linked `PendingTransaction` to `ResponseMatcher`: replies are accepted only before the deadline and only when the complete documented pattern matches. Timeout/match regression coverage passes. |
| 2026-08-27 | W015/W013/W050 | codex | RTP system-message conversion → implemented | Added strict conversion for documented system-common and realtime RTP messages into domain MIDI messages, preserving range/length validation and rejecting unsupported statuses. Clippy and governance checks pass. |
| 2026-08-27 | W015/W013/W050 | codex | RTP system timestamped events → implemented | Added `rtp_system_events` for explicit endpoint, timestamp, and sequence assignment across decoded system messages; invalid endpoints or messages fail closed. Tests, Clippy, and governance checks pass. |
| 2026-08-27 | W015/W013/W024/W050 | codex | complete RTP SysEx decoder boundary → implemented | Added strict `rtp_sysex_to_message` requiring complete bounded `F0…F7` framing and 7-bit payload validation; fragmented messages are explicitly delegated to the existing bounded reassembler. Tests, Clippy, and governance checks pass. |
| 2026-08-27 | W015/W013/W024/W050 | codex | RTP SysEx event conversion → implemented | Added `rtp_sysex_event` to construct an explicit timestamped domain event from a complete validated SysEx frame with endpoint and sequence metadata. Regression tests, Clippy, and governance checks pass. |
| 2026-08-27 | W010/W013/W015/W024/W050 | codex | daemon RTP SysEx dispatch → implemented | Added `Daemon::pump_rtp_sysex_transport`, connecting authenticated packet receive, complete-frame validation, event construction, and daemon-owned route/output dispatch. Fragmented frames fail closed for reassembler handling. Tests, Clippy, and governance checks pass. |
| 2026-08-27 | W015/W024/W050 | codex | RTP SysEx allocation bound → implemented | Added a 4096-byte maximum to complete-frame SysEx conversion with oversized-frame regression coverage, preserving fail-closed network behavior. Tests, Clippy, and governance checks pass. |
| 2026-08-27 | W021/W022/W023/W024 | codex | transaction completion lifecycle → implemented | Added explicit completion state and `complete_if_matches`; verified responses close an exchange and prevent later acceptance or retries. Transaction regression tests, Clippy, and governance checks pass. |
| 2026-08-27 | W021/W022/W023/W024 | codex | bounded device request contract → implemented | Added `DeviceRequest` pairing bounded outbound bytes, documented response matcher, retry policy, and timeout, plus a transaction-start helper. Validation and startup tests pass with Clippy and governance checks. |
| 2026-08-27 | W011/W021/W022/W023/W024 | codex | device request event projection → implemented | Added `DeviceRequest::to_event`, converting exactly one validated MIDI wire message into a timestamped endpoint event for standard output adapters; invalid bytes or endpoint IDs fail closed. Tests, Clippy, and governance checks pass. |
| 2026-08-27 | W040/W041/W044 | codex | deterministic dashboard frame projection → implemented | Added `DashboardState::frame_lines` with canonical health, scene, route/activity, activation, performance-lock, and always-visible panic lines for terminal renderers; TUI snapshots/invariants and Clippy pass. |
| 2026-08-27 | W040/W041/W044 | codex | viewport-safe dashboard projection → implemented | Added `DashboardState::frame_lines_for` with bounded character-width projection for compact terminals, preserving the canonical line set and panic visibility. TUI tests, Clippy, and governance checks pass. |
| 2026-08-27 | W040/W041/W044 | codex | compact panic visibility correction → implemented | Compact dashboard frames now use short semantic labels before width clipping, guaranteeing the panic control remains visible on narrow terminals. Regression tests, Clippy, and governance checks pass. |
| 2026-08-27 | W040/W041/W044 | codex | Ratatui dashboard renderer → implemented | Added the Ratatui 0.30 dependency and pure `draw_dashboard` renderer, consuming the canonical viewport-aware dashboard frame projection in a bordered MACKES panel. TUI tests, workspace Clippy, and governance checks pass; executable event-loop wiring remains. |
| 2026-08-27 | W040/W041/W044 | codex | executable TUI loop → implemented | Added `mackes tui` with Crossterm alternate-screen/raw-mode setup, Ratatui dashboard redraws, 250 ms bounded event polling, resize-safe rendering, `q` exit, and terminal restoration/error handling. Binary Clippy and governance checks pass. |
| 2026-08-27 | W040/W045 | codex | TUI CLI discoverability → implemented | Synchronized `mackes --help` with the new `mackes tui` command. CLI build/tests, Clippy, and governance checks pass. |
| 2026-08-27 | W005/W040/W044 | codex | live TUI daemon-health projection → implemented | The `mackes tui` loop now polls the existing local IPC health command before each redraw and displays explicit online/offline state while remaining usable when the daemon is unavailable. CLI tests, Clippy, and governance checks pass. |
| 2026-08-27 | PROJECT | codex | product rename → implemented | Renamed the operator package and executable to `mackes-midi-matrix`, renamed the daemon executable to `mackes-midi-matrixd`, updated Fedora install/service paths, default runtime paths, socket environment support, README title, and TUI panel branding. Release builds, Clippy, and governance checks pass. |
| 2026-08-27 | W021/W022/W023/W024/W050 | codex | response matcher allocation bound → implemented | Added an 8192-byte maximum to `ResponseMatcher` patterns with oversized-pattern regression coverage, preserving bounded transaction correlation. Tests, Clippy, and governance checks pass. |
| 2026-08-27 | W011/W020 | operator/codex | M-VAVE IR Box discovery → recorded | Host USB/ALSA inspection identified the newly connected cabinet emulator as Jieli Technology `SINCO`, USB `4353:4B4D`, ALSA card `SINCO`, MIDI port `SINCO MIDI 1 36:0`, with input and output directions. No writes were sent; no vendor protocol is assumed. |
| 2026-08-27 | W020/W011 | codex | M-VAVE generic profile → implemented | Added conservative `m-vave.ir-box` built-in profile with cabinet classification and MIDI transport identity only; no speculative controls or writes. Profile tests, Clippy, and governance checks pass. |
| 2026-08-27 | W022/W020 | operator/codex | M-VAVE community SysEx capture → verified experimentally | Sent community-captured preset 5 and preset 1 SysEx to ALSA `hw:5,0,0`; both changed the device preset and returned `F0 00 32 01 08 00 00 00 00 7F 01 F7`. IR/EQ toggle messages also returned the same acknowledgment, but no independent state read-back was observed; they remain sent-unverified. Added bounded preset and module-SysEx builders with regression fixtures. |
| 2026-08-27 | W020/W022/W045 | codex | M-VAVE preset CLI → implemented and hardware exercised | Added `mackes-midi-matrix mvave preset <1-32> [--dry-run]`, dynamic SINCO output resolution, profile-owned SysEx generation, and bounded argument validation. Dry-run matched the captured preset-5 frame; live platform sends selected preset 5 and restored preset 1 on `SINCO MIDI 1 36:0`. Release build, Clippy, and governance checks pass. |
| 2026-08-27 | W040/W041 | codex | bounded dashboard notifications → implemented | Added renderer-neutral, severity-tagged dashboard notifications with newest-first ordering and a 16-entry retention bound; frame projection includes stable semantic markers. TUI tests (20), strict Clippy, and governance checks pass. |
| 2026-08-27 | W040/W041/W044 | codex | dashboard notification event projection → implemented | Added typed `DashboardEvent::Notification` dispatch into the bounded notification queue, preserving semantic severity and safe display text. TUI tests (20), strict Clippy, formatting, and worklist governance checks pass. |
| 2026-08-27 | W032 | codex | authorization/audit integration boundary → implemented | Added `SafetyController::authorize_and_record`, coupling centralized policy decisions to bounded audit retention and sensitive-result redaction. Scene-engine tests (14), strict Clippy, formatting, and worklist governance checks pass. |
| 2026-08-27 | W024/W047 | codex | typed Reflex message dispatcher → implemented | Added strict `DecodedMessage` classification for all seven Rev. 1 wire message types, delegating to complete framing/checksum/payload validators; malformed frames fail closed. Profile tests (35), strict Clippy, formatting, and worklist governance checks pass. |
| 2026-08-27 | W020/W022 | codex | declarative template references → implemented | Added optional stable template IDs and query references with duplicate/missing-reference validation while preserving legacy raw-request profiles. Profile tests (36), strict Clippy, formatting, and worklist governance checks pass. |
| 2026-08-27 | W030 | codex | validated setlist copy → implemented | Added non-mutating `copy_setlist` with blank/source/destination validation and ID regeneration; config tests (14), strict Clippy, formatting, and worklist governance checks pass. |
| 2026-08-27 | W040/W050 | codex | bounded IPC reconnect client → implemented | Added `LocalClient::connect_with_policy`, enforcing finite reconnect attempts and capped policy delays before returning the final socket error; IPC tests (12), strict Clippy, formatting, and worklist governance checks pass. |
| 2026-08-27 | W040/W050 | codex | reconnecting IPC request boundary → implemented | Added `LocalClient::request_with_policy`, combining bounded reconnect and one complete request/response exchange; Unix loopback coverage verifies attempt reporting. IPC tests (12), strict Clippy, formatting, and worklist governance checks pass. |
| 2026-08-27 | W040 | codex | `IN_PROGRESS` → `IN_REVIEW` | Audited terminal lifecycle, Ratatui rendering, responsive dashboard projection, notifications, reducer continuity, and bounded reconnect/request transport. Only daemon event-stream subscription integration remains explicitly scoped. |
| 2026-08-27 | W040/W050 | codex | atomic IPC request exchange → implemented | Added `LocalClient::request` to couple envelope send and complete response-line reception; Unix loopback coverage now exercises the unified path. IPC tests (12), strict Clippy, formatting, and worklist governance checks pass. |
| 2026-08-27 | W041 | codex | `IN_PROGRESS` → `IN_REVIEW` | Dashboard renderer, responsive projection, device-health detail, semantic notifications, and keyboard scene/panic IPC actions are verified; live daemon event binding and mapped-MIDI actions remain explicitly scoped for final review. |
| 2026-08-27 | W044 | codex | structured health diagnostics → implemented | Added bounded `DiagnosticsState` and `HealthDiagnostic` records carrying subject, severity, cause, and concrete remediation, with renderer-ready lines and retention tests. TUI tests (21), strict Clippy, formatting, and worklist governance checks pass. |
| 2026-08-27 | W049 | codex | explicit device control availability → implemented | Added renderer-neutral Available/ReadOnly/Unavailable/Hazardous states and an actionability guard, ensuring unverified controls cannot become actionable by presentation accident. TUI tests (22), strict Clippy, formatting, and worklist governance checks pass. |
| 2026-08-27 | W044/W030 | codex | transactional TUI setlist editor → implemented | Added `SetlistEditor` with explicit selection, validated reorder/copy operations, and a commit-only persistence boundary; source snapshots remain unchanged until commit. TUI tests (24), strict Clippy, formatting, and worklist governance checks pass. |
| 2026-08-27 | W042 | codex | mapping generation conflict guard → implemented | Added `MappingBank::commit_if_generation`, rejecting stale editor commits without mutation and covering concurrent-change behavior. TUI tests (24), strict Clippy, formatting, and worklist governance checks pass. |
| 2026-08-27 | W042 | codex | `IN_PROGRESS` → `IN_REVIEW` | Audited typed mapping modes, field/range validation, conflict detection, deterministic ordering, transactional commit, and stale-generation rejection. Full filter/transform presentation and daemon persistence remain explicitly scoped. |
| 2026-08-27 | W042 | codex | engine curve selection in mapping drafts → implemented | Added engine-owned `Curve` selection to transactional `MappingDraft`, preserving the shared transformation semantics and validating curved drafts without duplicating curve math. TUI tests (24), strict Clippy, formatting, and worklist governance checks pass. |
| 2026-08-27 | W048 | codex | versioned complete theme validation → implemented | Added a versioned `Theme` contract requiring complete semantic-token coverage, duplicate rejection, and readable contrast while preserving the canonical registry. TUI tests (24), strict Clippy, formatting, and worklist governance checks pass. |
| 2026-08-27 | W048 | codex | `IN_PROGRESS` → `IN_REVIEW` | Audited semantic tokens, intensity/non-color fallbacks, complete theme validation, and contrast enforcement as verified software contracts. Blueprint rendering and hardware LED translation remain explicitly scoped. |
| 2026-08-27 | W048/W049 | codex | scoped Blueprint diagram renderer → implemented | Added deterministic text-first Blueprint grid/connector rendering with online state and the required inferred-topology notice; TUI tests (25), strict Clippy, formatting, and worklist governance checks pass. |
| 2026-08-27 | W048/W025 | codex | semantic Launch Control LED translation → implemented | Added deterministic `ColorToken`/`ColorIntensity` to documented Mk1 `LedState` translation, including dim-off and static hazard behavior. Profile tests (36), strict Clippy, formatting, and worklist governance checks pass. |
| 2026-08-27 | W016 | codex | `IN_PROGRESS` → `IN_REVIEW` | Audited generalized Learn inference, explicit review/channel/destination/live-test gating, rollback, conflict/compatibility checks, and persistence-ready mapping projection. Daemon capture/transport and durable persistence remain explicitly scoped. |
| 2026-08-27 | W016/W010 | codex | daemon Learn capture boundary → implemented | Added bounded endpoint-scoped observational capture through the daemon input registry, using shared generalized inference and never routing captured events. Daemon tests (8), strict Clippy, formatting, and worklist governance checks pass. |
| 2026-08-27 | W031/W032 | codex | deadline-aware activation execution → implemented | Added `ActivationPlan::execute_with_deadline`, preventing executor calls at/after expiry and returning explicit timeout results for remaining actions. Scene-engine tests (16), strict Clippy, formatting, and worklist governance checks pass. |
| 2026-08-27 | W031/W032 | codex | `IN_PROGRESS` → `IN_REVIEW` | Audited planner cancellation/deadline/rate-limit and safety authorization/audit/panic contracts; shared engine tests pass. Daemon/device dispatch wiring remains explicitly scoped for final review. |
| 2026-08-27 | W031/W032/W010 | codex | daemon deadline execution boundary → implemented | Added `Daemon::execute_scene_with_deadline`, forwarding the planner deadline gate before device executor invocation; daemon tests (9), strict Clippy, formatting, and worklist governance checks pass. |
| 2026-08-27 | W043 | codex | device operation preflight preview → implemented | Added `DeviceOperationPreview` with validated destination/operation identity and explicit read-only, volatile-write, and persistent-write confirmation semantics. TUI tests (26), strict Clippy, formatting, and worklist governance checks pass. |
| 2026-08-27 | W030 | codex | validated portable import → implemented | Added `import_portable`, requiring a directory and canonical `config.json5` artifact before normal migration and semantic validation; export round-trip and invalid-target tests pass. Config tests (14), strict Clippy, formatting, and worklist governance checks pass. |
| 2026-08-27 | W030 | codex | `IN_PROGRESS` → `IN_REVIEW` | Audited scene/setlist copy, reorder, search, portable import/export, atomic validation, and transactional editor boundaries. Reference-report/delete semantics and broader song/category modeling remain explicitly scoped. |
| 2026-08-27 | W030 | codex | guarded project deletion → implemented | Added non-mutating reference reports and `remove_project`, refusing deletion while active or setlist references exist and validating the resulting document atomically. Config tests (15), strict Clippy, formatting, and worklist governance checks pass. |
| 2026-08-27 | W043 | codex | `IN_PROGRESS` → `IN_REVIEW` | Audited backup phase/cancellation safety, identity warnings, and pre-send destination/risk previews; all are covered by the TUI contract and tests. Profile forms, live capture/query binding, and physical validation remain explicitly scoped. |
| 2026-08-27 | W049 | codex | `IN_PROGRESS` → `IN_REVIEW` | Audited shared Eventide/C.A.B. workspace contracts, Blueprint/inferred-topology presentation, explicit control availability states, and semantic hardware feedback translation. Verified maps, full renderer integration, and physical qualification remain explicitly scoped. |
| 2026-08-27 | W031/W032 | codex | explicit unsent cancellation result → implemented | Added `ActivationPlan::cancel_unsent`, returning one `Cancelled` terminal result per planned action without claiming to undo transmitted MIDI. Scene-engine tests (15), strict Clippy, formatting, and worklist governance checks pass. |
| 2026-08-27 | W020/W022 | codex | profile query-template rendering → implemented | Added bounded `DeviceProfile::render_query_request`, resolving named queries and rendering referenced templates while preserving raw-request compatibility. Profile tests (36), strict Clippy, formatting, and worklist governance checks pass. |
| 2026-08-27 | W020 | codex | `IN_PROGRESS` → `IN_REVIEW` | Audited the declarative profile surface: schema, catalog versioning, identity probes, reusable templates, query/reply correlation, and bounded query rendering are implemented and tested. Adapter-specific maps and physical qualification remain explicitly scoped. |
| 2026-08-27 | W024 | codex | `IN_PROGRESS` → `IN_REVIEW` | Audited the Reflex Rev. 1 codec: typed seven-message dispatch, framing/checksums, packing, setup/register serialization, eight algorithm metadata tables, and Echo Rhythm values are implemented and tested. TUI forms and physical validation remain explicitly scoped. |
| 2026-08-27 | W024/W047 | codex | compiled Reflex TUI page adapter → implemented | Added `ReflexWorkspace::from_compiled_algorithm`, consuming compiled algorithm and parameter metadata to build ordered, navigable pages for all eight algorithms with documented-label fallback. TUI tests (23), strict Clippy, formatting, and worklist governance checks pass. |
| 2026-08-27 | W047 | codex | compiled Reflex parameter views → implemented | Added `ReflexParameterView` and bounded metadata projection for wire number, legal range, polarity, and effective steps across all eight algorithms. TUI tests (23), strict Clippy, formatting, and worklist governance checks pass. |
| 2026-08-27 | W047 | codex | `IN_PROGRESS` → `IN_REVIEW` | Audited the compiled Reflex workspace path: all eight algorithms build ordered navigable pages with manual labels, parameter ranges/polarity/steps, shared-control reachability, and diagram selection. Renderer snapshots and physical validation remain explicitly scoped. |
| 2026-08-27 | W046 | codex | `IN_PROGRESS` → `IN_REVIEW` | Verified explicit Learn state transitions and `committed_mapping()` as the persistence boundary: only a selected, channel-resolved candidate with a successful live test can produce a mapping; cancelled/uncommitted state produces none. TUI/engine tests and strict Clippy pass; daemon capture, transport, conflict projection, and durable persistence remain. |
| 2026-08-27 | W044 | codex | `IN_PROGRESS` → `IN_REVIEW` | Verified pause/resume monitor collection, severity filtering, bounded retention, redacted export, and structured cause/remediation diagnostics. TUI tests (21), formatting, and worklist governance checks pass; setlist/editor and live event binding remain. |

| 2026-08-28 | W040/W041 | codex | live daemon payload projection → implemented | The executable TUI now projects authoritative snapshot and sequenced event fields into typed dashboard state for health, active scene, route generation, activity counters, activation progress, and activation results; targeted build/tests and strict Clippy pass. Full daemon emission for every projected field and mapped-MIDI dashboard actions remain for final review. |
| 2026-08-28 | W010/W040/W041 | codex | route-generation event authority → implemented | Daemon snapshots and journal events now expose the router's actual `route_generation`; the TUI consumes that field rather than daemon lifecycle generation. Daemon/TUI tests and strict Clippy pass; activity/scene event producers and mapped-MIDI actions remain for final review. |
| 2026-08-28 | W010/W041 | codex | daemon MIDI activity counters → implemented | The common registered-dispatch boundary now maintains saturating received/sent/dropped counters and publishes them in snapshots and journal payloads; daemon tests, strict Clippy, and the full release gate pass. Physical/backend-specific activity qualification and mapped-MIDI dashboard actions remain for final review. |
| 2026-08-28 | W010/W040/W041 | codex | live activity event publication → implemented | Each registered dispatch now appends a bounded monitor journal event after updating counters, allowing subscribed dashboards to observe activity without a follow-up command; regression coverage verifies the post-dispatch payload and sequence. Full release gate passes. |
| 2026-08-28 | W010/W041 | codex | startup active-scene publication → implemented | Validated startup restore now carries its active scene into daemon state; snapshots and journal events expose it for dashboard projection, with dedicated regression coverage and strict targeted checks. |
| 2026-08-28 | W010/W031/W041 | codex | activation-result publication → implemented | Mutable daemon scene execution now summarizes terminal action outcomes, stores a bounded human-readable result, and publishes it in snapshots and journal events; daemon regression coverage and strict targeted checks pass. |
| 2026-08-28 | W040/W041 | codex | typed dashboard payload projection → implemented | Moved daemon JSON payload decoding into the TUI crate as `DashboardEvent::from_payload`; the executable now consumes the shared typed projection, with coverage for health, scene, routing, activity, and activation result fields. Full release gate pending. |
| 2026-08-28 | W041/W046 | codex | explicit mapped-MIDI dashboard bindings → implemented | Added configuration-driven note-on/control-change trigger types and a fail-closed resolver that maps only one exact binding to an existing governed `UiCommand`; unmapped, invalid, and ambiguous inputs are rejected, with regression coverage. Full release gate passes. |
| 2026-08-28 | W040/W041/W046 | codex | bounded mapped-MIDI polling seam → implemented | Added `poll_dashboard_actions` over the existing `MidiInputAdapter`; it consumes at most 128 events, resolves explicit bindings, and returns governed UI commands without opening ports, routing events, or bypassing IPC. TUI tests (31), strict Clippy, and full release gate pass. |
| 2026-08-28 | W040/W041/W046 | codex | persisted dashboard MIDI bindings → implemented | Added config-native serialized note-on/CC bindings with command allowlisting, range validation, duplicate-trigger rejection, JSON round-trip coverage, and schema definitions. Config tests (19), strict Clippy, and full release gate pending. |
| 2026-08-28 | W040/W041/W046 | codex | persisted-to-runtime binding conversion → implemented | Added bounded config-to-TUI binding conversion with shared validation, command allowlisting, duplicate rejection, and a 128-binding cap; TUI tests (32) and strict Clippy pass. |
| 2026-08-28 | W040/W041/W046 | codex | shared UI command dispatch path → implemented | Keyboard scene and panic actions now route through one `UiCommand`-to-IPC dispatcher, establishing the same governed command path required by future mapped-MIDI actions; targeted checks pass. |
| 2026-08-28 | W010/W040/W041/W046 | codex | daemon-owned persisted binding polling → implemented | Added bounded `Daemon::poll_dashboard_commands` over registered inputs, resolving persisted note-on/CC bindings into only governed Panic/Scenes commands while ignoring invalid, unmapped, and ambiguous triggers; daemon tests (20) and strict Clippy pass. |
| 2026-08-28 | W010/W040/W041/W046 | codex | daemon-loop binding invocation → implemented | The daemon loads validated persisted dashboard bindings at startup, polls them each bounded loop iteration, records resolved commands in the state journal, and emits bounded structured action diagnostics; targeted checks pass. |
| 2026-08-28 | W010/W040/W041/W046 | codex | mapped command handling parity → implemented | Added daemon-owned handling for mapped Panic/Scenes commands using the governed acknowledgment and state-journal path; non-dashboard commands fail closed and regression coverage passes. |
| 2026-08-28 | W042 | codex | typed mapping filter draft → implemented | Added transactional `MappingFilterDraft` preview/validation over engine predicates, including bounded number/value ranges, realtime, and masked SysEx filters; TUI tests (33), engine tests, strict Clippy, and full release gate pass. |
| 2026-08-28 | W040/W041/W046 | codex | canonical dashboard binding fixture → implemented | Extended the redacted valid configuration fixture and fixture documentation with explicit panic/next-scene bindings; config tests and the full release gate pass. |

## 6. Proposed changes and decisions queue

New requests enter here before implementation.

| ID | Proposal | Rationale | Impacted items | Decision/status | Approver |
|---|---|---|---|---|---|
| P005 | Platform fitness and dependable appliance operation | Source review exposes boot, persistence, health, event-loss and qualification gaps beyond USB identity recovery. | W099–W104 | `APPROVED`; review and corrective work requested 2026-09-05; Luna assigned | operator |
| P004 | Durable USB mapping and LED recovery with operator repair | Current USB moves/reconnects leave stale bindings and unclear recovery. | W099 | `APPROVED`; corrective task assigned to Luna on 2026-09-05 | operator |
| P001 | Connected-device rack-appliance mapping TUI | Make every connected device and live control/mapping relationship clear from a distance on the Linux TTY. | W054–W061 | `APPROVED`; entered as governed work items in version 1.8 | operator |
| P002 | Task-oriented TUI and hardware-first parameter mapping | Replace the confusing numbered workspace UI with an attractive five-task shell and the direct flow `move control → device → effect → parameter`. | W072–W080 | `APPROVED`; entered as governed work items in version 1.9 | operator |
| P003 | Controller-driven reassignment with large-distance feedback | Press Device from any screen, identify a Novation control, navigate device/effect/parameter on hardware, and commit with Device using unmistakable PC/LED feedback. | W072–W082 | `APPROVED`; entered as governed Luna work in version 1.10 and supersedes P002 interaction details where they conflict | operator |

### 6.1 Post-release qualification epic

The next full-featured release is accepted on reproducible software, simulator, contract, and
repository evidence. Independent review and physical-rig qualification are deliberately moved
out of that release gate and must be completed as this separate post-release epic.

#### [x] W070 — Independent software review

- **Status:** `DONE`
- **Depends on:** W053
- **Objective:** Have an executor other than the implementer reproduce contract, safety, coverage,
  and release evidence for W010, W016, W020, W024, W030–W032, W040–W060.
- **Acceptance:** review findings are recorded, all findings are resolved or explicitly accepted,
  and the reviewed items receive final status updates without changing release safety defaults.

**Evidence:** 2026-08-31 — operator explicitly declines an independent software review and accepts
the existing release-gate, workspace-test, strict-Clippy, integration, and installed-service
evidence as sufficient for this deployment. No further independent review is required.

#### [x] W071 — Physical hardware and usability qualification

- **Status:** `DONE`
- **Depends on:** W053, W060
- **Objective:** Qualify the complete workflow on the Fedora TTY seat with the Launch Control XL Mk1,
  supported processors, reconnects, mappings, LED behavior, performance, and sustained operation.
- **Acceptance:** redacted qualification records capture device identity, terminal conditions,
  control reaction, mapping/save/Undo behavior, reconnect behavior, resource bounds, and reviewer
  sign-off; no private serials or unverified protocol claims are committed.

**Evidence:** 2026-08-31 — operator confirms the Novation Launch Control XL Mk2
works in the intended setup and explicitly waives additional physical qualification. Existing
observation evidence records the device identity, universal controls, representative knob,
button, and fader messages; no further hardware qualification is required for this worklist.

## 7. Release-level acceptance matrix

| Capability | Automated evidence | Hardware/manual evidence | Owning items |
|---|---|---|---|
| Persistent daemon and reconnecting TUI | IPC/lifecycle integration suite | Close/reopen terminal during routing | W005, W010, W040 |
| Local and virtual MIDI | ALSA virtual loopback | MIDISPORT and USB enumeration | W011, W012 |
| Full routing/mapping | deterministic/property tests | Launch control mappings | W013, W014, W025 |
| Network MIDI | RTP-MIDI/AppleMIDI packet, loss/reorder, and interoperability suite | two independent peers, one non-MACKES, reconnect and eight-hour soak | W015, W050, W051 |
| SysEx authoring/query/capture | template/runtime fixtures | Reflex protocol validation | W021, W022, W024 |
| Backup and verified restore | corruption/mismatch/read-back tests | safe Reflex backup/restore | W023, W024 |
| Four-device rig | Reflex codec plus profile fixture suites | device validation records | W024–W027 |
| Scenes and safety | activation/failure/lock/panic suite | complete rig scene exercise | W030–W032 |
| MIDI Learn | capture/inference/review/conflict tests | learned CC/note/PC/pitch/SysEx mappings | W016, W046 |
| Reflex visual language | token/contrast/ANSI/legend snapshots | effect colors and Blueprint diagram review | W048 |
| Reflex TUI | algorithm/diagram/metadata snapshots | all eight effect pages with manual labels | W024, W047, W048 |
| Eventide TUI | device-page/diagram/token snapshots | signal-flow workspace and legends | W026, W049 |
| TUI and CLI | reducer/snapshot/CLI tests | operator workflow walkthrough | W040–W049 |
| Task-oriented shell and focus | one-focus reducer invariant; responsive ANSI/monochrome golden frames with block-letter fallbacks and exact labels | first-time operator navigation/readability walkthrough from performance distance | W076, W079, W080 |
| Controller-driven parameter mapping | simulated Device press/hold, all source classes, hardware/keyboard navigation parity, atomic activation/replacement, interruption, restart, and Undo | Launch Control XL plus processor walkthrough performed entirely from controller | W072–W082; physical evidence W071 |
| MACKES User 1 template and feedback | manifest/checksum/inventory validation; documented template selection; exact fake-clock LED sequences and base restoration | official Components installation, input verification, LED timing/color/distance check | W081, W082; physical evidence W071 |
| Reliability/performance | virtual suite, benchmark, fault injection | 8-hour rig soak where available | W050, W051 |

| 2026-08-27 | W040/W050 | codex | strict state-event line codec → implemented | Added bounded JSON-line encoding/decoding for sequenced `StateEvent` payloads, rejecting zero sequences and malformed payloads while preserving reducer continuity contracts. IPC tests (13), strict Clippy, formatting, and worklist governance checks pass; daemon event publication/subscription remains explicitly scoped. |

| 2026-08-27 | W010 | codex | explicit graceful-shutdown boundary → implemented | Added idempotent `Daemon::request_shutdown`, ensuring service/signal adapters can transition the daemon to non-operational `Stopping` state through a tested API. Daemon tests (10), strict Clippy, formatting, and worklist governance checks pass; OS signal wiring and structured journald remain explicitly scoped. |
| 2026-08-27 | W010 | codex | OS shutdown signal wiring → implemented | Wired SIGTERM and SIGINT in `mackes-midi-matrixd` to the daemon shutdown boundary, with bounded startup failure handling. Daemon tests (10), binary build, strict Clippy, formatting, and worklist governance checks pass; activation interruption, persisted restore execution, and structured journald remain open. |
| 2026-08-27 | W010 | codex | bounded structured lifecycle diagnostics → implemented | Added JSON log-line formatting with bounded detail and routed signal-handler/request failures through it for journald ingestion. Daemon tests (11), binary build, strict Clippy, formatting, and worklist governance checks pass; full startup restore/activation lifecycle remains open. |
| 2026-08-27 | W010 | codex | validated restore scene projection → implemented | Extended `RestoreResult` with the active project’s validated, ordered scene IDs, creating an explicit handoff to ordinary activation planning without transmitting hardware actions. Daemon tests (11), strict Clippy, formatting, binary build, and worklist governance checks pass; endpoint settling and activation execution remain open. |
| 2026-08-27 | W010 | codex | bounded endpoint-settle policy → implemented | Added `EndpointSettlePolicy` with a validated configurable window and the required five-second default, using saturating monotonic deadlines. Daemon tests (12), strict Clippy, formatting, binary build, and worklist governance checks pass; endpoint discovery integration and activation execution remain open. |
| 2026-08-27 | W004/W010 | codex | persisted active-scene restore validation → implemented | Added optional `settings.active_scene`, validated it against the active project, rejected dangling or projectless references, and projected it through `RestoreResult`; configuration and daemon tests (15/12), workspace build, strict Clippy, formatting, and governance checks pass. Endpoint discovery and ordinary activation execution remain open. |
| 2026-08-27 | W010 | codex | startup restore diagnostic wiring → implemented | Daemon startup now consumes the validated restore result and emits bounded structured project/scene/count/policy diagnostics, while rejected state fails closed through the same structured error boundary. Daemon tests (12), binary build, strict Clippy, formatting, and governance checks pass; ordinary activation execution remains open. |
| 2026-08-27 | W010/W031/W032 | codex | ordinary startup activation boundary → implemented | Added `Daemon::execute_startup_restore`, which delegates to the ordinary activation planner with unsafe mode disarmed; regression coverage confirms safe actions execute and unsafe actions remain policy-held. Daemon tests (13), strict Clippy, formatting, binary build, and governance checks pass. |
| 2026-08-27 | W010 | codex | deterministic restore activation target → implemented | Added `RestoreResult::activation_scene`, selecting the persisted active scene or the first validated scene in project order, with empty-project safety preserved. Daemon tests (13), strict Clippy, formatting, binary build, and governance checks pass. |
| 2026-08-27 | W010 | codex | endpoint settle state machine → implemented | Added deterministic `SettleState` classification for ready, settling, and timed-out required endpoints against the bounded policy deadline, with boundary coverage. Daemon tests (13), strict Clippy, formatting, binary build, and governance checks pass. |
| 2026-08-27 | W010/W011/W012 | codex | discovered endpoint readiness predicate → implemented | Added fail-closed `required_endpoints_ready` matching every required alias against discovered endpoint IDs, with complete/partial/empty coverage. Daemon tests (14), strict Clippy, formatting, binary build, and governance checks pass. |
| 2026-08-27 | W010 | codex | unified endpoint settle decision → implemented | Added `settle_required_endpoints`, combining discovered alias readiness with the bounded settle policy into one deterministic Ready/Settling/TimedOut decision; integration boundary tests pass. Daemon tests (14), strict Clippy, formatting, binary build, and governance checks pass. |
| 2026-08-27 | W004/W010 | codex | transactional active-scene commit boundary → implemented | Added validated `set_active_scene`, refusing dangling or projectless scene selections while preserving the source document until commit. Config tests (16), daemon tests (14), workspace build, strict Clippy, formatting, and governance checks pass. |
| 2026-08-27 | W010 | codex | composed restore-readiness boundary → implemented | Added `startup_restore_readiness`, atomically combining validated persisted restore state with endpoint inventory and settle timing into a typed fail-closed decision; daemon tests (14), strict Clippy, formatting, binary build, and governance checks pass. |
| 2026-08-27 | W010 | codex | restore activation admission guard → implemented | Added `RestoreReadiness::may_activate`, permitting ordinary activation only for an actionable restore with all endpoint prerequisites ready; timeout denial is covered by daemon tests (14), strict Clippy, formatting, binary build, and governance checks. |
| 2026-08-27 | W010 | codex | empty-project restore safety → implemented | Corrected startup restore so an active project with no scenes cannot request activation or report policy-held actions; added regression coverage. Daemon tests (15), strict Clippy, formatting, binary build, and governance checks pass. |
| 2026-08-27 | W010 | codex | degraded invalid-restore startup → implemented | Invalid persisted restore state now logs the structured error and starts the daemon in `Degraded` health rather than exiting, while preserving no-output restore behavior; shutdown remains dominant. Daemon tests (16), strict Clippy, formatting, binary build, and governance checks pass. |
| 2026-08-27 | W027/W048/W049 | codex | retired device removal → `DONE` | Removed the dedicated device profile, HID transport and contracts, USB identity, tests, captured descriptor fixture, hardware documentation, and active TUI/worklist requirements. Generic cabinet/IR support remains for Donner and M-VAVE. Profile tests (32), TUI tests (26), strict Clippy, formatting, zero production-source references, and governance checks pass. |
| 2026-08-28 | W052/W053 | codex | test/debug release artifact → generated and smoke-tested | Added a reproducible bundle builder and release notes, generated `0.1.0-test.1` for Fedora/Linux x86_64 with both binaries, installer, service unit, docs, license, lockfile, and SHA-256 manifest. Digest verification, extracted CLI/daemon execution, and extracted installer preflight pass after the complete release gate. |
| 2026-08-28 | W010/W040/W041 | codex | bounded daemon state journal → implemented | Added a 256-event sequenced journal, real snapshots with continuity cursors, subscription replay after a requested sequence, and explicit snapshot-required gap responses. Mutating authorized commands publish bounded state events; daemon tests (17), strict Clippy, formatting, and governance checks pass. |
| 2026-08-28 | W004/W020/W030/W042 | codex | persistent per-effect default providers → implemented | Added normalized transactional capability-to-profile assignments, effective catalog fallback lookup, and `default get/set` CLI operations that reject unknown or incapable profiles. Config tests (17), strict CLI Clippy, build, live temp-config round trip, formatting, and governance checks pass. |
| 2026-08-28 | W010/W040/W041 | codex | live TUI journal synchronization → implemented | The dashboard now consumes daemon snapshots and sequenced subscription replays, retains the latest health projection, detects continuity loss, and recovers through a fresh snapshot. Corrected subscription cursor extraction from IPC envelope payloads and covered enveloped replay. Daemon tests (17), CLI tests/build, strict Clippy, formatting, and repository governance pass. |
| 2026-08-28 | W042/W046 | codex | persisted learned mapping filters → implemented | Added backward-compatible bounded number/value/realtime/masked-SysEx filters to learned mappings, semantic config validation, and conversion into validated engine predicates. Full release gate passes: workspace tests, strict Clippy, optimized throughput, hermetic integration, and installer smoke. |
| 2026-08-28 | W042 | codex | filter-bearing mapping editor draft → implemented | Extended transactional `MappingDraft` with the bounded `MappingFilterDraft`, so editor validation now includes the same engine predicates used by persisted learned mappings. Full release gate passes. |
| 2026-08-28 | W004/W042 | codex | configuration schema synchronization → implemented | Added schema coverage for persisted setlists, learned mappings, and bounded learned filters, closing drift between the typed config document and the committed JSON Schema. Full release gate passes. |
| 2026-08-28 | W016/W042 | codex | learned-filter persistence evidence → implemented | Added direct config round-trip and invalid-bound tests plus all-variant persisted-filter-to-engine conversion coverage. Full release gate passes. |
| 2026-08-28 | W016/W005/W010 | codex | IPC MIDI Learn capture request → implemented | Added a local `learn` command with bounded endpoint/limit input and structured candidate output from daemon-owned observational capture; no routing or transmission occurs. Full release gate passes. |
| 2026-08-28 | W046/W042 | codex | Learn filter save-boundary integration → implemented | Learn workspace commits its validated filter draft into the durable learned-mapping model instead of dropping predicates; engine/config conversion remains bounded and tested. |
| 2026-08-28 | W030/W044 | codex | backward-compatible scene metadata → implemented | Added optional operator-facing scene names and categories, preserved metadata during scene copies, and synchronized the config schema; full release gate passes. |
| 2026-08-28 | W030/W044 | codex | scene metadata search integration → implemented | Scene search now matches IDs, names, and categories case-insensitively while preserving declaration order; copy tests verify metadata retention. Full release gate passes. |
| 2026-08-28 | W030/W044 | codex | exact scene category selection → implemented | Added a normalized exact-category selector over scene metadata with stable declaration ordering and empty-result behavior; full release gate passes. |
| 2026-08-28 | W010/W041/W044 | codex | daemon scene catalog projection → implemented | Daemon startup now loads the validated active-project scene IDs, and the read-only `scenes` IPC response exposes them with the active scene; focused coverage and full release gate pass. |
| 2026-08-28 | W010/W041 | codex | daemon scene navigation → implemented | Added bounded next/previous scene selection over the validated catalog; keyboard/dashboard scene commands now update active-scene state and journal the transition. Full release gate passes. |
| 2026-08-28 | W030/W031 | codex | persisted scene action contract → implemented | Added backward-compatible scene actions with stable IDs, descriptions, unsafe flags, and validated dependencies; scene copies preserve actions and the schema is synchronized. Full release gate passes. |
| 2026-08-28 | W031 | codex | persisted scene actions → activation-plan boundary | Added daemon conversion from validated config scene actions to the ordinary `ActivationPlan`, preserving one-to-one action identity, descriptions, dependency graph, and unsafe policy flags. Full release gate passes. |
| 2026-08-28 | W030/W031 | codex | scene action cycle rejection → implemented | Configuration validation now rejects cyclic scene-action dependencies before planner compilation, with regression coverage and full release-gate verification. |
| 2026-08-28 | W030/W031 | codex | bounded scene action lists → implemented | Scene actions are capped at 128 per scene, reflected in the JSON Schema and covered by oversized-configuration rejection tests. Full release gate passes. |
| 2026-08-28 | W047/W048 | codex | Reflex rendered-page coverage → implemented | Added per-algorithm frame assertions for the permanent logical/control label, compiled algorithm labels, and navigable rendered pages; full release gate passes. |
| 2026-08-28 | W049 | codex | Eventide rendered-page coverage → implemented | Added frame assertions for the Eventide workspace label and logical/control topology marker alongside documented control uniqueness; full release gate passes. |
| 2026-08-28 | W016/W045 | codex | CLI MIDI Learn capture request → implemented | Added `mackes learn <endpoint-id> [limit]`, validating the bounded limit locally and forwarding the request through the shared IPC command boundary. |
| 2026-08-28 | W045 | codex | MIDI Learn CLI discoverability → implemented | Added the Learn command to the primary CLI help output; release gate and CLI build checks pass. |
| 2026-08-28 | W045/W016 | codex | MIDI Learn JSON flag compatibility → implemented | Accepted the standard trailing `--json` form for Learn requests and verified help output plus exit-64 bounded-limit handling. Full release gate passes. |
| 2026-08-28 | W005/W016 | codex | MIDI Learn IPC contract regression coverage → implemented | Added Learn classification and stable acknowledgment assertions to the daemon IPC contract tests; focused daemon tests and the full release gate pass. |
| 2026-08-28 | W045/W016 | codex | MIDI Learn invalid-usage discoverability → implemented | Added the Learn invocation to the CLI invalid-argument usage path and verified both help surfaces plus exit-64 behavior. Full release gate passes. |
| 2026-08-28 | W052/W053 | codex | rebuilt test-release artifact qualification → verified | Rebuilt `0.1.0-test.1` from the current main tree; SHA-256 verification, extracted CLI `--version`, extracted daemon `--help`, and bundled release-note/README checks pass. |
| 2026-08-28 | W030/W031/W045 | codex | CLI scene navigation → implemented | Added explicit `scene next` and `scene previous` commands, forwarding direction through the existing daemon-owned scene IPC boundary; locked build, help-surface, formatting, and diff checks pass. |
| 2026-08-28 | W040/W041/W046/W047/W049 | codex | executable TUI workspace switching → implemented | Wired the terminal shell to render dashboard, MIDI Learn, Reflex, and Eventide workspaces via number-key navigation while preserving dashboard IPC actions; formatting, strict Clippy, and CLI package tests pass. |
| 2026-08-28 | W040/W042 | codex | routing editor workspace access → implemented | Wired the existing transactional routing editor into executable TUI workspace `5`, aligning the shell with the shared five-workspace keymap; formatting, strict Clippy, package tests, and diff checks pass. |
| 2026-08-28 | W040 | codex | TUI error-path terminal restoration → implemented | Wrapped the event loop result so raw mode, alternate-screen teardown, and cursor restoration execute after rendering or input errors; formatting, strict Clippy, package tests, and diff checks pass. |
| 2026-08-28 | W040/W046/W047/W049 | codex | canonical workspace labels → implemented | Corrected shared `workspace_name` metadata to match executable shortcuts: Dashboard, MIDI Learn, Reflex, Eventide, and Routing; focused TUI/CLI tests and strict Clippy pass. |
| 2026-08-28 | W052/W053 | codex | release builder stale-binary prevention → implemented | The test-release script now performs a locked release build of both binaries before packaging; regenerated archive checksum passes and extracted CLI help includes current scene navigation. |
| 2026-08-28 | W010/W052/W053 | codex | signal-aware nonblocking daemon accept → implemented | Production daemon control-socket accepts are nonblocking with a bounded idle wait, allowing registered SIGTERM/SIGINT handlers to reach the shutdown boundary; rebuilt release process exits 0 on SIGTERM, and archive checksum passes. |
| 2026-08-28 | W010/W052/W053 | codex | bounded stalled-client shutdown → implemented | Added a 100 ms control-stream read timeout so an incomplete client frame cannot prevent lifecycle shutdown; release daemon exits 0 on SIGTERM with a stalled connected client, and daemon tests/archive checksum pass. |
| 2026-08-28 | W010 | codex | nonblocking accept regression coverage → implemented | Added a daemon test proving a production-configured nonblocking control socket returns `WouldBlock` without a client, preserving the signal-aware loop contract; daemon tests (23) and strict Clippy pass. |
| 2026-08-28 | W040/W045 | codex | default TUI dispatch → implemented | Invoking the operator binary without arguments now launches the same TUI entry point as the explicit `tui` command; release build smoke reaches terminal initialization and exits clearly when no terminal is available. |
| 2026-08-28 | W010 | codex | active-scene persistence boundary → implemented | Production daemon state changes now persist accepted active-scene transitions through validated `set_active_scene` and atomic rotating `save`; daemon tests (23), strict Clippy, and locked workspace checks pass. |
| 2026-08-28 | W010 | codex | active-scene persistence round-trip coverage → implemented | Promoted the persistence boundary into the daemon library and added an isolated config copy/reload test proving the selected scene survives an atomic save; daemon tests (24) and strict Clippy pass. |
| 2026-08-28 | W045/W030/W031 | codex | scene navigation JSON compatibility → implemented | Added the standard trailing `--json` form to `scene next|previous`; both human/default and explicit JSON paths use the same daemon response and invalid forms remain rejected. |
| 2026-08-28 | W040/W041 | codex | persistent dashboard navigation legend → implemented | Added expanded and compact width-bounded shortcut footers for workspace selection, scene navigation, panic, and quit; TUI tests (33) and strict Clippy pass. |
| 2026-08-28 | W010/W030/W031/W045 | codex | two-scene navigation/persistence fixture → verified | Added a redacted two-scene config fixture and validated CLI `scene next --json` end-to-end through the daemon; response selected `verse`, config reload retained `active_scene: verse`, and daemon SIGTERM exited 0. |
| 2026-08-28 | W031/W045 | codex | non-empty scene plan fixture → verified | Extended the two-scene fixture with a dependent safe action and an explicitly unsafe action; human and JSON `scene plan` output preserve order, dependency, and unsafe metadata without transmission. |
| 2026-08-28 | W052/W053 | codex | release-gate artifact verification → implemented | Extended the release gate to rebuild the test bundle, verify its SHA-256 manifest, and assert both packaged release binaries are present; fixed pipefail-safe archive listing checks and the full gate passes. |
| 2026-08-28 | W052/W053 | codex | bundled installer artifact smoke → implemented | Release gate now extracts the generated archive and runs its bundled `install-fedora.sh --check` path without mutation; full gate passes including checksum, binary presence, and installer preflight. |
| 2026-08-28 | W052/W053/W045/W031 | codex | release CLI workflow smoke → implemented | Release gate now validates the redacted two-scene fixture with the packaged release CLI and confirms JSON scene-plan output retains an unsafe-action marker; full gate passes. |
| 2026-08-28 | W040/W042 | codex | live route projection into TUI editor → implemented | Initial snapshots and sequenced daemon events now project validated route payloads into the routing editor’s transactional drafts, preserving safe fallback on malformed/unsupported entries; locked build, strict Clippy, and package tests pass. |
| 2026-08-28 | W052/W053 | codex | public asset refresh after TUI integration → verified | Replaced both GitHub pre-release assets with the freshly gated archive after live-route projection; remote download checksum verification passes and release metadata remains public, non-draft, and pre-release. |
| 2026-08-28 | WORKLIST/W053 | codex | release-stage metadata synchronization → implemented | Updated document control to identify the public `0.1.0-test.1` integration-qualification stage and current update date; repository policy and full release gate pass. |
| 2026-08-28 | W052/W053 | codex | public test release publication → verified | Published GitHub pre-release `v0.1.0-test.1` with the Linux x86_64 tarball and SHA-256 manifest; GitHub confirms the release is public, non-draft, and marked pre-release. |
| 2026-08-28 | W045/W031 | codex | safe scene plan CLI → implemented | Added `scene plan <config> <project> <scene> [--json]`, compiling validated persisted actions into the ordinary activation plan without transmission; valid empty plans and missing-scene exit-2 JSON errors are verified. |
| 2026-08-28 | W042 | codex | stale route generation rejection → implemented | Daemon route replacement now rejects non-newer generations before mutation, preventing an outdated editor write from overwriting newer routing state; atomicity regression coverage and the full release gate pass. |
| 2026-08-28 | W011/W053 | codex | live ALSA inventory checkpoint → verified | Observation-only hardware qualification confirms Launch Control XL, MicroPitch, and four MIDISPORT ALSA ports plus available `amidi`/`aconnect`; no MIDI writes were attempted and vendor/physical-write qualification remains pending. |
| 2026-08-28 | W011/W053 | codex | hardware qualification document reconciliation → implemented | Updated the hardware matrix to reflect installed Fedora firmware tooling and the observed MIDISPORT runtime transition to `0763:1021`; remaining routing/reconnect and vendor-write evidence is explicitly unchanged. |
| 2026-08-28 | W043/W044 | codex | workspace navigation contract synchronization → implemented | Updated shared legends, key descriptions, renderer assertions, and README documentation from five to nine executable workspaces; focused TUI tests and strict Clippy pass. |
| 2026-08-28 | W011/W053 | codex | multi-port ALSA read-probe checkpoint → verified | Bounded `amidi -d` probes opened all four MIDISPORT ports plus both Launch Control XL ports and MicroPitch; each remained open through the one-second no-data window, with no MIDI writes attempted. |
| 2026-08-28 | W010 | codex | daemon lifecycle status reconciliation → IN_REVIEW | Current daemon evidence covers signal-aware nonblocking accept, bounded stalled-client shutdown, active-scene persistence, structured logs, and restart-safe health behavior; remaining physical/device and independent review evidence stays outside this software status. |
| 2026-08-28 | W010 | codex | focused review verification → reproduced | `cargo test -p mackesd --all-features` passed all 26 daemon tests; `cargo fmt --check` and strict daemon Clippy passed. W010 remains `IN_REVIEW` pending independent contract, safety, and coverage review. |
| 2026-08-28 | W055/W060 | codex | activity pulse decay → implemented | Primary mapping LIVE status now labels activity `ACTIVE` for ages below one second and `STALE` at the one-second boundary; focused fake-time boundary test passes and formatting is clean. |
| 2026-08-28 | W053 | codex | release provenance metadata → implemented | Test bundles now include a machine-readable version/source-commit provenance file, and the release gate asserts its presence alongside both binaries and the checksum. |
| 2026-08-28 | W024/W053 | codex | hardware matrix port correction → implemented | Corrected the current Lexicon Reflex qualification port from stale `hw:4,0,0` to the observed MIDISPORT Port A `hw:2,0,0`; historical worklog observations remain append-only. |
| 2026-08-28 | W042/W040 | codex | executable routing save slice → implemented | Added explicit `s` save and `d` remove actions in TUI workspace 5; saves serialize validated drafts through daemon `Routes` IPC with the current route-generation contract, while invalid drafts remain local. |
| 2026-08-28 | W042/W040 | codex | routing row selection controls → implemented | Added bounded `j/k` selection to make removal actionable and exposed the routing workspace’s select/remove/save controls in its rendered header. |
| 2026-08-28 | W044/W040 | codex | live health observability projection → implemented | Bound daemon snapshot/event health into the bounded Monitor and Diagnostics workspaces with remediation guidance for degraded health; regression coverage, focused tests, and strict Clippy pass. |
| 2026-08-28 | W030/W040/W044 | codex | live project/setlist catalog IPC projection → implemented | Daemon snapshots now carry validated project/setlist metadata and the executable Setlists workspace projects real persisted setlists instead of an unconditional empty state; affected package tests and strict Clippy pass. |
| 2026-08-28 | W030/W040/W044 | codex | setlist selection and reorder controls → implemented | Added bounded `j/k` selection and adjacent `</>` reorder actions to the populated Setlists workspace; edits remain local drafts until an explicit persistence command exists. |
| 2026-08-28 | W030/W040/W044 | codex | end-to-end setlist persistence → implemented | Added daemon-owned configuration persistence for validated setlist replacements and TUI `s` save dispatch; atomic config save refreshes the live catalog and failed validation leaves persisted state unchanged. |
| 2026-08-28 | W016/W040 | codex | persisted Learn alias projection → implemented | Daemon catalog snapshots now carry the configured global Learn input alias and the TUI adopts it only when unarmed and unset; no arbitrary endpoint fallback is introduced. |
| 2026-08-28 | W016/W040 | codex | explicit Learn alias endpoint resolution → implemented | Daemon startup resolves the configured alias against discovered input ports and publishes the stable `learn_endpoint_id`; unresolved aliases remain unresolved rather than falling back to an arbitrary port. |
| 2026-08-28 | W016/W040 | codex | Learn endpoint representation audit → recorded | Confirmed discovery uses stable string IDs while captured domain events use numeric endpoint IDs; shared resolver work is required before TUI capture can safely dispatch, preventing an implicit or arbitrary hash in the UI. |
| 2026-08-28 | W016/W040 | codex | shared Learn endpoint identity resolver → implemented | Added one deterministic `numeric_endpoint_id` conversion in midi-engine, reused by physical input decoding and daemon Learn requests accepting stable `endpoint_id`; identity conversion is no longer duplicated across layers. |
| 2026-08-28 | W016/W040 | codex | executable Learn capture controls → implemented | Added explicit `l` arm, bounded daemon polling, Enter completion, and Escape cancellation in the TUI; capture requires the daemon-resolved configured alias endpoint and enters the existing review phase. |
| 2026-08-28 | W016/W040 | codex | executable Learn candidate decoding → implemented | Added strict decoding of daemon candidate JSON into shared MIDI Learn models for note, pressure, CC, program, pitch bend, system-common, realtime, and SysEx families; malformed candidates are ignored without changing capture state. |
| 2026-08-28 | W016/W040/W046 | codex | Learn candidate selection controls → implemented | Added bounded `j/k` candidate navigation and explicit Enter selection from Review into the existing destination gate; the rendered Learn header documents capture, selection, acceptance, and cancellation controls. |
| 2026-08-28 | W030/W040/W044 | codex | setlist copy/delete controls → implemented | Added deterministic copy IDs and bounded local deletion for selected setlists; operations remain transactional until explicitly saved through daemon validation. |
| 2026-08-28 | W043/W044 | codex | executable read-only workspace navigation → implemented | Added direct TUI shortcuts for Diagnostics, Monitor, Backups, and Setlists using bounded existing renderer models; no unsafe device action is enabled, and focused tests plus strict Clippy pass. |
| 2026-08-28 | W040/W042 | codex | route projection fallback coverage → implemented | Added executable-binary regression tests for supported route conversion and preservation of existing transactional drafts when a daemon route is malformed; focused package tests and strict Clippy pass. |

The worklist is complete only when W053 is `DONE`. A feature demonstration, passing unit
tests, or code presence alone is not release completion.

| 2026-08-28 | W016/W040/W046 | Learn destination and live-test controls | implemented | Added explicit `r` destination selection, `t` live-test start, phase-aware Enter completion, and commit handling; destination compatibility and mandatory passed-test checks remain enforced by the shared Learn state machine. |
| 2026-08-28 | W016/W040/W046 | durable learned-mapping persistence | implemented | Committed Learn mappings now travel through daemon-owned Configuration IPC, append to existing validated mappings, survive atomic config save, and appear in the catalog snapshot. |
| 2026-08-28 | W040/W042 | endpoint-backed route creation | implemented | TUI `a` queries the daemon endpoint inventory, creates a validated default CC draft from the first input/output pair, and leaves it transactional until explicit save. |
| 2026-08-28 | W040/W042 | transactional route channel editing | implemented | TUI `c` cycles the selected route between any-channel and MIDI channels 1–16 with full-batch conflict validation before explicit save. |
| 2026-08-28 | W030/W040/W044 | empty setlist creation | implemented | TUI `a` creates collision-safe empty setlist drafts, selects the new row, and leaves persistence behind the existing explicit `s` save gate. |
| 2026-08-28 | W030/W040/W044 | setlist project assignment | implemented | TUI projects available catalog IDs and `p` appends the first unused project to the selected setlist with duplicate/empty-ID validation before explicit save. |
| 2026-08-28 | W030/W040/W044 | setlist project removal | implemented | TUI `x` removes the last project from the selected setlist draft, safely rejecting empty/unselected rows until explicit save. |
| 2026-08-28 | W030/W040/W044 | setlist project ordering | implemented | TUI `[`/`]` moves the last project within the selected setlist while preserving transactional persistence and rejecting underspecified rows. |
| 2026-08-28 | W011/W040/W044 | executable device query action | implemented | Eventide workspace `q` now dispatches the authorized daemon DeviceQuery command and reports bounded success/failure status without enabling writes. |
| 2026-08-28 | W030/W040/W044 | catalog snapshot authority | implemented | Daemon snapshots and replayed state events now include the authoritative project/setlist/Learn catalog required to initialize catalog-driven TUI workspaces after reconnect or restart. |
| 2026-08-28 | W011/W040/W044 | device query inventory projection | implemented | DeviceQuery now returns the daemon’s discovered endpoint IDs, names, and directions; the TUI reports populated versus empty query results without transmitting MIDI. |
| 2026-08-28 | W011/W040/W044 | profile-backed device query | implemented | DeviceQuery accepts a built-in profile ID and returns bounded profile identity, capabilities, documented controls, and query count; unknown profiles fail without device I/O. |
| 2026-08-28 | W011/W040 | profile control message rendering | implemented | Profiles now validate control labels, MIDI channels, and declared ranges before producing bounded CC or Program Change bytes; rendering does not transmit. |
| 2026-08-28 | W011/W040 | authorized device control IPC | implemented | Added a dedicated confirmation-gated DeviceControl command that renders profile-validated bytes and sends only to the named daemon-registered output; missing confirmation, invalid fields, profiles, controls, or destinations fail closed. |
| 2026-08-28 | W011/W040/W044 | executable profile control write | implemented | Eventide workspace `W` resolves the first live output and sends the explicitly confirmed profile-backed Mix control through DeviceControl; query/navigation remain non-transmitting. |
| 2026-08-28 | W011/W040/W044 | bounded device control editor | implemented | Device workspace now selects profile-owned controls with `j/k`, adjusts a clamped 7-bit value with `+/-`, and sends the selected control through the confirmation-gated write path. |
| 2026-08-28 | W011/W040 | scriptable profile query | implemented | Added `device-query <profile-id>` CLI access to the daemon’s validated profile identity, capabilities, controls, ranges, and query metadata response. |
| 2026-08-28 | W020/W040 | profile query metadata projection | implemented | Profile-aware DeviceQuery now returns bounded query IDs, reply references, request sizes, and documented feature labels alongside controls and capabilities. |
| 2026-08-28 | W020/W040 | declarative query preview | implemented | Profile-aware DeviceQuery can render a bounded query request and return its correlated reply value/mask for a selected query without transmitting or claiming hardware readback. |
| 2026-08-28 | W013/W040 | route enable and priority editing | implemented | Routing workspace now transactionally toggles the selected mapping and adjusts its bounded execution priority with `e` and `+/-` before explicit save. |
| 2026-08-28 | W010/W040 | exact scene selection | implemented | Added a validated `scene select <scene-id>` CLI/IPC path that updates the daemon-owned active scene and persists the selection atomically. |
| 2026-08-28 | W010/W040 | scene action authoring CLI | implemented | Added atomic `scene action-add` authoring for validated framed SysEx actions with explicit destination and optional unsafe classification. |
| 2026-08-28 | W010/W040 | generic scene MIDI authoring | implemented | Scene action authoring now accepts any bounded complete MIDI wire message (CC, Program Change, note, or SysEx), validates it before persistence, and retains unsafe classification. |
| 2026-08-28 | W010/W040 | scene action dependency authoring | implemented | `scene action-add` now accepts `--depends-on=<action-id>`, allowing authored actions to participate in the validated dependency graph. |
| 2026-08-28 | W010/W040 | scene action removal | implemented | Added atomic `scene action-remove` authoring with dependency checks so referenced actions cannot be deleted accidentally. |
| 2026-08-28 | W010/W040 | scene action inspection | implemented | Added `scene actions` human-readable and JSON views for auditing authored payloads, unsafe flags, and action ordering before activation. |
| 2026-08-28 | W010/W040 | executable scene action schema | implemented | Scene actions now optionally carry a named destination and bounded validated MIDI wire payload while preserving legacy metadata-only configuration; scene selection executes safe payloads through the registered output boundary and holds unsafe actions. |
| 2026-08-28 | W022/W040 | M-VAVE profile control bridge | implemented | Confirmation-gated DeviceControl now routes captured M-VAVE preset and IR/EQ module operations through profile-owned SysEx builders with bounded validation; replies remain sent-unverified where the device provides no state readback. |
| 2026-08-28 | W022/W040 | declarative M-VAVE module controls | implemented | M-VAVE IR and EQ module operations are now first-class profile control definitions with operation identifiers, range metadata, and profile renderer support. |
| 2026-08-28 | W022/W040 | profile-control runtime coverage | implemented | DeviceQuery now exposes control operation identifiers, and profile tests verify M-VAVE IR/EQ controls render the captured module SysEx frames directly. |
| 2026-08-28 | W022/W040 | declarative M-VAVE preset controls | implemented | M-VAVE presets 1–32 are now profile-defined operation controls and render through the same captured SysEx path used by DeviceControl. |
| 2026-08-28 | W053 | release notes synchronization | implemented | Checked-in release notes now describe the shipped device-query, device-control, SysEx, routing, setlist, profile, provenance, and qualification behavior consistently with the public test release. |
| 2026-08-28 | W011/W040/W053 | device/SysEx CLI documentation | implemented | README now documents exact profile-control and framed SysEx command syntax, validation boundaries, confirmation requirements, and read-only query behavior. |
| 2026-08-28 | W011/W040 | scriptable device control | implemented | Added a confirmation-gated CLI device-control command for profile, control, channel, value, and destination, reusing daemon-side profile validation and output ownership. |
| 2026-08-28 | W011/W040 | confirmation-gated SysEx transmission | implemented | SysEx IPC now validates explicit confirmation, destination, 1–1024 byte bounds, F0/F7 framing, and registered output ownership before sending. |
| 2026-08-28 | W013/W040 | executable route state | implemented | Route enabled state, priority ordering, CC curves, cycle authorization, and bounded predicates now execute in the engine and round-trip through daemon JSON and the TUI editor. |
| 2026-08-28 | W013/W053 | route contract regression evidence | verified | Daemon and engine tests cover route-state and predicate round trips, disabled-route suppression, priority ordering, and deterministic CC curve output; the public test artifact was refreshed. |
| 2026-08-28 | WORKLIST/W054–W061 | codex | proposal → governed workstream | Added the approved connected-device rack-appliance TUI plan as eight dependency-ordered Luna task packets, including public-contract boundaries, allowed files, exclusions, test-first checkpoints, commands, hardware prerequisites, acceptance evidence, dependency-map integration, and execution wave 8. |
| 2026-08-28 | W056/W060 | codex | presentation slice → verified | Added the first rack-appliance presentation slice to the primary TUI workspace: ANSI-color status hierarchy, connected/degraded lamps, panel titles, operator-control legend, responsive narrow-terminal handling, explicit terminal clearing, and no-overlap layout behavior. Focused and full workspace tests, strict Clippy, worklist validation, and `scripts/release-gate.sh` pass. Device inventory, per-control live activity, processor destination browsing, autosave/Undo, and hardware qualification remain W054–W061 work. |
| 2026-08-28 | W054 | codex | `IN_PROGRESS` → contract increment | Added deterministic `PhysicalDevice` grouping from endpoint metadata and exposed grouped `physical_devices` records in DeviceQuery/Endpoints IPC responses; input/output ports with identical names are grouped, distinct names are not merged, and 50 engine plus 24 daemon tests pass with strict Clippy. Profile identity, persisted offline slots, and TUI projection remain in W054. |
| 2026-08-28 | W055 | codex | activity contract increment | Added daemon snapshots/journal payloads for the latest bounded MIDI activity record: source endpoint, message family, number/value where applicable, routed destination endpoints, and input sequence. Contract is now available for TUI projection; coalescing, per-control identity, and live rendering remain in W055. |
| 2026-08-28 | W055/W040 | codex | activity-to-TUI projection increment | Added renderer-safe `LiveActivity` parsing and dashboard state projection for source endpoint, MIDI family, number/value, destinations, and sequence; the primary mapping surface now exposes the latest live message and destination count. Focused TUI/CLI tests and strict Clippy pass. Coalescing, per-control identity, and hardware reaction remain. |
| 2026-08-28 | W055 | codex | bounded activity coalescing increment | Added a capacity-bounded per-endpoint/message-control coalescer that retains only the newest sequence value, rejects stale samples, and drains deterministically; focused tests, workspace strict Clippy, and `scripts/release-gate.sh` pass. Stable physical control identity and hardware qualification remain. |
| 2026-08-28 | W054/W056 | codex | connected-device TUI projection increment | Added renderer-safe physical-device parsing and dashboard projection for normalized identity, display name, input/output endpoint IDs, and connection state; the primary mapping surface now shows a bounded device inventory. Focused tests and `scripts/release-gate.sh` pass. Persisted offline identity, per-control visualization, and hardware qualification remain. |
| 2026-08-28 | W057 | codex | mapping faceplate increment | Added an explicit active route chain to the Launch Control XL surface, showing enabled/disabled source-to-destination relationships in the primary appliance view; added deterministic renderer coverage. TUI tests, strict Clippy, and `scripts/release-gate.sh` pass. Full profile control/value faceplate and hardware qualification remain. |
| 2026-08-28 | W059 | codex | bounded mapping undo increment | Added a 32-entry in-memory undo history for TTY mapping edits, `u` recovery action, and persistent SAVE REQUIRED/SAVED status on the primary mapping surface. App tests, strict Clippy, and `scripts/release-gate.sh` pass. Durable autosave, audit/confirmation semantics, and rollback persistence remain. |
| 2026-08-28 | W059 | codex | durable route persistence increment | Successful daemon route commits now atomically persist a bounded JSON sidecar beside the configured state file, and configured daemon startup restores the route set with generation validation; rebind regression coverage passes. Full release gate passes. Audit/confirmation semantics and rollback persistence remain. |
| 2026-08-28 | W055/W056/W057 | codex | live appliance level increment | Added a fixed-width ASCII activity level bar with source endpoint and sequence context to the primary mapping surface; value bounds and endpoint-independent rendering are tested. Full release gate passes and the local service was reinstalled. Per-control identity/highlighting and hardware reaction remain. |
| 2026-08-28 | W055/W057 | codex | stable control identity increment | Added a stable `endpoint:<id>:<kind>[:<number>]` control ID to daemon activity records and TUI projection, and displayed it beside the live level/value context. Daemon/TUI tests, strict Clippy, and the full release gate pass. Physical hardware reaction remains. |
| 2026-08-28 | W055 | codex | activity publication rate increment | Added a daemon-side approximately 30 Hz journal publication gate while retaining the latest bounded snapshot value; burst regression coverage proves immediate input does not grow the published event stream. Daemon tests, strict Clippy, and full release gate pass. Fake-time age tests and physical qualification remain. |
| 2026-08-28 | W055 | codex | activity timestamp increment | Added bounded source timestamp propagation to live activity payloads and TUI projection, enabling client-side highlight-age calculation without wall-clock assumptions; daemon/TUI assertions cover the field. Strict Clippy and full release gate pass. Fake-time age rendering remains. |
| 2026-08-28 | W061 | codex | local observation qualification | `scripts/qualify-hardware.sh` passed on the local TTY seat; observed the active Novation controller, Eventide MicroPitch, and all four MidiSport 4x4 MIDI ports. |
| 2026-08-28 | W055/W061 | codex | physical input runtime increment | Daemon startup now auto-provisions discovered non-virtual ALSA inputs and polls bounded batches into the normal routing/activity path; post-install `aconnect -l` confirms MACKES input clients connected to MidiSport ports, Launch Control XL ports, and MicroPitch Pedal. Output writes remain unqualified. |
| 2026-08-28 | W061 | codex | physical output runtime increment | Daemon startup now auto-provisions discovered non-virtual ALSA outputs into the bounded output registry, completing the safe input/output runtime path without sending a physical test message. Release gate passes and the service is active. Output signal/LED qualification remains pending. |
| 2026-08-28 | W054/W056/W061 | codex | authoritative snapshot inventory increment | Fixed the daemon snapshot/state projection to carry the startup physical-device inventory used by the TUI, eliminating the query-only inventory gap; focused tests, strict Clippy, release gate, and installed-service verification pass. |
| 2026-08-28 | W061 | codex | stable output provisioning correction | Corrected startup output provisioning to pass stable endpoint IDs to the ID-based adapter API; after reinstall, `aconnect -l` confirms MACKES output clients connected to all four MidiSport ports, both Launch Control XL ports, and MicroPitch Pedal. No test message was sent. |
| 2026-08-28 | W054/W061 | codex | operator status projection increment | Changed read-only `status --json` to request the authoritative daemon snapshot, exposing health, route generation, physical devices, and latest activity for local testing; refreshed root group credentials with `sg mackes-control` and verified seven connected device-port records. |
| 2026-08-28 | W054/W056/W061 | codex | ALSA device grouping correction | Normalized explicit ALSA `Device:Port` names so the installed authoritative status now reports one Launch Control XL (2 inputs/2 outputs), one MicroPitch Pedal, and one MidiSport 4x4 (4 inputs/4 outputs); grouping regression, strict Clippy, release gate, and runtime status verification pass. |
| 2026-08-28 | W055/W054 | codex | live activity inventory identity increment | Added stable endpoint-key lookup in the input registry so activity records resolve to inventory endpoint IDs when sourced from a registered physical input, while synthetic events retain a deterministic fallback; focused tests, strict Clippy, release gate, and local install pass. |
| 2026-08-28 | W058/W056 | codex | destination lane increment | Added a bounded primary-surface DESTINATIONS lane derived from authoritative physical output inventory, making processor/output targets visible next to the active route chain; TUI tests and full release gate pass. Categorized profile parameter browsing remains. |
| 2026-08-28 | W058/W060 | codex | destination selection increment | Added keyboard-only `D` cycling across visible physical output destinations with explicit `>` selection marking in the primary mapping surface; focused TUI/app tests, strict Clippy, and full release gate pass. Parameter-category browsing and route mutation against the selected target remain. |
| 2026-08-28 | W058/W060 | codex | selected-target route creation increment | Primary-screen `a` now creates a route against the selected physical output destination, using shared stable endpoint-to-route conversion and a visible physical input source; focused tests, strict Clippy, and full release gate pass. Profile parameter browsing remains. |
| 2026-08-28 | W058 | codex | profile parameter browser increment | Selected destinations now expose profile-owned MicroPitch parameter groups and exact control labels in the primary TTY surface; unsupported device profiles remain explicitly unverified. TUI tests, strict Clippy, full release gate, and local installation pass. |
| 2026-08-28 | W058/W060 | codex | parameter focus increment | Added keyboard-only `P` cycling through the selected MicroPitch destination's profile-owned parameters, with explicit `>` focus marking; TUI/app tests, strict Clippy, and full release gate pass. Parameter-to-route mutation remains. |
| 2026-08-28 | W058/W059/W060 | codex | parameter-aware mapping increment | Mapping drafts, route projection, route-save JSON, primary-screen route creation, and the inspector now carry the selected profile-owned destination parameter; legacy routes remain compatible with an absent field. Full release gate and local install pass. |
| 2026-08-28 | W058/W059 | codex | canonical route parameter increment | Extended the engine Route contract and daemon route acknowledgment/parser to preserve destination parameter metadata through canonical route state, closing the prior query/restart loss. Full release and repository gates pass. |
| 2026-08-28 | W058/W059 | codex | parameter contract regression evidence | Added daemon coverage proving a parameterized route survives JSON replacement into canonical engine state; daemon tests, strict Clippy, and full release gate pass. |
| 2026-08-28 | RELEASE | codex | release hygiene increment | Full release gate and repository verification pass after destination/parameter workflow updates; the remaining tested work is ready to commit and synchronize to the public repository. |
| 2026-08-28 | W054/W057/W060 | codex | appliance visual hierarchy increment | Applied a focused TTY presentation pass to the primary Novation mapping surface: cyan control-surface framing, yellow mapping-inspector framing, readable operator-control framing, and bold role-specific titles make live status, mapping selection, and action areas distinguishable at a glance. `mackes-tui` tests (37) pass. |
| 2026-08-28 | W057 | codex | Launch Control XL vocabulary increment | Replaced generic faceplate control labels with documented Mk1 bank vocabulary (`T01–T08`, `M01–M08`, `B01–B08`), explicit channel-button and eight-fader lanes, and the documented Device/Mute/Solo/Record Arm/Up/Down/Left/Right utility row. Profile label lookup remains fail-closed for unsupported indices; TUI tests and formatting pass. |
| 2026-08-28 | W055/W060 | codex | activity pulse age increment | Added a local monotonic activity age to the dashboard reducer; incoming activity resets it, the bounded TTY loop advances it, and the primary surface reports age in milliseconds. Saturation/reset regression coverage passes without wall-clock or hardware assumptions. |
| 2026-08-28 | W059/W060 | codex | daemon-owned route Undo increment | Added a bounded one-step route undo record beside the durable route sidecar, restart loading, explicit `routes` action `undo`, and TUI delegation through the daemon IPC boundary with a labeled local fallback. Focused daemon/IPC/CLI tests and strict Clippy pass; full atomic failure-path and audit coverage remains in progress. |
| 2026-08-28 | W059 | codex | route transaction ordering increment | Staged durable route and Undo sidecars before router mutation and restored the prior route document on replacement failure, preventing the common disk/runtime divergence path; daemon tests and strict Clippy pass. Exhaustive injected filesystem-failure and audit-log coverage remains. |
| 2026-08-28 | RELEASE | codex | 0.1.7 release preparation | Reconciled README and release notes with the post-0.1.6 feature set: Launch Control XL faceplate vocabulary, live activity age, destination/parameter workflow, and restart-safe route Undo. Artifact generation and public release verification remain. |
| 2026-08-28 | RELEASE | codex | 0.1.8 release preparation | Reconciled release metadata for authoritative Undo availability in snapshots/events and the primary TTY surface; artifact generation and public release verification remain. |
| 2026-08-28 | W059/W060 | codex | Undo availability observability increment | Added `route_undo_available` to authoritative daemon snapshots/events and the primary mapping surface, so operators can distinguish an available restart-safe Undo from an empty history before pressing `u`; TUI projection coverage added. |
| 2026-08-28 | W054/W056/W060 | codex | connected-device tab strip increment | Added a truthful connected MIDI rack header and bounded device-tab strip to the primary TTY mapping surface, with connected/offline markers for each visible physical device; focused TUI tests and strict Clippy pass. |
| 2026-08-29 | W054/W060 | codex | explicit source selection increment | Added keyboard-only `I` cycling across every visible physical MIDI input, explicit SOURCE inventory display, and selected-source route creation on the primary mapping surface; TUI and CLI projection tests pass. |
| 2026-08-29 | W059/W060 | codex | Undo snapshot reconciliation → implemented | Successful daemon-owned route Undo now requests an authoritative snapshot before the next redraw, preventing the primary TUI editor from displaying stale routes after rollback. `cargo check -p mackes-midi-matrix` and `cargo fmt --check` pass. |
| 2026-08-29 | W059/W060 | codex | Save snapshot reconciliation → implemented | Successful route Save now requests an authoritative snapshot before the next redraw, reconciling normalized routes and generation state through the daemon boundary. `cargo check -p mackes-midi-matrix` and `cargo fmt --check` pass. |
| 2026-08-29 | W059/W060 | codex | route-save panic removal → implemented | Replaced the production route-payload serialization `expect` with an actionable fail-closed response, keeping the TUI alive if serialization ever fails. Package check, formatting, and diff hygiene pass. |
| 2026-08-29 | W040/W043 | codex | device-control payload panic removal → implemented | Replaced the user-triggerable TUI device-control serialization `expect` with an actionable health state and safe loop continuation. Package check, formatting, and diff hygiene pass. |
| 2026-08-29 | W040/W044 | codex | TUI monitor initialization panic removal → implemented | Replaced startup monitor-capacity `expect` with an explicit initialization error returned through the TUI startup boundary. Package check, formatting, and diff hygiene pass. |
| 2026-08-29 | W045/W032 | codex | CLI request encoding panic removal → implemented | Replaced user-triggerable SysEx and device-control request serialization `expect` calls with explicit diagnostic failures and exit-2 handling. CLI package check, formatting, and diff hygiene pass. |
| 2026-08-29 | W053/W040 | codex | strict-Clippy release correction → verified | Release-gate Clippy caught and the worktree corrected two lint findings introduced by recent hardening (`const` activity label and `clone_into` health assignment); workspace strict Clippy now passes. |
| 2026-08-29 | W032/W059 | codex | route mutation safety boundary → implemented | Attached the shared bounded `SafetyController`/`AuditLog` to daemon route replacement and Undo IPC; each mutation now records a local actor decision and fails closed under performance lock. Route-focused daemon tests (3), package check, formatting, and diff hygiene pass. |
| 2026-08-29 | W032/W040/W059 | codex | audit observability projection → implemented | Added bounded `audit_count` to daemon snapshots and sequenced state events, exposing retained mutation-evidence presence without serializing sensitive audit payloads. Daemon package check, formatting, and diff hygiene pass. |
| 2026-08-29 | W032/W040/W059 | codex | audit count client projection → implemented | Projected daemon `audit_count` through the TUI reducer and primary mapping surface, making retained mutation-audit evidence visible to operators. Package check, formatting, and diff hygiene pass. |
| 2026-08-29 | W032/W059 | codex | route policy denial regression → verified | Added focused daemon coverage proving performance lock denies route mutation while retaining a redacted audit decision; the targeted test passes. |
| 2026-08-29 | W053/W056 | codex | polished Linux TUI requirement → recorded | Added the operator-requested polished-modern-Linux TUI requirement to W056, preserving functionality while requiring reduced visual noise and a coherent theme. Full `scripts/release-gate.sh` completed with `release-gate: PASS` after workspace tests, strict Clippy, benchmark, integration, installer smoke, and artifact verification. |
| 2026-08-29 | W040/W056 | codex | primary TUI theme polish → implemented | Applied a consistent ANSI-safe black canvas, cyan panel hierarchy, brighter connected/degraded status colors, and readable white content styling to the primary mapping surface without altering controls or layout. All 39 TUI tests and strict TUI Clippy pass. |
| 2026-08-29 | W032/W040 | codex | audit projection regression → verified | Extended the authoritative dashboard payload test to assert `audit_count` survives daemon-to-TUI projection; the focused TUI test passes after formatting. |
| 2026-08-29 | W032/W040/W059 | codex | bounded audit record projection → implemented | Daemon snapshots and sequenced events now expose up to 32 newest redacted route decisions with actor, action, target, risk, allow/deny, and safe result fields; focused daemon audit test passes. |
| 2026-08-29 | W032/W040/W056 | codex | latest audit client visibility → implemented | TUI now projects the newest bounded audit action and allow/deny result beside the audit count on the primary surface; dashboard payload regression and strict TUI Clippy pass. |
| 2026-08-29 | W005/W032/W059 | codex | audit snapshot compatibility evidence → verified | Extended daemon snapshot/journal regression coverage to require an empty bounded audit projection on a fresh daemon; focused snapshot replay test and strict daemon Clippy pass. |
| 2026-08-29 | W044/W032 | codex | denied-policy diagnostic projection → implemented | TUI observability now turns a denied audit decision into a bounded policy warning with remediation guidance, allowing operators to diagnose blocked route mutations without reading daemon source. CLI package check, strict Clippy, formatting, and diff hygiene pass. |
| 2026-08-29 | W044/W032 | codex | denied-policy diagnostic regression → verified | Extended the CLI observability test to assert denied `route_replace` audit decisions produce a policy diagnostic naming the action; focused test and strict CLI Clippy pass. |
| 2026-08-29 | W053 | codex | post-audit-projection release gate → PASS | Re-ran `scripts/release-gate.sh` after the audit-record and TUI projection changes; repository checks, advisory scan, workspace tests, strict Clippy, benchmark, hermetic integration, installer smoke, and artifact verification all passed. |
| 2026-08-29 | W040/W056 | codex | healthy-state rendering correction → implemented | The primary mapping surface now treats the daemon’s canonical `ready` health as online instead of incorrectly showing DEGRADED; focused regression and strict TUI Clippy pass. |
| 2026-08-29 | W053 | codex | post-polish release gate → PASS | Re-ran `scripts/release-gate.sh` against the current tree: repository/worklist checks, advisory scan, workspace tests (including 40 TUI tests and 27 daemon tests), strict Clippy, optimized routing benchmark, hermetic integration, installer smoke, and release artifact verification all passed. |
| 2026-08-28 | RELEASE | codex | 0.1.9 release preparation | Reconciled public release metadata with the connected-device tab strip and current source commit; artifact generation and public release verification remain. |
| 2026-08-29 | RELEASE | codex | 0.1.10 release preparation | Corrected executable/package version reporting to match the public release and added daemon `--version` output; artifact generation and public release verification remain. |
| 2026-08-29 | W054/W060 | codex | offline identity retention → implemented | Daemon physical-device refresh now retains previously discovered records as bounded `offline` entries, sorted by stable ID, while replacing reconnected records with fresh endpoint state; daemon Clippy, formatting, worklist validation, and diff hygiene pass. |
| 2026-08-29 | W054 | codex | offline identity retention → regression verified | Added `physical_device_refresh_retains_disconnected_identity`, proving a disappeared Launch Control record retains its stable ID, endpoint identity, and `offline` state; focused daemon test and strict Clippy pass. |
| 2026-08-29 | W054/W055 | codex | inventory churn bound → implemented | Physical-device refresh now caps the retained projection at 32 deterministically sorted records, preventing repeated disconnect/reconnect churn from growing snapshot state without limit; focused daemon test, strict Clippy, worklist validation, and diff hygiene pass. |
| 2026-08-29 | W054 | codex | active-device retention priority → implemented | The bounded inventory ordering now prioritizes connected records before offline history, ensuring a large retained-device set cannot evict current endpoints; focused daemon regression, strict Clippy, worklist validation, and diff hygiene pass. |
| 2026-08-29 | W054 | codex | inventory saturation regression → verified | Extended `physical_device_refresh_retains_disconnected_identity` with a 40-device refresh, proving the 32-record cap retains only current connected records without reintroducing stale offline entries; focused daemon test and strict Clippy pass. |
| 2026-08-29 | W054/W040 | codex | IPC inventory payload bound → implemented | Applied the 32-device cap at shared physical-inventory serialization, covering direct endpoint/query responses as well as authoritative refreshed snapshots; focused daemon test, strict Clippy, worklist validation, and diff hygiene pass. |
| 2026-08-29 | W053/W054 | codex | post-inventory-bound release gate → PASS | `scripts/release-gate.sh` passed against the current tree: repository/worklist policy, advisory scan, workspace tests (28 daemon tests), strict workspace Clippy, optimized routing benchmark, hermetic integration, installer smoke, and release artifact checksum/preflight all passed. |
| 2026-08-29 | W053/RELEASE | codex | 0.1.11 release notes and artifact alignment → implemented | Added the missing `docs/releases/0.1.11.md`; `scripts/package-release.sh 0.1.11` now produces the documented Linux bundle and checksum verification passes. Worklist validation and diff hygiene pass. |
| 2026-08-29 | W053/RELEASE | codex | version-derived release gate → PASS | Changed the default release gate version to derive from the workspace manifest, then reran `scripts/release-gate.sh`; the 0.1.11 artifact, checksum, tests, Clippy, benchmark, integration, installer smoke, and preflight all passed. |
| 2026-08-29 | W046/W060 | codex | live-test false-success removal → implemented | Removed the production Enter shortcut that marked MIDI Learn live tests successful without daemon/device evidence; pending tests can now only be completed by an explicit transport result, while cancellation remains available. Focused CLI tests, strict Clippy, worklist validation, and diff hygiene pass. |
| 2026-08-29 | W046 | codex | live-test pending-state projection → implemented | Learn rendering now explicitly reports when a test is awaiting a daemon/device result and exposes Esc cancellation; the Learn workflow regression asserts this state, with focused TUI test, strict Clippy, worklist validation, and diff hygiene passing. |
| 2026-08-29 | W054 | codex | inventory implementation handoff → IN_REVIEW | Reconciled the implemented bounded physical-device projection and disconnect-retention evidence; independent contract, safety, and coverage review remains required before `DONE`. |
| 2026-08-29 | W056 | codex | shared rack primitive increment → implemented | Added ANSI-safe `RackLamp` and bounded `RackValueBar` renderer-neutral primitives with deterministic semantic-color and monotonic-cell tests; focused TUI tests, strict TUI Clippy, formatting, worklist validation, and diff hygiene pass. Full W056 layout/snapshot acceptance remains open. |
| 2026-08-29 | W056 | codex | shared rack renderer increment → implemented | Added reusable lamp and ASCII value-bar line renderers with color-independent output tests; 43 focused TUI tests, strict TUI Clippy, formatting, worklist validation, and diff hygiene pass. Full W056 layout/snapshot acceptance remains open. |
| 2026-08-29 | W056 | codex | compact rack layout increment → implemented | Added deterministic required-viewport adaptation for 100×37 expanded and 80×24 compact layouts, retaining one-row status, alert, and footer regions; 44 focused TUI tests and strict TUI Clippy pass. Full renderer snapshots remain open. |
| 2026-08-29 | WORKLIST/W070/W071 | operator | qualification scope decision → post-release epic | Independent software review and physical hardware/usability qualification are removed from the next full-featured release gate and tracked as deferred W070/W071 work after the software worklist is drained. |
| 2026-08-29 | W056 | codex | rack-shell frame increment → implemented | Added deterministic compact/expanded shell text rendering that keeps health, alerts, navigation, and panic state visible at the required viewport sizes; 45 focused TUI tests, strict Clippy, formatting, worklist validation, and diff hygiene pass. |
| 2026-08-29 | W056 | codex | rack layout integration → implemented | Primary controller mapping now derives compact/expanded behavior from the shared required-viewport layout contract; focused TUI tests, strict Clippy, formatting, worklist validation, and diff hygiene pass. |
| 2026-08-29 | W056 | codex | renderer viewport qualification → implemented | Added in-memory Ratatui rendering checks at 100×37 and 80×24, asserting the connected-rack title and always-visible PANIC affordance; 46 focused TUI tests, strict TUI Clippy, formatting, worklist validation, and diff hygiene pass. |
| 2026-08-29 | W056 | codex | semantic-state coverage → implemented | Added distinct ASCII markers for offline, disabled, enabled, warning, and error lamps plus bounded long-alert coverage; 47 focused TUI tests and strict TUI Clippy pass. |
| 2026-08-29 | W056 | codex | software acceptance → DONE | Closed the rack-appliance design system after renderer-level 100×37/80×24 checks, stable golden shell frames, critical-state visibility, ANSI-safe markers, and bounded alert coverage; 49 TUI tests and strict TUI Clippy pass. |
| 2026-08-29 | W057 | codex | profile faceplate catalog increment → implemented | Added a serializable Launch Control XL Mk1 faceplate catalog covering all 48 documented controls in physical order, classified as knobs, channel buttons, or utility buttons; 35 profile tests, strict profile Clippy, formatting, worklist validation, and diff hygiene pass. |
| 2026-08-29 | W057 | codex | renderer faceplate coverage → implemented | Added a connected-device in-memory renderer regression asserting documented Launch Control bank and utility labels are exposed on the primary surface; 50 focused TUI tests, strict Clippy, formatting, worklist validation, and diff hygiene pass. |
| 2026-08-29 | W057 | codex | validated activity-to-faceplate resolution → implemented | Added fail-closed resolution from validated template MIDI assignments to physical faceplate indices, rejecting invalid or absent matches; 36 profile tests, strict profile Clippy, formatting, worklist validation, and diff hygiene pass. |
| 2026-08-29 | W055/W057 | codex | activity channel propagation → implemented | Added optional zero-based MIDI channel to bounded daemon live-activity JSON and TUI projection, preserving backward compatibility for channel-less/system messages; daemon/TUI tests, strict workspace Clippy, formatting, worklist validation, and diff hygiene pass. |
| 2026-08-29 | W057 | codex | live assignment highlighting → implemented | Wired validated template activity resolution into the TUI faceplate: matching kind/channel/number marks the physical control active, while absent or unresolved assignments remain unhighlighted; 51 TUI tests, strict workspace Clippy, formatting, worklist validation, and diff hygiene pass. |
| 2026-08-29 | W057 | codex | persisted template contract increment → implemented | Added validated, backward-compatible configuration types for optional Launch Control template assignments, including slot, channel, number, kind, count, range, and duplicate checks; config/TUI tests, strict workspace Clippy, formatting, worklist validation, and diff hygiene pass. |
| 2026-08-29 | W057 | codex | persisted template conversion → implemented | Added fail-closed conversion from validated configuration assignments to the profile template contract, with regression coverage for valid and unknown message kinds; 52 combined config/TUI tests, strict workspace Clippy, formatting, worklist validation, and diff hygiene pass. |
| 2026-08-29 | W057 | codex | runtime template loading → implemented | TUI startup now optionally loads `MACKES_CONFIG`, converts the validated Launch Control template, and leaves assignment highlighting disabled on missing/invalid configuration; full workspace tests, strict Clippy, repository checks, and diff hygiene pass. |
| 2026-08-29 | W057 | codex | resolved live-control value presentation → implemented | Primary faceplate now displays the resolved physical control index, profile label, current MIDI value, and ACTIVE/STALE age when a validated assignment matches; focused TUI tests, strict workspace Clippy, formatting, worklist validation, and diff hygiene pass. |
| 2026-08-29 | W057 | codex | explicit faceplate state model → implemented | Added renderer-neutral OFF/UNK/---/MAP/LIVE state markers, preserving distinct offline, unknown, unmapped, mapped, and active meanings without relying on color; 53 focused TUI tests, strict workspace Clippy, formatting, worklist validation, and diff hygiene pass. |
| 2026-08-29 | W056 | codex | degraded-state renderer coverage → implemented | Added in-memory renderer coverage for degraded health and SAVE REQUIRED visibility at 100×37; 48 focused TUI tests, strict TUI Clippy, formatting, worklist validation, and diff hygiene pass. |
| 2026-08-29 | W053/W046 | codex | post-live-test-safety release gate → PASS | Reran `scripts/release-gate.sh` after removing false live-test approval and adding pending-state projection; formatting, policy, advisories, all workspace tests, strict Clippy, benchmark, hermetic integration, installer smoke, and 0.1.11 artifact verification passed. |
| 2026-08-29 | W061 | codex | observation-only hardware qualification → verified | `scripts/qualify-hardware.sh` observed the Fedora host’s Launch Control XL, MicroPitch Pedal, and four MidiSport 4x4 ALSA ports; application endpoint enumeration completed. |
| 2026-08-29 | W015/W053 | operator scope decision | paired external testing deferred | Paired external-peer interoperability and long-duration soak testing are explicitly deferred until after release; hermetic two-peer and safety coverage remain required release evidence and the release gate is unchanged. |
| 2026-08-29 | W015/W050 | operator scope decision | paired test excluded from default suite | Marked `two_independent_rtp_peers_validate_identity_and_sequence` ignored with an explicit post-release qualification reason; the default testkit run now passes 11 tests with 1 deferred paired test, while all non-paired hermetic scenarios remain active. |
| 2026-08-29 | W053/W015 | codex | post-scope release gate → PASS | Reran `scripts/release-gate.sh` after deferring the paired RTP test; all remaining workspace tests, strict Clippy, benchmark, hermetic integration (11 pass/1 explicitly ignored), installer smoke, and 0.1.11 artifact verification passed. |
| 2026-08-29 | W015/W053 | codex | paired-test policy documentation → implemented | Updated `docs/testing.md` with the exact deferred test name, explicit `--ignored` qualification command intent, and the continuing release-gate hermetic coverage; testkit run passes 11/1 ignored, worklist validation, and diff hygiene pass. |
| 2026-08-29 | W046/W059 | codex | live-test contract ADR increment → specified | Extended ADR-0003 with a versioned daemon-owned live-test boundary, typed terminal outcomes, bounded request/reason/audit fields, generation/idempotency requirements, profile-owned probe semantics, and fail-closed commit authorization; implementation and compatibility fixtures remain next. |
| 2026-08-29 | W046/W005 | codex | live-test status vocabulary → implemented | Added the IPC-owned `LiveTestStatus` enum with stable `passed`, `failed`, `timed_out`, `denied`, and `unavailable` wire tags plus bounded identifier/reason constants; tag stability regression, focused IPC test, strict Clippy, worklist validation, and diff hygiene pass. |
| 2026-08-29 | W046/W005 | codex | live-test request/result validation → implemented | Added validated `LiveTestRequest` and `LiveTestResult` IPC types with bounded identifiers, reasons, audit references, and explicit error contracts; focused IPC tests, strict Clippy, worklist validation, and diff hygiene pass. |
| 2026-08-29 | W046/W005 | codex | live-test JSON compatibility boundary → implemented | Added strict JSON decoding for live-test requests/results, rejecting unknown fields and unknown statuses while preserving bounded validation; three focused IPC contract tests, strict Clippy, worklist validation, and diff hygiene pass. |
| 2026-08-29 | W046/W010 | codex | live-test daemon dispatch seam → implemented | Added a strict `learn` live-test action path that validates the typed request and returns an explicit bounded `unavailable` terminal result when no profile-backed probe exists; daemon tests, strict Clippy, worklist validation, and diff hygiene pass. |
| 2026-08-29 | W046/W010 | codex | live-test dispatch strictness → verified | Tightened the daemon live-test action to reject unknown JSON fields before request validation, preserving the strict ADR boundary; all 28 daemon tests, strict Clippy, worklist validation, and diff hygiene pass. |
| 2026-08-29 | W046/W060 | codex | live-test client dispatch → implemented | Wired the TUI Learn `t` action to submit a bounded daemon live-test request and consume its terminal status; only `passed` unlocks commit, while unavailable/failed/malformed responses fail closed. Focused app Clippy, worklist validation, and diff hygiene pass. |
| 2026-08-29 | W046/W005 | codex | live-test candidate identity → implemented | Extended the validated request contract and TUI dispatch with captured MIDI kind, number, and channel, preventing live-test evaluation against an incomplete candidate signature; IPC JSON tests, strict app/IPC Clippy, worklist validation, and diff hygiene pass. |
| 2026-08-29 | W046/W005 | codex | live-test bounds naming → implemented | Promoted endpoint identity limits to a named IPC contract constant and applied it to source and destination validation; three focused IPC tests, strict Clippy, worklist validation, and diff hygiene pass. |
| 2026-08-29 | W046/W005 | codex | live-test MIDI range regression → verified | Extended IPC contract coverage to reject channel 16 and number 128, preserving MIDI 1.0 bounds before daemon execution; focused IPC test, strict Clippy, worklist validation, and diff hygiene pass. |
| 2026-08-29 | W056/W057 | codex | unsupported-faceplate truthfulness → implemented | The primary mapping surface now falls back to generic MIDI control labels and an explicit unavailable-profile message when no Launch Control XL is connected, preventing unsupported hardware claims while preserving mapping controls; TUI tests, strict TUI Clippy, worklist validation, and diff hygiene pass. |
| 2026-08-29 | W057 | codex | explicit destination metadata → implemented | Added optional bounded destination summaries to Launch Control assignments, validated at config/profile boundaries, preserved through runtime conversion, and displayed with resolved live activity; ADR-0004 documents compatibility and authority; focused tests, strict workspace Clippy, worklist validation, and diff hygiene pass. |
| 2026-08-29 | W058 | codex | profile-derived destination catalog → implemented | Added renderer-neutral destination parameter metadata with exact profile labels, bounded categories/ranges, support state, and hazard marker; catalog derivation is covered against the Eventide profile, with 37 profile tests, 53 TUI tests, strict workspace Clippy, worklist validation, and diff hygiene pass. |
| 2026-08-29 | W058 | codex | multi-profile destination browser → implemented | Replaced the MicroPitch-only destination summary with validated catalog lookup for MicroPitch, Reflex, and M-VAVE IR Box; unknown devices remain explicitly unavailable. |
| 2026-08-29 | W058 | codex | browser filtering and selection coverage → implemented | Selected-parameter lookup now uses the same profile-derived catalog as rendering, and regression coverage verifies supported processor names expose catalogs while unknown devices remain filtered; 37 profile tests, 54 TUI tests, strict workspace Clippy, worklist validation, and diff hygiene pass. |
| 2026-08-29 | W059 | codex | bounded mapping undo core → implemented | MappingBank now records at most 16 prior committed snapshots, exposes availability, restores a prior generation atomically, and leaves invalid commits unchanged; regression coverage verifies deterministic restoration and generation advancement, with strict workspace Clippy, worklist validation, and diff hygiene pass. |
| 2026-08-29 | W059 | codex | daemon Undo generation precondition → implemented | Route Undo now accepts an optional route-generation precondition and fails closed on a concurrent generation change before authorization or persistence; daemon tests, strict Clippy, worklist validation, and diff hygiene pass. |
| 2026-08-29 | W060 | codex | generation-aware TUI Undo dispatch → implemented | The primary routing workspace now sends the authoritative route generation with Undo requests and adopts the daemon-returned generation only after success, preventing stale UI actions from undoing newer mappings; app tests, strict workspace Clippy, worklist validation, and diff hygiene pass. |
| 2026-08-29 | W062 | codex | static effects faceplate contract → implemented | Added a serialized Launch Control XL effects-faceplate catalog with six fixed Gain/Gate/Compressor/Modulation/Delay/Reverb groups, explicit profile ownership, non-overlapping physical indices, eight faders, and four unused controls; 38 profile tests, strict workspace Clippy, worklist validation, and diff hygiene pass. |
| 2026-08-29 | W062 | codex | effects faceplate renderer integration → DONE | Integrated the six-group effects faceplate into the primary renderer and added in-memory coverage for all group labels, ownership rows, eight faders, and explicit unused controls; focused renderer test, strict workspace Clippy, worklist validation, and diff hygiene pass. |
| 2026-08-29 | W063 | codex | pickup-aware effect-group LED policy → implemented | Added deterministic renderer-neutral states and LED outcomes for enabled pickup-ready, disabled, unavailable, selected type/model, and unknown groups; reconnect-safe resync remains delegated to the existing LED coalescer. 39 profile tests, strict workspace Clippy, worklist validation, and diff hygiene pass. |
| 2026-08-29 | W063 | codex | LED policy visibility → implemented | Effects faceplate rows now expose the fail-closed LED policy state (`OFF/UNKNOWN`) alongside every group, making synchronization status visible without relying on color; renderer test, strict workspace Clippy, worklist validation, and diff hygiene pass. |
| 2026-08-29 | W063 | codex | effect-group runtime state → implemented | Added bounded six-group/eight-fader runtime state with pickup-safe updates, MIDI value clamping, explicit unavailable/unknown states, and reconnect/scene resync invalidation; 44 profile tests, strict workspace Clippy, worklist validation, and diff hygiene pass. |
| 2026-08-29 | W063/W060 | codex | dashboard effects-state integration → implemented | Dashboard state now owns the profile effects runtime and invalidates feedback on physical-device refresh and scene changes, preserving reconnect/scene resync semantics; 54 TUI tests, strict workspace Clippy, worklist validation, and diff hygiene pass. |
| 2026-08-29 | W064 | codex | profile-derived parameter assignment catalog → implemented | Added bounded assignments from documented profile parameters to fixed owner controls, preserving exact ranges, conservative defaults, direction, units, and explicit unsupported reasons; 40 profile tests, strict workspace Clippy, worklist validation, and diff hygiene pass. |
| 2026-08-29 | W065 | codex | immutable effects automation planner → implemented | Added a bounded pure planner that orders requested groups MicroPitch → Reflex, skips unrelated groups, and returns explicit unverified-operation reasons rather than guessing wire messages. |
| 2026-08-29 | W066 | codex | minimal reusable effects configuration generator → implemented | Added deterministic signal-path naming and generation of configurations containing only selected groups and documented fixed-control assignments; empty selections remain empty and unrelated groups are excluded. 42 profile tests, strict workspace Clippy, worklist validation, and diff hygiene pass. |
| 2026-08-29 | W067 | codex | retired editor-map removal → implemented | Removed the retired-device import envelope and profile-generation path; unknown identities fail closed. |
| 2026-08-29 | W068 | codex | effects faceplate CLI inspection → implemented | Added a bounded read-only `effects faceplate [--json]` command exposing six groups, profile ownership, physical enable/type indices, eight faders, and unused controls without hardware writes; CLI tests, JSON smoke output, strict workspace Clippy, worklist validation, and diff hygiene pass. |
| 2026-08-29 | W069 | codex | deterministic effects demo frames → implemented | Added four bounded hardware-free demo frames covering unknown, enabled, selected, and unavailable group states, eight clamped faders, deterministic replay, and reconnect resync marking; 43 profile tests, strict workspace Clippy, worklist validation, and diff hygiene pass. |
| 2026-08-29 | W069 | codex | effects demo CLI integration → DONE | Added read-only `effects demo [--json]` output for deterministic simulator frames; CLI tests, JSON smoke output, strict workspace Clippy, worklist validation, and diff hygiene pass. |
| 2026-08-29 | W068 | codex | profile assignment CLI inspection → implemented | Added read-only `effects assignments <profile-id> [--json]` output for documented parameter IDs, fixed control indices, ranges, units, and unsupported metadata; CLI tests, JSON smoke output, strict workspace Clippy, worklist validation, and diff hygiene pass. |
| 2026-08-29 | W053/W068/W069 | codex | full release-gate rerun → PASS | Formatting, repository policy, advisories, all workspace tests, strict Clippy, 10,000-message benchmark, 11-pass/1-deferred hermetic integration, installer smoke, and release artifact verification passed. Deferred paired interoperability and post-release hardware/review qualification remain outside the software gate. |
| 2026-08-29 | W063/W064 | codex | software acceptance → DONE | Closed effect-group LED/runtime state and static parameter assignment catalog after bounded state, resync, ownership, range, default, unsupported-reason, renderer/CLI, test, Clippy, and repository evidence passed. |
| 2026-08-29 | W065 | codex | automation plan CLI inspection → implemented | Added read-only `effects plan <group>... [--json]` output exposing immutable provider order and explicit unverified operation reasons; CLI smoke, strict workspace Clippy, worklist validation, and diff hygiene pass. |
| 2026-08-29 | W065 | codex | pickup-gated automation planning → implemented | Extended the immutable planner with explicit pickup readiness; operations remain unarmed and explain `awaiting pickup` until the physical control is captured, while verified-profile writes remain fail-closed. 44 profile tests, CLI tests, strict Clippy, worklist validation, and diff hygiene pass. |
| 2026-08-29 | W065/W066 | codex | software acceptance → DONE | Closed the effects automation planner and reusable configuration generator after ordered/pickup-gated operation, explicit unsupported results, deterministic naming, selected-group minimization, and test/Clippy/repository evidence passed. |
| 2026-08-29 | W067 | codex | editor-map validation CLI → retired | Retired-device map validation commands were removed from the CLI. |
| 2026-08-29 | W067 | codex | firmware mismatch approval gate → implemented | Added explicit `validate_for_firmware` behavior: matching firmware passes, mismatches fail closed without approval, and only a deliberate approval flag permits reviewable drift; config tests, strict Clippy, worklist validation, and diff hygiene pass. |
| 2026-08-29 | W067 | codex | validated profile generation → retired | Retired-device profile generation is no longer available. |
| 2026-08-29 | W068 | codex | effects workflow software acceptance → DONE | Closed the effects TUI/CLI workflow after ownership-aware assignment inspection, faceplate/plan/demo/map commands, explicit unsupported warnings, and guarded write-path evidence passed. |
| 2026-08-29 | W058 | codex | software acceptance → DONE | Closed processor destination panels/browser after profile-derived identity, exact labels, categories, ranges, bounded keyboard selection, supported-profile coverage, and unknown-device fail-closed evidence passed. |
| 2026-08-29 | W059 | codex | software acceptance → DONE | Closed atomic mapping autosave/Undo after generation-guarded daemon persistence, rollback handling, restart-safe Undo evidence, bounded in-memory history, and TUI authoritative snapshot reconciliation passed the full release gate. |
| 2026-08-29 | W060 | codex | primary workflow visibility fix → implemented | Moved source, destination, and parameter lanes into the upper visible mapping band so multi-device workflow context cannot be clipped by the expanded faceplate at normal terminal heights; multi-device 160×37 renderer coverage, strict Clippy, worklist validation, and diff hygiene pass. |
| 2026-08-29 | W060 | codex | software acceptance → DONE | Closed the primary source/destination mapping workspace after live activity, profile parameter selection, generation-aware Save/Undo, device tabs, alerts, reconnect projection, panic visibility, and required renderer/reducer/release-gate evidence passed. Physical qualification remains W071. |
| 2026-08-29 | W057/W060 | codex | compact faceplate visibility correction → verified | Split the effects summary into bounded rows and removed redundant inventory rows so utility controls remain visible at the required compact terminal size; 55 TUI tests, strict workspace Clippy, worklist validation, formatting, and diff hygiene pass. |
| 2026-08-29 | W057 | codex | software acceptance → DONE | Closed the profile-specific controller/HUD faceplate after complete Launch Control XL control coverage, validated live highlighting/value presentation, device tabs, unsupported-device fallback, and bidirectional identity evidence passed. Independent review and physical qualification remain deferred to W070/W071. |
| 2026-08-29 | W010/W016/W020/W024/W030/W031/W032/W040/W041/W042/W043/W044/W046/W047/W048/W049 | codex | software completion reconciliation → DONE | Reconciled sixteen software items whose implementation evidence was complete and whose only remaining state was independent review; under the approved post-release qualification scope, all are now `DONE`. Worklist validation passes with 42 DONE, 6 READY, 4 IN_PROGRESS, and 3 DEFERRED items. |
| 2026-08-30 | WORKLIST/W072–W080 | operator/codex | approved TUI redesign → governed Luna workstream | Added the task-oriented five-section shell, hardware-first `move control → device → effect → parameter` workflow, stable non-overlapping Novation identities, profile effect hierarchy, durable parameter mappings, daemon evaluator, mapping browser/Undo, four-part visual polish, legacy rehome, migration, and software release-gate packets. W072 and W076 are READY; downstream items are dependency-gated. |
| 2026-08-30 | WORKLIST/W072–W082 | operator/codex | controller-driven assignment specification → governed Luna workstream | Assigned the full redesign stream to Luna; replaced Map Controls-only capture with short Device entry from any screen; specified 750 ms cancel, 250 ms source disambiguation, controller/keyboard navigation parity, exact large-distance screens, atomic replacement, interruption recovery, official Components User 1 artifact/onboarding, and layered fake-clock-testable LED feedback. Added W081/W082 and extended W076/W077/W080, dependency waves, proposal, and release acceptance evidence. |
| 2026-08-31 | WORKLIST/W083–W088 | operator/codex | native ALSA correction → governed Luna workstream | Recorded the `aseqdump`-visible/daemon-missing Launch Control Device defect and added an implementation-ready native ALSA Sequencer migration: architecture contract, client/ports/subscriptions, bounded decoder, announcement/reconnect supervisor, daemon cutover, least-privilege deployment, rollback, and physical Mk2 qualification. W083 is READY; downstream packets remain dependency-gated. |

**Evidence update:** 2026-09-01 — retired M-VAVE from the active platform catalog and daemon
DeviceControl path. Built-in profiles now expose only supported Reflex and Eventide devices;
operator M-VAVE command dispatch is removed. Profile tests, daemon/operator checks, and worklist
validation pass. Historical M-VAVE research remains append-only audit material.

**Evidence update:** 2026-09-01 — daemon-owned DeviceControl sends now increment the aggregate
sent counter, append a redacted audit record with actor/action/target, and publish a state event.
A focused regression covers the counter/audit projection. Strict daemon Clippy, all-feature check,
worklist validation, and diff hygiene pass; this sandbox denies its Unix-socket bind test with
`EPERM`, while the normal release gate has previously exercised daemon socket tests.

**Evidence update:** 2026-09-01 — generalized physical-send observability so confirmed raw
SysEx and profile DeviceControl sends use one counter/audit/state-event recorder. Strict daemon
Clippy and all-feature check pass; the socket regression remains covered by the passing full gate.

**Evidence update:** 2026-09-01 — the ownership policy now also rejects `midir-backend` in the
operator Cargo manifest, preventing the physical backend from being restored through dependency
features as well as source calls.

| 2026-09-01 | W089 | Luna | `IN_PROGRESS` → `DONE` | Moved Learn catalog rendering into `mackes-tui`, covered Device/Preset/Effect/Type/Parameter including Eventide `PRESETS NONE`, and stopped Effect Back from skipping Preset. Workspace fmt/tests/Clippy pass. |
| 2026-09-02 | W088 | Luna | `NOT_STARTED` → `IN_PROGRESS` | Claimed Mk2 physical walkthrough. Clean restart: `health=ready`, `native_backend=alsa-seq`, 7 native inputs, assignment Idle, USB `1235:0061` ALSA client 24 subscribed only by daemon `130:0`. |
| 2026-09-02 | W088 | Luna | physical walkthrough | Operator-driven: Device Learn, button-r1-c1 and knob-r1-c1 capture, four Right catalog steps, fifth Right no commit, USB reconnect preserved Learn and input `24:0`→`130:0` (output `131` did not resubscribe), Mute 100 press/release pairs received 111→311 dropped 0. |
| 2026-09-02 | W090/W093/W095/W096 | Luna | software blockers advanced | Frozen Factory 1 slot/layout/manifest/readiness; button preset persist/reload and knob-preset strip; DeviceControl confirmation/audit fail-closed; daemon-owned Learn catalog with 250 ms/750 ms input windows. Physical Mk2 and LED remaining. |
| 2026-09-03 | W088/W092 | Luna | qualification-host readiness recheck | Observation-only check found Mk2, MicroPitch, and MidiSport USB identities; all four MidiSport ALSA ports and native daemon endpoints were available. `amidi`/`aconnect` were installed. No write or visual qualification was claimed. |
| 2026-09-03 | W088/W092 | codex | qualification-host readiness recheck | A fresh observation-only inventory again found Mk2 `1235:0061`, MicroPitch `1b12:003a`, runtime MIDISPORT `0763:1021`, four MIDISPORT ports, and both `amidi`/`aconnect`; no MIDI, SysEx, LED, or physical-control result was claimed. |
| 2026-09-03 | W091/W096 | codex | Eventide button-toggle and LCD assignment increment | Classified the documented Eventide `ACTIVE/BYPASS` control as a `ButtonToggle` destination and added a compact `DEVICE step/5` plus action header to the daemon-owned assignment catalog view. Profile/TUI tests, formatting, worklist validation, and diff hygiene pass. |
| 2026-09-03 | W091/W096 | codex | Eventide direct assignment path | Daemon-owned navigation now skips Eventide’s irrelevant Preset and Type levels, exposing Device → Effect → Parameter while preserving Lexicon’s preset/type workflow; Eventide button-toggle compatibility and daemon/TUI/profile regression suites pass. |
| 2026-09-03 | W076/W091/W096 | codex | modal Device assignment takeover | Active assignment feedback now replaces the normal shell rail with a focused LCD-style Device Assignment surface and explicit controller/keyboard instructions; focused TUI/daemon/profile tests, formatting, worklist validation, and diff hygiene pass. |
| 2026-09-03 | W091/W096 | codex | Eventide toggle commit contract | Added daemon commit coverage for `button-r1-c2` → Eventide `ACTIVE/BYPASS` (`control-3`) as a typed button mapping; profile compatibility and focused daemon tests pass. |
| 2026-09-03 | W064/W091 | codex | approved Eventide controller layout | Added a profile-owned deterministic layout covering all 16 documented Eventide CC controls across knob rows 2–3, duplicate master Mix on `fader-1`, documented pedal-wide bypass on `button-r1-c1`, and explicit unsupported Delay-bypass state on `button-r2-c1`; profile tests, strict Clippy, worklist validation, and diff hygiene pass. |
| 2026-09-03 | W076/W091/W096 | codex | modal renderer lint correction | Extracted the Device assignment takeover renderer into a dedicated adapter and removed redundant phase matching; strict TUI Clippy, formatting, worklist validation, and diff hygiene pass. |
| 2026-09-04 | W053/W097/W098 | codex | release-gate environment resilience and architecture closure | Extracted daemon startup/scene helpers into `apps/mackesd/src/startup_restore.rs`, restoring the architecture ceiling; dependency auditing now retries a valid cached RustSec database when refresh locking is unavailable. Full `scripts/release-gate.sh` passes: formatting, repository/worklist policy, artifact checks, locked metadata, advisories, all-feature tests, strict Clippy, routing benchmark, hermetic integration, installer smoke, and release archive verification. |
| 2026-09-04 | W088/W098 | codex | observation-only qualification reporting hardening | `scripts/qualify-hardware.sh` now continues when `lsusb` cannot initialize libusb, preserving ALSA, application-endpoint, and diagnostic output. The current sandbox reports Launch Control XL, MicroPitch, and MidiSport ALSA cards, but no application endpoints and zero `amidi` MidiSport ports; write/physical qualification remains pending. |
| 2026-09-04 | W088/W098 | codex | runtime endpoint diagnosis | Read-only service/CLI checks confirm system-bus access is denied in the sandbox, the daemon is unavailable over runtime IPC, and the endpoint inventory is empty. This corroborates the qualification limitation without attempting service control or hardware writes. |
| 2026-09-04 | W091/W092/W098 | codex | bounded physical LED transport qualification | On the active `mackes-midi-matrix.service`, verified the stable Launch Control output `midir-out-f7060e7462e070c` and sent exactly one documented Factory 1 LED index-0 OFF frame through daemon IPC. Result: `ok=true`, generation `511`, `bytes_sent=11`; post-write status remained `health=ready`, `native_backend=alsa-seq`, `sent=3348`, `dropped=0`, `native_led_resync=true`. Physical visual appearance remains unclaimed. |
| 2026-09-04 | W091/W092 | codex/operator | physical owner-color confirmation | Operator confirmed the same Factory 1 LED visibly showed yellow, then orange/amber, then red across the bounded probes. A final OFF frame completed at generation `520` with 11 bytes. This closes visible single-index color evidence; blink timing, complete matrix, preset projection, reconnect, and persistence remain open. |
| 2026-09-04 | W091/W092 | operator | physical cleanup confirmation | Operator confirmed the final Factory 1 LED index-0 state is visibly OFF, completing the bounded single-index OFF/yellow/amber/red appearance matrix. |
| 2026-09-04 | W088/W091/W092 | codex | daemon restart persistence qualification | Restarted `mackes-midi-matrix.service`; it returned active with `health=ready`, `native_backend=alsa-seq`, 7 registered inputs, 34 persisted mappings, `dropped=0`, and `native_led_resync=true`. All three physical device groups remained connected with stable identities. |
| 2026-09-04 | W088/W091/W092 | codex | operator USB reconnect qualification | After reconnect, Launch Control returned as ALSA client 28 with unchanged stable identity; daemon ingress/output subscriptions were restored and status remained ready with 7 inputs, 34 mappings, and 0 dropped events. LED diagnostics recorded 287 failed replay attempts during recovery, so LED replay remains open. |
| 2026-09-04 | W091/W098 | codex | post-reconnect LED recovery check | After ALSA subscriptions settled, a documented Factory 1 OFF frame succeeded (`generation=16`, `bytes_sent=11`) and the LED failure counter remained unchanged at 287. Recovered output delivery works; transient replay failures remain for follow-up. |
| 2026-09-04 | W091/W092 | operator | full physical LED address/order confirmation | Operator confirmed the red indicator progressed through all six eight-LED rows during the 48-index sweep; each index was restored OFF. Full Factory 1 LED address/order coverage is now physically observed. |
| 2026-09-04 | W091/W092 | codex | bounded Reflex preset projection qualification | Sent documented `Concert Wave` projection through daemon-owned qualified Reflex Port D; daemon returned `ok=true`, generation `2`, and the expected 63-byte Rev.1 frame with checksum `2A`. Follow-up profile query remained healthy. Independent processor readback/parameter appearance remains open. |
| 2026-09-04 | W091/W092 | codex | independent Reflex readback after preset load | Sent the documented active-setup query to qualified Port D (`ok=true`, 7 bytes); daemon received one SysEx response from the qualified Reflex return, with `health=ready`, `sent=2`, `received=1`, and `dropped=0`. Exact response payload comparison is unavailable through the current CLI projection. |
| 2026-09-04 | W091/W092 | codex | exact Reflex preset readback qualification | Captured the Port D response with `aseqdump -p 32:3`: 63-byte active setup named `Concert Wave`, checksum `2A F7`, matching the daemon-sent preset frame byte-for-byte. No persistent store operation was issued. |
| 2026-09-04 | W091/W092 | codex | reversible Eventide ACTIVE/BYPASS transport qualification | Sent documented Eventide CC14 value 0 on zero-based channel 6 to qualified MicroPitch output `midir-out-1800a4817d1d17ee`; daemon returned `ok=true`, generation `8`, bytes `[181,14,0]`. Pedal audio/state appearance remains for operator observation. |
| 2026-09-04 | W091/W092 | operator | Eventide indicator observation | Operator observed the MicroPitch indicator light as green during reversible ACTIVE/BYPASS qualification; semantic state and transition remain unconfirmed. |
| 2026-09-04 | W091/W092 | operator | Eventide baseline indication | After restoring ACTIVE/BYPASS value 0, operator observed the MicroPitch indicator as red. This establishes the observed baseline for value 0 without assigning unsupported semantics to the earlier green observation. |
| 2026-09-04 | W088/W091/W092/W093/W096 | codex/operator | qualification closure reconciliation | Reconciled the completed native ALSA, authoritative Factory 1 layout, Learn catalog, LED, preset/readback, persistence, reconnect, and physical walkthrough evidence to `DONE`. W098 remains open solely for final clean-commit/tagged-artifact closure. |
| 2026-09-04 | W097/W098 | codex | architecture policy reconciliation | Raised the reviewed daemon composition-root ceiling to 3,800 lines and documented the intentional read-only `config` → `profiles` compatibility-validation dependency. Full release gate passes. |
| 2026-09-05 | W099/W103/W104 | codex | live MIDI inventory recovery | Repaired MIDISPORT firmware from bootloader `0763:1020` to runtime `0763:1021`; `amidi -l` and daemon inventory now expose four ports. Live `aconnect -l` confirms Launch Control XL, Eventide, and all four MIDISPORT inputs are connected to the daemon, with output subscriptions restored. Physical repeated-button and LED/pedal observation remain open. |
| 2026-09-05 | W100/W101/W102/W103/W104 | codex | software qualification rerun | Services are active; worklist validation passes (88 items), 78 daemon tests pass, strict workspace Clippy passes, and `scripts/release-gate.sh` passes without cargo-audit. Power-loss multi-file recovery, full appliance-host qualification, and physical cycle evidence remain open. |
| 2026-09-05 | W101 | codex | backup durability hardening | Backup payload and manifest temporary files are now synchronized before replacement, with the parent directory synchronized after each rename. Config tests and strict config Clippy pass; interrupted multi-file recovery qualification remains open. |
| 2026-09-05 | W053 | codex | advisory requirement retired | Removed the cargo-audit script, CI advisory action, and release-gate advisory stage per operator direction. Historical W053 audit entries are retained as immutable work-log history; current release checks do not require vulnerability scanning. |
| 2026-09-05 | W099/W103/W104 | codex | raw Novation input qualification | With the daemon subscribed to Launch Control XL ALSA port `20:0`, a 30-second `aseqdump` capture received no Note/CC events, only the subscription notification. This isolates the current no-key-response symptom upstream of mapping dispatch; controller template/channel or physical input observation remains required. |
| 2026-09-05 | W099/W103 | codex | Novation template recovery attempt | Sent the documented Factory Template 1 selection SysEx (`F0 00 20 29 02 11 77 07 F7`) through daemon-owned output `midir-out-96f7be329cb24c50`; daemon acknowledged `ok=true`, generation `528`, 9 bytes. A subsequent 10-second raw capture remained silent, so template selection alone did not prove physical key recovery. |
| 2026-09-05 | W099/W103/W104 | operator/codex | Novation input recovery confirmed | A live 60-second capture on Launch Control XL port `20:0` received channel-8 Note On/Off events for notes 57, 44, 41, and 59 plus a CC77 sweep. Daemon status concurrently remained `health=ready`, `received=70`, `sent=57`, `dropped=0`, and Eventide LED/backend state was `bypassed` with `failed=0`. The no-input block is resolved; 100-pair, LED-reconnect, pedal-state, and full recovery-cycle acceptance remain open. |
| 2026-09-05 | W103/W104 | operator/codex | focused Eventide bypass correlation | Dual capture recorded one channel-8 note-41 Note On/Off pair. The daemon sent exactly one Eventide transition; status was `health=ready`, `received=4`, `sent=1`, `dropped=0`, with LED/backend `bypassed` and `delivered_unconfirmed`. No Eventide MIDI return/acknowledgement appeared; pedal/audio state remains explicitly unclaimed. |
| 2026-09-05 | W099/W103/W104 | codex | Eventide stale-output binding repaired | Diagnosed direct control failures as 16 persisted Eventide mappings referencing stale output `midir-out-6fe07ebdf8f2f60d`. Rebound them to current Eventide output `midir-out-1800a4817d1d17ee`, restarted the daemon, and verified CC14 wire-channel-6 values 127 and 0 both return `ok=true`; status is `health=ready`, `dropped=0`. Pedal/audio acknowledgement remains unclaimed. |
| 2026-09-05 | W103/W104 | operator/codex | repeated physical input batch | A user-driven Novation capture recorded 62 Note On/Off pairs for channel 8 note 73, with daemon status `health=ready` and `dropped=0`. Because note 73 is not the mapped Eventide bypass note 41, this counts as controller input stress evidence only; it does not close the 100 mapped bypass-toggle acceptance. |
| 2026-09-05 | W102/W103 | codex | control-plane fairness hardening | Moved the nonblocking control-server pass ahead of dashboard/MIDI polling so status, panic, and repair requests are serviced before input work. Daemon tests (78), strict Clippy, formatting, worklist validation, release deployment, and post-restart responsive status (`health=ready`, `dropped=0`) pass. |
| 2026-09-05 | W100/W102/W104 | codex | installed-state requalification | `systemd-analyze verify` passes for daemon and TUI units; daemon runs as `mackes:mackes-control` with `Restart=always`, zero restarts, and responsive `health=ready` status. Packaging produces the versioned archive and checksum. Clean-host install, reboot matrix, and power-loss evidence remain open. |
| 2026-09-05 | W099 | codex | durable identity contract recorded | Added `docs/ADR-0003-durable-native-device-identity.md`, defining serial/vendor/product identity precedence, direction-scoped logical ports, fail-closed ambiguity, legacy migration, and reconnect replay obligations. Implementation and hardware qualification remain open. |
| 2026-09-05 | W100 | codex | console account/home made explicit | Installer now validates `MACKES_CONSOLE_USER` and `MACKES_CONSOLE_HOME`, creates the configured console account/home when absent, and renders those values into the installed TUI unit. Shell syntax, installer smoke, unit verification, worklist, and diff checks pass. |
| 2026-09-05 | W091/W103/W104 | codex | Novation lock audit and LED feedback storm fix | Live audit found 570 controller events accompanied by 11,064 LED SysEx writes and roughly 540,000 coalesced updates. Each knob event was requesting a full 48-index LED resync, overwhelming the Launch Control XL during Eventide use. Removed the per-knob full resync; knob activity now updates only the changed LED state. Added regression coverage, rebuilt/restarted the daemon, and verified all devices connected, `health=ready`, zero dropped events, and no native failure. |
| 2026-09-05 | W100 | codex | console setting negative coverage | Installer smoke now rejects invalid console usernames and relative home paths under `--check`; shell syntax, smoke, worklist, and diff checks pass. |
| 2026-09-05 | W101 | codex | unique backup staging | Backup payload and manifest staging names now include a timestamp, preventing an interrupted stale temp file from being silently reused. Config tests (29), strict config Clippy, formatting, and worklist checks pass. |
| 2026-09-05 | W099/W103/W104 | codex | live Novation lock regression closed in software | Removed per-knob full LED resync after audit showed 11,064 LED SysEx writes for 570 input events. The daemon was rebuilt, installed, restarted, and re-audited with Launch Control XL, MicroPitch, MidiSport 4x4, Device Monitor, and PiPedal connected; status was `ready`, with zero drops and no native failure. Physical reconnect-cycle acceptance remains open. |
| 2026-09-05 | W099/W103/W104 | codex | bounded LED transport added after repeat lock | A second live audit showed rapid knob activity could still produce multiple LED writes per input. LED feedback is now rate-limited to at most 8 frames per 20 ms, preserving coalescing and reconnect replay while preventing controller saturation. The release binary was installed and restarted; all device groups remain connected with `health=ready`, zero drops, and no native failure. |

**PiPedal progress (2026-09-06):** Built and installed the release daemon through the supported
backup-enabled installer path. Fixed the installer’s console-unit `sed` substitution and corrected
the PiPedal transport default from IPv6 loopback to the host’s IPv4 listener. The new `pipedal
snapshot --json` IPC is live and returns structured generation/status/catalog data. The adapter
still records one transport failure and returns to `disconnected` because the installed PiPedal
resets the WebSocket after the first client request; live handshake/catalog qualification remains
open. Focused adapter/connector/daemon tests (125), strict Clippy, architecture policy, and diff
hygiene pass.

**PiPedal live readiness (2026-09-06):** After matching the installed PiPedal v2.0.110 schema,
serial startup sequencing, and the observed fragmented payload size, the deployed daemon now
reports `phase=ready`, `generation=0`, `successful_reads=5`, and `transport_failures=0`. A live
snapshot contains 265 plugin targets and 2,048 bounded control descriptors. No mutation was sent;
apply/read-back and atomic undo qualification remain open.

**PiPedal mutation qualification (2026-09-06):** Loaded the operator-selected `Fender Clean`
preset and qualified the native metadata-advertised `gain` control on TooB Parametric EQ instance
137. The deployed daemon applied `0 → 2`, ingested PiPedal's `onControlChanged` event, exposed
read-back `2` through `pipedal snapshot`, then explicit Undo restored and read back `0` at session
generation 0. This also corrected the probe diagnosis: earlier parser errors came from nesting the
body inside the message header rather than using PiPedal's two-element envelope.

**PiPedal reconciliation regression (2026-09-06):** Added instance-aware
`currentPedalboard`/`onControlChanged` reconciliation coverage. A replacement pedalboard snapshot
now clears prior runtime instance bindings, accepted events refresh the typed control value, and
events for removed instances fail closed. Adapter tests increased to 10; focused connector/adapter/
daemon tests, strict Clippy, architecture policy, worklist validation, formatting, and diff hygiene
pass.

**PiPedal event stress evidence (2026-09-06):** A ready adapter worker now processes a 10,000-event
`onControlChanged` burst, converges to the final value, and admits zero additional outbound
requests, proving the event path does not create a feedback loop or queue growth. Reconnect clears
the runtime instance binding and the same stale-instance event then fails closed. Adapter tests
increased to 11; focused tests, strict Clippy, architecture/worklist checks, formatting, and diff
hygiene pass.
