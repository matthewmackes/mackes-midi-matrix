# Graphical device renderer ledger

This ledger is the W168 audit baseline for the graphical studio interface epic. A renderer is
required for every endpoint kind before W177 can close. `qualified` means the renderer may expose
model-specific controls backed by governed capability data; `generic` means it must show useful
chassis, ports, identity, availability, and activity without inventing unsupported controls.

| Endpoint family | Renderer key | Representation required | Qualification boundary | Current status |
|---|---|---|---|---|
| Novation Launch Control XL Mk1/Mk2 | `novation.launch-control-xl` | Faithful controller faceplate: 24 knobs, 16 channel buttons, 8 utility controls, 8 faders, LEDs, ports | Programmer Reference and qualified profile; no native readback claim without observation evidence | Existing partial SVG; not complete |
| Eventide MicroPitch | `eventide.micropitch` | Pedal chassis, footswitches, preset area, parameter controls, MIDI ports | Eventide profile and manufacturer guide; sent-unverified when feedback is unavailable | Catalog only; renderer missing |
| Lexicon Reflex | `lexicon.reflex` | Rack front panel, algorithm/parameter area, Echo Rhythm, patches, registers, MIDI ports | MIDI implementation and codec metadata; persistent operations visibly distinguished | Catalog only; renderer missing |
| PiPedal | `pipedal` | Pedalboard/plugin graph, plugin controls, snapshots, presets, levels, MIDI ports | Pinned server operation audit; unsupported operations remain explicit | Catalog only; renderer missing |
| M-Audio MIDISPORT 4x4 | `m-audio.midisport-4x4` | Interface chassis with four inputs and four outputs, activity and cable/route state | Manufacturer capability evidence; firmware readiness separate from cable state | Catalog only; renderer missing |
| RTP-MIDI | `rtp-midi` | Network/session node with peer, invitation, handshake, virtual ports, health counters | AppleMIDI/RFC session behavior; reconnect and reorder state are distinct | Catalog only; renderer missing |
| Generic MIDI | `generic-midi` | Generated MIDI chassis with named direction-aware ports and activity | Qualified generic endpoint schema; no model-specific controls | No dedicated renderer |
| MACKES virtual/monitor | `mackes.virtual-monitor` | Virtual endpoint/monitor topology with ports, activity, and availability | Daemon-owned endpoint projection; no physical hardware claims | No dedicated renderer |
| Unknown future endpoint | `generic.endpoint` | Safe generated chassis with identity, ports, capabilities, availability, and help | Must fail closed for unsupported controls while remaining graphical | Fallback missing |

## Audit findings

The normal browser surface currently contains these violations:

- `index.html`: raw JSON5 draft, SysEx destination/data fields, technical device-control inputs,
  assignment catalog `<pre>`, faceplate `<pre>`, route JSON, scene-action JSON, and state `<pre>`.
- `app.js`: JSON parsing/edit flows for routes, scenes, and configuration; raw state serialization;
  technical identifiers and protocol values in normal controls; text-only inventory cards.
- Renderer coverage: only the Novation control SVG is present, and it is a control overlay rather
  than a complete renderer registry for all endpoint families in this ledger.

Readable status and help text are not findings. Downloadable raw backups and monitor captures are
allowed artifacts, but their contents must not become an in-browser editor or dump viewer.

## Closure evidence required for W168

1. A static guard scans normal HTML/JS surfaces and fails when forbidden editor/dump patterns return.
2. The runtime renderer registry has one exact qualified entry for each supported family plus the
   mandatory `generic.endpoint` fallback.
3. Browser fixtures prove every ledger row renders a graphical representation and an accessible
   list equivalent, including disconnected and unknown states.
4. The audit is re-run after W169–W175 and the findings above are either removed or documented as a
   permitted download-only boundary.
