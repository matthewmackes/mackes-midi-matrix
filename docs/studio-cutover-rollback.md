# Studio cutover rollback

This runbook returns the web adapter to the last known-good installed binary without touching the
daemon configuration or MIDI mappings. Use it when the installed Studio route fails its release or
operator qualification checks.

1. Stop the web adapter and preserve the failing binary for inspection:

   ```sh
   sudo systemctl stop mackes-web.service
   sudo install -m 0755 /usr/local/libexec/mackes-midi-matrix/mackes-web \
     /var/lib/mackes-midi-matrix/mackes-web.failed
   ```

2. Restore the release artifact that was recorded before cutover:

   ```sh
   sudo install -m 0755 /var/lib/mackes-midi-matrix/mackes-web.rollback \
     /usr/local/libexec/mackes-midi-matrix/mackes-web
   sudo systemctl start mackes-web.service
   ```

3. Verify service health and the root response before allowing browser traffic:

   ```sh
   systemctl is-active mackes-web.service
   curl --fail --max-time 5 http://127.0.0.1:8081/api/v1/health
   curl --fail --max-time 5 http://127.0.0.1:8081/
   ```

The rollback artifact is a web binary only. It does not restore or rewrite `/etc/mackes-midi-matrix`,
the control socket, persisted mappings, scenes, or device state. After recovery, rerun
`scripts/release-gate.sh`, the installed browser qualification fixtures, and the W202 sign-off
checklist before attempting another cutover.

The rollback artifact currently installed for this cutover is
`/var/lib/mackes-midi-matrix/mackes-web.rollback` with SHA-256
`14074731612a191606e0bc220dc8a45ae1b6dde244da261632d63905fc942908`.
