# Novation software qualification — 2026-09-06

This report records software-only evidence for W125. It does not claim visible LED color,
controller buffer state, or physical reconnect behavior.

## Executed gates

The following commands passed against the working tree:

```text
cargo fmt --all -- --check
cargo test -p mackes-profiles -p mackes-pipedal-adapter -p mackesd
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 scripts/check-architecture.py
python3 scripts/check-worklist.py
scripts/release-gate.sh
git diff --check
```

The release gate included 56 profile tests, 11 PiPedal-adapter tests, 85 daemon tests, the
10,000-message routing benchmark, 14 hermetic integration scenarios (13 passed and one paired
RTP scenario explicitly ignored), installer smoke checks, and release-artifact verification.

## Software scenario evidence

| Scenario | Software evidence | Result |
| --- | --- | --- |
| Complete clear/restore | Template-scoped reset encoder plus complete desired 48-index render tests | PASS |
| Bottom-row starvation | Ordered bounded batch emitter and retry preservation tests | PASS |
| Unchanged idle | Coalescer cache and no-replay regression tests | PASS |
| Independent yellow/amber | Distinct protocol bytes and golden color tests | PASS |
| Failed batch / stale generation | Retry and binding-generation guards | PASS |
| Duplicate physical units / HUI separation | Native supervisor fail-closed identity tests | PASS |
| Active-template and reconnect state | Template reset/selection sequencing and supervisor transition tests | PASS (software) |
| Input during LED refresh | Bounded event and LED scheduling tests | PASS (software) |
| PiPedal absent/restarting | Adapter reconnect and bounded pickup tests | PASS (software) |

The recorded transport bytes are asserted at the profile/daemon boundary; tests do not treat a
successful host write as proof of hardware acknowledgement.

## Remaining qualification

W125 remains open for physical LED appearance, controller-visible buffer/reset behavior, repeated
USB reconnect under live input, and paired external-device observations. Those are carried by
W126 and the hardware qualification matrix and are intentionally not inferred from this report.
