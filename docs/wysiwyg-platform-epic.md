# WYSIWYG platform epic — Luna execution specification

Status: planned; implementation and exhaustive resource audit remain open.
Requested by operator 2026-09-07. Canonical scheduling: WORKLIST.md W144–W152.
“Luna format” means the repository's §0.8 eleven-field packet, not an assumed external model format.

## Product outcome

A musician can select an actual device, see its controls in a recognizable layout, edit any
qualified addressable parameter, connect controls to destinations, build scenes, and observe
the outcome without entering JSON, raw MIDI bytes, runtime endpoint numbers, or plugin instance IDs.
A visual connection represents a control/MIDI relationship unless authoritative device metadata
establishes an audio connection. Device faceplates and parameter inspectors share one state model.

Every product feature gets a disposition: supported read/write, read-only, send-unverified,
unsupported by the product, protocol research pending, or retired. Unknown is not unsupported.
Research-pending addressable features remain open work; a disabled button does not satisfy their delivery.
Device-wide “complete” requires reconciliation against the pinned product feature inventory.

## Initial resource review and product boundaries

Reviewed repository sources: docs/web-feature-coverage.md, docs/novation-protocol-audit.md,
docs/pipedal-connector-design.md, WORKLIST.md §§2.3–2.4,
crates/profiles/src/lib.rs and the existing browser shell. This is an initial review, not an
exhaustive audit of every vendor document or the installed product versions.

