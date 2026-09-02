# Architecture map

The Rust workspace is the canonical implementation tree. Top-level documentation and packaging
directories support that tree; empty compatibility directories are not product APIs and must not
be used as alternate implementations.

| Path | Boundary | Allowed workspace dependencies |
| --- | --- | --- |
| `crates/domain` | MIDI-neutral values and validation | none |
| `crates/config` | persisted configuration and migration | `domain` |
| `crates/ipc` | bounded local daemon protocol | `config`, `domain` |
| `crates/midi-engine` | routing plus optional ALSA/midir adapters | `domain` |
| `crates/profiles` | profile contracts and controller layout | `domain` |
| `crates/scene-engine` | scene planning | `domain` |
| `crates/tui` | rendering and operator input adaptation | `config`, `domain`, `ipc`, `midi-engine`, `profiles` |
| `crates/testkit` | deterministic cross-layer fixtures | workspace crates only |
| `apps/mackes` | operator composition; no physical MIDI ownership | workspace crates only |
| `apps/mackesd` | daemon composition and sole physical-MIDI owner | workspace crates only |

`midir` and native ALSA adapter construction are confined to `crates/midi-engine` and daemon
startup. The operator application talks to hardware only through daemon IPC.

## Module-extraction policy

Oversized roots are being split without changing public behavior. New code belongs in a focused
private module; a root `lib.rs` or `main.rs` should become composition and deliberate re-exports,
not a second implementation. Until the extraction is complete, the architecture check enforces
reviewed ceilings for the current roots:

| File | Maximum lines |
| --- | ---: |
| `crates/profiles/src/lib.rs` | 3,100 |
| `crates/midi-engine/src/lib.rs` | 3,100 |
| `crates/tui/src/lib.rs` | 4,200 |
| `apps/mackesd/src/lib.rs` | 3,600 |
| `apps/mackes/src/main.rs` | 800 |

Extracted private modules hold characterized behavior: `profiles::lexicon_reflex`,
`midi-engine::rtp`, `tui::render`, operator `cli`/`interactive`, and crate-local `tests.rs`
files. Ceilings track the remaining composition roots. They may only be raised with an ADR.

Run `python3 scripts/check-architecture.py` locally; repository verification and the release gate
run it automatically.
