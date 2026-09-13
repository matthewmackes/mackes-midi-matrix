# MOD-inspired visual instrument and artwork epic — W203–W212

## Mandate

Use approved artwork and interaction patterns from MOD Audio's wiki, `mod-ui`, MOD SDK, and
qualified plugin `modgui` bundles to turn MACKES Studio into a visual pedalboard and instrument
workspace. This is a private project and the operator confirmed on 2026-09-13 that the assets are
approved in the remote repository and fully authorized for this work. The executor is **Orion**, a successor AI independent of Luna's current
W186–W202 work.

Authorization is confirmed, but is not an excuse to lose provenance. Every imported file must retain
its source repository, source path, revision, original license notice, import date, modifications,
and intended MACKES use in a machine-readable manifest. MOD names, logos, product photographs, store
art, and third-party plugin branding are imported only when the assumed permission clearly covers
that class. Unclear assets use a MACKES-owned visual replacement.

Primary references:

- <https://wiki.mod.audio/wiki/MOD_Web_GUI_User_Guide>
- <https://mod.audio/platform/ui/>
- <https://github.com/mod-audio/mod-ui>
- <https://github.com/mod-audio/mod-sdk>
- <https://github.com/mod-audio/mod-screenshot>

## Product outcome

The Controller remains the primary 56-control Launch Control XL workspace. The Devices workspace
gains a PiPedal pedalboard canvas and reusable visual instrument components. Musicians can understand
the current signal/control topology, inspect a plugin as a recognizable pedal or rack unit, see
truthful live controls and meters, browse pedalboards and snapshots, and start an assignment by
selecting a rendered control. MACKES continues to own mappings, scenes, layers, persistence,
generation checks, hardware writes, Undo, recovery, and feedback truth.

The interface must feel visually rich without becoming a simulation that invents state. Imported
art is decorative until bound to a typed capability. A pictured knob, switch, jack, meter, cable,
LED, preset, or bypass state is interactive only when the authoritative snapshot advertises the
corresponding feature and its read/write semantics.

## Non-negotiable boundaries

1. No runtime CDN, remote font, image hotlink, wiki dependency, MOD service dependency, or online
   plugin-store dependency. Approved assets are vendored, hashed, optimized, and served same-origin.
2. No copied MOD JavaScript, Python, server behavior, persistence model, or protocol assumptions.
   Reuse artwork and interaction concepts; implement against MACKES contracts.
3. No fabricated audio, MIDI, or CV topology. Audio/CV sockets and cables appear only when PiPedal
   supplies authoritative ports and connections. MACKES MIDI routes remain visually distinct.
4. No protocol identifiers in normal workflows. URI, symbol, runtime instance ID, endpoint ID, raw
   MIDI, SysEx, or JSON remains confined to deliberate read-only Advanced Details.
5. Preserve `observed`, `acknowledged`, `sent-unverified`, `last-known`, `stale`, `unavailable`,
   `pending`, `conflict`, and `disconnected` distinctions. Artwork may reinforce but never replace
   textual and accessible state.
6. Imported art must not weaken the existing 44 px target, keyboard, 200% zoom, reduced-motion,
   light-theme, clean-console, offline, asset-budget, or same-origin requirements.
7. Continuous meters and control values may be coalesced. Button, bypass, preset, snapshot,
   lifecycle, error, and persistence events may not be dropped or visually inferred.
8. The daemon remains the sole mutation and persistence authority. Canvas drag, cable drawing,
   plugin control selection, and library browsing are previews until a typed, generation-checked
   confirmation succeeds.

## Artwork acquisition and packaging

Orion must inventory candidate files before importing them. Expected useful classes include generic
pedal chassis, rack panels, knobs, rotary caps, toggles, footswitches, LEDs, meters, input/output
jacks, audio/MIDI/CV connector states, cable endpoints, shadows, pedalboard surfaces, and neutral
snapshot/bank/preset/routing icons.

Create `apps/mackes-web/static/vendor/mod-art/manifest.json` with one record per file:

- stable MACKES asset ID;
- upstream project and canonical URL;
- upstream path and immutable commit SHA;
- upstream file SHA-256 and vendored file SHA-256;
- media type, pixel dimensions or SVG view box, and compressed size;
- copyright/creator and original license notice;
- permission basis (`operator-confirmed-remote-repository-approval-2026-09-13` for this epic);
- transformations such as crop, recolor, optimization, or SVG sanitization;
- allowed component roles and prohibited branding use;
- replacement/fallback asset ID.

