# ADR-0002: Repository layout

- Status: accepted for W001
- Date: 2026-08-25

The workspace follows the canonical layout in `WORKLIST.md`: two application binaries and
dependency-directed crates for domain, configuration, IPC, MIDI, profiles, scenes, TUI, and test
support. Rust workspace paths are the canonical implementation tree; documentation and system
packaging are supporting product contracts, not alternate source trees. No crate may open MIDI
ports outside `midi-engine` or the daemon adapter.

The exact permitted workspace dependency edges, physical-MIDI ownership boundary, and temporary
monolith-growth ceilings are recorded in [`../architecture.md`](../architecture.md) and checked by
`scripts/check-architecture.py`. Core code is extracted into private modules with characterization
coverage; root files are composition boundaries rather than a location for new feature code.
