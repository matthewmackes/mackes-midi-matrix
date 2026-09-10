# Full-schema configuration boundary

Status: mixed design/implementation record, 2026-09-07. The operation envelope
schema is published at `schemas/configuration-boundary.schema.json`; durable
draft storage, operation recovery, and full HTTP/IPC wiring remain W154–W158
implementation work under W139/W146.

## Scope and ownership

The daemon owns one complete configuration service. Forms, the advanced JSON5
editor, CLI, imports/restores, mapping editors and project/setlist editors must
use its validation and commit boundary. HTTP adapts this service; it must not
maintain an independent configuration model or write the file itself.

Complete means every persisted field of `ConfigDocument`, including all nested
settings, endpoints, projects/scenes/actions, profile references, setlists,
learned mappings, control mappings and inactive mapping drafts. The field catalog
must record type, default/nullability, bounds, references, editability and runtime
effect. Profile reference editing is not profile payload installation; external
profile assets retain their own validation and lifecycle. Runtime/network settings
not stored in this document must be catalogued with their actual owner and linked
operation, not invented as new fields or silently omitted from the interface.

Saving configuration must not recall a scene, transmit MIDI/SysEx, write a device
template, install a profile or restart a service. Those remain separately armed
operations. Firebox remains retired. No new device reverse engineering is needed.

## Existing boundary and gaps

The Rust model is the current persisted-data authority, with semantic validation
in the config crate. The published JSON Schema is incomplete: examples include
`settings.default_providers`, `ControlMapping.destination_channel`,
`SceneAction.destination/message` and `ProfileRef.endpoint_alias`. Schema parity
must cover every nested field, enum, default, range and reference—not just these
examples. `learned_mapping.channel_policy` also needs an actual schema.

The existing configuration command handles setlists and learned mappings only;
these have different replace/add semantics. Its acknowledgment is not a full
configuration snapshot. Global daemon generation is not a configuration revision.
`save` locks and atomically replaces the file, but accepts no expected revision;
a load/modify/save caller can therefore overwrite another writer's change.
Persistence serializes JSON, losing JSON5 comments and formatting.

## Proposed contract

Version the configuration contract independently from document `schema_version`.
Publish exact IPC and HTTP types together in W154, using the W146 envelope and
W140 operation identity conventions. Proposed HTTP resource root is
`/api/v1/configuration`; draft subresources are additive. Do not change existing
mutation semantics silently: retain a documented compatibility adapter through
the new service, or explicitly version and retire the old route.

| Operation | Required input | Authoritative result |
| --- | --- | --- |
| Read | None | Complete document, source JSON5, schema version, config revision, runtime revision and capabilities |
| Create/update draft | Base revision; complete structured document or JSON5 source; draft revision on update | Draft ID/revision and preserved input; parse/field diagnostics do not discard rejected text |
| Validate | Draft ID/revision | Schema and semantic diagnostics by JSON Pointer; source spans when available; no persistence or runtime effects |
| Diff | Draft ID/revision | Base-to-candidate field changes, reference impacts and live/restart/explicit-operation classification; stale-base flag |
| Apply | Draft ID/revision, expected config revision, operation ID, explicit confirmation | Durable operation result with persisted revision, runtime revision, restart-required fields and errors |
| Operation read | Operation ID | Same durable outcome after timeout, reconnect or daemon restart |

All envelopes deny unknown operation fields. Unsupported schema versions and
unknown document fields are rejected for apply, never silently dropped; original
draft text remains recoverable. Migration is explicit and previews its changes.
Structured and raw editors share one draft, with optimistic draft revision checks.
Forms must preserve fields they do not expose. Preserve JSON5 text/comments through
edits using a syntax-aware representation; if a particular edit cannot preserve
them, require an explicit normalization preview/confirmation. Never silently
round-trip an existing file through lossy typed serialization.

Use opaque string revision tokens and string IDs; do not route 64-bit identities
through JavaScript numbers. Config revision identifies exact persisted bytes
(including external text edits), not daemon reads or unrelated activity. Missing
file has an explicit initial revision. Drafts have bounded durable storage and
documented expiry; expiry cannot masquerade as a successful save.

Set document, nesting, string, collection, draft-storage and response limits in
the contract. IPC currently limits an entire encoded frame to 1,048,576 bytes;
the HTTP body limit cannot equal that and assume envelopes or escaping fit.
Check encoded frame size before dispatch. Provide bounded/paginated diffs and
diagnostics. If full reads cannot fit, define bounded snapshot transfer before
advertising support; never truncate a configuration document.