| Product/connection | Technical authority and current evidence | Required useful surface and remaining research |
|---|---|---|
| Novation Launch Control XL Mk1/Mk2 | [Official downloads](https://downloads.novationmusic.com/novation/launch/launch-control-xl-mk1mk2), re-opened 2026-09-07; programmer-reference hash/pages in docs/novation-protocol-audit.md; Factory-1 manifest and ADRs 0005, 0010, 0011 | Faithful knobs/faders/buttons, assignment destinations, input activity, template identity, pickup and LED intent/delivery, reconnect policy. Resolve inconsistent historical Mk1/Mk2 labels from product/version evidence. Do not substitute Launchpad or newer XL protocols. Test using Novation emulator per operator instruction. |
| Eventide MicroPitch Delay | [Official QRG](https://cdn.eventideaudio.com/uploads/2021/10/MicroPitchDelay-QRG-Web.pdf), firmware v1.0+, part 141347 Rev A, opened 2026-09-07; eventide_micropitch.rs | Primary/secondary parameters, expression, tap, bypass, FLEX and presets. QRG identifies CC 4/9/14/15/20–31 and PC presets. Its five global settings are stored through a physical power-up System Setup flow, while six Device Manager capabilities are application-level features with no published protocol; the decomposed inventory therefore keeps them manual-only or research-pending. Persist/save mode semantics still need explicit handling. |
| Lexicon Reflex | Vendor-authored MIDI Implementation Details, part 070-10748 Rev 1; source-copy hash and protocol requirements in WORKLIST §2.3; lexicon_reflex.rs | Algorithm-dependent parameters, base/effective values, patches, presets, setup, queries, dumps and system tasks. Verify every operation against source pages and fixtures; independently reconcile algorithm tables. Busy intervals and persistent storage writes belong in the editor lifecycle. Mirror provenance requires verification; do not treat it as a new vendor publication. |
| PiPedal | Local sibling pipedal source; [official architecture](https://github.com/rerdavies/pipedal/blob/main/docs/Architecture.md); connector design and connector/adapter operation enums | Actual pedalboard/plugin metadata, control types/ranges/units, enablement, snapshots, presets, MIDI bindings, levels, hardware/status and service operations. Pin local/source/server revisions before claiming compatibility. Compare every operation in web-feature-coverage.md with server handlers; browser currently exposing three choices is not exhaustive. |
| M-Audio MIDISPORT 4x4 | Manufacturer capability page plus pinned Fedora `midisport-firmware`/`fxload` packages, udev rule, firmware hashes and live `0763:1021` four-port enumeration in docs/device-feature-inventory.md | Stable per-port identity/direction, aliases, connections, activity, reconnect and firmware readiness. It is a MIDI transport; do not invent DSP controls. Browser evidence must distinguish firmware readiness from cable state. |
| RTP-MIDI / generic MIDI | Existing MIDI engine, IPC and endpoint schemas; RFC 6295 and qualified AppleMIDI session reference required | Peer/session lifecycle, routing, channel/message filters, transforms, timing/health and reconnection. Keep session addresses separate from persisted identity. Inventory all supported message classes, not only CC. |
| Two Notes C.A.B. M+ | [Official manual](https://media.two-notes.com/product_manuals/en/legacy/hardware/torpedo/torpedo_cab_m_plus_user_guide.pdf) opened 2026-09-07; worklist explicitly retires this device | Retired inventory record only unless reintroduced by operator. Archive Remote/USB research and known gaps; do not enable older C.A.B. MIDI commands on M+. This epic does not silently reverse retirement. |

## Evidence inventory contract

W145 creates one row per feature and source, not one row per device. Required columns:
stable feature ID; product/model/firmware; connection transport; source URL/path, revision,
retrieval date and hash/commit; section/page or symbol; operation direction; value type,
range/unit/enumeration; request/reply/notification semantics; persistence and side effects;
current implementation path; canonical UI owner/control; support disposition; missing evidence;
owning work item; fixture and browser acceptance evidence.
Cross-check manual tables, profile definitions, connector enums, IPC commands, configuration
schema and live capability catalogs. Differences must produce explicit rows and tasks.
Remote-control documentation does not by itself prove an implemented wire protocol.

## Interaction and architecture requirements

- Live workspace: device readiness and meaningful current activity; technical dumps in disclosure panels.
- Device workspace: recognizable code-native faceplate plus accessible parameter inspector,
  readable labels/units, current versus requested values, capability/source details and search.
- Mapping: select a physical control, select device/block/parameter, preview behavior, commit,
  replace/delete/undo and recover interrupted drafts. Preserve existing user assignments.
- Routing: select named ports, connect/disconnect visually, edit all predicates/transforms/priority/
  curve/cycle policy, validate and apply atomically. Include keyboard equivalent for each gesture.
- Scenes: compose actions and ordering visually, preview recall and durable changes, manage setlists.
- Settings: visual forms for all editable schema fields and advanced device operations.
- Daemon remains transport and generation authority. Browser drafts survive polling, navigation,
  disconnects and rejected saves; conflicts require reconciliation. Preserve fields not touched by
  an editor. Apply success requires authoritative response; show unverified sends accurately.
- Shared controls support both themes, responsive layouts, focus, accessible names, ranges and
  non-color status cues. Avoid pretending send-only devices supply live physical feedback.
- No external asset dependency; reuse code-native geometry and the current component tokens.
  Browser must never open a competing device writer.

## Known browser concerns to verify before extension

The recently added route board rebuilds routes using only source, destination and enabled,
and refresh assigns returned routes unconditionally. Verify preservation of advanced route fields
and drafts before trusting it with existing configurations. Numeric endpoint fields are not the
final musician-facing endpoint chooser. Existing Rust HTTP tests and asset checks do not exercise
browser interactions or prove rendered usability. Prior “goal complete” did not verify this broader scope.

## Completion evidence

Each feature-inventory row resolves to a working control and runtime/browser evidence or an
evidenced product limitation/retirement. Research gaps are not silently closed.
Exercise read, edit, apply, rejection, undo/reload and reconnect for each operation family;
assert requests against qualified fixtures and authoritative state. Test dual-client conflicts,
draft preservation, all routes' round trips, offline behavior, keyboard use, both themes and narrow
screens. Novation physical interaction is simulated with the designated emulator; do not claim
native LED observations. Prior four-hour soak acceptance remains recorded; add only verification
needed for changed behavior. Deliver screenshots and exact browser scenarios alongside the
coverage reconciliation, then package and verify the installed LAN interface.

## Design requirements added for the gap review

### Screen composition

Desktop (at least 1200 CSS pixels): persistent workspace navigation, device/port list at left,
central stage, and selected-object inspector at right. At 768–1199, collapse the device list into
a drawer and allow the inspector to replace the stage. Below 768, stack list/stage/inspector with
explicit back navigation and preserved selection. No essential control depends on hover.
Use 320 CSS pixels as the narrowest acceptance viewport; also inspect 768 and 1440 widths.
Global header holds connection state, current workspace and theme. Panic remains directly accessible;
backup, firmware and raw diagnostics belong to their relevant workspaces, not a global button wall.

A device card shows model, alias, transport, connection state and last observation age.
A faceplate groups controls by real physical geometry where documented. Logical-only layouts
must be labeled. Selection exposes name, destination, value, units, support state and applicable
actions in the inspector. Provide a searchable list equivalent for every faceplate control.
Do not draw cabinets, processors or audio cables as an authoritative signal chain from MIDI routes.

### Editing behavior

Sliders and knobs have editable numeric equivalents and visible values; keyboard arrows step
through the metadata domain, with Home/End selecting bounds. Enums display vendor labels, not
numeric encodings. Continuous controls may preview only if the capability explicitly permits it;
otherwise edits stay in a draft until Apply. Escape cancels an uncommitted gesture.
Toggle, momentary and trigger are distinct types. Trigger controls must not masquerade as stored
boolean state. Disable an unavailable action with a visible reason and the relevant recovery action.
Pending, unsaved, applied, rejected, stale, disconnected and unknown values have text as well as color.

Connection creation supports selecting a source then a destination, with optional pointer dragging.
Show port direction/type and reject incompatible candidates before Apply; server validation remains
authoritative. Deleting a visual edge removes that draft route only. Selection must not emit MIDI.
Scene action cards show target, operation, arguments, order/dependencies, delay and failure policy.
A recall preview identifies unresolved targets and permanent changes before submission.

## Technical requirements added for the gap review

### Capability and state contract

W146 must publish concrete versioned schemas and golden examples for:
- Capability: stable product/feature IDs; supported firmware/protocol range; operation kind;
  tagged value type; min/max/step/default/unit or enum options; readable/writable/subscribable;
  preview support; persistence class; source reference; qualification and unavailable reason.
- Target: stable configured device ID, logical port and feature ID; optional plugin URI/symbol
  and explicit instance selector. Resolve volatile transport/plugin addresses only in the daemon.
- Value: observed value and timestamp, requested value, freshness state and origin.
  No readback means observed value is absent, not copied from the request.
- Draft: base generation, complete original object, edited paths and validation results.
  Untouched fields round-trip exactly; new unknown fields never disappear on browser save.
- Mutation: request ID, target, typed action/payload, expected generation and required confirmation;
  reply includes operation ID, resulting generation and lifecycle state.
- Event: monotonically increasing sequence, affected stable IDs, generation and typed changes.
  A sequence gap requires resnapshot without discarding local edits.

Schema validation must reject non-finite values, out-of-domain integers/enums, missing targets,
unsupported commands and stale generations. Empty numeric input is invalid, never zero.
Distinguish transport failure, protocol rejection, timeout/unknown outcome and confirmed result.
Retry only with established idempotency semantics; do not automatically repeat preset stores or triggers.
Capabilities are supplied by the qualified adapter/profile, not inferred from UI controls.

### Browser state and transport

Separate authoritative snapshots, local drafts and operation status. Poll/stream updates refresh
unmodified values, mark overlapping edits conflicted, and never replace an unsaved draft.
Navigation and reconnect preserve draft selection and content. Warn before document exit with
unsaved changes. Explicit discard reloads current authoritative state.
Use one daemon-owned adapter/session per configured connection, bounded input/output queues and
coalescing for permitted continuous previews. Preserve final value and ordered discrete operations.
Publish queue overflow, reconnect and freshness through the same state contract.
Document actual payload limits, rates and timeouts from current schemas/adapters in W146; no
implementation proceeds with an unspecified bound or a fabricated universal device rate.
Browser gesture traffic must respect these advertised limits.

### Device-specific completion boundaries

Novation: model-qualified control/LED geometry, template configuration and feedback ownership.
Eventide: complete documented CC/PC surface, MIDI channel and preset addressing conventions;
separate locally configured system setup from remotely addressable setup and unknown editor APIs.
Reflex: per-algorithm names/domains, base versus modulation-adjusted values, four MIDI patches,
supported query/dump/setup/system tasks, busy guards and distinct temporary versus stored edits.
PiPedal: dynamic plugin metadata, version-qualified full operation catalog, target re-resolution
after pedalboard changes and explicit ambiguous/missing instance repair.
MIDISPORT/RTP/generic endpoints: direction-aware port aliases, stable reattachment, message-class
coverage, session health and endpoint repair. Do not imply a configured logical destination proves
a cable is connected.

### Acceptance scenarios for Luna

1. Load a route with every advanced field, edit one endpoint, apply and reload: all other fields match.
2. Edit a value, receive poll/events and navigate away/back: the draft survives; stale Apply conflicts.
3. Disconnect during a write: UI reports unknown outcome; reconnect obtains actual state before retry.
4. Load a product with an undocumented feature: show the research gap; emit no invented request.
5. Add a plugin/control to qualified metadata: correct typed editor appears without UI code changes.
6. Select and map each Novation control using keyboard and pointer; verify emulator input/LED/reconnect.
7. Use both themes at all specified widths: readable controls, visible focus, no hidden actions or
   horizontal page overflow; test zoom and screen-reader labels alongside rendered screenshots.
8. Compare complete feature inventory to working browser actions and recorded requests/results.
   HTTP unit tests, presence of buttons and a green ledger check alone cannot satisfy this scenario.

### Execution sequencing and open decisions

Start W145, then W146. Complete shared controls in W147; W148/W149/W150 can proceed in parallel
only with disjoint claimed file ownership. W151 shares shell/state code and must coordinate commits
with those streams. W152 is the integration and installed-delivery gate.
Existing W129–W143 remain owners of their platform contracts; new tasks add product completeness
and evidence requirements rather than duplicating independent implementations.
Open protocol facts belong to named feature rows with source/provider and next action. Research
work may proceed without hardware writes. A task with unresolved protocol prerequisites cannot
claim full product coverage; other product tasks continue independently.

## Current implementation evidence

As of 2026-09-07, the deployed port-8081 shell includes visual mapping control tiles, an advanced
route board, scene cards, endpoint/device cards, a product-aware feature board with evidence
labels and explicit read-only boundaries, capability status chips, and active-workspace
navigation. Connected endpoint identities populate a destination picker, product feature entries
open only guarded qualified editors, and route drafts survive polling with explicit discard
confirmation. The Novation faceplate now renders all 24 knobs, 24 buttons, and 8 faders reported
by the qualified 56-control profile. The release web binary was rebuilt and installed;
`mackes-web.service` accepts the
configured LAN Host header. Health, endpoint, Novation, and capability projections returned
successfully.

`scripts/release-gate.sh` passed, including workspace tests, strict Clippy, web coverage/schema/
assets, Novation emulator qualification, throughput, hermetic integration, installer smoke, and
release checksums. This proves software and deployment integrity; it does not prove native LED
observation, visual human review, or exhaustive product protocol coverage. Those remain open in
W145–W152.

The live `/api/v1/capabilities` projection currently reports implemented operation families and
`unsupported.remaining_mutations=W138-W139`. The Devices workspace displays that gap
explicitly; an absent mutation control is not evidence that the product has no addressable feature.

Frontend reproducibility pins for this checkpoint: `index.html`
`0ea4718dbcc2d55a7dee7ea3f1e1309b56ba6c4ffc95f7580424691195a1861b`, `app.js`
`8b78a00bc6d0643f52356d36cd97661e2061f4bd4576a56cd4bfb09f802009cb`, `app.css`
`21ae2a9bd48c1e66858e9d636bb0776a87b5e31701be300abd7b73ac81a90fe9`, and current release binary
`d834dd925a4abb6a3480ade65275077a79f8751925428b35c56c8c4092b4374e`.
