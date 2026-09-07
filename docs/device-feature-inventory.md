# Device feature inventory

W145 research checkpoint, 2026-09-07. Partial inventory; not a completeness declaration.
Canonical delivery owners: W149 (Eventide), W150 (PiPedal), W148 (Novation),
W151 (transport/platform). All browser acceptance evidence below is pending.

## Source register

EV-QRG: [MicroPitch QRG](https://cdn.eventideaudio.com/uploads/2021/10/MicroPitchDelay-QRG-Web.pdf),
part 141347 Rev A, firmware v1.0+, retrieved 2026-09-07. SHA-256
`99030f8a3da3f3c88745ee5f207065661814fc9e4af8c89427132d4b6ecab110`.
Page 3 supplies MIDI mapping; pages 4–6 distinguish power-up System Setup, preset behavior,
and Device Manager-only operations.
NOV-QRG: [Launch Control XL Programmer Reference](https://fael-downloads-prod.focusrite.com/customer/prod/s3fs-public/downloads/launch-control-xl-programmers-reference-guide.pdf),
retrieved 2026-09-07. SHA-256
`98b6183c4d03fcf7b64a8db2f33f50f63ab192af689613a3b633372d814be0fe`.
EV-CODE: crates/profiles/src/eventide_micropitch.rs, profile() and cc_control(),
inspected 2026-09-07 in dirty worktree; SHA-256
`27389ca0a288c53898ab37d4437e6c4b4113068aa8d7f5f9c08ff3f634db2b5a`.
REFLEX-CODE: crates/profiles/src/lexicon_reflex.rs, SHA-256
`6e765c65a1fad621b9e636ba7c38efb11791ff838f35cc779b8fd32c4f0958d8`.
NOVATION-CODE: crates/profiles/src/lib.rs, SHA-256
`d61d72fec8d85580846b26be39c8ceb7f129fb061f9fe1fea61517e1bf3c0d58`.
NOVATION-FACTORY1: docs/mackes-launch-control-xl-mk2-factory1-manifest.json, SHA-256
`31bb2516c070290e8a8fd1a14ac750d49351f097e47e040127509964a3333829` (contract 1.0.1).
PP-CODE: crates/pipedal-connector/src/lib.rs Operation enum and catalog.
SHA-256 `097c255dca356a6a882f7507013c35e595694cb0f0dbe7fa73f6d9a2198fc2bf`.
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
| eventide.system.midi-channel | Omni or channel 1–16; changed in power-up System Setup and stored as changed | Manual-only configuration in current evidence; do not expose as a MIDI-addressable editor control | EV-QRG p.4; audited W145 |
| eventide.system.midi-clock | On/off, factory default off; changed in power-up System Setup and stored as changed | Manual-only configuration in current evidence; its receive/send semantics are not specified sufficiently for a browser control | EV-QRG p.4; audited W145 |
| eventide.system.bypass-mode | Buffered/relay/DSP+FX/kill-dry toggles; factory default buffered | Manual-only configuration in current evidence; do not conflate it with CC 14 active/bypass | EV-QRG pp.3–4; audited W145 |
| eventide.system.catch-up | On/off, factory default off; suppresses knob changes until the physical knob reaches the stored value | Manual-only configuration in current evidence; document its effect on physical editing, not as remote value readback | EV-QRG pp.4–5; audited W145 |
| eventide.system.expression-jack | EXP, EXP+AUX, triple AUX, MIDI box, or MIDI TRS | Manual-only configuration in current evidence; surface transport/setup guidance only, with no inferred detection or mutation | EV-QRG p.4; audited W145 |
| eventide.manager.firmware-update | Device Manager can update pedal firmware; a TAP button + TAP footswitch boot chord enters software-update mode | Protocol unpublished; exclude from the MIDI profile and require a separately qualified Device Manager integration before enabling | EV-QRG p.6; audited W145 |
| eventide.manager.backup-restore | Device Manager can back up/restore the entire device to a file | Protocol and file format unpublished; research pending, no browser operation authorized | EV-QRG p.6; audited W145 |
| eventide.manager.factory-reset | Device Manager can restore factory settings; ACTIVE footswitch + ACTIVE button at boot is also documented | Destructive and protocol unpublished; no browser operation authorized without protocol, confirmation, and physical recovery evidence | EV-QRG p.6; audited W145 |
| eventide.manager.system-edit | Device Manager can edit system settings | Protocol unpublished; this does not make the five System Setup settings MIDI-addressable | EV-QRG pp.4,6; audited W145 |
| eventide.manager.preset-transfer | Device Manager can import/export presets | Protocol and file format unpublished; keep separate from documented MIDI PC load/store semantics | EV-QRG pp.3,6; audited W145 |
| eventide.manager.preset-editor | Device Manager can view, edit, and organize presets | Protocol unpublished; no arbitrary query/write behavior may be inferred from the application feature | EV-QRG p.6; audited W145 |

The current profile encodes every CC as a numeric control. The visual editor must distinguish
triggers and toggle thresholds from continuous values rather than rendering sixteen identical sliders.
No production protocol change is authorized by a speculative physical-unit conversion.

## Remaining audit queue

| Product | Next concrete evidence action | Missing artifact | Owner |
|---|---|---|---|
| Novation | Reconcile browser coverage with the pinned Factory-1 and LED inventory | Browser-to-daemon assignment/template/pickup/reconnect scenarios and physical LED observation | W148 |
| Reflex | Reconcile typed browser editors with the pinned codec/algorithm inventory | Browser-to-codec scenarios, busy/storage lifecycle and physical readback | W149 |
| PiPedal | Reconcile each inventoried handler/HTTP route/event with client serializers and model side effects | Per-operation payload/readback examples and source-to-binary provenance | W145/W150 |
| MIDISPORT | Reconcile browser cards with the pinned manufacturer/Fedora/runtime matrix | Per-port activity, route membership and cable-state browser evidence | W151 |
| RTP/generic MIDI | Reconcile browser controls with the pinned route/session inventory | Browser request/response, reconnect, conflict, and peer-interoperability scenarios | W151 |

PiPedal checkpoint: [per-handler audit](pipedal-server-operation-audit.md) inventories the complete
message-registration list, 24 HTTP path segments, and 37 outbound event names in the pinned local
server source, including operations absent from the 18-item connector catalog. The installed
binary and checkout both identify as PiPedal 2.0.110. The installed executable is byte-identical
to the current Release build artifact, and Ninja reports that artifact current against the clean
pinned checkout. Per-operation payload/readback qualification remains open.

Eventide System Setup and Device Manager groups are now decomposed above. The official QRG proves
their user-facing behavior but publishes no remote protocol for them; that is an explicit
manual-only or protocol-research-pending disposition, not an omitted feature.

Next checkpoint: finish PiPedal per-operation source/fixture reconciliation before W145 can close.
Unknown or unreviewed operations are not classified as product-unsupported.

## Live endpoint reconciliation checkpoint

Read-only LAN probes on 2026-09-07, using the configured Host header, returned `ok=true` from
`/api/v1/endpoints` and `/api/v1/novation`. The live inventory identified `MicroPitch Pedal`,
`Launch Control XL`, `MidiSport 4x4` (four input and four output ports), `PiPedal`, and the
MACKES virtual/monitor endpoints. The Novation projection reported identity `Mk2`, 56 physical
controls, 48 LEDs, template selection, and `led_readback=false`; its LED phase was `absent`, so
the UI must distinguish configured target state from physical observation. Lexicon Reflex and
the retired Firebox was not present in this runtime inventory or active product catalog. This proves inventory reconciliation and truthful
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

The normative per-algorithm names, used/unused parameter slots, polarity, ranges, effective steps,
documented wire steps, Echo Rhythm enum, patch domains, busy intervals and safety classes are
fully enumerated in WORKLIST §2.3. The following table reconciles every public codec family to
that source contract and its focused profile evidence:

| Codec family | Supported domain | Named evidence and UI boundary |
|---|---|---|
| Algorithm registry/selection | Algorithms 1–8; algorithm-specific legal parameters and initialized active setup | `reflex_algorithm_registry_matches_manual_order_and_numbers`, `reflex_parameter_metadata_excludes_unused_slots_and_bounds_values`; browser must discard stale parameter domains |
| Controller normalization | Normalized 0–127 input snapped to documented range/step | `pcm70_translation_values_are_on_reflex_wire_steps`; never send unused slots |
| PCM70 translations | Five named, bounded translations projected as complete active setups | `pcm70_catalog_translates_to_valid_named_reflex_setups_and_sysex`; label as translations, not native Reflex presets |
| Request type 3 | Active, register 0–127, packed/nibblized parameter 0–127, all-register bank | `reflex_packing_checksum_and_nibbles_match_contract`; query is read-only and silence is not a typed device error |
| Packed parameter type 2 | Channel 0–15, parameter 0–127, 16-bit packed value | Same codec regression; encoder now rejects parameter 128 instead of masking it |
| Nibblized parameter type 5 | Channel 0–15, parameter 0–127, four MIDI-safe nibbles | Same codec regression; preferred write form and oversized parameter rejection |
| System task type 6 | Store/recall register 0–127; bypass 0/1 | Same codec regression; all oversized arguments reject instead of wrapping; store is persistent/hazardous |
| Active setup type 0 | Exactly 49 raw / 56 packed setup bytes plus checksum | Packing/setup round-trip and hardware-active-setup fixture; sent state is not observed state |
| Stored register type 1 | Register 0–127 plus one complete setup | Register-frame round-trip; stored-register writes require busy/storage lifecycle |
| All registers type 4 | Exactly 128 × 49 raw / 7,168 packed bytes | Register-bank and all-frame round-trip; destructive write requires backup/arm/15-second busy guard |
| Setup fields | Algorithm, ten little-endian values, 16-byte name, four patch sources/destinations/scales | `ReflexSetup`/`ReflexPatch` round-trip in codec regression; preserve unknown name bytes |
| MIDI patches | Four source/destination/signed-scale rows | Patch round-trip and domain rejection; show base and effective values separately |
| Packing/checksum | Seven raw bytes per MSB group; `sum(packed) & 0x7f` | Packing corpus, corrupt checksum, length and non-MIDI rejection |
| Typed decoder | Active/register/all-register setup, packed/nibblized parameter and task variants | `decode_message` assertions in codec regression; malformed/unknown frames remain errors |

This closes W145's Reflex source/operation reconciliation. It does not close W149: typed browser
editors, algorithm-change invalidation, operation results, persistent-store confirmation, busy
projection and physical readback remain required there.

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

The profile table is the sole runtime Factory-1 input authority. Its exact ordered groups are:

| Physical controls | Input tuple on zero-based channel 8 | Behavior | Feedback address |
|---|---|---|---|
| `knob-r1-c1` … `knob-r1-c8` | CC 13–20 | continuous 0–127 | LED 0–7 |
| `knob-r2-c1` … `knob-r2-c8` | CC 29–36 | continuous 0–127 | LED 8–15 |
| `knob-r3-c1` … `knob-r3-c8` | CC 49–56 | continuous 0–127 | LED 16–23 |
| `button-r1-c1` … `button-r1-c4` | Note 41–44 | nonzero press / zero release | LED 24–27 |
| `button-r1-c5` … `button-r1-c8` | Note 57–60 | nonzero press / zero release | LED 28–31 |
| `button-r2-c1` … `button-r2-c4` | Note 73–76 | nonzero press / zero release | LED 32–35 |
| `button-r2-c5` … `button-r2-c8` | Note 89–92 | nonzero press / zero release | LED 36–39 |
| `fader-1` … `fader-8` | CC 77–84 | continuous 0–127 | none; proxy policy only |
| `utility-1` … `utility-4` | Note 105–108 | nonzero press / zero release | LED 40–43 |
| `utility-5` … `utility-8` | CC 104–107 | nonzero press / zero release | LED 44–47 |

The audit found and corrected a stale manifest/ADR claim that used contiguous button notes
41–48 and 57–64. Physical captures and the production profile instead establish the four banks
above. `scripts/verify-artifacts.py` now checks every ordered MIDI-number list, so an artifact
with the old numbers cannot pass merely because its count and tuples are unique.

| Operation | Exact software contract | Evidence / remaining boundary |
|---|---|---|
| Identify model | USB `1235:0061` is Mk2; Launchpad/HUI roles do not own this surface | Classifier and endpoint-role tests; replacement/ambiguity stays fail-closed |
| Select Factory 1 | Hardware slot 1 corresponds to wire template byte 8 | Profile constant and manifest; selected template has no readback |
| Decode input | Exact unique tuple resolves to stable physical ID; zero-value release is not an activation | Complete-layout and every-control resolver tests |
| Background LED | Bounded template 0–15, index 0–47, 7-bit value | NOV-QRG pp. 3–4 and golden encoder tests; host send is not visible confirmation |
| Batch LED | One template and 1–48 unique bounded index/value pairs | Batch golden/rejection tests; bounded worker delivery required |
| Traditional Note/CC LED | Template-local Note On/Off or CC update | NOV-QRG; kept distinct from background SysEx |
| Toggle state | Template 0–15, button index 0–23, explicit on/off | NOV-QRG and golden tests |
| Reset | Template-scoped controller reset message | NOV-QRG; no device state readback |
| Desired-state replay | Coalesced desired frame survives disconnect and invalidates sent cache | LED surface/reconnect tests; native appearance remains W126 |
| Assignment/pickup overlay | Error > result > pressed > pickup > base priority | Overlay policy tests; browser lifecycle remains W148 |

This closes the W145 source reconciliation for all 56 physical inputs, 48 feedback addresses,
and the implemented operation families. W148 retains browser workflow/emulator acceptance and
W126 retains native faceplate/reconnect observation.

## Source and implementation pin requirements

Before W145 closes, record a commit/hash for each local source file and a retrieval date/hash for
each external document. For every table row, link a fixture or browser request/response scenario.
Grouped statements such as “supports presets” are insufficient when load, save, recall, reset,
query, feedback, and persistence have different semantics.

## MIDISPORT, network, and retired-device requirements

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

Fedora 44 loader provenance was recorded from installed, RPM-verified packages on 2026-09-07:

| Layer | Authority and exact evidence | Meaning |
|---|---|---|
| Firmware package | `midisport-firmware-1.2-38.fc44.noarch`, Fedora-signed source RPM `midisport-firmware-1.2-38.fc44.src.rpm` | Owns the loader image, 4x4 image and udev rule |
| Loader package | `fxload-2008_10_13-34.fc44.x86_64`, Fedora-signed source RPM `fxload-2008_10_13-34.fc44.src.rpm` | Udev helper for Cypress FX/FX2 firmware download |
| Udev rule | `/usr/lib/udev/rules.d/42-midisport-firmware.rules`, SHA-256 `e8dca7f55a0220690c0ddff721beb6ec7f71bff5a2c31110250a4c39929adca7` | On USB `0763:1020`, invokes `fxload` with the loader and 4x4 images |
| Loader image | `/usr/lib/firmware/MidiSportLoader.ihx`, SHA-256 `7c2f261aef1a09091ee16e04fc38898066ecfc81a83b8f9f3791635cb0574b3d` | First-stage firmware supplied by the Fedora package |
| 4x4 image | `/usr/lib/firmware/MidiSport4x4.ihx`, SHA-256 `275cc317c9f2a162eb15bdf1a7b07627e1ab87f95e00b9a8ae661668ea7cd3e2` | Device-specific runtime firmware supplied by the Fedora package |
| Loader binary | `/usr/bin/fxload`, SHA-256 `17535593e2257bbc852b7f655001cd8b293de65bf236c7fd86123286f3503237` | Executable referenced by the udev rule (`/sbin` resolves compatibly on Fedora) |
| Application identity | `MIDISPORT_4X4_LOADER_USB=0763:1020`, `MIDISPORT_4X4_RUNTIME_USB=0763:1021` | Exact pre/post identities; no name-only inference |
| Live device | USB `0763:1021`, device revision 1.30, no serial string; ALSA client 28 ports 0–3 | Firmware transition completed; all four logical ports enumerated |
| Native driver | Fedora RT kernel `7.1.13-300.vanilla.fc44.x86_64+rt`, `snd-usb-audio` | Kernel owns the class-compliant runtime ALSA endpoints |

`rpm -V fxload midisport-firmware` produced no differences. Runtime enumeration exposed MIDI 1–4,
each subscribed to the daemon application input. This proves loader/runtime and port enumeration,
not cable continuity or external-device response. Serial-less durable identity remains an explicit
operator binding with logical port and direction; four ports must never collapse to one alias.

This closes W145's MIDISPORT manufacturer/driver/firmware provenance row. W151 retains visual
per-port activity, route membership, repair and cable-state acceptance.

### RTP-MIDI and generic MIDI

The visual routing editor must expose port aliases, direction, MIDI channel, message kind,
number/value predicates, SysEx masks, realtime predicates, priority, curve, cycle policy and
enabled state. The session inspector exposes peer identity, allowlist status, lifecycle,
sequence/reorder disposition, packet counters, reconnect backoff and last-seen time. Handshake
state is distinct from message delivery; rejected network input never reaches routing.

Reproducibility pins (retrieved 2026-09-07): IETF/RFC Editor
[RFC 6295](https://www.rfc-editor.org/rfc/rfc6295.html), SHA-256 of the canonical text
`a6d0a020308f205fd37bf5e2d76c0ef1d1ae7871df8efab205fa9c865ddeafe0`; and
[RFC 3550](https://www.rfc-editor.org/rfc/rfc3550.html), SHA-256 of the canonical text
`4c210e9434b5b4c029e8536ad8991f3709bc3cbfa0999e951bcb4c2143c539e8`.
RFC 3550 is authoritative for RTP v2 framing, sequence, timestamp and SSRC fields. RFC 6295 is
authoritative for the MIDI command section, running status, System Common/Realtime commands,
SysEx segmentation/cancellation and recovery journals. `AppleMIDI` invitation/synchronization
commands are implemented from fixture-backed compatibility behavior, not claimed as RFC 6295.

Local source pins: `crates/domain/src/lib.rs`
`9eed54c1b1aa1f61528d53876c79b9afa83290ea9a29756781e7a86b3ba27624`;
`crates/midi-engine/src/lib.rs`
`9669f3798d5ebc3ef42845370ce4cd912cc8e019008fc2528730e9ce953c7510`; and
`crates/midi-engine/src/rtp.rs`
`d5d93e249c5afd2ab97ca757e6ade50d5c7cbf60013384934234793c3bb29d6e`.

| Feature ID | Current authoritative contract | Required visual control / state | Evidence boundary |
|---|---|---|---|
| `midi.endpoint` | Stable alias/ID, input or output direction; volatile ALSA address is runtime-only | Named direction-aware endpoint picker, live address and repair state | Durable identity and reconnect tests; never persist enumeration order |
| `midi.message-class` | Note on/off, poly pressure, CC, program, channel pressure, pitch bend, SysEx, System Common and Realtime | Typed class selector; show only fields valid for that class | Domain enum and decoder tests |
| `route.channel-class` | Optional channel and message-class filters | Optional channel 1–16 and typed class selectors | Router compound-filter test |
| `route.number-range` | Inclusive 0–127 note/controller/program range | Paired bounded numeric inputs | `router_applies_number_value_realtime_and_masked_sysex_predicates` |
| `route.value-range` | Inclusive 0–16,383 velocity/pressure/value/pitch range | Metadata-bounded paired numeric inputs | Same predicate regression; class determines meaningful maximum |
| `route.realtime` | Clock, Start, Continue, Stop, Active Sensing or Reset exact match | Enumerated selector with transport-impact label | Domain enum plus exact-match router regression |
| `route.sysex-mask` | Equal-length 1–1,024-byte 7-bit pattern and mask; framing excluded | Hex pattern/mask editor with length and 7-bit validation | Masked SysEx router regression; no hardware transmission in editor tests |
| `route.priority` | Lower `u16` values execute first | Bounded priority input and ordering preview | Router enabled/priority regression |
| `route.curve` | Linear, square or square-root CC shaping | Three-choice curve selector and before/after preview | Endpoint-preservation and curve-order tests |
| `route.cycle` | Disabled by default; explicit cycles remain bounded by hop limit | Hazard-labelled opt-in plus cycle/path preview | Cycle rejection/bounded-cycle regression |
| `route.enabled` | Disabled routes do not evaluate | Toggle with explicit disabled state | Router enabled/priority regression |
| `rtp.peer-policy` | Explicit nonempty, duplicate-free allowlist; maximum 64 peers | Peer/address list with rejection reason | `PeerAllowlist` validation and inbound-policy tests |
| `rtp.session` | Disconnected, Invited or Established; token, remote SSRC/name | Lifecycle timeline with identity and reconnect action | `RtpMidiPeer` identity/reset tests; delivery is not handshake success |
| `rtp.packet` | RTP v2 header plus bounded nonempty MIDI payload | Packet counters and malformed/drop diagnostics | RFC 3550/6295 parsers and golden framing tests |
| `rtp.sequence` | In-order, forward gap, duplicate or late within a 1–1,024 packet window | Counters and last disposition; no silent loss | `SequenceTracker` wrap/gap/late/duplicate tests |
| `rtp.jitter` | Capacity-bounded timestamp/sequence ordering | Capacity, depth, overflow and drain state | `JitterBuffer` ordering/overflow tests |
| `rtp.sysex` | Complete or segmented SysEx, bounded locally to 4,096 bytes | Progress/abort/error state; never present partial data as applied | RFC 6295 plus reassembler/framing tests |
| `rtp.recovery-journal` | RFC-defined feature, not implemented by the current engine | Explicit unsupported capability; no toggle | Source absence is an implementation gap owned by W151 |

This inventory closes W145's grouped RTP/generic-MIDI research row. It does not prove that the
current browser exposes these contracts: lossless editors, stale-save conflict handling, runtime
session projection and independent-peer interoperability remain W151/W152 acceptance work.

### Retired Two Notes C.A.B. M+

The product remains retired and has no active interface entry. Its official
[C.A.B. M+ user guide](https://media.two-notes.com/product_manuals/en/legacy/hardware/torpedo/torpedo_cab_m_plus_user_guide.pdf)
was retrieved 2026-09-07 (1,847,759 bytes; SHA-256
`d758529e08c33bdfeecbdd30f4c5049a1494269d9cc763f5fc7155fda6d7326b`). The manual records the
Remote/USB product boundary but is not a protocol authorization. Current profiles, schemas,
commands and the browser contain no C.A.B. product identity, older C.A.B. MIDI map, or raw-HID
write path. Reactivation requires an explicit scope decision and new protocol qualification.

This closes W145's retirement-record check. Historical WORKLIST entries remain append-only audit
history and W027 remains the authoritative removal task.
