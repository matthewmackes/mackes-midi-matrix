# PiPedal first-class connector — implementation handoff

Status: design for another AI to implement. Parent: W111; execution: W112–W116.
The operator requires every task to be recorded in WORKLIST.md before execution.
This document authorizes no claim that implementation or live EQ assignments exist.

## Outcome and scope

PiPedal appears as a native controllable device in Devices, Map Controls, diagnostics,
CLI and TUI. Operators browse its actual pedalboard/plugin controls, assign physical
controls, see current values and repair missing targets without JSON edits. The connector covers every operation exposed by the qualified installed PiPedal
control interface. The first
use case assigns Novation row 3 columns 4–8 to the installed EQ. Reuse the connector
for other PiPedal plugins through metadata, rather than hard-coding an EQ-only profile.

| Physical control | Destination |
|---|---|
| R3C4 | Selected EQ parameter 1 — resolve during inspection |
| R3C5 | Selected EQ parameter 2 — resolve during inspection |
| R3C6 | Selected EQ parameter 3 — resolve during inspection |
| R3C7 | Selected EQ parameter 4 — resolve during inspection |
| R3C8 | Selected EQ parameter 5 — resolve during inspection |

Preserve Eventide R1C1–2, R2C1–3, R3C1–3 and sliders 1–4. Preserve the explicit
R1C4 unassignment. Audit actual installed state: earlier conversation claims about
live assignments are not evidence. Show and remove conflicting destinations for the
five EQ knobs atomically; preserve every unrelated mapping. Do not reintroduce the
obsolete generic grouped board layout. PiPedal audio-chain placement is not specified:
read it, but do not change audio routing or the Eventide-to-Lexicon chain.

## Evidence and protocol decision

