# Unified live website — persistent planning record

## Operator instructions (2026-09-08)

- Plan the redesign collaboratively. Add all work to the governed worklist.
- Ask multiple-choice questions one at a time until the desired site is fully defined.
- Save instructions, answers, decisions, and planned work continuously so disconnection does not lose progress.
- Make the website unified and LIVE; explore creative and innovative approaches.
- Remove the Carbon requirement. No replacement framework or visual system is selected yet.
- Aim for the best MIDI processor built around this Novation product and the existing MIDI endpoints as the first first-class devices.
- Show current assignments; preserve this preceding request in the redesign scope.
- Prefer project source, developer manuals, and manufacturer documentation before reverse engineering; record sources in project governance.
- Firebox remains removed from scope.
- Interview amendment: automatically adopt recommended answers for unanswered and future decisions with a clear path. Ask only unresolved choices without a clear recommendation, one at a time. Preserve the operator's explicit earlier answers, including those differing from recommendations.

## Scope and evidence

W164 owns this interview. W159 owns frontend state architecture; W160 backend reliability;
W161 current assignments and the Novation surface; W162 navigation; W163 browser and live-release verification.
The existing site has reported empty/stale assignments, unreliable backend status, excessive
controls, and confusing assignment workflows. Previous deployment claims do not establish that
these user-visible problems are resolved. The interrupted assignment-list patch is unverified.
Planning does not authorize treating an unverified edit as complete.

The current project identifies Launch Control XL MK2, Eventide MicroPitch, MIDISPORT MIDI ports,
Reflex, and PiPedal in its existing records. Confirm exact physical devices, endpoint identities,
and first-class scope during the interview; do not infer connectivity from historical records.

## Decisions

- D001: Carbon compliance and Carbon-specific release gates are removed by operator instruction.
- D002: This turn is planning; retain unfinished implementation work in the worklist.
- D003: The design centers on Novation and the current MIDI endpoints as first-class devices.
- D004: Operator selected B for question 1: a live editor prioritizing assignment and route configuration, with monitoring secondary. The editor continuously reflects authoritative state.
- D005: Operator selected A for question 2: Novation-centered workspace with the grid in the center, selected control's assignments and destination editor beside it, and routing in a linked view.
- D006: Operator selected B for question 3: each valid assignment edit saves automatically, with saving/saved feedback and Undo.
- D007: Operator requires device/interface synchronization within 10 seconds. Treat this as a maximum delay, not a mandatory polling interval. Cover interface-originated changes and observable device-originated changes. Show pending, confirmed, stale, or disconnected state truthfully; never label an unconfirmed hardware state synchronized. Identify endpoint readback/feedback limitations from authoritative sources and resolve them during planning.
- D008: Operator selected B for question 4: hardware wins when physical movement conflicts with an on-screen value edit. Replace the conflicting value edit with observed hardware state; invalidate superseded queued saves so they cannot later overwrite it. This priority concerns the same control value, not unrelated assignment destination fields.
- D009: Operator answered "1" to question 5, interpreted as the first option (A): each grid control shows destination device, assigned parameter, current value, and compact synchronization status without requiring selection. Unavailable values remain explicitly unknown.
- D010: Operator selected C for question 6: support multiple destinations per control and conditional assignments by scene or modifier button. Each destination retains its own range/direction. Layer interaction, modifier behavior, and grid presentation are still to be defined.
- D011: Operator selected A for question 7: activating a layer replaces base assignments for affected controls; unaffected controls retain their base assignments. Concurrent layer precedence remains to be defined.
- D012: Operator selected B for question 8: modifier buttons toggle their layer on/off. Provide LED confirmation on all keys or knobs involved, including the modifier and affected controls. Determine supported LEDs, colors, and feedback messages from exact-model manufacturer documentation and existing source; record references in governance. Unsupported physical feedback must be identified explicitly, with an agreed alternative, rather than claimed as delivered. The interface must display the same active-layer state.
- D013: Operator selected A for question 9: only one modifier layer is active at a time. Activating another deactivates the previous modifier layer and updates assignments, the interface, and involved LEDs together.
- D014: Operator selected A for question 10: changing scenes clears the active modifier layer and starts the new scene with its base assignments. Update effective assignments, interface state, and involved LEDs together.
- D015: Q11=A adopted under the operator's recommendation delegation: studio-instrument visual direction, dark neutral panels, compact readable controls, restrained status colors. Retain light-theme support; Carbon is optional, not required.
- D016: Recommended navigation adopted: open on the Novation editor; use consistent per-device pages and a compact sidebar for Devices, Routing, Scenes, and System. Put diagnostics, backups, raw configuration, and SysEx in their appropriate task views, not above the grid. Keep a compact global connection/sync indicator and emergency stop accessible.
- D017: Recommended inspector adopted: show existing effective assignments immediately on selection, list every destination with device/parameter/value, and expose range, direction, and advanced behavior progressively. Use searchable authoritative destination choices. Ordinary editing must not require the current multi-step assignment wizard.
- D018: Recommended live-state design adopted: authoritative backend snapshots plus ordered updates, with bounded recovery and timestamps. Automatic saves distinguish pending, acknowledged, and observed state. A failed read must never erase known assignments or label a control unassigned. Preserve the latest known state with a stale indicator; do not blindly replay uncertain mutations.
- D019: Recommended release behavior adopted: identify the loaded build, detect newer deployed assets automatically, and refresh the application when pending saves are settled without losing selection or drafts. Verify the browser actually receives the tested build.
- D020: Recommended delivery sequence adopted: first repair authoritative assignment display and backend reliability, then the unified editor and automatic saving, then multiple destinations/layers/LED coordination. Preserve existing mappings throughout migration. Plan backend and UI changes together; use existing source and documented contracts before choosing implementation technology.
- D021: Q12=A: reserve a dedicated group of physical buttons for layer switching. Recommended initial layout: four adjacent channel buttons, bottom row columns 5–8, labeled L1–L4 in the interface. This is a design allocation, not a live remap. Inspect existing assignments before migration; retain displaced assignments in the migration record and resolve collisions without silently deleting or changing destinations. Leave utility buttons in their documented platform roles.
- D022: Correct the faceplate from profile-owned geometry: 24 knobs, 8 faders, 16 channel buttons in two rows, and 8 separate utility controls. Do not create a fictional third channel-button row. Faders have no individual LED address in the current contract; reuse the documented proxy policy where applicable.
- D023: Use a scene base plus one optional modifier override. Four dedicated toggles initially select L1–L4; pressing the active toggle clears it. Per-control destination lists show effective assignments and their layer. Display one readable primary destination plus a destination count on crowded grid tiles; the inspector always lists all destinations and their values.
- D024: First-class delivery scope uses the established project inventory: Novation Launch Control XL Mk2, Eventide MicroPitch, Lexicon Reflex, PiPedal, and MIDISPORT transport ports. Reconcile current endpoint identities from configuration/runtime before migration; disconnected devices retain their pages and assignments with honest connection state. Do not introduce Firebox.
- D025: The 10-second bound applies under a healthy connection to observable state changes and acknowledgment of UI writes; expired confirmation produces a visible stale/pending error within the same bound. Hardware knob positions cannot be motorized by software. Distinguish physical input, target value, and last-sent value; use the existing documented pickup policy when a layer changes targets.

