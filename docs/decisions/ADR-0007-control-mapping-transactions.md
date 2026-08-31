# ADR-0007: Control-mapping transaction authority

## Status

Accepted — 2026-08-31

## Decision

`ControlMappingStore` is the authoritative in-memory transaction for hardware-first
parameter assignments. Complete active records and incomplete drafts are separate
collections in the versioned configuration document. Drafts may be autosaved and
resumed, but are never executable.

Every mutation supplies the client generation. A stale generation, invalid identity,
occupied physical control, or occupied destination rejects the operation without
changing the store. Successful mutations increment generation and retain exactly one
bounded Undo snapshot. Replacement is explicit and never implicit.

The IPC `Mappings` envelope uses typed `MappingPayload` variants for mapping, draft,
behavior, enablement, and deletion operations. Results return the authoritative
generation, projections, Undo availability, and a bounded terminal outcome. Ordinary
endpoint routes remain independent of parameter mappings.

Persistence uses the existing validated, atomic configuration save path. Consumers
commit runtime state only after the corresponding durable configuration write
succeeds; failed writes therefore leave runtime and disk unchanged.

## Consequences

The schema carries stable controller profile, physical-control, source endpoint,
source message, destination, behavior, enabled state, and profile provenance. Future
daemon and TUI consumers must use these contracts rather than inventing JSON payloads
or writing configuration files directly.