Map malformed input to 400, unsupported media to 415, excessive payload to 413,
semantic rejection to 422, revision/idempotency conflicts to 409, absent resources
to 404 and unavailable daemon to 503. Use structured serialized errors, not
interpolated JSON. HTTP must inspect daemon outcomes, not turn every IPC reply
into HTTP 200. Admission/authentication follows existing local/LAN policy.

## Commit, runtime and recovery

1. Under the shared writer lock, reread disk and compare expected revision;
   validate the complete candidate and all references. A stale base returns a
   conflict and retains the draft, with no automatic overwrite/rebase.
2. Prepare runtime changes without side effects and record a durable operation
   intent containing operation ID, request digest and old/new revision. Reusing
   an ID with different input conflicts; identical retries recover one outcome.
3. Persist candidate via temporary file, fsync, atomic rename and directory fsync;
   retain bounded backups. Recovery reconciles the intent against disk revision
   at each crash point. Never claim failure implies the old file survived when
   rename succeeded but a later durability step failed.
4. Install the prepared live state and durably record the outcome. Expose persisted
   and runtime revisions separately. Runtime failure after persistence is a
   persisted-but-not-applied outcome, not success or an invisible rollback;
   restart reconciliation retries safe state loading and reports pending work.

All in-process and supported offline writers must share this lock/revision
protocol, including backups/restore, mapping saves and project/scene mutations.
Audit actual call sites in W155. Out-of-band editors do not honor advisory locks:
document unsupported concurrent writes, detect changes on reread and before
replace where possible, and surface divergent disk state. Do not promise atomic
CAS against arbitrary external writers. Import/restore is a staged candidate with
the same conflict/diff/apply contract, not a privileged bypass.

Runtime classification must be explicit per field and consumer. Updating active
project/scene references selects persisted state only; any resulting device action
requires its separate operation. Endpoint resolution and mapping rebuilds must
not substitute guessed devices or expose partially rebuilt state. Fields not
supported for live adoption are honestly marked restart-required or unsupported.

## Delivery and minimum verification

W153 establishes model/schema parity; W154 freezes the proposed protocol with an
ADR and fixtures; W155 implements transactional persistence; W156 integrates
runtime consumers and all writers; W157 exposes the HTTP/editor boundary; W158
checks the integrated workflow. These feed W139/W146 completion and W151
integration; they do not depend on those parent tasks being DONE.

Use focused automated cases: complete non-default round trips, unknown fields and
versions, reference errors, two competing writers, external disk changes, duplicate
operation IDs, failures before/after rename, restart recovery, runtime adoption
failure, rejected-draft recovery, JSON5 preservation, maximum encoded IPC frames,
and HTTP status mapping. No hardware qualification or deployment is required to
close this design. Implementation evidence should be one concise record of changed
behavior and targeted test results, reusing the sources below.

## Governance source record

Authority: project implementation and governance (primary local source).
Inspected 2026-09-07 at immutable Git commit
`b6fde0d57ed164fb728ce6c46dc822828b5f2840` (revision identifier replaces a file hash).
Evidence class: source inspection, not runtime or hardware qualification.

| Source and exact section/symbol | Established fact / limit |
| --- | --- |
| `crates/config/src/lib.rs`: `ConfigDocument`, `Settings`, `ControlMapping`, `SceneAction`, `ProfileRef`, `save`, `SaveLock` | Persisted fields; lock/rename behavior; no expected-revision argument; JSON serialization |
| `schemas/config.schema.json`: root properties and `$defs` | Published schema coverage and omissions relative to Rust |
| `apps/mackesd/src/main.rs`: `Command::Configuration` branch | Existing partial mutation semantics; not a full draft service |
| `apps/mackes-web/src/main.rs`: `configuration_operation` | Existing HTTP forwarding/admission boundary, not full-schema management |
| `crates/ipc/src/lib.rs`: `MAX_FRAME_BYTES` | 1 MiB encoded frame ceiling |
| `WORKLIST.md`: W139, W140, W146; `CONTRIBUTING.md` | Existing feature ownership, public-contract ADR requirement and provenance policy |

The envelope shape and limits in `schemas/configuration-boundary.schema.json` are
implemented contract artifacts. The commit, runtime, and recovery behavior above
remain proposed until the corresponding service and integration tests land.
Implementation tasks should update this record only when an authoritative fact
changes; no repeated source discovery is required.
