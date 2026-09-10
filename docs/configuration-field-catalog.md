# Configuration field catalog

This catalog is the W153 inventory checkpoint. The Rust root and schema root were
compared on 2026-09-07; both contain the same nine persisted roots. Nested fields
remain owned by their Rust types and are listed here to make the remaining audit
explicit.

| JSON path | Rust owner | Schema | Editability | Runtime effect |
| --- | --- | --- | --- | --- |
| `schema_version` | `ConfigDocument::schema_version` | integer | migration controlled | gates parsing and migration |
| `settings` | `ConfigDocument::settings` / `Settings` | object | structured + raw | global policy and provider settings |
| `endpoints` | `EndpointAlias` | array | structured + raw | endpoint identity resolution |
| `projects` | `Project` | array | structured + raw | project and scene catalog |
| `profiles` | `ProfileRef` | array | structured + raw | profile destination bindings |
| `setlists` | `Setlist` | array | structured + raw | ordered project recall |
| `learned_mappings` | `LearnedMapping` | array | structured + raw | MIDI Learn dispatch |
| `control_mappings` | `ControlMapping` | array | structured + raw | hardware mapping registry |
| `control_mapping_drafts` | `ControlMappingDraft` | array | structured + raw | resumable inactive drafts |

Nested `settings` ownership is: `active_project`, `active_scene`, `default_providers`,
`pipedal_mappings`, `novation_device`, `profile_bindings`, and `learned_filters`.
Nested scene action ownership is `SceneAction::{kind, destination, message,
depends_on}`. Profile endpoint aliases are owned by `ProfileRef::endpoint_alias`,
and mapping destination channels by `ControlMapping::destination_channel`.

The schema uses `additionalProperties: false` at every governed object boundary.
Unknown fields therefore fail validation and are not silently discarded. The next
W153 checkpoint is a non-default fixture that populates every row and nested owner,
followed by invalid-fixture coverage for bounds, references, and version handling.

Source authority: `crates/config/src/lib.rs` and `schemas/config.schema.json`,
inspected 2026-09-07. Evidence class: matching source and schema inspection; nested
fixture evidence remains pending.
