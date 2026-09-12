# Superseded clean-sheet Web GUI plan — prototype context

> The active implementation and acceptance authority is
> `docs/plug-and-play-live-instrument-epic.md` (W186–W202).

## Product outcome

Build a new browser interface for a musician configuring the Novation Launch Control XL. The main
job is to assign PiPedal, Eventide MicroPitch, and Lexicon Reflex functions to the Novation's knobs,
faders, buttons, and supported LEDs. A person must be able to complete that job by recognizing the
hardware, choosing a musical function, and seeing the result. They must not need to know MIDI CCs,
SysEx, JSON, internal IDs, or the shape of the daemon API.

This is a clean-sheet interface. Reuse the daemon, schemas, validated device catalogs, stable
physical-control IDs, and operation boundaries. Do not use the existing DOM, page composition, CSS,
or incremental workspace layout as a visual starting point. Existing frontend code is reference
material for request/response behavior only.

There is no Carbon requirement or compatibility target. Do not add Carbon packages, Carbon tokens,
Carbon fonts, Carbon icons, or a Carbon compliance gate. Build a small local visual system for this
product. All runtime assets remain bundled and usable without a CDN or internet connection.

## Experience model

The default route opens directly into **Controller**. It shows a recognizable Launch Control XL
faceplate with all 56 physical controls in their real groups: 24 knobs, 16 channel buttons, eight
faders, and eight utility controls. The faceplate is the navigation surface as well as the status
display. A compact header shows the current scene, active modifier layer, save/synchronization state,
device connections, performance lock, and panic.

Selecting a control opens one assignment drawer. The drawer always answers four questions in this
order:

1. Which physical control am I editing?
2. What does it do now, including every destination and active layer?
3. What musical function can I assign to it?
4. What will happen when I move or press it?

The destination browser uses three prominent device cards: **PiPedal**, **Eventide**, and
**Lexicon**. Selecting a card reveals searchable, grouped, human-readable functions. Compatibility
filtering is automatic. Continuous parameters are offered to knobs and faders; actions, bypass,
preset, snapshot, tap, toggle, and momentary behaviors are offered to buttons where supported.
Incompatible choices stay visible only when their explanation helps the user; otherwise omit them.

The primary assignment gesture is select a Novation control, select a device function, preview, and
save. Drag-and-drop may be added as a shortcut, but click/tap and keyboard operation are complete on
their own. MIDI Learn is a secondary shortcut labeled **Touch a control to select it**. Selecting or
hovering in the browser must never emit MIDI.

## Desktop composition

```text
┌ Scene · Layer · Saved/Sync ───────── Connections ─── Lock · PANIC ┐
│ Controller   Devices   Routing   Scenes   System                  │
├──────────────────────────────────────────┬────────────────────────┤
│                                          │ Selected: Top knob 1   │
│       NOVATION LAUNCH CONTROL XL         │ Current assignments    │
│                                          │  • PiPedal: Drive Gain │
│   [24 live knobs with assignment rings]  │  • Eventide: Mix       │
│   [16 live buttons with LED previews]    │                        │
│   [8 live faders]                         │ [+ Add destination]    │
│   [8 clearly separate utility controls]  │ [PiPedal][Eventide]    │
│                                          │ [Lexicon]              │
│ Layer: Base  L1  L2  L3  L4              │ Search functions…      │
└──────────────────────────────────────────┴────────────────────────┘
```

On tablet, the assignment drawer overlays or follows the faceplate while keeping the selected
control visible. On phone, use a two-step Controller → Assignment flow with a persistent selected-
control summary and Back action. Do not shrink the whole faceplate into unusable miniature controls;
provide grouped horizontal sections and a complete list equivalent.

## Controller graphics and feedback

Each control has a large hit target and a concise label. Before selection it shows its primary
destination, a destination count when more than one exists, its current or last-known value, its
layer, and a synchronization state. Use rings, fills, fader tracks, button illumination, icons, and
short text together. Color never carries meaning alone.

States are visually distinct and named: unassigned, assigned, selected, moving/pressed, saving,
saved, observed, last sent, stale, conflicted, disconnected, disabled, and error. Preserve the last
known assignment during read failures. Never replace it with “unassigned” because a refresh failed.

Render LED intent on every LED-capable control. The editor offers musician-facing behaviors such as
**On when effect is active**, **Show selected layer**, **Blink while pending**, and **Use device
status**, followed by an Advanced color/mode override. Use the documented device palette. Faders do
not gain fictional LEDs; show their state in the browser and use only the already documented proxy
policy where applicable. Unsupported readback is labeled **Sent — device does not confirm**.

## Assignment drawer

The current-assignment area lists every effective destination as a card. Each card shows device,
effect or algorithm, function, live/last-known value, source and target ranges, direction, curve,
layer, enable state, and observation quality. The common fields are graphical and immediately
visible. Range, curve, inversion, button mode, and LED override live under **Fine tune**.

**Add destination** opens the device-card browser. Its content comes only from authoritative profile
and PiPedal catalogs:

- PiPedal: current pedalboard first, then plugin blocks; offer plugin controls, bypass/enable,
  presets, snapshots, and supported pedalboard actions with names, units, ranges, and enum labels.
- Eventide: presets and the documented MicroPitch parameters/actions, grouped as Pitch, Delay,
  Mix/Output, and Footswitch actions. Clearly distinguish observed values from last-sent values.
- Lexicon: presets/registers, algorithms and their compatible parameters, bypass, and qualified
  patch/actions. Changing algorithm refreshes compatible parameters without discarding unrelated
  drafts.