Official [architecture](https://github.com/rerdavies/pipedal/blob/main/docs/Architecture.md)
describes asynchronous JSON WebSocket control and server state events. The official
[client model](https://github.com/rerdavies/pipedal/blob/main/vite/src/pipedal/PiPedalModel.tsx)
is the protocol investigation entry point. Follow its socket, pedalboard, plugin metadata,
and MIDI binding types, then cross-check server handlers at the same source revision.
The [manual index](https://github.com/rerdavies/pipedal/blob/main/docs/Documentation.md)
and [snapshot documentation](https://github.com/rerdavies/pipedal/blob/main/docs/Snapshots.md)
provide operator context. These references are mutable; W112 must pin a revision matching
the installed release and save a protocol evidence table and sanitized fixtures.

Design decision: prefer WebSocket discovery, writes and event/read-back feedback through
a qualified adapter. Treat this as a version-dependent application protocol, not an
assumed stable public API. Determine endpoint/config discovery and authentication from
the installed version. Do not invent message names, request IDs, CC values or ports.
Unsupported versions may expose connection diagnostics but must not send control writes.
MIDI is an explicit alternative when existing PiPedal bindings are verified; never silently
send both transports for one assignment. MIDI send success alone is sent-unverified.

## Architecture and contracts

Add a reusable connector abstraction with discover, list targets, read state, set control,
subscribe, health and shutdown operations. Put PiPedal protocol code behind that boundary;
keep plugin catalog data separate from physical-controller assignment. Locate the exact
module boundaries after inspecting crates/profiles, crates/config, crates/ipc,
apps/mackesd and crates/tui. A dedicated workspace crate is appropriate if it keeps
network dependencies out of the existing MIDI engine; avoid a broad unrelated refactor.

The daemon owns one connection/session per configured instance. A worker handles network
I/O with bounded queues; the MIDI dispatch loop only enqueues validated commands. Expose
connection health, protocol support, last successful read, queue pressure, timeouts and
mapping-resolution failures over existing IPC. A connected socket is not proof of a
resolved EQ or successful parameter change.

Persist a schema-versioned connector record: stable local connector ID, endpoint,
qualified protocol/version, transport mode and credential reference if required. Never
export credentials. A target contains plugin URI, explicit instance selector, control
symbol, preset scope and expected metadata signature. Runtime instance IDs are resolved
from a fresh pedalboard snapshot; do not assume they survive reloads. Labels and array
positions are presentation only. Multiple matching instances are ambiguous, not a reason
to choose the first. Export reusable templates without volatile ALSA IDs or session IDs.

Catalog entries expose label, symbol, units, min/max/default, enum/step/log properties,
writability and current value with freshness. Unsupported metadata stays unavailable.
Map physical 0–127 input using the verified parameter domain; clamp, quantize enumerations
and respect logarithmic controls. Five knobs do not imply five gain bands: inspect the EQ
and ask the operator only if its five intended parameters cannot be established.

## State, persistence and operating behavior

Defaults proposed by this design: absolute knobs with pickup; preserve current PiPedal
values on assignment, reconnect and preset change; feedback enabled from confirmed state;
all five knobs remain parameter controls, with bypass outside this requested assignment.
Store reusable mapping templates globally but bind live targets to explicit preset/plugin
scope. Do not automatically copy MIDI bindings into every PiPedal preset.

Use a generation for each pedalboard/preset/session. Reject stale writes and responses.
A snapshot or external browser edit updates displayed state and re-arms pickup as needed.
On reconnect, rediscover and read state before accepting writes; discard pre-disconnect
commands. Never replay old knob movements or preset commands. State matching a requested
value is control-state confirmation, not proof of audible response. Distinguish pending,
confirmed, sent-unverified, stale, unavailable and failed in both interfaces.

Initial tuning budgets: 20 ms control coalescing window, 128 distinct pending targets,
one latest value per target, 2 s request deadline, 1 MiB frame ceiling, exponential
reconnect backoff from 250 ms to 10 s with jitter. Validate against observed protocol;
record justified changes. Do not retry non-idempotent actions automatically. Feed existing
bounded LED scheduling only when state changes; a knob event must never request a full
surface resync. Suppress echoed writes and ensure continuous activity cannot starve other
devices or status requests. Keep PiPedal service restart independent of matrix restart.

Configuration changes follow preview, validation, durable commit and runtime apply with
rollback/undo. Detect concurrent edits using generation checks. Use PiPedal's qualified
control interface for live changes, not direct edits to its running state files. Surface
unsaved PiPedal changes honestly; changing a value and saving a preset are separate actions.

## Work packets and qualification

Execute W112 → W113 → W114 → W115 → W116, updating status and evidence before each action.
W112 inspects installed service/version, active pedalboard and EQ metadata read-only;
records existing MIDI bindings and routing, and resolves the five musical targets.
Do not ask the operator for technical details that installed configuration can establish.
If multiple EQs or parameter sets remain plausible, record the alternatives and ask.

W113 adds fixture-backed adapter and dynamic catalog. W114 connects persistence, mapping
and operator workflows with preview/undo. W115 adds events, pickup and recovery bounds.
W116 deploys a qualified build and records physical validation. Reuse existing CLI/TUI
conventions; include list/connect/inspect, assign/preview/apply, export/import, rescan/repair
and diagnostics capabilities without requiring a second browser workflow for routine use.

Required tests: protocol fixtures and unsupported versions; metadata scaling including
log/enum controls; duplicate plugin instances; removed plugin; preset replacement; stale
responses; reconnect with pending writes; malformed/oversized frames; hung server; failed
save/undo; external browser edits; feedback echo and rapid movement across all five knobs.
Demonstrate bounded memory and continued Eventide/Lexicon dispatch under PiPedal failure.
Run relevant crate tests, strict Clippy, formatting and repository/worklist checks.

For live qualification record version, target URI/symbols, before/after mapping diff,
read-back of five physical sweeps, pickup after preset change, PiPedal restart, Novation
reconnect and a sustained simultaneous knob test. Verify no duplicate MIDI/API path and
no LED storm or lockup. Provide rollback artifacts and an accurate full board reference.
Do not close W111 until physical evidence is present; software tests alone are insufficient.

## Handoff completion contract

Deliver source, configuration migration, protocol evidence/fixtures, operator documentation,
reusable connector export, test results and deployment/rollback record. Record all deviations
in W111 before execution. The unanswered ten-question survey is not a blanket blocker:
this design supplies technical defaults; unresolved EQ identity/parameter choices remain
specific questions. This design task itself changes documentation only.

## Expanded scope: all controllable PiPedal features

Operator direction: expose all PiPedal features controllable through this connector.
The EQ assignment is only the first hardware layout. W112 must enumerate the installed
server handlers and official client operations at a pinned matching revision, rather than
infer completeness from the manual or a list of familiar features.

Create a capability matrix with one row per operation: stable capability ID, source
handler/reference, version, request/response/event contract, arguments and validation,
read/write/action classification, required permissions, confirmation policy, persistence,
CLI/TUI entry points, physical-mapping eligibility, and test/evidence reference. Mark each
supported, unavailable in this version, or pending implementation with an explicit reason.
An unimplemented exposed operation keeps the delivery open; never silently omit it.

Inventory these families and implement each operation actually available:

- Plugin controls, bypass, metadata, meters/status, plugin-specific properties and files.
- Pedalboard creation/editing, plugin insertion/removal/order, splits and routing.
- Presets and banks: browse, load, save, rename, copy, reorder, import/export and delete.
- Snapshots: select, capture/update, names and supported transition settings.
- MIDI devices, channel/port selection, learn, parameter and system bindings.
- Audio devices, channels, sample rate/buffer settings, engine status and supported restart.
- Plugin presets, uploads/downloads, file management and model selection where exposed.
- Supported host configuration, network settings, updates, shutdown/reboot and diagnostics.

This list is an investigation checklist, not evidence that every named function exists in
this installation. Unsupported features must be visible with the specific limitation.
Treat functions requiring a separate privileged service or credentials as explicit
capabilities; never bypass PiPedal permissions. Document capabilities not reachable through
the available interface, with evidence and any official alternative.

Extend the connector boundary beyond set-control: typed commands, queries, subscriptions,
file transfers and long-running jobs. Use bounded streaming for files instead of enlarging
the control-frame limit. Track progress, cancellation where supported, timeout, outcome
and reconnect reconciliation. Never automatically retry uncertain destructive operations.
Commands that alter the graph or restart audio invalidate affected cached state and require
fresh discovery. UI events must reconcile edits from PiPedal's own browser.

All supported actions must be accessible through CLI and TUI with discoverable descriptions
and appropriately typed inputs. Only suitable scalar, toggle or discrete actions appear
as physical-mapping destinations. File operations, networking, deletion and host maintenance
must not become accidental knob actions. Confirm destructive/disruptive actions at invocation;
ordinary parameter control remains immediate. Preserve unrelated device operation while
PiPedal applies disruptive changes. Designing these capabilities does not authorize executing
shutdowns, updates, routing changes or deletions during implementation qualification.

Use table-driven contract tests for every capability plus family-specific integration tests.
Qualify safe representative live operations and report destructive paths tested in fixtures
or an isolated instance. Deliver the completed capability matrix with W116; EQ-only operation
cannot satisfy W111. Record any proposed deferral for operator decision instead of reducing
this scope implicitly.

The current typed connector catalog exposes the following qualified wire operations:
`setControl`, `previewControl`, `updateCurrentPedalboard`, `setSelectedPedalboardPlugin`,
`setPedalboardItemEnable`,
`setSnapshot`, `setSystemMidiBindings`, `setInputVolume`, `setOutputVolume`, `loadPreset`,
`saveCurrentPreset`, `getAlsaDevices`, `getJackStatus`, `restart`, and `shutdown`. The
catalog is enumerable and serializes each capability using its exact wire name. Persistent,
host-wide, or disruptive mutations carry an explicit confirmation requirement; ordinary
parameter control remains immediate. Operations outside this qualified set remain pending
until an installed-version fixture supplies their request and response contracts.