Store the imported source notice beside the manifest. Strip scripts, event handlers, external URLs,
embedded fonts, metadata leaks, and unsafe SVG features. Raster assets receive bounded dimensions,
responsive variants only when materially useful, and deterministic optimization. SVG assets are
sanitized and assigned stable view boxes. A repository guard validates the manifest, hashes,
same-origin references, duplicate IDs, unsupported media, dimensions, and total compressed budget.

## Visual component system

Build a MACKES-owned renderer layer around the approved art. Components consume typed values and
emit ordinary DOM with an equivalent accessible list:

- `InstrumentChassis`: generic pedal, double pedal, compact rack, full rack, utility box, and
  fallback panel;
- `RotaryControl`, `FaderControl`, `ToggleControl`, `FootswitchControl`, and `MomentaryControl`;
- `StatusLed`, `LedRing`, `LevelMeter`, and clipping/peak-hold indication;
- `PortJack` and `SignalCable` for authoritative audio, MIDI, and CV links;
- `PluginFace`, `PluginInspector`, `PedalboardCanvas`, and `LibraryTile`;
- neutral preset, snapshot, bank, save, bypass, routing, warning, and unavailable icons.

All components support dark/light themes, forced colors, reduced motion, keyboard focus, touch,
high contrast, 320/768/1440 px layouts, and 200% zoom. Decorative imported images use empty alt
text; meaningful controls receive names, roles, values, ranges, availability, read/write truth,
and state from the typed capability model.

When a plugin-specific approved `modgui` face is available, place it inside the same semantic shell.
When it is missing, unsafe, too large, inaccessible, or unlicensed, generate a deterministic generic
face from plugin name, category, port count, and control metadata. The fallback is a first-class
design, not a broken-image state.

## PiPedal pedalboard canvas

Add a read-first canvas to `/studio/devices`:

- render input/output hardware blocks and plugin instances in authoritative order;
- render stereo/mono ports individually and label direction;
- distinguish audio, MIDI, and CV connections with shape, text, and pattern as well as color;
- show bypass, availability, stale state, and last-known state on each plugin;
- show selected plugin in a persistent inspector, collapsing below the canvas on narrow screens;
- pan/zoom with buttons, keyboard, pointer, and touch; provide Fit and Reset view actions;
- preserve selection, pan, zoom, focus, and scroll through bounded refresh/reconnect;
- provide an equivalent ordered topology list and connection table;
- keep editing disabled until a later typed operation explicitly supports it.

Unknown plugins use the generic renderer. Missing instances and late events are marked stale and
removed only after an authoritative replacement snapshot. A connector restart must not attach an old
runtime instance event to a replacement plugin.

## Plugin faces and inspector

The compact face exposes a curated safe subset: plugin name, bypass, a bounded number of primary
controls, signal activity, and truthful state. The inspector exposes all catalogued parameters,
search, groups, units, ranges, enumerations, presets, snapshots, writable/read-only distinction,
current/last-known value, pending mutation, and source limitation.

Parameter controls use catalog ranges directly; no 0–127 assumption is allowed for native PiPedal
ranges. Read-only meters cannot be edited. Send-only features never look observed. Persistent or
destructive operations require explicit confirmation. The normal face and inspector must not expose
plugin URIs, symbols, or runtime IDs.

## Cables, meters, and gain staging

Adopt MOD's useful physical metaphor while retaining MACKES truth:

- audio, MIDI, and CV links have distinct color plus line pattern and text legend;
- ports expose source/destination, channel count, connection state, and accessible relation;
- disconnected links remain visible as last-known only when the snapshot says so;
- input/output and inter-plugin meters use the existing coalesced event path;
- meter scales, clipping thresholds, units, and peak hold come from capability metadata;
- a missing meter is `Unavailable`, never zero;
- animation is frame-bounded and disabled/reduced under reduced-motion preferences.

The canvas must remain responsive during a 1,000-event burst and must not cause assignment, scene,
or device writes.

## Pedalboard, bank, preset, and snapshot library

Create a hierarchy that explains rather than conflates state:

```text
MACKES setlist
  └─ MACKES scene
       ├─ mapping layer and assignments
       └─ PiPedal pedalboard
            └─ PiPedal snapshot
                 └─ plugin preset or parameter state
```

Library tiles use approved neutral art or generated thumbnails. Search and grouping operate on
musician-facing names. Save, Save As, rename, delete, reorder, import, and load actions appear only
when their connector operation is qualified. Preview precedes Apply/Load. Snapshot edits clearly
state when the parent pedalboard must also be saved. Selection does not mutate. Destructive actions
require confirmation and preserve the current selection/draft on rejection.

## Direct visual assignment

