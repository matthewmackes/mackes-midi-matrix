# Graphical studio human sign-off checklist

This checklist is the moderated review record for W176/W177. It is written for a musician who
understands studio equipment but does not need to know MIDI protocol. The reviewer records the
result for each scenario as `PASS`, `FAIL`, or `NOT OBSERVED`; an automated green check does not
replace a human observation.

## Review setup

- [ ] Use the installed host at `http://172.20.222.222:8081`.
- [ ] Capture the browser size and theme for each scenario: 320, 768, and 1440 CSS pixels;
  light and dark themes.
- [ ] Enable reduced motion for one pass and browser zoom at 200% for one pass.
- [ ] Record browser console output; expected result is no application errors.
- [ ] Keep the daemon as the sole writer. Do not paste JSON, JSON5, SysEx bytes, runtime IDs, or
  protocol addresses into the browser.
- [ ] Do not perform a physical write unless the device-specific procedure and operator approval
  are recorded separately. A missing readback is reported as unknown, never inferred as success.

## Novice walkthrough

| Scenario | Reviewer action | Expected graphical result | Result / evidence |
|---|---|---|---|
| Find a device | Open Devices and identify Launch Control XL, MicroPitch, MIDISPORT, and PiPedal | Each item has a recognizable SVG chassis/topology, named state, ports, and an accessible summary | |
| Understand signal flow | Open the studio flow and select a node | Source/destination ports and connection state are understandable without IDs or code | |
| Inspect a control | Select an assigned Novation knob, then an unassigned control | Inspector explains destination/source/behavior and says readback is unavailable when it is | |
| Build a connection | Open Routing and add or edit a connection using named selectors | The card uses device/port names, conditions, range/curve controls, and plain-language validation | |
| Preserve a draft | Edit a route, refresh or navigate away, then return | The draft remains visible and stale/conflict handling asks for reconciliation before Apply | |
| Build a scene | Open Scenes and add/reorder an action card | Target, operation, value, order, preview, and failure/recovery state are graphical | |
| Recover safely | Open System/Recovery with the service unavailable or a stale write | Health, unknown outcome, reconnect, and next action are explained without a dump | |
| Use keyboard only | Tab through the primary task, activate a control with Enter/Space | Focus is visible; no essential operation requires hover, drag, or color alone | |

## Acceptance disposition

- [ ] No normal view presents a code editor, raw protocol input, state dump, or internal identity.
- [ ] Every connected, disconnected, researched, generic, virtual, and unknown endpoint remains
  graphical and has an accessible equivalent.
- [ ] States are communicated with text or shape/label cues in addition to color.
- [ ] The reviewer can explain what Apply, Cancel, Reconnect, and Unknown outcome mean without
  learning MIDI terminology.
- [ ] Failures have an obvious safe next step and do not silently retry hazardous operations.
- [ ] Any failed or unobserved row is linked to a worklist item; W176/W177 remain open until the
  reviewer records a disposition.

Reviewer: ____________________  Date: ____________________  Build/revision: ____________________

Overall disposition: `PASS` / `FAIL` / `NOT OBSERVED`

Notes and artifact paths:

________________________________________________________________________________

________________________________________________________________________________
