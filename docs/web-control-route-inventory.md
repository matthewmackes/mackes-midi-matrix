# Web control-to-route inventory

This inventory is the W162 baseline for splitting the former shell into focused task routes.
The clean-sheet Studio shell is now the deployed owner at `/` (with `/studio` retained as an
explicit deep link). The `/devices/*` entries below are compatibility and historical references;
they must not be presented as primary navigation.

| Current control/workflow | Current surface | Target route | Ownership | Migration note |
| --- | --- | --- | --- | --- |
| Live event/state projection | Live | `/live` | monitoring | Keep compact; link to selected device/control. |
| Rescan devices | Devices | `/recovery` | recovery | Keep emergency shortcut visible. |
| Panic all outputs | Devices | `/recovery` | recovery | Keep emergency shortcut visible and confirmed. |
| Previous/Next scene | Devices | `/scenes` | scenes | Context shortcut may remain in shell. |
| Novation control grid | Devices | `/devices/novation` | Novation device | Canonical first-class surface. |
| Current assignments list | Devices | `/devices/novation` and `/mappings` | assignments | Same authoritative mapping projection. |
| Selected-control inspector | Devices/Map Controls | `/devices/novation` | assignments | One owner; context link from Map Controls. |
| Start/Capture/Commit/Cancel assignment | Map Controls | `/mappings` | assignments | Preserve automatic-save and generation handling. |
| Mapping behavior preview/save/replace/delete/enable | Map Controls | `/mappings` | assignments | Advanced editor below selected control. |
| Destination catalog/search | Map Controls | `/mappings` | assignments | Searchable authoritative catalog. |
| Device feature selectors | Devices | `/devices/<device>` | device editors | Novation/Eventide/Reflex/PiPedal pages. |
| Device control send | Devices | `/devices/<device>` | device editor | Confirmation and endpoint validation stay local. |
| Routing refresh/add/undo/apply | Routing | `/routes` | routing | Visual route board is canonical. |
| Scenes refresh/activate/create/update/delete | Scenes | `/scenes` | scenes | Setlist and scene state remain daemon-owned. |
| Diagnostics download | Devices | `/system/diagnostics` | diagnostics | Stable deep link now resolves through System owner; keep a recovery shortcut. |
| Configuration export/import/validate | Devices | `/system/configuration` | configuration | Stable deep link now resolves through System owner; remove from the Novation surface. |
| Raw JSON5 draft/apply | Devices | `/system/configuration/raw` | configuration | Stable deep link now resolves through System owner; preserve draft and CAS behavior. |
| Backup inspect/create/restore | Devices | `/system/backups` | recovery/configuration | Stable deep link now resolves through System owner; confirmation and managed backup semantics remain. |
| SysEx send | Devices | `/devices/<device>/sysex` | hardware operations | Keep explicit confirmation and byte validation. |
| Refresh Novation device | Devices | `/devices/novation` | Novation device | Device-local refresh. |
| PiPedal catalog/operation | Devices | `/devices/pipedal` | PiPedal device | Typed catalog and confirmation remain. |
| Theme switch | Global shell | all routes | shell | Persisted preference. |
| Keyboard help | Global shell | all routes | shell | Route-independent help. |
| Backend/reconnect status | Global shell | all routes | shell/health | Truthful web/daemon/feature states. |
| Monitor pause/clear/filter/download | Monitor | `/monitor` | monitoring | No duplicate monitor editor elsewhere. |

## Omissions and collisions to resolve

- `Devices` currently contains recovery, scenes, configuration, hardware, device editors, and the
  Novation surface together. Configuration and Hardware are collapsed; route extraction remains open.
- The same selected-control editor is reachable from Devices and Map Controls. W162 must retain one
  canonical editor and use context links rather than duplicate mutation owners.
- `/` and `/studio` are the live Studio controller surfaces. Legacy page paths such as
  `/devices/novation` remain compatibility deep links, but now serve the Studio shell; new product
  links must point to Studio.
- This inventory does not claim implementation of the W165 multi-destination/layer model.

## Evidence commands

```text
rg -n 'id="|data-view=|data-assignment-action' apps/mackes-web/static/studio.html
python3 scripts/check-worklist.py
bash scripts/browser-smoke.sh
```
