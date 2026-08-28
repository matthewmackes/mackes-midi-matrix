# ADR: RTP-MIDI/AppleMIDI transport boundary

*Status: proposed for W015 implementation*  
*Date: 2026-08-26*

## Decision

MACKES will expose configured RTP-MIDI/AppleMIDI sessions as MIDI-only endpoints.
The transport will implement the interoperable session-control and RTP-MIDI behavior
described by RFC 6295 and the AppleMIDI session protocol, using a reviewed Rust
implementation rather than copying vendor source. Discovery is disabled by default;
configured peers and explicit bind addresses are the v1 path.

## Boundary and security

- UDP session traffic carries MIDI data only. It cannot carry MACKES IPC envelopes,
  configuration, backups, scene activation, unsafe-mode arming, or health commands.
- Network input is not authentication. Peer allowlists are explicit configuration.
- SysEx and application actions are denied by the default inbound route policy; enabling
  them requires local configuration and still passes the local unsafe-action policy.
- Session sockets bind only to configured addresses and use bounded ingress, reorder, and
  SysEx reassembly buffers. Malformed datagrams are discarded with health counters.
- Configured MACKES-to-MACKES administrative links, when needed, remain a separate
  authenticated TLS/PSK channel and are never multiplexed with RTP-MIDI.

## Session behavior

Implement invitation, acceptance/rejection, synchronization, receiver feedback,
end-session, token/SSRC tracking, names, timeout, reconnect, sequence/timestamp rollover,
loss/reorder handling, running status, multiple commands per packet, and bounded SysEx
fragment reassembly. The default jitter buffer is 3 ms. Incomplete SysEx expires silently
as MIDI output but increments a visible counter.

## Verification targets

Tests must include golden datagrams, malformed/truncated packets, invitation collisions,
token mismatch, rollover, loss/reorder/duplicate injection, running status, multi-command
packets, SysEx expiry, bounded overflow, reconnect, and proof that network input cannot
invoke IPC. Hardware/network evidence must include two independent peers, one non-MACKES,
bidirectional MIDI cases, reconnect, and loss/reorder injection.

## Alternatives rejected

MACKES IPC will not be tunneled over RTP-MIDI. Automatic network discovery, unauthenticated
peer acceptance, and unbounded packet/reassembly buffers are excluded from v1.
