# ADR-0017: Versioned mapping-layer schema proposal

**Status:** Proposed; no runtime contract changed.

## Scope

This proposal is the implementation checkpoint for W165. It defines the lossless shape to qualify
before changing the persisted mapping envelope; it does not enable multi-destination dispatch.
The machine-checkable draft is `schemas/mapping-layers.v2.proposed.schema.json` and is design-only.

```json
{
  "schema_version": 2,
  "base": {
    "control_id": "knob-r1-c1",
    "destinations": [
      {"id": "primary", "endpoint": "…", "profile": "…", "effect": "…", "parameter": "…", "channel": 1, "behavior": {"kind": "linear"}}
    ]
  },
  "layers": [{"id": "scene-a", "destinations": []}],
  "active_layer": null
}
```

The daemon must validate stable destination IDs, deterministic ordering, duplicate rejection,
single active-layer selection, and complete single-binding round trips. It must publish one
generation for persistence acknowledgement, effective projection, reconnect snapshot, and LED
intent. Browser code may edit drafts only; it may not merge layers. Migration must preserve the
existing `ControlMapping` field set and report displaced controls explicitly.

## Required qualification before adoption

Add schema/migration fixtures, persistence and reconnect tests, effective-projection tests, and
qualified Novation LED/profile references. Until those pass, the existing version-1 single-binding
contract remains authoritative.
