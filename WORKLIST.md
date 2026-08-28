# MACKES MIDI Controller — Governed Worklist

> Canonical implementation backlog. This file is the source of truth for scope,
> sequencing, acceptance, and evidence. Executors must update it as work moves.

## 0. Document control

| Field | Value |
|---|---|
| Worklist version | 1.8 |
| Product stage | 0.1.6 public release / integration qualification |
| Target release | v1.0 |
| Primary platform | Fedora Linux 44, x86_64 |
| Language | Rust |
| Last updated | 2026-08-28 |
| Overall status | `IN_PROGRESS` |
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
- An executor other than the implementer reviews contract, safety, and test coverage.

### 0.4 Executor update protocol

Before editing code, an executor must:

1. Read this entire file, `README.md`, relevant ADRs, and all directly affected types.
2. Confirm dependencies are `DONE`; otherwise mark the item `BLOCKED` and stop.
3. Change only one item to `IN_PROGRESS`, enter owner and start date, and add a work-log row.
4. Run the existing verification suite once to establish a clean baseline.
5. Avoid changing another item's public contract without first updating its work item
   and recording an approved decision in `docs/decisions/`.

Before handing off, the executor must:

1. Run the required verification and record exact evidence.
2. Update status to `IN_REVIEW`, not `DONE`.
3. List changed files and any remaining risks in the work log.
4. Leave the tree buildable; partial experiments must remain behind disabled feature flags.

The reviewer moves the item to `DONE` only after reproducing the evidence. If review
fails, return it to `IN_PROGRESS` with a concrete finding.

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
| Dependency advisories | `cargo-audit` unavailable | Resolved: `cargo-audit v0.22.2` is installed, auto-discovered by the audit script, and part of the passing release gate. |

The retired C.A.B. device is not a blocker or release capability. Historical ledger entries are
retained only as an audit trail.

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

The initial hardware topology is fixed and serial: `Donner Arena2000 → stereo → Eventide
MicroPitch → stereo → Lexicon Reflex`. The Lexicon Reflex is the final stereo processor and
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

