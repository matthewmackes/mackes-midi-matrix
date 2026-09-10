# PiPedal source-to-connector reconciliation

This is the governed checkpoint for W150. Re-run `python3 scripts/check-pipedal-audit.py`
before changing the connector boundary; do not infer unsupported operations from names alone.

## Audited boundary

| Evidence | Count | Meaning |
|---|---:|---|
| Pinned PiPedal server registrations | 104 | Source operations found in the audited server implementation |
| Connector operation families | 42 | Operations currently exposed by the typed connector boundary |
| Explicit pending W150 rows | 58 | Registrations intentionally not enabled until payload/event semantics are qualified |
| Live catalog controls | 3,076 | Installed read-only catalog projection |
| Live catalog targets | 265 | Installed target projection |

## Pending families

| Family | Pending rows |
|---|---:|
| Device assets/library | 15 |
| Device session/preferences | 3 |
| Device system settings | 14 |
| Monitoring and MIDI | 5 |
| Pedalboard and parameter inspector | 2 |
| Presets and snapshots | 19 |

The installed catalog currently qualifies all 42 connector operation families. The 58 pending
rows are not silently represented as implemented features; each requires source-level payload,
event, error, and persistence qualification before promotion. Firebox is removed from active
scope; historical research remains governed separately.

## Reproduction

```text
python3 scripts/check-pipedal-audit.py
python3 scripts/pipedal-catalog-smoke.py http://172.20.222.222:8081
```
