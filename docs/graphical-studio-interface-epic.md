# Graphical Studio Interface Epic — W167

## Operator brief

The web interface must make sense to a person who understands music and equipment but does not
know MIDI protocol, JSON, JSON5, SysEx, internal IDs, or software implementation terminology.
Every device and endpoint must have a graphical representation. Every editable capability must use
an understandable graphical control or guided builder. The browser must not show or accept code,
configuration text, raw protocol bytes, or state dumps as a normal interaction.

Readable labels, values, help text, status, accessibility names, and recovery messages remain
required. The prohibition is against code/protocol-oriented entry and dumps. Raw configuration and
diagnostic captures may remain downloadable files but their contents are not normal browser editors.

Decisions inherited from W164:

- Primary mental model: studio signal-flow canvas.
- Visual language: faithful responsive schematics, implemented as local code-native SVG.
- Advanced capabilities: visual builders only.
- First-class inventory: Novation Launch Control XL, Eventide MicroPitch, Lexicon Reflex, PiPedal,
  M-Audio MIDISPORT 4x4, RTP-MIDI, generic MIDI, and MACKES virtual/monitor endpoints.
- Retired products remain excluded unless explicitly reintroduced.

## Current gap review

The current shell still contains raw JSON5 configuration, route JSON, scene-action JSON, predicate
JSON, raw SysEx byte input, `<pre>` assignment/state surfaces, and device cards that primarily
expose labels and IDs. Existing SVG work covers the Novation faceplate but not every device or
endpoint. Existing browser tests prove availability and selected interactions; they do not prove
graphical completeness or novice usability.

## Product and interaction requirements

### Studio signal-flow canvas

- The landing workspace shows named devices, ports, connections, current scene, and health together.
- Devices are movable, zoomable, selectable, and keyboard navigable. Connections work by pointer
  drag or explicit source-then-destination actions.
- Desktop uses a device palette, central canvas, and contextual inspector. Tablet and phone layouts
  use staged list/canvas/inspector navigation without losing drafts or selection.
- Port direction, media type, availability, and connection state are visible before Apply. Invalid
  candidates are visibly rejected before submission.
- The UI never presents an unverified MIDI route as an authoritative audio signal chain.

### Complete device graphics

- Every qualified device has a local SVG schematic based on actual control or port geometry.
- Novation shows 24 knobs, 16 channel buttons, eight utilities, eight faders, LEDs, layers,
  assignments, pickup, and truthful readback state.
- Eventide MicroPitch shows pedal controls, footswitches, presets, parameter domains, and
  sent-unverified state where feedback is unavailable.
- Lexicon Reflex shows rack controls, algorithm-dependent parameters, Echo Rhythm, patches,
  registers, busy/storage state, and temporary versus persistent operations.
- PiPedal shows pedalboard/plugin graph, controls, snapshots, presets, levels, ports, and operation
  states. MIDISPORT shows four inputs and four outputs with direction, activity, firmware readiness,
  and separate cable/route state.
- RTP-MIDI shows peer, invitation/handshake, session health, sequence/reorder metrics, reconnect
  state, and virtual ports.
- Generic and MACKES virtual/monitor endpoints receive generated chassis, ports, capabilities,
  activity, and availability. Unknown endpoints never degrade to text-only cards.
- Every faceplate has a searchable accessible list equivalent; selection never emits MIDI.

### Graphical editing and language

- Replace route/predicate JSON with visual connection cards, condition chips, range controls, curve
  previews, priority controls, and cycle warnings.
- Replace scene-action JSON with ordered action cards showing target, operation, values, dependencies,
  delays, failure policy, preview, and recovery in plain language.
- Replace raw SysEx entry with qualified manufacturer/device command builders. Unsupported arbitrary
  SysEx remains clearly identified and outside the normal browser editor.
- Replace raw JSON5 editing with schema-driven forms, visual lists, device pickers, toggles, ranges,
  enum labels, and guided validation. Backup import/export remains a file action.
