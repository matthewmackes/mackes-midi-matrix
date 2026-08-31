# Atomic Ampli-Firebox V1 transport research

## Outcome

The original Atomic Ampli-Firebox should not be modeled as an ordinary MIDI endpoint. The V1
manual documents USB editor, preset/IR transfer, backup/restore, and firmware functions, but no
DIN MIDI connector or MIDI implementation. The official Windows editor uses a vendor-defined USB
HID transport. On the local Fedora host the connected unit confirms that design:

- product: `Atomic Amplifiers AmpliFireBox`
- USB identity: `04d8:003f`
- interface: vendor-defined HID (`/sys/class/hidraw/hidraw3` during observation)
- HID descriptor: one 64-byte input report and one 64-byte output report, with no report ID
- USB endpoints: interrupt IN `0x81` and interrupt OUT `0x01`, each 64 bytes at interval `1`
- ALSA MIDI endpoint: none observed

The platform therefore has a discovery-only `atomic.ampli-firebox.v1` profile using
`ControlTransport::UsbVendor`. It exposes no controls and authorizes no writes.

## Realtime communication requirement

Realtime editor behavior requires listening continuously to the HID input report stream. The
official editor contains an `AmplifireUSBPacketIO` implementation backed by HID read/write calls,
and its release instructions say the editor synchronizes after USB connection. A MIDI-port listener
cannot observe this traffic because the device does not enumerate as ALSA MIDI.

A future adapter needs a dedicated bounded HID worker that:

1. opens only USB `04d8:003f` after product-string and descriptor confirmation;
2. continuously reads complete 64-byte reports on a nonblocking or bounded-timeout path;
3. records direction, monotonic timestamp, report length, and raw bytes before decoding;
4. correlates host requests with device responses without treating unsolicited reports as replies;
5. publishes decoded state only after repeated captures establish stable semantics;
6. keeps all output reports disabled until reviewed fixtures prove command framing and safety.

The existing MIDI input provisioning in `mackesd` is not reusable for this transport. It discovers
ALSA MIDI through `midir`; the Firebox requires a separate HID adapter feeding the same bounded
daemon event/state boundary after decoding.

## Capture plan

Use the official V1 editor and the connected pedal for two independent capture sessions. Begin
with a read-only baseline, then change exactly one reversible control at a time:

1. editor launch and initial synchronization with no user interaction;
2. one hardware knob movement while the editor is open;
3. one editor knob movement and return to its original value;
4. one preset selection and restoration;
5. one effect enable toggle and restoration;
6. editor disconnect/reconnect without firmware update, backup restore, IR upload, or preset save.

Capture USB HID reports with timestamps and preserve firmware `3.3.0`, editor `3.3.2.1`, USB
identity, descriptor digest, and before/after state. Firmware update, system restore, preset writes,
and cabinet IR transfer remain excluded until ordinary read/write framing is understood.

## Evidence

- Atomic support page: <https://atomicamps.com/atomic-amps-support/>
- Official V1 manual: <https://atomicamps.com/wp-content/uploads/2017/10/ATM_Ampli-FireBox-Manual-V1.1.1_10.16.17.pdf>
- Official V1 release notes: <https://atomicamps.com/wp-content/uploads/2019/01/AFB-Release-Notes-3.3.2.1.pdf>
- Official editor package inspected: `AmpliFireboxEditor_3_3_2_1_Win.zip`, SHA-256
  `15c81c321c72131f2ded57d3df33fe4b48087b299b20b8e8435f61568f6ac8a2`

The editor binary contains source-path and diagnostic strings for `AmplifireUSBPacketIO.cpp`,
`hid_read`, `hid_read_timeout`, `hid_write`, and 64-byte HID packet handling. This establishes the
transport family but not command semantics.

A disassembly pass initially appeared to show packet-construction routines beginning with
`0xB2`, `0xB1`, `0xC4`, `0xC2`, `0xC0`, and `0xC5`. The Mac build resolves the ambiguity: these
values are used as parameter IDs in a metadata table, with associated default IEEE-754 values.
They are not authorized USB command opcodes.

The observed parameter metadata is:

| Parameter ID | Static metadata |
| --- | --- | --- |
| `B1`, `C0`, `C2`, `C4`, `C5` | Parameter-table entries, with values populated by the editor’s preset metadata |
| `B2` | Also appears in the parameter table; earlier payload interpretation was withdrawn |

This evidence is not sufficient to authorize writes: the actual USB framing, command semantics,
and response/CRC rules still need to be tied to an observed editor transaction.

