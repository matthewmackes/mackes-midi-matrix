# ADR-0014: Coalesce short-lived read bursts at the web boundary

Status: Accepted (2026-09-08)

## Context

The same-origin web adapter creates one local IPC exchange per incoming HTTP
request. A bounded soak of 100 Health, Novation, and Mappings requests exposed
the cost of that fan-out: the daemon stayed active but later Health requests
returned `503 Resource temporarily unavailable (os error 11)`, with broken-pipe
records from abandoned client sockets. The authoritative daemon projection and
the typed IPC protocol remain the source of truth; the browser does not need a
fresh identical read for every concurrent caller.

## Decision

Cache successful read responses for a bounded window, keyed by the control-socket
path and IPC command, before opening another daemon connection. Health uses 100 ms,
Snapshot 500 ms, and Mappings 2 seconds because mapping projection is the sustained
load hotspot. The bounded cache covers Health, Mappings, and complete Snapshot reads.
Mutations, assignment sessions,
configuration, and hardware operations are never cached. Entries expire by
monotonic time and are isolated across sockets and commands.

## Consequences

This coalesces request bursts while preserving the daemon response contract and
limits bounded staleness to the command-specific windows above. A read-flight
mutex prevents simultaneous identical misses from stampeding the daemon. It does not claim hardware readback or
replace generation ordering. A live 100-request-per-endpoint soak after
deployment returned 100/100 successes for all three endpoints; a 1,000-cycle
Health probe returned zero failures. Injected slow-feature browser fixtures and
long-duration mixed-load qualification remain required before the reliability
worklist item is closed.

## Source and evidence

- `apps/mackes-web/src/main.rs` read-cache implementation and regression test.
- `crates/ipc/src/lib.rs` `LocalClient` bounded IPC policy.
- `apps/mackesd/src/main.rs` nonblocking single-threaded control loop.
- `scripts/probe-feature-isolation.sh` and `scripts/probe-health-stability.sh`.
- W160 in `WORKLIST.md` for exact commands, measurements, and remaining gaps.
