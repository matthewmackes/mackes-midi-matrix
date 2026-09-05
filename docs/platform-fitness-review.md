# Platform fitness review — 2026-09-05

## Verdict

Not yet fit for dependable unattended reboot/connect/reconnect operation. The platform has
useful software coverage and working MIDI paths, but current source contains operational gaps
and this session has observed lost bindings and missing late-connected outputs. A running
service or successful host MIDI send is insufficient evidence of device response or recovery.
This is a source and read-only installed-state review, not a reboot or power-cut qualification.

## Findings and assigned work

| Priority | Evidence in current tree | Consequence | Work |
|---|---|---|---|
| Critical | `apps/mackesd/src/lib.rs`, `dispatch_registered`: Factory 1 tuple fallback bypasses source device identity; `main.rs` provisions general outputs and profile bindings at startup; lifecycle polling restores the Novation output specifically. | Wrong-device activation is possible; moved devices and late-connected processors can remain unusable. | W099 |
| High | `scripts/package-test-release.sh` omits the local CLI wrapper, appliance drop-in and console unit consumed by `install-fedora.sh`; `installer-smoke.sh` only runs preflight/invalid arguments. | A passing archive gate does not prove the shipped bundle can install. | W100 |
| High | `packaging/10-appliance.conf` places StartLimit directives in Service; console unit hard-codes `mm` and `/home/mm`; installer enables only daemon, and couples daemon to PiPedal through PartOf. | Boot behavior depends on host-specific setup; rate limiting, console startup and companion-service restart behavior need qualification. | W100 |
| High | `crates/config/src/lib.rs::save` writes a temporary file then renames it without file/directory synchronization; route and backup writers likewise use separate writes/renames. | Atomic rename alone does not establish power-loss durability or consistency across related files. | W101 |
| High | `mapping_runtime::health_after_authorized_command` returns Ready for non-Health commands; subscription reconciliation errors are discarded; Devices and status inventory differed in the incident. | Operator can see misleading readiness and cannot reliably distinguish missing device, transport failure, or unsaved state. | W102 |
| High | `poll_inputs` drains events, then `poll_and_dispatch_inputs` consumes only `take(limit.min(128))`. | Excess drained events can be discarded; losing release events can leave toggles stuck. Needs producer/consumer pressure tests. | W103 |
| High | Prior bypass test exercised velocity-zero Note On releases; real incident showed Note Off. Channel evidence describes zero-based 6 alongside bytes B5 (human channel 6), while mapping originally sent B6. | Current tests and historical claims do not establish repeated real button behavior or pedal channel correctness. | W103 |
| High | `interactive.rs` reads template readiness from optional MACKES_CONFIG at startup; console unit does not set it. | UI readiness can diverge from daemon state. | W102 |

## Installed-state evidence and limits

Read-only `systemctl show` reported daemon active/running with NRestarts=0 and the console
enabled. Configuration directory was `mackes:mackes 0750`, config file
`root:mackes-control 0644`; verify actual service-account save and operator access through
the installed workflow rather than inferring access from file mode alone. Earlier live
observations in this session established stale mapping IDs and an Eventide output absent
from daemon state until restart. No new physical writes, restart, reboot, or power cut were
performed for this review. Hardware receipt, bypass polarity, current receive channel,
power-loss recovery and repeated reconnect behavior remain unproven.

## Fitness acceptance

Luna must close W099–W103, then execute W104 against the exact installed release artifact.
Record cold/warm boots, late/missing devices, USB moves, duplicate devices, failures during
saves, controller pressure and end-to-end pedal/LED observations. Report recovery times,
loss/duplicate counts and unresolved failures. Historical DONE entries do not override the
current regression. Vulnerability scanning is excluded per operator direction.
