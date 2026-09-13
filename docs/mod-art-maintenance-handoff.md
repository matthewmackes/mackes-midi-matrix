# MOD-inspired art maintenance handoff

Owner: Orion. Scope: W203–W212. The current approved source is `mod-audio/mod-ui` at commit
`c3004836e3466fa3a0d34de3ca855530e75304d1`; the permission basis is
`operator-confirmed-remote-repository-approval-2026-09-13`.

## Adding or replacing art

1. Pin the upstream repository and immutable commit in
   `docs/mod-artwork-source-inventory.md` before copying anything.
2. Record one manifest asset ID, upstream path, upstream hash, final vendored hash, role,
   dimensions, transformation, and fallback in
   `apps/mackes-web/static/vendor/mod-art/manifest.json`.
3. Keep the upstream notice beside the manifest. Do not add logos, product photography, store/social
   art, fonts, screenshots, runtime code, remote URLs, or unqualified third-party branding.
4. Run `scripts/sanitize-mod-art-svg.py` for SVGs. It rejects executable elements and external
   references; update the final hash after transformation.
5. Register the asset in `apps/mackes-web/src/main.rs` with a typed same-origin route and the
   correct MIME type. The server intentionally does not expose a filesystem wildcard.
6. Run the manifest, resilience, web-asset, architecture, worklist, formatting, and diff checks.
7. Run `python3 scripts/check-installed-mod-art-assets.py http://172.20.222.222:8081` only after
   rebuilding and installing the exact release binary with a managed configuration backup.

## Component rules

Art is decorative until a typed capability binds it. Every visual control needs an accessible text
label, explicit state truth, disabled/unavailable behavior, and a generic fallback. PiPedal nodes,
ports, cables, meters, assignments, presets, and snapshots must come from authoritative snapshots;
missing fields remain unavailable. The daemon remains the only mutation and persistence authority.

## Release and rollback

Use `bash scripts/release-gate.sh`, then `cargo build --release --locked -p mackes-web` and
`MACKES_CONFIRM_CONFIG_BACKUP=1 scripts/install-fedora.sh`. Preserve the generated backup and run
`bash scripts/qualify-web-installed.sh http://172.20.222.222:8081`. The current rollback procedure is
in `docs/studio-cutover-rollback.md`; do not delete the managed rollback binary or configuration
backups while testing a new visual release.
