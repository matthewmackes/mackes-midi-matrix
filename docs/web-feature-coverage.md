# Web feature coverage ledger

This is the canonical inventory for the port-8081 application. `scripts/check-web-coverage.py`
validates its IDs, semantics, and canonical ownership. A page may own multiple capabilities;
an editor owner may not be duplicated. `planned` rows are explicit implementation dependencies,
not claims that the browser feature already exists.

| ID | Source | Availability | Semantics | Canonical owner | API contract | Control | Persistence | Evidence |
|---|---|---|---|---|---|---|---|---|
| WEB-001 | `crates/ipc/src/lib.rs:Command::Health` | implemented | read | page:Live | IPC health snapshot | status cards | derived | IPC tests |
| WEB-002 | `crates/ipc/src/lib.rs:Command::Panic` | implemented | write | editor:live-emergency | IPC panic command | confirmation button | none | scene-engine tests |
| WEB-003 | `crates/ipc/src/lib.rs:Command::Mappings` | implemented | read/write | editor:map-controls | typed mapping request; `POST /api/v1/mappings` | mapping table | config document | daemon tests; web IPC regression |
| WEB-004 | `crates/ipc/src/lib.rs:Command::Routes` | implemented | read/write | editor:routing | route request; `POST /api/v1/routes` | route table | routes sidecar | daemon tests; web boundary |
| WEB-005 | `crates/ipc/src/lib.rs:Command::Scenes` | implemented | read/write | editor:scenes-setlists | scene request; `POST /api/v1/scenes` | scene editor | config document | scene-engine tests; web boundary |
| WEB-006 | `crates/ipc/src/lib.rs:Command::Endpoints` | implemented | read | page:devices | endpoint snapshot | inventory table | derived | MIDI-engine tests |
| WEB-007 | `crates/ipc/src/lib.rs:Command::Rescan` | implemented | write | editor:devices-recovery | rescan command | rescan action | none | IPC/daemon tests |
| WEB-008 | `crates/ipc/src/lib.rs:Command::PiPedal` | partial | read/write | editor:devices-pipedal | PiPedal request; `POST /api/v1/pipedal` | catalog/control form | config document | adapter tests; web boundary; live qualification open |
| WEB-009 | `crates/ipc/src/lib.rs:Command::DeviceQuery` | implemented | read | editor:devices-profiles | profile query request | query form | none | daemon tests |
| WEB-010 | `crates/ipc/src/lib.rs:Command::DeviceControl` | implemented | write | editor:devices-profiles | confirmed control request | bounded control form | audit/config | daemon tests |
| WEB-011 | `crates/ipc/src/lib.rs:Command::Sysex` | implemented | write | editor:system-sysex | confirmed SysEx request | framed byte editor | audit | daemon tests |
| WEB-012 | `crates/ipc/src/lib.rs:Command::Backups` | implemented | read/write | editor:system-backups | backup request | preview/apply/restore | backup artifacts | config tests |
| WEB-013 | `crates/ipc/src/lib.rs:Command::Migrate` | implemented | write | editor:system-migration | migration request | dry-run/apply | config document | config tests |
| WEB-014 | `crates/ipc/src/lib.rs:Command::Learn` | implemented | write | editor:map-controls | learn request | candidate chooser | mapping draft | IPC/daemon tests |
| WEB-015 | `crates/ipc/src/lib.rs:Command::Monitor` | implemented | read | page:Live | bounded monitor stream | filters/pause | capture export | TUI tests |
| WEB-016 | `crates/ipc/src/lib.rs:Command::UnsafeMode` | implemented | write | editor:live-emergency | safety request | timed confirmation | audit only | scene-engine tests |
| WEB-017 | `crates/ipc/src/lib.rs:Command::Configuration` | implemented | read/write | editor:system-settings | configuration request; `GET /api/v1/configuration` export | validated settings form and export | config document | config tests; web shell |
| WEB-018 | `crates/pipedal-connector/src/lib.rs:OperationCapability` | partial | read/write | editor:devices-pipedal | versioned web API TBD | capability picker | config document | connector tests; W131 open |
| WEB-019 | `apps/mackesd/src/led_surface.rs:LedDiagnostics` | partial | read/write | editor:devices-novation | daemon snapshot/LED action | bounded LED test | none | daemon tests; physical evidence open |
| WEB-039 | `apps/mackesd/src/lib.rs:novation_device_snapshot` | implemented | read | editor:devices-novation | `GET /api/v1/novation` | Novation device refresh/projection | persisted device policy | daemon/web emulator tests |
| WEB-020 | `crates/midi-engine/src/lib.rs:RtpMidiPeer` | implemented | read/write | editor:routing | RTP session request TBD | peer/session form | config document | MIDI-engine/testkit tests |
| WEB-021 | `crates/ipc/src/lib.rs:Command::Hello` | implemented | read | page:System | IPC version handshake | connection details | derived | IPC tests |
| WEB-022 | `crates/ipc/src/lib.rs:Command::Subscribe` | implemented | write | page:Live | sequenced event subscription | reconnect control | none | IPC tests |
| WEB-023 | `crates/ipc/src/lib.rs:Command::Validate` | implemented | read | editor:system-settings | configuration validation request; `GET /api/v1/validation` | validation action | none | config tests; web boundary |
| WEB-024 | `crates/ipc/src/lib.rs:Command::Configuration` | implemented | read/write | editor:system-settings | configuration request | settings editor and export | config document | config tests; web shell |
| WEB-025 | `crates/ipc/src/lib.rs:Command::Assignment`; `AssignmentAction::Snapshot` | implemented | read/write | editor:map-controls | assignment-session request; `GET /api/v1/assignment`; `POST /api/v1/assignment` | guided assignment wizard | config document | IPC/daemon/TUI/web tests |
| WEB-026 | `crates/ipc/src/lib.rs:Command::Shutdown` | implemented | write | editor:system-service | shutdown request | confirmed service action | none | daemon tests |
| WEB-027 | `crates/ipc/src/lib.rs:Command::Health` | implemented | read | page:Live | health projection | status cards | derived | daemon tests |
| WEB-028 | `crates/ipc/src/lib.rs:Command::Snapshot` | implemented | read | page:Live | complete state snapshot | state panels | derived | daemon/TUI tests |
| WEB-029 | `crates/tui/src/lib.rs:UiCommand::NextScene` | implemented | write | editor:scenes-setlists | scene request | next-scene action | config document | TUI tests |
| WEB-030 | `crates/tui/src/lib.rs:UiCommand::PreviousScene` | implemented | write | editor:scenes-setlists | scene request | previous-scene action | config document | TUI tests |
| WEB-031 | `crates/tui/src/lib.rs:UiCommand::Panic` | implemented | write | editor:live-emergency | IPC panic command | panic control | audit only | TUI/daemon tests |
| WEB-032 | `crates/tui/src/lib.rs:UiCommand::OpenPalette` | implemented | read | page:System | local navigation | command palette | none | TUI tests |
| WEB-033 | `crates/tui/src/lib.rs:UiCommand::Quit` | implemented | write | page:System | local lifecycle | exit action | none | TUI tests |
| WEB-034 | `crates/tui/src/lib.rs:UiCommand::MoveUp` | implemented | read | page:Live | local navigation | keyboard navigation | none | TUI tests |
| WEB-035 | `crates/tui/src/lib.rs:UiCommand::MoveDown` | implemented | read | page:Live | local navigation | keyboard navigation | none | TUI tests |
| WEB-036 | `crates/tui/src/lib.rs:UiCommand::MoveLeft` | implemented | read | page:Live | local navigation | keyboard navigation | none | TUI tests |
| WEB-037 | `crates/tui/src/lib.rs:UiCommand::MoveRight` | implemented | read | page:Live | local navigation | keyboard navigation | none | TUI tests |
| WEB-038 | `crates/tui/src/lib.rs:UiCommand::OpenWorkspace` | implemented | read | page:Live | local navigation | workspace selector | none | TUI tests |