A new compatible choice starts with profile-derived range and behavior defaults. Show a plain-
language preview such as “Moving Fader 3 from bottom to top changes PiPedal › Compressor › Mix from
0% to 100%.” Saving is automatic after a valid edit, matching the accepted product decision. Show
Saving, Saved, or Needs attention beside the changed card and keep one-level Undo available. Coalesce
rapid edits and apply ordered generation acknowledgments so an older response cannot overwrite a
newer edit.

One control may have multiple destinations. Each destination owns its range, direction, curve, and
layer. Base assignments remain active except where the selected modifier layer replaces them. Only
one modifier layer is active. L1-L4 use the reserved bottom-row channel buttons 5-8, toggle on press,
switch each other off, and clear on scene change. Inventory and preserve existing mappings before
claiming those buttons; surface collisions for resolution without deleting assignments.

## Device and supporting views

**Devices** gives PiPedal, Eventide, Lexicon, Novation, and transport endpoints individual graphical
pages. Each page shows connection truth, named controls/actions, active preset or algorithm, and a
link back to every Novation control assigned to that device. Editing an assignment still uses the
same assignment drawer and state store.

**Routing** uses ports and connection cards for endpoint routing. **Scenes** shows scene cards and
layer effects. **System** holds setup, backup, diagnostics, and recovery. These views support the
controller workflow; they do not compete with it or duplicate assignment editors.

## Visual language

Aim for a focused studio instrument: charcoal and warm near-black surfaces, crisp off-white type,
subtle depth, high-contrast focus, and restrained device/status colors. Use the Novation's physical
grouping and LED palette to build recognition without making a photorealistic skin. Use generous
space around the controller, compact information inside assignment cards, and motion only to explain
selection, signal, or state change. Retain a polished light theme and respect reduced motion.

Define local tokens for color, type, spacing, radius, elevation, focus, and motion. Prefer system
fonts or bundled licensed fonts, simple local icons, CSS, and code-native SVG. Avoid ornamental
meters, unlabeled tiny knobs, excessive glass effects, and constant animation. The most saturated
color belongs to selection, live control movement, warnings, and errors.

## State and technical boundaries

- The daemon remains the only persistence, generation, validation, mapping, route, scene, MIDI, and
  LED authority. The browser never synthesizes protocol messages.
- Build a new frontend root and component/state modules behind a temporary development entry point.
  Keep the served interface usable during construction, then switch the canonical routes only after
  the clean-sheet acceptance suite passes. Remove the retired frontend after cutover.
- Normalize API data into one store keyed by stable physical control and destination identity.
  Separate authoritative data, local draft, pending mutation, last-known observation, and connection
  freshness. Never derive assignment truth from rendered DOM.
- Preserve focus, selection, scroll, and valid drafts across updates, navigation, reconnect, and
  deployed-build refresh. Hardware movement wins only for the value of that same physical control;
  it does not erase destination edits.
- Healthy observable hardware/UI changes and UI write acknowledgments appear within 10 seconds.
  Expiration changes the state to stale or failed; it never fabricates confirmation.
- Keep existing API/schema fields lossless. If the GUI cannot safely represent a field, show a
  read-only explanation and block only that edit. Do not silently drop or reset it.
- Framework choice is Luna's implementation decision, provided the release stays offline-capable,
  bundled, maintainable, within the existing asset budget, and free of a runtime Node dependency.

## Required accessibility and input behavior

All 56 controls and every destination card are reachable and operable with keyboard, pointer, and
touch. The SVG faceplate has an equivalent structured list backed by the same selection and editor.
Provide visible focus, semantic names, current values and states, logical focus order, live-region
announcements for save results, 44px touch targets, 200% zoom support, and no hover-only or drag-only
action. Test dark/light themes, reduced motion, and widths of 320, 768, and 1440 CSS pixels.

## Acceptance walkthrough

Use populated fixtures for PiPedal, Eventide, and Lexicon and perform this walkthrough in the
installed browser build:

1. Open Controller and identify all 56 controls, their physical grouping, current assignments,
   connection state, current scene, layer, and sync state without opening an inspector.
2. Select a knob, add one PiPedal parameter and one inverted Eventide parameter, fine-tune both,
   observe automatic save, reload, and prove both assignments and untouched fields persisted.
3. Select a fader, find a Lexicon parameter by product/algorithm/function rather than ID, preview the
   value mapping, save it, move the hardware, and see the truthful value state within 10 seconds.
4. Assign a button to a supported preset/bypass/toggle action and configure semantic LED feedback;
   verify browser intent, emulator output, and the sent-versus-observed label.
5. Toggle L1 and L2, verify mutual exclusion and effective assignment/LED changes, change scene, and
   verify return to the new scene's Base layer.
6. Disconnect one destination during a pending edit. Keep known assignments visible, show the
   uncertain outcome, reconnect, reconcile by generation, and prevent an obsolete write replay.
7. Complete the same core assignment with keyboard only and at 320px width; verify screen-reader
   names, 200% zoom, reduced motion, both themes, and no clipped essential action.
8. Verify no normal flow exposes JSON, JSON5, SysEx bytes, CC numbers, internal IDs, state dumps, or
   a raw configuration editor, and verify no Carbon package, asset, token contract, or compliance
   check is present.

Completion requires browser interaction fixtures, contract and losslessness tests, Novation
emulator coverage, installed asset hashes/screenshots, a clean console, service restart/reconnect
checks, the repository release gate, and a short human usability pass. Screenshots alone do not
prove interaction, persistence, hardware truth, or accessibility.
