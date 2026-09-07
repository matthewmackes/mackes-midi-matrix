# Device feature inventory

W145 research checkpoint, 2026-09-07. Partial inventory; not a completeness declaration.
Canonical delivery owners: W149 (Eventide), W150 (PiPedal/Firebox), W148 (Novation),
W151 (transport/platform). All browser acceptance evidence below is pending.

## Source register

EV-QRG: [MicroPitch QRG](https://cdn.eventideaudio.com/uploads/2021/10/MicroPitchDelay-QRG-Web.pdf),
part 141347 Rev A, firmware v1.0+, retrieved 2026-09-07. SHA-256
`99030f8a3da3f3c88745ee5f207065661814fc9e4af8c89427132d4b6ecab110`.
Page 3 supplies MIDI mapping.
NOV-QRG: [Launch Control XL Programmer Reference](https://fael-downloads-prod.focusrite.com/customer/prod/s3fs-public/downloads/launch-control-xl-programmers-reference-guide.pdf),
retrieved 2026-09-07. SHA-256
`98b6183c4d03fcf7b64a8db2f33f50f63ab192af689613a3b633372d814be0fe`.
EV-CODE: crates/profiles/src/eventide_micropitch.rs, profile() and cc_control(),
inspected 2026-09-07 in dirty worktree; SHA-256
`27389ca0a288c53898ab37d4437e6c4b4113068aa8d7f5f9c08ff3f634db2b5a`.
REFLEX-CODE: crates/profiles/src/lexicon_reflex.rs, SHA-256
`6efe5a71d694c5ca5a2b85fdb21f399ec71d21d0ce84ddc1ccec6471c649de48`.
NOVATION-CODE: crates/profiles/src/lib.rs, SHA-256
`d61d72fec8d85580846b26be39c8ceb7f129fb061f9fe1fea61517e1bf3c0d58`.
PP-CODE: crates/pipedal-connector/src/lib.rs Operation enum and catalog.
SHA-256 `097c255dca356a6a882f7507013c35e595694cb0f0dbe7fa73f6d9a2198fc2bf`.
FIREBOX-CODE: docs/firebox-findings.md, SHA-256
`a6150f3420d15584c68d7fb6d5bba1dc3a094e8f717a60e028b3907c6c16a36d`.
Sources describe different evidence: code proves available definitions, vendor documentation
describes device behavior; neither proves browser-to-device execution.

## Eventide CC feature reconciliation

All rows: USB/TRS MIDI; integer wire range 0–127 in EV-CODE; no query/reply definition
in current profile. UI owner W149 device inspector; persistence/readback unresolved unless
separately documented. Do not label last-sent values as measured values.
EV-QRG p.3 cross-checks the CC numbers; control presentation below is a design requirement.
Physical-unit conversion remains a separate research requirement.

| Feature ID | CC | Required editor kind | Source | Current definition | Browser evidence |
|---|---|---|---|---|---|
| eventide.expression | 4 | continuous | EV-QRG p.3; EV-CODE | present; metadata incomplete | pending W149 |
| eventide.tap | 9 | trigger | EV-QRG p.3; EV-CODE | present; metadata incomplete | pending W149 |
| eventide.active | 14 | toggle | EV-QRG p.3; EV-CODE | present; metadata incomplete | pending W149 |
| eventide.flex | 15 | toggle | EV-QRG p.3; EV-CODE | present; metadata incomplete | pending W149 |
| eventide.mix | 20 | continuous | EV-QRG p.3; EV-CODE | present; metadata incomplete | pending W149 |
| eventide.pitch-a | 21 | continuous | EV-QRG p.3; EV-CODE | present; metadata incomplete | pending W149 |
| eventide.pitch-b | 22 | continuous | EV-QRG p.3; EV-CODE | present; metadata incomplete | pending W149 |
| eventide.depth | 23 | continuous | EV-QRG p.3; EV-CODE | present; metadata incomplete | pending W149 |
| eventide.rate-sensitivity | 24 | mode-dependent | EV-QRG p.3; EV-CODE | present; metadata incomplete | pending W149 |
| eventide.pitch-mix | 25 | continuous | EV-QRG p.3; EV-CODE | present; metadata incomplete | pending W149 |
| eventide.tone | 26 | continuous | EV-QRG p.3; EV-CODE | present; metadata incomplete | pending W149 |
| eventide.delay-a | 27 | mode-dependent | EV-QRG p.3; EV-CODE | present; metadata incomplete | pending W149 |
| eventide.delay-b | 28 | mode-dependent | EV-QRG p.3; EV-CODE | present; metadata incomplete | pending W149 |
| eventide.modulation | 29 | enumeration pending | EV-QRG p.3; EV-CODE | present; metadata incomplete | pending W149 |
| eventide.feedback | 30 | continuous | EV-QRG p.3; EV-CODE | present; metadata incomplete | pending W149 |
| eventide.output-level | 31 | continuous | EV-QRG p.3; EV-CODE | present; metadata incomplete | pending W149 |

## Additional Eventide gaps and requirements

| Feature ID | Finding | Design/technical requirement | Source; owner |
|---|---|---|---|
| eventide.preset-recall | Profile defines only Preset 1; documented range is 1–127 | Full selector; verify user-slot/wire numbering with fixtures | EV-CODE; EV-QRG p.3; W149 |
| eventide.preset-store | Save mode changes PC semantics | Separate storage action and mode preconditions; never auto-retry uncertain save | EV-QRG pp.3,5; W149 |
| eventide.parameter-domains | Generic range used for all CC definitions | Add units, conversion functions, enum thresholds and mode-dependent bounds only with source proof | EV-CODE; W145/W146 |
| eventide.feedback | Queries/replies empty | Show requested/sent-unverified state; research any supported observation API | EV-CODE; W145 |
| eventide.system-setup | Remote addressing not established by current profile | Inspect channel/clock/jack/bypass-mode/catch-up setup individually; split into feature rows with remote/manual disposition | EV-QRG p.4; W145 |
| eventide.device-manager | Extended editor operations not represented | Inventory firmware, backup/restore, reset, system editing and preset transfer independently; qualify protocol before enabling | EV-QRG p.6; W145 |

The current profile encodes every CC as a numeric control. The visual editor must distinguish
triggers and toggle thresholds from continuous values rather than rendering sixteen identical sliders.
No production protocol change is authorized by a speculative physical-unit conversion.

## Remaining audit queue

| Product | Next concrete evidence action | Missing artifact | Owner |
|---|---|---|---|
| Novation | Compare programmer guide, Factory-1 manifest and input/LED encoders | One row per control/operation with exact model and page | W145/W148 |
| Reflex | Reconcile every codec operation and algorithm table against vendor revision | Parameter domains, patch/task/dump inventory and fixture links | W145/W149 |
| PiPedal | Reconcile each inventoried handler/HTTP route/event with client serializers and model side effects | Per-operation payload/readback examples and source-to-binary provenance | W145/W150 |
| Firebox | Reconcile capture-backed semantics with connector operations | Parameter identity/range/persistence evidence; unsupported transfer framing | W145/W150 |
| MIDISPORT | Find matching manufacturer manual and driver provenance | Port/firmware identity matrix | W145/W151 |
| RTP/generic MIDI | Reconcile typed route/session domains and RFC sources | All message/transform/session feature rows | W145/W151 |
| Retired C.A.B. M+ | Preserve archived research reference and retirement | No active implementation task without scope change | W145 |

PiPedal checkpoint: [per-handler audit](pipedal-server-operation-audit.md) inventories the complete
message-registration list, 24 HTTP path segments, and 37 outbound event names in the pinned local
server source, including operations absent from the 18-item connector catalog. The installed
binary and checkout both identify as PiPedal 2.0.110; the installed binary hash is pinned, but
source-to-binary provenance and per-operation payload/readback qualification remain open.

Next checkpoint: complete source pins and expand grouped research rows before W145 can close.
Unknown or unreviewed operations are not classified as product-unsupported.

## Live endpoint reconciliation checkpoint

Read-only LAN probes on 2026-09-07, using the configured Host header, returned `ok=true` from
`/api/v1/endpoints` and `/api/v1/novation`. The live inventory identified `MicroPitch Pedal`,
`Launch Control XL`, `MidiSport 4x4` (four input and four output ports), `PiPedal`, and the
MACKES virtual/monitor endpoints. The Novation projection reported identity `Mk2`, 56 physical
controls, 48 LEDs, template selection, and `led_readback=false`; its LED phase was `absent`, so
the UI must distinguish configured target state from physical observation. Lexicon Reflex and
Firebox were not present in this runtime inventory; their feature cards remain source-defined
and are not presented as connected hardware. This proves inventory reconciliation and truthful
absence handling, not exhaustive per-device operation coverage.

## Reflex feature reconciliation

The local codec exposes eight algorithms, algorithm-specific metadata, 14 Echo Rhythm values,
four patch slots, setup/register dumps, active setup, parameter requests, system tasks, algorithm
selection and bypass. The WYSIWYG editor must generate its parameter controls from
`lexicon_reflex::parameters(algorithm)`: each control displays description, bipolar/unipolar
range, effective steps and current base/effective value. Unused parameters remain absent.

| Feature | UI requirement | Safety / evidence boundary |
|---|---|---|
| Algorithms 1–8 | Algorithm selector with dynamic parameter inspector | Revalidate values after selection; source is `lexicon_reflex.rs` metadata |
| Parameters 0–9 | Range-aware numeric slider plus keyboard numeric field | Clamp only to documented range; show base and live patch-adjusted effective value |
| Echo Rhythm | Labeled enum with 14 musical values | Do not synthesize MIDI clock; show clock-dependent behavior |
| Four MIDI patches | Four source/destination/scale rows with signed percentage display | Null source/destination and source domains must be validated before write |
| Register 0–127 | Named register browser, preview, recall and store actions | Store/recall are distinct; persistent writes require explicit confirmation |
| Active/all-register setup | Read-only dump inspector and guarded import/export | 49 raw / 56 packed bytes per setup; all-register writes are hazardous |
| Bypass and tasks | Explicit bypass toggle and task status | Prefer task `72`; direct input-level writes are hazardous |
| Setup/algorithm selection | Advanced settings disclosure | Preserve unknown bytes; unsupported values fail closed |

## Novation feature reconciliation

The local profile provides a physical catalog, Factory-1 input layout, faceplate, LED indices,
template/reset encoders, feedback layers and reconnect snapshots. The browser must render the
actual 8-column control geometry and utility controls from the catalog, with searchable list
fallback. Faders and knobs are continuous; channel buttons are press/release; utility controls
retain their declared behavior. LED intent, queued delivery, failure and physical observation are
separate labels. The WYSIWYG surface must never report a successful write as visible hardware state.

Acceptance requires every catalog control to be selectable by keyboard and pointer, assignment
targets to come from the qualified catalog, and emulator input/reconnect/LED scenarios to update
the surface without opening a competing writer. Native LED or reconnect observations remain
operator evidence and cannot be inferred from emulator or daemon counters.

## Source and implementation pin requirements

Before W145 closes, record a commit/hash for each local source file and a retrieval date/hash for
each external document. For every table row, link a fixture or browser request/response scenario.
Grouped statements such as “supports presets” are insufficient when load, save, recall, reset,
query, feedback, and persistence have different semantics.

## Firebox, MIDISPORT, network, and retired-device requirements

### Atomic Ampli-Firebox V1

The current evidence identifies a vendor HID connection and read-only report monitoring. The UI
must show USB identity, hidraw path, report freshness, reconnect state, raw-report capture and
correlated fields with an “observed correlation” label. It must not expose writable parameters,
presets, IR, firmware, or factory-reset actions until each has a source-backed request, reply,
value domain, persistence effect, and recovery procedure. The known parameter ACK proves transport
only; it does not prove semantic identity or readback.

### M-Audio MIDISPORT 4x4

The UI is a transport workspace: four direction-aware port cards, stable alias, current ALSA
address, firmware/loader state, connection age, input/output activity, reconnect result and route
membership. It must not invent device parameters. Port repair selects a discovered endpoint by
stable identity and direction; a changed runtime ALSA number is not a changed logical device.
Firmware readiness and cable signal are separate states. The manufacturer’s [4x4 Anniversary
Edition product page](https://www.m-audio.com/legacy/midisport-4x4-anniversary-edition.html)
documents four MIDI inputs, four MIDI outputs, 64 discrete USB input channels, 64 discrete USB
output channels, bus-powered USB operation, and a MIDI activity indicator for each port. The
visual inventory must therefore show eight direction-specific port capabilities and per-port
activity; it must not collapse the interface into one generic MIDI endpoint. Reproducibility pin:
retrieved 2026-09-07, downloaded HTML SHA-256
`56e32867f16370b47f65e3690de677824b685e1a860f3a4022ac953ce737947c` (35,869 bytes).

### RTP-MIDI and generic MIDI

The visual routing editor must expose port aliases, direction, MIDI channel, message kind,
number/value predicates, SysEx masks, realtime predicates, priority, curve, cycle policy and
enabled state. The session inspector exposes peer identity, allowlist status, lifecycle,
sequence/reorder disposition, packet counters, reconnect backoff and last-seen time. Handshake
state is distinct from message delivery; rejected network input never reaches routing.

### Retired Two Notes C.A.B. M+

The product remains an archived inventory entry. The interface may show its documented Remote/USB
boundary and why raw HID writes are unavailable. It must not expose the older C.A.B. MIDI map or
imply that generic USB provides control semantics. Reactivation requires a new scope decision.
