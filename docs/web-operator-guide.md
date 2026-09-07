# MACKES web operator guide

The bundled web service listens on port `8081` and exposes the same-origin browser shell without
login or authorization. The daemon remains the authority for MIDI, configuration, generations,
and hardware outcomes.

## Workspace ownership

| Workspace | Canonical responsibility |
| --- | --- |
| Live | Health, activity, panic, rescan, and emergency visibility |
| Map Controls | Physical-control assignment, capture, commit, behavior editing, replacement, enable/disable, delete, and mapping undo |
| Routing | Generation-checked route drafts, hop limits, apply, and undo |
| Scenes & Setlists | Scene selection and next/previous recall requests |
| Devices | Device controls, Novation status, and PiPedal catalog projection |
| System | Diagnostics, configuration export, validation, and backup inventory |
| Monitor | Sequenced SSE event view with bounded-poll fallback, pause/clear, and capture download |

Advanced JSON route editing is the same generation-checked route draft submitted by the Routing
workspace; it is not a second configuration store.

The Monitor workspace opens the same-origin `/api/v1/events/stream` feed with its last sequence
ID. The server emits heartbeats for a bounded five-minute session; the browser reconnects with the
last ID and falls back to bounded polling when EventSource is unavailable. A resnapshot signal
clears stale presentation events and reloads authoritative state before monitoring resumes.

## Interaction

Every action has a button equivalent. Forms accept keyboard focus and Enter submission; browser
native touch controls provide the same actions on narrow screens. Scene selection, route apply,
assignment commit, mapping behavior/replacement/lifecycle actions, device control, panic, and other disruptive operations display their result in
the bounded status line and refresh authoritative state.

## Service and network operation

```text
systemctl status mackes-web.service
systemctl enable --now mackes-web.service
curl http://127.0.0.1:8081/api/v1/health
```

The packaged unit binds `0.0.0.0:8081`, runs as `mackes:mackes-control`, uses the daemon control
socket, and restarts on failure. LAN exposure is intentional and unauthenticated; restrict the
host network when that is not acceptable. A bind conflict is reported by systemd rather than
silently moving to another port.

For LAN browser access, set the origin to the host’s reachable address and open
`http://<host-ip>:8081`; the browser `Host` header must match the configured origin. Persist it with
a systemd drop-in, for example `MACKES_WEB_ORIGIN=http://192.168.1.50:8081`, then run
`systemctl daemon-reload` and `systemctl restart mackes-web.service`.

## Recovery and truth boundaries

Refresh a workspace after a daemon restart or stale-generation response. HTTP `409` means the
authoritative generation changed; reload the view before retrying. Hardware sends report acceptance
or failure, not visible LED or processor confirmation. The backup inventory, configuration import/
restore, full scene CRUD, broad PiPedal operations, physical Novation checks, and long-lived event
stream remain explicitly incomplete where the capability ledger says so; the browser does not
pretend those controls are implemented.