Selecting a rendered writable plugin control starts the existing assignment workflow with the
destination preselected. The musician then chooses a compatible Novation control, previews the
complete mapping, and explicitly assigns it. Alternatively, selecting a Novation control first
continues to open the compatible destination browser.

The direct flow must preserve ranges, curves, invert, layers, LED intent, multi-destination fields,
unknown extension fields, generation, and Undo. Read-only meters, unavailable controls, and
incompatible physical control roles cannot be assigned. Physical movement may select/capture but
never commit.

## Qualification matrix

Automated evidence must cover:

- asset manifest/hash/provenance and SVG sanitization failures;
- missing, malformed, oversized, and mismatched artwork fallbacks;
- known and unknown plugin faces, pedal/rack variants, bypass and unavailable states;
- mono/stereo audio, MIDI, CV, branching, disconnected, stale, and replacement topology;
- equivalent accessible topology and full keyboard navigation;
- inspector search, range/enum units, read-only meters, send-only controls, and hidden identifiers;
- 1,000-event meter/control burst, bounded DOM growth, p95 local response, and memory stability;
- snapshot/pedalboard hierarchy, preview, confirmation, conflict, partial failure, and reconnect;
- both assignment directions, compatibility, multi-destination, lossless round trip, reload, and Undo;
- 320/768/1440 px, 200% zoom, light/dark, forced colors, reduced motion, pointer/touch/keyboard;
- offline startup, delayed assets, missing asset fallback, stream gaps, stale generations, and no echo;
- installed root/deep links, content hashes, clean console, service restart, rollback, and release gate.

Human review verifies recognizability, visual hierarchy, cable legibility, control discoverability,
and that imported art does not overwhelm state truth. Automated accessibility and programmatic
acceptance remain mandatory even when moderated review is waived.

## Delivery packets

### W204 — Permission record and artwork provenance

Inventory candidate MOD and plugin assets, pin revisions, build the manifest and notice, record the
operator-confirmed remote-repository approval, reject out-of-scope branding classes, and add
hash/sanitization guards.

### W205 — Art package and semantic instrument primitives

Vendor the approved files; build generic pedal/rack/control/port/cable/meter/icon components and
fallbacks; integrate themes, accessibility, and component-gallery coverage.

### W206 — Authoritative PiPedal signal-flow canvas

Project the current pedalboard graph into a scalable, selectable, accessible read-only canvas with
stable layout, reconnect/replacement safety, pan/zoom, and topology-list parity.

### W207 — Metadata-driven plugin faces and inspector

Render approved or generic plugin faces and bind every visible control to catalog range, units,
read/write capability, feedback truth, search, grouping, bypass, presets, and snapshots.

### W208 — Cables, meters, and gain-staging feedback

Render typed connection families, live coalesced meters, clipping/peak truth, unavailable states,
legends, reduced motion, burst performance, and accessible connection relations.

### W209 — Pedalboard/snapshot/preset library

Implement the hierarchy and thumbnail library with preview-first operations, qualified action
visibility, conflict safety, explicit parent-save semantics, search, keyboard/touch use, and empty,
loading, unavailable, stale, and partial-failure states.

### W210 — Direct graph-to-controller assignment

Connect rendered plugin controls to the existing assignment transaction in both directions while
preserving compatibility, advanced fields, layers, LEDs, multi-destination behavior, generation,
reload, and Undo.

### W211 — Resilience, accessibility, and performance closure

Exercise the full qualification matrix, resolve findings, retain useful fallbacks under missing art
or API/device failure, and prove event, DOM, memory, accessibility, responsive, and input budgets.

### W212 — Install, qualify, and hand off the visual release

Run the release gate, build and install the exact tested artifact with configuration backup, verify
served hashes and service recovery, run the aggregate installed suite, update screenshots/rollback,
record the asset manifest revision, and provide a successor-maintenance guide.

## Execution order

```text
W203 → W204 → W205 ─┬─ W206 ─┐
                    ├─ W207 ─┤
                    ├─ W208 ─┼─ W210 → W211 → W212
                    └─ W209 ─┘
```

W206–W209 may proceed in parallel after W205 only when each executor owns disjoint files and the
component/data contracts are frozen. Orion remains accountable for integration and may delegate
bounded research or fixture work without transferring the epic.

## Definition of done

W203 closes only when W204–W212 are all `DONE`, every imported asset is present in the validated
manifest, all programmatic acceptance passes against the installed artifact, normal workflows
remain protocol-free, no unsupported topology or device state is invented, rollback is rehearsed,
and the worklist records exact commands and observed results.
