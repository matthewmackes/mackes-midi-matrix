# Reproducible development tooling

The first-class environment is Fedora Linux 44 x86_64. The following tools are required for the
default local checkpoint:

| Tool | Minimum/pinned source | Purpose |
|---|---|---|
| Rust compiler/Cargo | Fedora Rust 1.97.1; crate MSRV 1.85 | Build and tests |
| rustfmt | Matching Fedora Rust package | Formatting gate |
| Clippy | Matching Fedora Rust package | Lint gate |
| Python 3 | Fedora system Python | Worklist/schema/artifact checks |
| ALSA utilities | Fedora package set | MIDI port enumeration and virtual-port tests |
| MIDI virtual-port backend | Testkit plus ALSA sequencer | Simulator-first routing tests |
| Packet capture utility | Release/test host only | RTP-MIDI interoperability evidence |

Run `scripts/verify-repository.sh` before every handoff. Hardware and network interoperability
tools are not required for default CI and must never be used to bypass simulator tests.