## Explicit gaps

- The port-8081 HTTP process, shared API schema, diagnostics bundle, event poll, and initial
  mapping/rescan/panic web operations are implemented; remaining operation families and full
  acceptance are tracked by W135–W143.
- The bundled frontend shell and responsive navigation are implemented, while complete Carbon
  component coverage and feature-workspace parity remain open under W133–W139.
- Rows marked `partial` require the owning work item to complete the missing browser contract;
  they must not be treated as browser-ready capabilities.

## Delivery dependency ledger

These work items are intentionally visible dependencies of the exhaustive browser surface. The
coverage checker requires every ID, so a renamed or newly split epic must update this inventory.

| Work item range | Dependency scope |
|---|---|
| W117–W126 | First-class Novation device platform, protocol evidence, LED encoding, reconnect, diagnostics, software qualification, and deployment. |
| W127–W128 | Novation device boundary and complete assignment workflows. |
| W129–W143 | Web epic: inventory, architecture, service, shell, live operation, assignment, routing, scenes, devices, configuration, synchronization, diagnostics, verification, and release. |

Tracked IDs: `W117`, `W118`, `W119`, `W120`, `W121`, `W122`, `W123`, `W124`, `W125`, `W126`,
`W127`, `W128`, `W129`, `W130`, `W131`, `W132`, `W133`, `W134`, `W135`, `W136`, `W137`, `W138`,
`W139`, `W140`, `W141`, `W142`, `W143`.

