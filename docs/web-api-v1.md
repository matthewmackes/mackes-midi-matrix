# Web API v1 boundary

The port-8081 process is a thin HTTP adapter. The daemon remains the sole authority for MIDI,
device lifecycle, validation, durable commits, operation IDs, and state revisions. HTTP handlers
never open ALSA, write configuration files, or execute processor operations directly.

## Request model

`GET /api/v1/capabilities` returns the web adapter’s explicit implemented and unsupported route
inventory. `GET /api/v1/state`
returns the daemon snapshot response. Read-only routes also include `/health`, `/endpoints`,
`/routes`, `/scenes`, `/devices`, `/novation`, `/assignment`, `/mappings`, `/monitor`, `/backups`, and `/configuration`.
The assignment route returns the daemon-owned session/catalog snapshot without changing assignment state;
`POST /api/v1/assignment` forwards validated assignment actions and returns the authoritative session result.
Its request is defined as `$defs.assignment_request` in the bundled schema, including the complete
bounded action catalog and optional physical/destination identities.
Mutations use
`POST /api/v1/operations`; `rescan`, explicitly confirmed `panic`, and explicitly confirmed
`device_control` forward validated payloads through the existing local IPC boundary and return an
operation ID. Device control requires bounded `profile_id`, `control`, `channel` (0–15), `value`
(0–65535), and `destination` fields; unknown fields and missing confirmation fail closed.
The capabilities response advertises this operation as `implemented_with_confirmation`.
Its `payload` is defined by `$defs.device_control_payload` in the bundled schema.
Operation responses include the daemon result object for authoritative status and diagnostics.
`POST /api/v1/mappings` forwards the strict generation-checked `MappingRequest` contract for
atomic draft, activate, replace, behavior, enable, delete, and undo operations.
The Map Controls workspace selects a physical control from the Novation faceplate, edits the
selected mapping's ranges/curve/inversion, replaces its destination, or enables/disables/deletes
it; every mutation carries the displayed daemon generation and refreshes from the authoritative
registry after success. Replace payloads use the complete validated mapping record; the browser
does not synthesize missing source identity or behavior fields.
`GET /api/v1/pipedal` requests a typed connector snapshot, and `POST /api/v1/pipedal` forwards the
strict `PiPedalRequest` contract to the daemon-owned connector; unsupported connector capabilities
remain explicit in the daemon response.
The Devices workspace exposes the connector's `Snapshot`, confirmed `Apply`, and confirmed `Undo`
operations; the browser does not maintain plugin or mapping state independently.
`POST /api/v1/routes` forwards bounded route replacement or explicit undo JSON with an explicit
generation (and hop limit for replacement); the daemon remains responsible for graph validation,
authorization, persistence, and atomic install.
`POST /api/v1/scenes` forwards exactly one bounded scene selection or `next`/`previous` navigation
action to the daemon-owned scene catalog.
`POST /api/v1/backups` supports confirmed `create`, `restore`, and `portable_import` actions. Create snapshots the
configured file into daemon-managed immutable artifacts; restore accepts only an inventoried
basename and uses verified digest/identity compatibility plus atomic replacement. Browser paths
are never accepted. The same endpoint accepts a bounded `export` action, returning only the
configured UTF-8 document up to 1 MiB for a raw JSON5 download. `portable_export` serializes the
validated daemon document, while portable import is size-bounded, semantically validated, and
committed through the daemon's atomic configuration writer.
`POST /api/v1/sysex` forwards a confirmed framed request with a bounded destination alias and
1–1024 seven-bit data bytes; malformed, unconfirmed, or oversized payloads are rejected before IPC.
`GET /api/v1/validation` forwards the daemon's read-only configuration validation command.
`GET /api/v1/diagnostics` reports web build/API/IPC configuration and explicitly separates web
limitations from daemon health, which is returned by `/api/v1/health`.
The response also exposes the effective configured origin and the supported service settings
(bind environment variable/default, unit, restart policy, and boot-enable owner) as read-only data.
`GET /api/v1/diagnostics/bundle` exports the same bounded, daemon-independent inventory as a
downloadable JSON document; it deliberately excludes arbitrary host files and logs. Both
diagnostic routes are listed in capability discovery.

Every mutation is either rejected before daemon admission or returns an operation ID. A disconnect
after acceptance does not cancel the operation; the client resynchronizes through `state`.
Generation conflicts return HTTP 409 with a structured error and current generation. Unsupported
operations return HTTP 501 with the owning work-item ID.

## Bounded state stream

`GET /api/v1/events?after_sequence=N` is a bounded JSON event poll forwarded to the daemon's
`Subscribe` command. Each event has `sequence`, `generation`, `kind`, and a bounded JSON payload.
The browser keeps at most 256 events in its presentation buffer. A reconnectable
`GET /api/v1/events/stream?after_sequence=N` Server-Sent Events transport emits the same
authoritative events, heartbeats, and resnapshot signal for a bounded five-minute session; clients
reconnect with the last event ID. A slow client remains isolated by the bounded socket writer and
cannot delay daemon IPC or MIDI.
If a client reports a sequence gap, it must fetch `/api/v1/state` before applying later events.
The remaining mutation families are explicit W138-W139/W141 implementation work and are not
advertised as implemented by the current process.

HTTP bodies are capped at 1 MiB, JSON depth and string lengths are validated by the typed IPC
decoder, and responses use `application/json` with escaped output. No credentials, roles, database,
CDN, runtime Node process, or second configuration writer is introduced.

## Operation envelope

```json
{
  "request_id": "web-opaque-id",
  "operation": "mappings.snapshot",
  "generation": 12,
  "confirm": false,
  "payload": {}
}
```

The exact operation payload is owned by the corresponding IPC contract and the canonical editor
identified in `docs/web-feature-coverage.md`; the web layer does not reinterpret domain fields.
