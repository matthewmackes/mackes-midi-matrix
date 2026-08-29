# mackes-midi-matrix

`mackes-midi-matrix` is a Fedora Linux MIDI routing and controller-management application written in Rust. It combines a persistent MIDI daemon with a terminal user interface (TUI), giving musicians and technicians a single place to route MIDI, manage device profiles, work with SysEx, and recall scenes.

The daemon owns MIDI I/O and continues running when the TUI exits, while the operator interface communicates with it through a local IPC contract. Core behavior is kept in reusable crates so routing, configuration, profiles, scenes, and UI behavior can be tested independently.

## Project status

This repository is in foundation implementation and integration work toward v1.0. The current release line is `0.1.9`. Hardware and network interoperability features are deliberately opt-in until they have the required documentation, fixtures, and validation evidence.

## What it provides

- Persistent `mackes-midi-matrixd` daemon for MIDI I/O and routing.
- `mackes-midi-matrix` terminal application for operator control.
- Typed domain model for ports, routes, messages, devices, profiles, and scenes.
- JSON5 configuration with schema validation and configuration hashing.
- Device profiles and scenes for repeatable controller mappings and states.
- SysEx and device-control foundations with safeguards around writes.
- Local IPC separating the UI lifecycle from the daemon lifecycle.
- Simulator-first tests that do not require physical hardware.

## Architecture

The Cargo workspace contains two applications and shared crates:

| Component | Responsibility |
| --- | --- |
| `apps/mackes` | TUI/CLI operator application (`mackes-midi-matrix`) |
| `apps/mackesd` | Persistent MIDI daemon (`mackes-midi-matrixd`) |
| `crates/domain` | Shared domain types and contracts |
| `crates/config` | JSON5 loading, validation, and hashing |
| `crates/ipc` | Local daemon/client communication |
| `crates/midi-engine` | MIDI routing abstractions and optional `midir` backend |
| `crates/profiles` | Device profile loading and behavior contracts |
| `crates/scene-engine` | Scene state and recall logic |
| `crates/tui` | Reusable terminal UI components |
| `crates/testkit` | Fakes, fixtures, and deterministic test support |

See `WORKLIST.md` and `docs/decisions/` for governing decisions and contracts.

## Language and tools

- **Language:** Rust, edition 2021
- **Build:** Cargo; pinned development toolchain Rust 1.97.1; crate MSRV Rust 1.85
- **Platform:** Fedora Linux 44, x86_64
- **MIDI:** `midir`, ALSA utilities, and virtual MIDI ports for Linux testing
- **TUI:** `ratatui` and `crossterm`
- **Data:** Serde, JSON/JSON5, and SHA-256 via `sha2`
- **IPC/Unix integration:** `nix` and local Unix socket contracts
- **Quality tooling:** Rustfmt, Clippy, Cargo tests, Python 3 checks, and release scripts

## Getting started

Install the Fedora prerequisites described in [`docs/tooling.md`](docs/tooling.md), then verify the repository:

```bash
./scripts/verify-repository.sh
```

Build and run either application:

```bash
cargo build --workspace --all-targets
cargo run --package mackes-midi-matrix
cargo run --package mackesd
```

Launch the interactive terminal UI with `mackes-midi-matrix` (or `mackes-midi-matrix tui`). While it is running,
number keys select the available workspaces: `1` Dashboard, `2` MIDI Learn, `3` Reflex,
`4` Eventide, `5` Routing, `6` Diagnostics, `7` Monitor, `8` Backups, and `9` Setlists.
The Routing workspace supports transactional add/remove/edit/save operations: `m` cycles
message class, `c` cycles channel scope, `e` enables or disables a route, `y` toggles bounded
cycle authorization, and `+`/`-` adjusts execution priority. Route rows also display the
effective curve and cycle policy before saving.
From the dashboard, `n`/`p` navigate scenes, `!` issues
the governed panic command, and `q` exits. The UI restores the terminal state on normal
and error exits.

