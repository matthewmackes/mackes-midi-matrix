# ADR-0002: Repository layout

- Status: accepted for W001
- Date: 2026-08-25

The workspace follows the canonical layout in `WORKLIST.md`: two application binaries and
dependency-directed crates for domain, configuration, IPC, MIDI, profiles, scenes, TUI, and test
support. Device profiles, schemas, fixtures, documentation, and system packaging have dedicated
top-level directories. No crate may open MIDI ports outside `midi-engine` or the daemon adapter.