## Source-based planning constraint

`docs/device-feature-inventory.md`, source register and Eventide CC reconciliation, records
no query/reply definition in the current Eventide profile. This is a documented implementation
gap, not proof that hardware feedback is impossible. W145/W149 must resolve supported observation
paths; until then the UI distinguishes last-sent from observed values. The 10-second requirement
does not permit fabricated readback. Exact references and manufacturer source hashes are already
registered there and must be reused.

Controller layout sources inspected for this plan: `docs/mackes-launch-control-xl-mk2-factory1-manifest.json`
(contract 1.0.1), `crates/profiles/src/lib.rs` physical-control construction,
`docs/decisions/ADR-0010-launch-control-xl-mk2-factory-template-1.md`, and
`docs/decisions/ADR-0011-launch-control-xl-programmer-contract.md`.
They distinguish 16 channel buttons from eight utilities and separate MIDI input addresses from
LED feedback indices. The manifest marks utilities reserved, while ADR-0010 describes their
input messages; implementation must reconcile profile ownership rather than treating utilities
as ordinary channel buttons. Manufacturer reference and hash are registered in the device feature inventory.

## Interview checkpoint

Question 1 — What should LIVE mean in daily use?

- A (recommended): A performance cockpit showing current assignments, physical control movement,
  MIDI activity, and connection state automatically, with editing available in context.
- B: A live editor focused on configuring assignments and routes, with performance monitoring secondary.
- C: Separate Perform and Edit modes within one consistent application, both continuously synchronized.

Answer: B — Live editor.

Question 2 — How should the live editor be organized?

- A (recommended): Novation-centered workspace: control surface in the center, selected control's assignments and destination editor beside it, routing in a linked view.
- B: Signal-flow workspace: Novation and endpoint devices on a routing canvas; select a device or connection to edit it.
- C: Device workspaces: a consistent page per device, with its controls, assignments, and routing together.

Answer: A — Novation-centered workspace.

Question 3 — When should assignment edits become active?

- A (recommended): Apply per control: edit a draft, then press Apply; show confirmation from the backend and offer Undo.
- B: Immediate: each valid change takes effect automatically; show saving/saved state and offer Undo.
- C: Apply a batch: stage edits across multiple controls, then apply the whole set together.

Answer: B — Immediate automatic saving. Additional operator requirement: device and interface must be within a 10-second synchronization window.

Question 4 — What should happen if a physical control moves while you are editing that same control in the interface?

- A (recommended): Preserve the edit in progress; show the physical movement live alongside it, then apply the completed valid edit.
- B: Hardware wins: replace the interface edit with the latest physical-device change.
- C: Latest change wins: whichever valid hardware or interface change arrives last becomes the current value.