Scenes can also be selected exactly from the CLI with `mackes-midi-matrix scene select <scene-id>`;
the daemon validates the active catalog and persists the selection.
Executable MIDI actions can be added atomically with `scene action-add <config> <project> <scene>
<action-id> <description> <destination> <midi-hex> [--unsafe]`; the payload may be a valid CC,
Program Change, note, or framed SysEx message.
Use `--depends-on=<action-id>` to place the new action after an existing action in the scene plan.
Remove an action with `scene action-remove <config> <project> <scene> <action-id>`; removal is
rejected when another action depends on it.
Scene action records may additionally declare a validated `destination` and complete MIDI `message`
payload for executable scene operations; legacy metadata-only actions remain supported.

For system-wide installation and service operation, see [`docs/installation-fedora.md`](docs/installation-fedora.md).

### Device and SysEx commands

The CLI exposes the same daemon-owned validation boundary used by the TUI. Device controls
require a profile control label, a one-based channel, a value within the profile range, a
registered output endpoint, and an explicit confirmation flag:

```bash
mackes-midi-matrix device-control eventide.micropitch Mix 1 64 <output-id> --confirm
mackes-midi-matrix device-query <profile-id> <query-id>
```

The confirmation-gated device-control path also supports captured M-VAVE IR Box operations:
`Preset 1`–`Preset 32`, `IR`, and `EQ`.

Raw SysEx is bounded to 1–1024 bytes, must be written as framed `F0 ... F7` hexadecimal, and
also requires explicit confirmation and a registered output:

```bash
mackes-midi-matrix sysex <output-id> "F0 7D 01 02 F7" --confirm
```

Both commands fail closed for unknown profiles or controls, invalid MIDI ranges, malformed
SysEx, unavailable destinations, or missing confirmation. Querying devices and inspecting
Route inspection remains read-only; `routes apply <routes.json>` performs an explicit,
bounded atomic route replacement through the daemon.

## Verification and testing

The default suite is deterministic and must not require hardware, network peers, privileged paths, sleeps, or external services:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

Hardware and RTP/network interoperability tests are ignored by default, require explicit features and named targets, and follow the safety rules in [`docs/testing.md`](docs/testing.md) and [`docs/hardware-qualification.md`](docs/hardware-qualification.md).

## Test-release qualification

The current release is `0.1.9`. Recreate its Fedora/Linux x86_64 bundle with:

The published release is available at
[`v0.1.9`](https://github.com/matthewmackes/mackes-midi-matrix/releases/tag/v0.1.9),
with assets named `mackes-midi-matrix-0.1.9-linux-x86_64.tar.gz` and its `.sha256`
manifest.

```bash
cargo build --workspace --release --locked
bash scripts/package-release.sh 0.1.9
(cd dist && sha256sum -c mackes-midi-matrix-0.1.9-linux-x86_64.tar.gz.sha256)
```

The bundle contains both binaries, the Fedora installer and service unit, the locked dependency
file, documentation, license, release notes, and a checksum manifest. The automated release gate
verifies formatting, repository policy, dependency advisories, workspace tests and documentation,
strict Clippy, optimized routing throughput, hermetic integration scenarios, and installer
preflight. Physical MIDI cabling, device writes, independent RTP-MIDI peers, and long-duration
hardware/network soaks remain explicit post-release qualification activities.

## Safety and contributing

Hardware writes, bulk dumps, and dense MIDI traffic are potentially destructive. Such tests are opt-in, display their exact destination and operation, and require explicit confirmation. Do not guess vendor SysEx bytes, checksums, LED messages, or reply semantics: changes must be supported by cited documentation, redacted fixtures, or physical validation.

Read `WORKLIST.md`, relevant ADRs, and [`CONTRIBUTING.md`](CONTRIBUTING.md) before contributing. Security concerns belong in [`SECURITY.md`](SECURITY.md). The project is licensed under the MIT License (`LICENSE`).

## Goals

1. Provide reliable, low-latency MIDI routing on Fedora Linux.
2. Make complex controller setups reproducible through profiles and scenes.
3. Keep the MIDI service available independently of the terminal UI.
4. Make device-control and SysEx workflows observable, reviewable, and safe.
5. Support deterministic simulator-based development before hardware testing.
6. Build a maintainable Rust foundation for future hardware and RTP-MIDI interoperability.

Near-term work is tracked in [`WORKLIST.md`](WORKLIST.md), the canonical implementation backlog.
