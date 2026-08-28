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
