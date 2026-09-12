# ADR-0018: Studio capability and feedback truth contract

- **Status:** Accepted
- **Date:** 2026-09-12
- **Owners:** W187 / Luna

## Decision

The Studio API advertises capability per named feature, not as one boolean for a whole device. A
feature separately declares whether it is readable, writable, queryable, subscribable, a meter, or
eligible for controller LED projection. The projection also carries a qualification label and a
plain-language unavailable reason when a promised capability cannot be used.

Displayed values are classified as `observed`, `acknowledged`, `sent-unverified`, `last-known`,
`stale`, or `unavailable`. A write acknowledgment records acceptance by the daemon or device
adapter; it never upgrades a value to `observed`. Only a device or authoritative service observation
may do that. Browser-local pending values remain drafts until the authoritative result arrives.

`StudioCapabilitySnapshot` is schema version 1, bounded to 128 devices and 512 features or
observations per device, and carries the daemon generation plus the last included event sequence.
Stable device identities and feature keys are unique within a snapshot. Unknown schema versions,
duplicate identities, oversized values, and invalid capability relationships fail closed.

The daemon remains the source of generation, persistence, validation, transport, and hardware
writes. The browser may render and retain last-known observations, but it cannot infer device state
from a successful HTTP response or from a rendered control.

## Compatibility and migration

Existing `/api/v1/capabilities`, device, mapping, assignment, PiPedal, and event envelopes remain
valid. The Studio snapshot is additive. Consumers that do not understand the new projection keep
using the existing endpoints; producers must preserve existing fields and publish schema version 1
only after validating all bounded entries.

## Evidence boundary

Protocol-specific readback and LED claims remain governed by the existing profile and hardware
qualification records. This ADR defines how truth is represented; it does not qualify a vendor
feature or authorize an unverified MIDI message.
