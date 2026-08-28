# ADR-0001: Toolchain and dependency policy

- Status: accepted for W001
- Date: 2026-08-25

MACKES targets Fedora Linux 44 x86_64 and pins the Fedora-aligned Rust 1.97.1 toolchain in
`rust-toolchain.toml` for reproducible development; crate manifests retain Rust 1.85 as the
minimum supported language version. The workspace forbids unsafe Rust. Libraries use typed errors;
application binaries may add context at process boundaries. New runtime dependencies require
license/advisory review in W002 and must preserve the dependency direction in `WORKLIST.md`.
