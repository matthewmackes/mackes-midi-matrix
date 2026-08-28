# Hardware qualification matrix

This matrix records observation-only results from the Fedora 44 x86_64 host. It
does not authorize MIDI, SysEx, HID, or LED writes.

| Device | USB identity | Host transport observed | Current status | Required before production |
| --- | --- | --- | --- | --- |
| Lexicon Reflex | DIN via MIDISPORT Port A (`hw:4,0,0`) | ALSA MIDI Port A | Parameter and register operations verified | Checksum/recovery and physical reconnect are post-release qualification |
| Eventide MicroPitch | `1b12:003a` | ALSA MIDI (`MicroPitch Pedal MIDI 1`) | PC1, CC4 sweep, CC15, and ALSA reopen verified; an earlier CC2 transmission is not control evidence because the official map assigns ACTIVE/BYPASS to CC14 | Re-run reversible ACTIVE/BYPASS on CC14; independently confirm audio behavior |
| Novation Launch Control XL Mk1 | `1235:0061` | ALSA MIDI plus HUI endpoint | Input/output endpoints visible; controls are user-template programmable | Bind imported template assignments, then verify page mapping and LED reports |
| M-Audio MIDISPORT 4x4 | `0763:1020` loader → `0763:1021` runtime | ALSA USB-MIDI; four MIDI ports | Firmware-loaded and enumerated | Four-port routing and physical disconnect/reconnect are post-release qualification |

Reproduce the observation report with:

```text
scripts/qualify-hardware.sh
```

For a MIDISPORT that is present as loader identity `0763:1020`, install
`fxload` and `midisport-firmware`, then trigger its udev add rule for the
device path (the path varies by topology):

```text
sudo udevadm trigger --action=add /sys/bus/usb/devices/<midisport-path>
sudo udevadm settle
amidi -l
aconnect -l
```

Successful initialization changes the USB identity to `0763:1021` and exposes
four MIDI input/output ports.

Physical qualification must use a verified map record and explicit
`MACKES_CONFIRM_PHYSICAL_WRITE=1`; see `scripts/physical-write-guard.sh`.

The Fedora `fxload` and `midisport-firmware` packages are installed on the
qualification host, and the connected MIDISPORT has transitioned from loader
identity `0763:1020` to runtime identity `0763:1021`. The four runtime ports
are therefore available for the remaining physical routing and reconnect
qualification; package installation is no longer a prerequisite.
