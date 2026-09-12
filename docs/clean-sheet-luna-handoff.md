# Superseded clean-sheet Studio handoff

Updated 2026-09-12. This handoff is retained as prototype evidence. The authoritative execution
specification is now [`plug-and-play-live-instrument-epic.md`](plug-and-play-live-instrument-epic.md)
and the active packet order is W186–W202.

W180–W185 are superseded and must not be claimed complete from the evidence below. W187's capability
contract and discovery projection are complete; Luna next follows event synchronization, visual shell, reviewed
starter setup, tiered device views, assignments, scenes, recovery, qualification, and W202 cutover.

## Already in the tree

- `/studio` is the clean Novation Launch Control XL workspace, with 24 knobs, 16 channel buttons,
  8 faders, and 8 utility controls.
- `/studio/gallery` contains the local component gallery.
- PiPedal, Eventide MicroPitch, and Lexicon Reflex destination browsing uses named functions and
  filtered control compatibility. PiPedal choices carry the stable plugin/symbol payload internally.
- Assignment preview and explicit Commit use `/api/v1/assignment`; behavior editing uses the typed
  generation-checked `Behavior` mapping operation.
- Saved destinations, observed values, per-control sync state, capture events, and layer toggling
  are visible in the clean-sheet surface.
- `/studio/devices`, `/studio/routing`, `/studio/scenes`, and `/studio/system` are deep-linkable and
  have bounded status panels with no raw JSON or protocol entry workflow.
- Devices reports authoritative PiPedal named-control counts and offers retry on unavailable state;
  supporting views link back to the single Controller assignment editor.
- Each control now exposes live, observed, saving, or stale state; hardware capture events select
  the matching control, and layer buttons expose their active state to assistive technology.
- Layer-aware hydration keeps the faceplate and drawer aligned when mappings include layer metadata;
  LED-capable controls project authoritative active/blink intent without adding behavior to faders.
- Utility controls are labeled Device, Mute, Solo, Record, Up, Down, Left, and Right; observed
  values move knob indicators and fader handles, and conflict responses reconcile before retry.
- Local drafts are shape-validated on restore; malformed browser storage is discarded, and valid
  drafts reopen without being presented as daemon-confirmed assignments.

## Superseded next order

1. Complete W180 hardware capture fixtures and verify native movement/press selection against the
   stable physical-control IDs.
2. Complete W181 catalog fixtures at PiPedal scale and error/reconnect compatibility cases.
3. Complete W182 multi-destination layer persistence, conflict ordering, coalescing, undo, and
   daemon-backed LED projection. Keep the current LED intent preview truthful until projection exists.
4. Complete W183 full device, routing, scene, and system panels without introducing a second mapping
   editor.
5. Complete W184 browser interaction, accessibility-tree, responsive, timing, reconnect, and
   musician walkthrough evidence.
6. W185 cutover is superseded; use W202 for canonical cutover and installed-release qualification.

## Pivot rule

If native Novation hardware or a live PiPedal catalog is unavailable, continue with the bounded
event/catalog fixtures and emulator traces. Mark the affected packet as in progress, preserve an
honest unavailable or unverified state in the UI, and record the missing native evidence in the
packet. Do not substitute a simulated success or close W180–W184 until the required evidence exists.

## Boundaries to preserve

Use the existing typed daemon routes and generation fields. Do not expose CC numbers, SysEx bytes,
runtime IDs, raw configuration, or protocol-shaped forms in normal flows. Keep all Studio assets
same-origin and bundled; the local visual system has no Carbon dependency or compatibility target.

Every mutation must show an explicit preview or save action, retain drafts on failure, and refresh
from authoritative state after acceptance. A disconnected or unsupported device must remain readable
with an honest state rather than being represented as successfully controlled.

The current qualification commands are `scripts/check-live-instrument-epic.py`,
`scripts/check-clean-sheet-ui.py`, `scripts/check-worklist.py`, `scripts/check-web-assets.py`,
`cargo fmt --all -- --check`, and `cargo test -p mackes-web`; run all of them before changing
packet status or beginning W202 cutover.
The latest bundled web asset checkpoint is 55,818 compressed bytes; treat a budget increase as a
review item before adding dependencies or visual assets.
At this checkpoint the governed worklist contains 186 packets: 155 complete, 15 in progress, and
16 unchecked packets including READY and DEFERRED items. W190 and W191 are complete; W192, W193, W194, W195, W196, W197, W198, W199, W200, W201, and W202 are active. W192 now has reviewed atomic Apply and Undo evidence; W193 has software unity and pickup truth; W194 has installed capability-card, catalog, and external-observation evidence; W197 has deterministic endpoint-state coverage; W198 has pointer/keyboard/touch, conflict, capture, and Undo evidence; W199 has scene save/recall, empty-state, and layer evidence; W200 has refresh-failure, stream-gap, reconnect, and stale-value evidence; W201 has mobile-overflow, novice, accessibility, and latency evidence; W202 serves the clean-sheet Studio shell at `/`, has compatibility deep-link assertions, an executable checksum-guarded rollback artifact, and repeated release-gate passes. Native feedback/readback qualification and final human sign-off remain open.
normal dependency chain, not by a missing product decision.