## Configuration-field classification

The schema is the authority for persisted fields. Collection nodes are edited through their
canonical editor; runtime IDs, health and transport state remain derived and are never persisted.

| Schema path | Classification | Reason / canonical editor |
|---|---|---|
| `schema_version` | read-only | Migration-owned compatibility marker; System displays it. |
| `settings.active_project`, `settings.active_scene` | editable | Scenes & Setlists selection and durable recall state. |
| `settings.learn_input_alias` | editable | Map Controls chooses the observational Learn source. |
| `settings.dashboard_midi_bindings` | editable | Live owns dashboard trigger bindings and validation. |
| `settings.launch_control_template` | editable | Devices owns template identity and assignments. |
| `settings.novation_device`, `.stable_id`, `.template`, `.auto_reapply`, `.feedback_enabled` | editable | Devices owns persisted Novation selection and reconnect/feedback policy. |
| `settings.pipedal_mappings` | editable | Map Controls owns physical-to-PiPedal mappings. |
| `endpoints[].id`, `name`, `stable_id`, `vendor_id`, `product_id`, `serial`, `logical_port`, `direction`, `role` | editable | Devices owns explicit aliases and repair; discovered runtime addresses are not inferred. |
| `projects[].id`, `projects[].scenes[]` | editable | Scenes & Setlists owns projects and scene ordering. |
| `scenes[].id`, `name`, `category`, `actions[]` | editable | Scenes & Setlists owns scene content and activation planning. |
| `scene_action.id`, `description`, `unsafe_action`, `depends_on` | editable | Scenes editor owns validated action plans and safety annotations. |
| `profiles[].id`, `profiles[].version` | read-only | Profile catalog is supplied by Profiles; System may inspect compatibility. |
| `setlists[].id`, `setlists[].projects[]` | editable | Scenes & Setlists owns ordered performance sets. |
| `learned_mappings[].source_alias`, `message_kind`, `channel_policy`, `number`, `raw`, `destination`, `mode`, `enabled`, `priority`, `filters[]` | editable | Map Controls owns Learn results, filters, enablement and priority. |
| `control_mappings[].id`, controller/source/destination identity fields | editable | Map Controls owns the canonical mapping editor and atomic commit. |
| `control_mappings[].behavior` | editable | Map Controls owns ranges, inversion and curve behavior. |
| `control_mappings[].enabled`, `profile_version` | editable | Map Controls owns activation and profile compatibility. |
| `control_mapping_drafts[].id`, `step`, `physical_control_id`, `destination` | editable | Map Controls owns interrupted drafts and resume/discard. |
| `settings.pipedal_mappings.mappings[].physical_control_id`, `plugin_uri`, `symbol`, `scope` | editable | Map Controls owns stable PiPedal mapping identity; runtime instance IDs are deliberately excluded. |
| `control_mappings[].controller_profile`, `source_endpoint`, `source_kind`, `source_channel`, `source_number`, `destination_endpoint`, `destination_profile`, `destination_effect`, `destination_parameter` | editable | Map Controls owns the complete source/destination identity and channel contract. |
| `control_mappings[].behavior.source_range`, `destination_range`, `invert`, `curve` | editable | Map Controls owns validated scaling and response behavior. |
| `launch_control_template.assignments[].index`, `channel`, `kind`, `needs_review` | editable | Devices owns physical MIDI assignment and fail-closed review state. |
| runtime endpoint address, connection state, readiness, queue counters, LED delivery state | derived | Daemon/device snapshots are authoritative; browser cannot persist or edit these values. |

