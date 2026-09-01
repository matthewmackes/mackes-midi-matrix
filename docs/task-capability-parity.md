# Task-shell capability parity

This matrix is the migration checklist for the five-section shell. Legacy routes remain available
until the corresponding row has a tested destination; this document does not authorize removing a
legacy action.

| Legacy capability | Primary section | Current destination | Parity status |
| --- | --- | --- | --- |
| Dashboard health, scene, panic, live activity | Live | Live landing and HUD | Covered |
| MIDI Learn capture and review | Map Controls | Assignment/browser workflow | Partial: controller capture pending Factory Template 1 qualification |
| Routing and durable parameter mappings | Map Controls | Mapping browser and Advanced inspector | Partial: browser/mutations wired; full end-to-end fixture pending |
| Reflex profile and algorithm controls | Devices | Profile-backed device view | Covered by profile workspace tests |
| Eventide MicroPitch controls | Devices | Profile-backed device view | Covered by device workspace tests |
| Scene selection and setlists | Scenes | Scene/setlist task | Existing editors retained; landing rehome pending |
| Diagnostics and monitor | System | Diagnostics/monitor task | Existing workspaces retained; landing rehome pending |
| Backup inspection and restore | System | Backup task | Existing workspace retained; landing rehome pending |
| Low-level/legacy numbered routes | System | Advanced → Legacy compatibility path | Explicitly retained during migration |

## Rules

- Every row must retain an executable action or an explicit empty/recovery state.
- New navigation uses Live, Map Controls, Scenes, Devices, and System; numeric workspace keys are
  compatibility input only.
- A capability is not marked Covered until a focused parity test proves its action and state remain
  reachable through the named section.
- Hardware qualification and the official Components artifact remain separate W071/W081 evidence.

## Executable route contract

The shell uses these stable destinations as the migration contract. A capability is considered
re-homed only when its destination has both a visible landing and a focused action/state test.

| Destination | Entry action | Required visible state |
| --- | --- | --- |
| Live | Select Live, then Enter | Active scene, health, activity, and panic state |
| Map Controls | Select Map Controls, then Enter or Device | Mapping browser, Learn/assignment state, and recovery outcome |
| Scenes | Select Scenes, then Enter | Project, scene, song, and setlist state or an explicit empty state |
| Devices | Select Devices, then Enter | Profile-backed Reflex/MicroPitch controls and connection state |
| System | Select System, then Enter | Diagnostics, monitor, backups, configuration, and Advanced → Legacy entry |