Answer: B — Hardware wins.

Question 5 — What should each control show on the Novation grid before you select it?

- A (recommended): Destination device, assigned parameter, and current value; use a compact status indicator for synchronization.
- B: Assigned parameter and current value only; show device and synchronization details in the inspector.
- C: Physical control labels with assignment colors; show full assignment and value details only when selected.

Answer: 1 — First option (A), full useful detail. Unknown or unavailable values must be labeled honestly.

Question 6 — How many destinations should a single Novation control be able to operate?

- A: One destination per control, for a simple direct assignment.
- B (recommended): Multiple destinations, each with its own range and direction; for example, one knob increases delay while reducing reverb.
- C: Multiple destinations plus conditional behavior by scene or modifier button, for layered control setups.

Answer: C — Multiple destinations with scene/modifier layers. Backend contract support must be checked before implementation; gaps are tracked in W165.

Question 7 — When a scene or modifier activates another assignment layer, what should happen to the base assignments?

- A (recommended): Replace them for affected controls; unaffected controls keep their base assignments.
- B: Add the layer's destinations to the base assignments so both run together.
- C: Choose per layer whether it replaces or adds to the base assignments.

Answer: A — Replace assignments for affected controls; retain base assignments elsewhere.

Question 8 — How should a modifier button activate its layer?

- A (recommended): Hold: the layer is active only while the button is held; releasing restores the prior layer.
- B: Toggle: press once to activate, press again to return.
- C: Configurable per modifier: choose Hold or Toggle for each modifier button.

Answer: B — Toggle, with LED confirmation on all keys or knobs involved.

Question 9 — If you activate a second modifier layer, what should happen to the first?

- A (recommended): Switch layers: deactivate the first modifier layer and activate the second.
- B: Keep both active: the most recently activated layer wins wherever their controls overlap.
- C: Keep both active: a configured priority decides which layer wins on overlapping controls.

Answer: A — Switch layers; deactivate the first modifier layer when activating the second.

Question 10 — When you change scenes, what should happen to an active modifier layer?

- A (recommended): Clear it: start the new scene with its base assignments and update all involved LEDs.
- B: Keep it active: apply the same modifier over the new scene where that modifier is defined; otherwise clear it.
- C: Remember per scene: restore whichever modifier was last active in the scene you enter.

Answer: A — Clear the modifier on scene change and use the new scene's base assignments.

Question 11 — What visual direction should the unified website take?

- A (recommended): Studio instrument: dark neutral panels, compact readable controls, and restrained colors that match device/assignment status.
- B: Clean workstation: light neutral panels, crisp typography, generous separation, and restrained status colors.
- C: Hardware-inspired: a recognizable Novation faceplate with illuminated controls, surrounded by clean editing panels.

Answer: A — Recommended studio-instrument direction, adopted under the operator's delegation.

Question 12 — Which physical buttons should be available as layer modifiers?

- A: Reserve a dedicated group of buttons for layer switching, reducing buttons available for ordinary assignments.
- B: Let any assignable button be designated as a modifier in its inspector, replacing that button's ordinary assignment explicitly.

Answer: A — Dedicated group of layer buttons. Operator also instructed CONTINUE.

Next: execute the source/configuration reconciliation and implementation packets below when proceeding from planning to implementation. No additional preference question currently blocks planning. Ask only if actual mapping collisions or unsupported device behavior leave no clear solution.

## Execution plan and handoff

| Order | Worklist owner | Concrete deliverable |
| --- | --- | --- |
| 1 | W145/W146/W160/W161 | Reconcile endpoint IDs, physical geometry, mapping generations, readback support, and daemon request contention. Establish one authoritative assignment view; failed reads preserve known state. |
| 2 | W159/W161/W162 | Build the unified studio editor: accurate central surface, adjacent live inspector, compact navigation, per-device pages, current assignments visible on entry. |
| 3 | W159/W161 | Automatic valid-edit saving, ordered acknowledgments, Undo, hardware-priority value conflicts, observable synchronization within 10 seconds. |
| 4 | W165 | Multiple destinations, four dedicated modifier toggles, replacement layers, scene reset behavior, documented LED feedback and assignment-preserving migration. |
| 5 | W163 | Real browser interactions, error/reconnect scenarios, sync timing, layout checks, deployed asset identity, and automatic application update with draft protection. |

Acceptance walkthrough: open the live site and immediately see stored assignments; click any
real control to see all its effective destinations; change a valid field and observe automatic
save and acknowledgment; move the physical control and see matching observed state within
10 seconds; toggle layers and scenes and confirm routing, grid, and supported LED behavior;
disconnect and reconnect without blanking assignments or replaying obsolete writes. A new release
must reach an already-open browser without losing pending edits. Record observed results rather
than treating a successful build or service restart as proof of these workflows.

This is an executable design plan, not a claim that these features are implemented or live.