The official Mac editor was also obtained from Atomic’s V1 support link. Its Mach-O executable
(`AmpliFireboxEditor.app/Contents/MacOS/AmpliFireboxEditor`) is 64-bit and hashes to
`e13f38ef3c4c51f33e6b9cade23013f17a87693ad0ec4e5fef0d1fa6508c404f`. It retains the same
`AmplifireUSBPacketIO.cpp` diagnostics and protocol error strings as the Windows build, but is
stripped and exposes no additional command semantics or standalone firmware image.

## Linux connector status

`mackes-firebox` provides a raw hidraw connector with passive capture and an
evidence-gated stateful request/response session, plus the
`firebox-capture` utility. Its tab-separated output is:

The installed `firebox-monitor [hidraw-path]` provides continuous read-only
monitoring with bounded reconnect reopening. It emits timestamps, changed
offsets, currently correlated analog/switch deltas, and the complete raw
report; it never opens the Firebox writer or sends an output report.

1. report sequence;
2. monotonic elapsed microseconds;
3. changed zero-based byte offsets;
4. named analog values (`Gain` through `Presence`);
5. named raw switch fields for offsets 10 and 11; and
6. the complete 64-byte report as hexadecimal.

The analog and switch fields are correlations from live captures. Position labels are not
assigned where the captures did not prove them. No output report is emitted by this connector.

The official V1.2 manual identifies three physical three-position toggles: STYLE, CHANNEL, and
CAB 1/4-inch/CAB XLR. STYLE and CHANNEL select one of nine preset slots; CAB selects cabinet IR
routing for the 1/4-inch output, both outputs, or the XLR output. The manual describes USB as the
editor path for preset editing, cabinet-IR upload, and firmware, and does not document a MIDI
interface for the Firebox. These documented identities do not yet establish their raw HID byte
offsets or position encodings.

### Operator usage

With the unit connected, the connector auto-discovers its hidraw node:

```text
cargo run -p mackes-firebox --bin firebox-capture -- 100
cargo run -p mackes-firebox --bin firebox-sync
cargo run -p mackes-firebox --bin firebox-request -- c5w:01:000000
```

`firebox-raw` accepts a complete 128-character hexadecimal report for replay of a recovered
editor frame. These tools preserve the full response and do not claim a semantic write succeeded
unless a later capture demonstrates the corresponding device-state change.

The official editor trace also captured one reversible parameter transaction: the editor logged
`setParameterValue(113, 21.360001)`, emitted `C7 08 71 00 00 00 48 E1 AA 41` (zero-padded to the
64-byte HID report), and received a `C0 01 C7` acknowledgment. Linux replay is available as
`firebox-request -- c7f:71:41aae148`; this confirms frame and acknowledgment transport only. The
parameter identity, complete range map, and persistence semantics remain unassigned, so the
global vendor-write authorization stays disabled.

The Mac binary also exposes useful class-level names such as `AmplifireProtocol::readData`,
`sendData`, `setCurrentPreset`, `getCurrentPreset`, `ping`, and `getFirmwareVersionInfo`, plus
`FireboxParamMap`. These names establish the operation boundary for a future protocol codec, but
the stripped binary still does not reveal the underlying request bytes or response checksum.

Further Mac disassembly identified one higher-confidence frame candidate: a protocol helper writes
the two-byte buffer `CE 00` and submits it through the packet-transaction virtual call, with a
nearby bounded structured-response path. The helper’s exact class method could not be recovered
from the stripped symbol table, so `CE 00` remains an unverified request candidate only; it has
not been sent to the unit.

The only recovered call site for this helper lies in the editor’s connect/refresh workflow, near
the paths that report failed preset refresh and firmware/version status. This raises confidence
that `CE 00` is control-plane traffic, but does not distinguish ping, preset request, or version
query; no connector operation is assigned to it.

Native Linux sequence replay established stateful behavior: `B5 00` returns `C0 07 B5`, `B4 00`
returns `C0 01 B4`, and after `C5 05 01 00`, `C0 00` returns a `C3 3E` response containing
structured fields with IEEE-754-looking values. The same `C0 00` request times out when isolated.
This is evidence of a preset-read transaction sequence; field meanings and write semantics remain
unassigned.

The editor’s internal API supplies logical lengths to `hid_write`, while the Linux connector
currently emits the device’s negotiated 64-byte output report. The short C5 payload is therefore
zero-padded at the report boundary; this transport choice is validated for the observed V1 HID
descriptor, but larger transfer framing still requires separate qualification.

An additional public search found no independent USB command implementation. Atomic’s product
page confirms that USB is used for deep editing, preset backup/restore, footswitch customization,
and preset import, while the V1.2 manual defines the physical preset selection and CAB routing but
does not publish a USB packet specification.