- Replace internal IDs and numeric endpoint fields with named device, port, control, and parameter
  selectors. Technical identifiers are optional read-only Advanced Details only.
- Pending, unsaved, applied, rejected, stale, conflicted, disconnected, unknown, observed, and
  sent-unverified states use text and non-color visual cues.

### Usability and safety

- A novice can identify a device, connect ports, map a knob, adjust it, save/recall a scene, diagnose
  a disconnected endpoint, and restore a backup without protocol knowledge.
- Triggers, momentary controls, toggles, continuous values, and persistent actions are visibly
  distinct. Hazardous operations require preview and explicit confirmation.
- Keyboard-only operation, focus, screen-reader names, reduced motion, 200% zoom, light/dark themes,
  and 320/768/1440 CSS-pixel layouts are mandatory.

## Technical delivery requirements

- Extend capability/state projections with renderer key, device kind/model, geometry, port layout,
  control type, units, enum labels, readback semantics, and qualification status.
- Add a renderer registry with exact qualified renderers and a mandatory generic endpoint renderer;
  missing qualification fails closed.
- Keep the daemon as the sole generation, validation, persistence, transport, and hardware writer.
  Browser drafts survive polling, navigation, reconnect, rejected saves, and conflicts.
- Preserve all untouched route, scene, mapping, and configuration fields losslessly; visual metadata
  is additive and migration-compatible.
- Split browser responsibilities into canvas, device-renderer, mapping-builder, route-builder,
  scene-builder, settings-form, monitor-timeline, and shared-state modules.
- Use locally bundled code-native SVG/CSS/assets with no CDN or runtime internet dependency.

## Execution packets and closure evidence

| Item | Result | Required evidence |
|---|---|---|
| W168 | Audit all code surfaces and renderer coverage | Static guard plus complete device/endpoint renderer ledger |
| W169 | Deliver responsive signal-flow shell | Pointer/keyboard canvas scenarios at 320/768/1440 |
| W170 | Deliver Novation, Eventide, and Reflex schematics | Geometry, control, accessibility, state, and emulator fixtures |
| W171 | Deliver PiPedal, MIDISPORT, RTP, generic, and virtual graphics | Connected/disconnected/unknown renderer matrix |
| W172 | Deliver visual mapping and routing builders | Advanced-field losslessness, conflict, validation, reconnect tests |
| W173 | Deliver visual scenes and setlists | Ordered action-card preview/apply/recovery scenarios |
| W174 | Deliver graphical settings, diagnostics, and recovery | No code textbox/dump in rendered DOM; guided flows |
| W175 | Complete visual capability and draft contracts | Schema, golden payload, migration, draft, generation tests |
| W176 | Qualify novice usability and accessibility | Walkthroughs, accessibility tree, themes, zoom, responsive evidence |
| W177 | Install and close the graphical release | Screenshots, clean console, hashes, service restart, release gate, sign-off |

## Acceptance scenarios

1. Edit one endpoint on a route containing every advanced field; apply/reload and prove untouched
   fields are identical.
2. Edit a value, receive updates, navigate away, reconnect, and return; preserve the draft and
   require reconciliation for stale Apply.
3. Disconnect during a write; show unknown outcome and recover authoritative state without unsafe
   automatic retry.
4. Load every known, generic, virtual, and unknown endpoint; each has a graphical representation
   and accessible list equivalent.
5. Select/map every Novation control by pointer and keyboard; verify emulator input, layer, pickup,
   LED intent/delivery, and reconnect without claiming native observation.
6. Complete novice walkthroughs without entering JSON, JSON5, SysEx bytes, internal IDs, or protocol
   numeric addresses.
7. Verify themes, reduced motion, screen-reader labels, zoom, and all required widths without
   hidden, clipped, hover-only, or horizontally overflowing essential actions.
8. Run browser, contract, service, release, emulator, hermetic integration, installer, and full-gate
   checks against the installed final interface.
