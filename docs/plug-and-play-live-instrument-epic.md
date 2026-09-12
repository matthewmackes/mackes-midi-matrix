# Plug-and-Play Live Instrument Web Experience — W186–W202

This is the authoritative specification for the replacement of W180–W185. It turns the Studio
prototype into a polished, controller-first musical instrument interface for non-technical users.
The Launch Control XL remains the primary workspace. PiPedal, Eventide MicroPitch, Lexicon Reflex,
transports, virtual ports, and generic endpoints receive capability-appropriate supporting views.

## Product decisions

- Device discovery and safe binding happen automatically. Replacing mappings requires one reviewed
  Apply action; no connection may silently overwrite saved work or send a preset/parameter change.
- Feedback is evaluated per feature. A device may expose observed presets, send-only parameters,
  queryable status, writable controls, or no feedback at all.
- The UI uses `observed`, `acknowledged`, `sent-unverified`, `last-known`, `stale`, and `unavailable`
  as truthful states. An acknowledgment is never rendered as device observation.
- The home screen is the complete 56-control Novation faceplate. A compact status rail shows device
  connection, current scene/layer, preset or algorithm, sync, and actionable faults.
- Visible local response targets 100 ms p95. Supported device state converges within 500 ms p95.
  Continuous values and meters may be coalesced; button, lifecycle, preset, and error events may not.
- Normal workflows never expose MIDI, CC, SysEx, channels, endpoint IDs, plugin URIs, JSON, or dumps.
  Advanced Details may expose read-only technical identity for diagnosis.
- Use a dark-first studio-instrument visual language, a light theme, local fonts, code-native SVG/CSS,
  restrained meaningful motion, 44 px targets, visible focus, and no Carbon dependency or contract.

## Shared contracts

Add a versioned composite Studio snapshot containing stable device identity, lifecycle, per-feature
capability, mappings, scene, layer, presets/modes, control observations, LED state, meters,
freshness, generation, and event sequence. Extend the existing sequenced event stream with typed
device lifecycle, control, preset, mode, meter, mapping, scene, layer, LED-intent, and LED-delivery
events. Preserve existing API envelopes and generation checks.

Add one atomic, generation-checked starter-layout operation. The daemon recomputes and validates the
recommendation at Apply time, rejects stale or ambiguous input, and uses the existing one-level Undo
boundary. The daemon remains the only persistence and hardware-write authority.

## Acceptance walkthroughs

1. Start with no hardware, then connect supported devices in any order and see identification without
   a reload; ambiguous duplicates remain visible for explicit selection.
2. Review a recommended starter layout, apply it once, reload, and Undo without losing custom work.
3. Move every Novation control and verify matching graphical response within the latency budget.
4. Change PiPedal state externally and verify browser/controller readback without an event echo loop.
5. Change a qualified Reflex algorithm or parameter and verify active-setup convergence.
6. Send an Eventide operation and verify the UI remains explicit when the pedal cannot confirm it.
7. Disconnect during a pending write, preserve the draft, resnapshot after reconnect, and prevent
   uncertain-write replay.
8. Complete the assignment by keyboard at 320 px and 200% zoom with a clean accessibility tree.
9. Render every known, generic, virtual, limited, disconnected, ambiguous, and unknown endpoint
   graphically and with an equivalent accessible list.
10. Verify installed `/`, asset hashes, service restart, browser reconnect, release gate, clean
    console, and documented rollback.

## Execution order

```text
W186 → W187
          ├─ W188 ───────────────┐
          ├─ W189 → W190 ────────┤
          └─ W191 ───────────────┤
                                 ↓
                        W192 guided setup
                                 ↓
                 W193–W197 parallel device views
                                 ↓
                        W198 assignments
                                 ↓
                        W199 scenes/recall
                                 ↓
                        W200 resilience
                                 ↓
                        W201 qualification
                                 ↓
                        W202 cutover
```

W188, W189, and W191 may run in parallel after W187. W193–W197 may run in parallel after their
shared state and visual dependencies are complete. Fixtures may unblock software work, but native
hardware evidence is required before claiming physical feedback.