## Routing and transform coverage

Routing’s single editor must expose these existing typed domains through the shared route draft:
`RoutePredicate::NumberRange`, `RoutePredicate::ValueRange`, `RoutePredicate::Realtime`,
`RoutePredicate::SysExMask`, `TypedMappingKind::Note`, `TypedMappingKind::ControlChange`,
`TypedMappingKind::ProgramChange`, and the route fields `enabled`, `priority`, `curve`,
`predicates`, and `allow_cycle`. Unsupported combinations remain validation errors from the
daemon rather than browser-local alternatives.

## Profile capability coverage

The current built-in profile capability IDs are `chorus`, `delay`, `detune`, `gated_reverb`,
`hall_reverb`, `inverse_reverb`, `modulation`, `multi_tap_delay`, `pitch_shift`, `plate_reverb`,
`resonator`, `reverb`, `room_reverb`, `spring_reverb`, and `flanger`. Devices owns their
availability and evidence presentation; unsupported operations remain explicit gaps.

## Nested operation coverage

The canonical editors expose all typed operation variants: mapping operations
`MappingOperation::Snapshot`, `MappingOperation::Draft`, `MappingOperation::Activate`,
`MappingOperation::Replace`, `MappingOperation::Behavior`, `MappingOperation::Enabled`,
`MappingOperation::Delete`, and `MappingOperation::Undo`; PiPedal operations
`PiPedalOperation::Snapshot`, `PiPedalOperation::Apply`, and `PiPedalOperation::Undo`; and
assignment actions `AssignmentAction::Start`, `AssignmentAction::ControlCaptured`,
`AssignmentAction::Up`, `AssignmentAction::Down`, `AssignmentAction::Enter`,
`AssignmentAction::Back`, `AssignmentAction::ConfirmReplace`, `AssignmentAction::Commit`,
`AssignmentAction::Cancel`, `AssignmentAction::Retry`, `AssignmentAction::Resume`,
`AssignmentAction::Interrupt`, `AssignmentAction::Succeed`, `AssignmentAction::Fail`, and
`AssignmentAction::Discard`. The daemon remains the validator and generation authority.

The PiPedal connector operation catalog is also inventoried: `Operation::SetControl`,
`Operation::PreviewControl`, `Operation::UpdateCurrentPedalboard`,
`Operation::SetSelectedPedalboardPlugin`, `Operation::SetPedalboardItemEnable`,
`Operation::SetPedalboardItemUseModUi`, `Operation::SetPedalboardItemTitle`,
`Operation::SetSnapshot`, `Operation::SetSnapshots`, `Operation::SetSystemMidiBindings`,
`Operation::SetInputVolume`, `Operation::SetOutputVolume`, `Operation::LoadPreset`,
`Operation::SaveCurrentPreset`, `Operation::GetAlsaDevices`, `Operation::GetJackStatus`,
`Operation::Restart`, and `Operation::Shutdown`.
