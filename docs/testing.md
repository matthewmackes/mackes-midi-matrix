# Test taxonomy

The default workspace suite is deterministic and must not require hardware, network peers,
privileged paths, sleeps, or external services.

| Class | Invocation | Allowed dependencies |
|---|---|---|
| Unit/property | `cargo test --workspace --all-features` | in-memory values and fake clocks |
| Build/lint | `cargo build --workspace --all-targets`; `cargo clippy --workspace --all-targets --all-features -- -D warnings` | local toolchain |
| Hardware | `cargo test --workspace --features hardware -- --ignored` | explicitly named device/port and `--arm-hardware-write` |
| Network interoperability | `cargo test --workspace --features network-interop -- --ignored` | explicitly provisioned peers; never default CI |
| Repository policy | `scripts/verify-repository.sh` | Python standard library and local files |

Hardware and network-interop features remain opt-in. A test that can write device memory must be
ignored by default, print the exact destination and operation, and require explicit arming.

The test `two_independent_rtp_peers_validate_identity_and_sequence` is additionally deferred from
the release suite as post-release paired-peer qualification. Run it explicitly with `--ignored`
when two provisioned peers are available; the remaining hermetic RTP and safety scenarios remain
part of the release gate.