- Donner Arena2000: declare its effect families and MIDI controls; distinguish documented
  PC/CC mappings from community mappings; keep deep USB/BLE editor controls gated on protocol
  evidence.
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
Donner Arena2000 → stereo → Eventide MicroPitch → stereo → Lexicon Reflex (master output)
```

The chain is immutable. The Human Interface configures or bypasses blocks within each unit but
never reorders devices or places a device after the Lexicon. When multiple requested effects are
owned by the Arena2000, they are represented as enabled blocks in one coordinated Arena2000
preset/chain; unrelated Arena2000 blocks remain bypassed and hidden.

Initial capability ownership for Human Interface resolution is:

| Capability group | Default provider |
|---|---|
| Gain/preamp, gate, cabinet/IR, EQ, compression, drive/distortion, amp modeling, looper, drum machine, tuner, wah, noise reduction, acoustic/instrument simulation, ambient/modulated reverb, tape-style delay, tempo-synchronized delay, pitch shift | Donner Arena2000 |
| Hall, room, plate, spring, gated, inverse/reverse, shimmer, chorus, flanger, phaser, tremolo, vibrato, rotary, multi-tap delay, ping-pong/stereo delay, reverse delay, analog-style delay | Lexicon Reflex |
| Digital delay, slapback delay, detune/micro-pitch, pitch shift, feedback pitch effects | Eventide MicroPitch |

The ownership table is a routing contract, not proof that every listed subtype is supported by
the current firmware. Each entry must carry its evidence status in the profile. The Arena2000
deep USB/BLE editor protocol remains gated until captured and decoded. Lexicon deep controls use
the compiled bidirectional SysEx implementation; Eventide uses its documented MIDI contract.

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
  - Run format, Clippy, unit/integration/doc tests, schema freshness, dependency audit,
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

#### [~] W010 — Daemon lifecycle, persistence, and health

- **Status:** `IN_REVIEW`
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
- **Hardware/network evidence:** interoperate bidirectionally with at least two independent peers,
  one of which is not MACKES; record peer/version, network topology, packet capture summary,
  note/CC/PC/clock/SysEx cases, reconnect, and loss/reorder injection.
- **Acceptance:** Fedora 44 MACKES exchanges supported MIDI classes with the independent peers for
  eight hours with zero silent engine drops; restart/reconnect succeeds; malformed sessions are
  isolated; and no network packet can arm unsafe mode or invoke MACKES administrative commands.
- **Evidence:** `docs/decisions/ADR-rtp-midi.md` freezes the RFC 6295/AppleMIDI scope, MIDI-only
  network boundary, security policy, bounded buffers, session behavior, and interoperability
  evidence requirements. `crates/midi-engine/src/lib.rs` implements validated AppleMIDI commands,
  RTP framing/decoding, session identity, ordering/recovery, SysEx handling, allowlists, and
  reconnect behavior; hermetic integration tests pass. Independent-peer and eight-hour soak
  qualification remain external evidence.

#### [~] W016 — MIDI Learn capture and inference service

- **Status:** `IN_REVIEW`
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

#### [~] W020 — Declarative device-profile schema and loader

- **Status:** `IN_REVIEW`
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

#### [~] W024 — Hardcoded Lexicon Reflex Rev 1 codec and device service

- **Status:** `IN_REVIEW`
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

#### [~] W030 — Project/setlist/song/scene repository

- **Status:** `IN_REVIEW`
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

#### [~] W031 — Scene activation planner and executor

- **Status:** `IN_REVIEW`
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

#### [~] W032 — Performance lock, panic, and hazardous-action policy

- **Status:** `IN_REVIEW`
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

#### [~] W040 — Ratatui shell, client state, and reconnect

- **Status:** `IN_REVIEW`
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

#### [~] W041 — Live dashboard

- **Status:** `IN_REVIEW`
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

#### [~] W042 — Routing and mapping editor

- **Status:** `IN_REVIEW`
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

#### [~] W043 — Device, SysEx, and backup workspaces

- **Status:** `IN_REVIEW`
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

#### [~] W044 — Setlist editor, monitor, and diagnostics

- **Status:** `IN_REVIEW`
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

#### [~] W046 — MIDI Learn workspace

- **Status:** `IN_REVIEW`
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

#### [~] W048 — Global device visual language and color-token system

- **Status:** `IN_REVIEW`
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

#### [~] W047 — Lexicon Reflex algorithm workspaces and signal-flow diagrams

- **Status:** `IN_REVIEW`
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

#### [~] W049 — Eventide signal-flow workspace

- **Status:** `IN_REVIEW`
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

#### [ ] W054 — Physical-device inventory and identity projection

- **Status:** `IN_PROGRESS`
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
- **Luna checkpoint:** finish the typed contract and fixtures first, obtain review, then wire daemon
  projection. Stop if a stable grouping fact is unavailable.

#### [~] W055 — Per-control real-time activity stream

- **Status:** `IN_PROGRESS`
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
- **Luna checkpoint:** implement pure coalescer tests before daemon wiring; record CPU/queue evidence
  before handoff.

#### [~] W056 — ANSI rack-appliance design system

- **Status:** `IN_PROGRESS`
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
- **Luna checkpoint:** land shared primitives and golden snapshots before any device faceplate.

#### [~] W057 — Profile-specific controller and HUD faceplates

- **Status:** `IN_PROGRESS`
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
- **Luna checkpoint:** complete Launch Control golden geometry before generic/HUD variants.

#### [~] W058 — Effects-processor destination panels and parameter browser

- **Status:** `IN_PROGRESS`
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
- **Luna checkpoint:** prove catalog derivation from profile metadata before rendering browser UI.

#### [~] W059 — Atomic mapping autosave and bounded Undo

- **Status:** `IN_PROGRESS`
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
- **Luna checkpoint:** obtain ADR/schema review before production changes; implement persistence
  transaction tests before the TUI calls the command.

#### [~] W060 — Source/destination mapping workspace integration

- **Status:** `IN_PROGRESS`
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
- **Luna checkpoint:** implement reducer tests and static snapshots first, then daemon integration,
  then keyboard workflow; never combine all three in one unreviewed change.

#### [~] W061 — Local hardware, performance, and usability qualification

- **Status:** `IN_PROGRESS`
- **Owner:** codex
- **Start date:** 2026-08-28
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
  dropped activity. Independent reviewer reproduces the evidence before `DONE`.
- **Luna checkpoint:** this item begins only after W060 is reviewed; every defect is filed against
  W054–W060 and qualification resumes from the failed step after repair.

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
  - Audit dependency licenses/advisories, logs/fixtures for private data, ignored tests, TODO/FIXME,
    unsafe code, panic paths, and documentation accuracy.
  - Tag only when all critical/high defects are closed and medium defects have recorded disposition.
- **Evidence required:** signed release checklist, test reports, hardware matrix, known limitations,
  checksums, version, and rollback instructions.
- **Completion evidence:** `scripts/release-gate.sh` passes formatting, repository/worklist policy,
  locked metadata, RustSec advisory scanning, workspace tests, Clippy, routing benchmark, hermetic
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
| 8 Connected-device TUI | W054–W061 | Physical-device identity, live per-control activity, rack-appliance rendering, destination-first autosave/Undo, integrated workflow, and local hardware qualification pass. |

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
| P001 | Connected-device rack-appliance mapping TUI | Make every connected device and live control/mapping relationship clear from a distance on the Linux TTY. | W054–W061 | `APPROVED`; entered as governed work items in version 1.8 | operator |

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
| 2026-08-28 | W061 | codex | local observation qualification | `scripts/qualify-hardware.sh` passed on the local TTY seat; observed Launch Control XL, Eventide MicroPitch Pedal, Arena 2000, and all four MidiSport 4x4 MIDI ports. No physical writes or LED assumptions were made; write qualification remains pending. |
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
