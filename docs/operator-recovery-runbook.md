# Operator recovery runbook

Use the daemon-owned inventory as the source of truth when a USB MIDI device is missing,
reconnected under a new ALSA address, or reported ambiguous.

1. Inspect the current inventory without editing configuration:

   ```text
   mackes devices --json
   mackes endpoints --json
   ```

2. Ask the daemon to perform one bounded native rescan. This is local-only and does not
   restart the service:

   ```text
   mackes rescan --json
   ```

3. Accept a candidate only when the persisted alias identity is proven by serial, or by the
   operator's explicit serial-less binding with matching vendor/product and logical direction.
   Display names, ALSA client numbers, endpoint hashes, and MIDI tuples are not proof.

4. For legacy configuration, preview before applying and retain the generated backup:

   ```text
   mackes migrate /path/to/config.json5 --dry-run
   mackes migrate /path/to/config.json5 --json
   ```

   An ambiguous or unverified reference must remain unresolved and be repaired explicitly.

   For persisted PiPedal destinations, preview the stable identities before any future live
   repair:

   ```text
   mackes pipedal mappings /path/to/config.json5
   mackes pipedal mappings /path/to/config.json5 --json
   ```

   The preview validates physical-control ID, plugin URI, symbol, and optional scope, rejects
   duplicates, and performs no PiPedal or MIDI write. A valid preview does not prove that the
   external plugin or parameter is currently available.

5. Recheck `mackes devices --json` and the daemon snapshot. A host MIDI send counter proves
   delivery to the host only; pedal response and visible LED recovery must be observed separately.

   ```text
   mackes status --json
   ```

   In the JSON response, inspect `led.phase`, `led.target_id`, `led.desired_indices`,
   `led.pending_indices`, `led.sent`, `led.failed`, and `led.last_error`:

   - `absent` means no uniquely selected Launch Control XL MIDI output is available; HUI is not an
     LED target.
   - `initializing` means the daemon is sending the template-scoped reset/template setup.
   - `animating` means the bounded reconnect indication is active; it will restore the desired
     surface when complete.
   - `ready` means the host-side surface is eligible for normal writes. It is not proof that the
     controller visibly accepted or rendered the bytes.
   - A nonzero `pending_indices` or `failed` requires retaining the snapshot and `last_error` for
     diagnosis; do not repeatedly restart the service.

   `mackes rescan --json` is the supported targeted recovery action. It rechecks native endpoint
   identity and subscriptions without restarting the service; it does not claim hardware LED
   acknowledgment. There is no generic force-resync command: a matching output return, template
   change, scene change, or reconnect invalidates the daemon's sent cache and schedules replay.

6. Check the `config_persistence` object in `mackes status --json` (or the dashboard's
   `config=...` line):

   - `ready`: the configured file is writable and validates successfully.
   - `unconfigured` or `missing`: set or restore the intended configuration file.
   - `read_only`: fix ownership/permissions for the daemon service account.
   - `corrupt`: restore a verified backup before attempting further edits.

   Do not treat a successful command response as proof that a persistence failure is resolved;
   recheck the state after the repair.

   Preview and then explicitly apply a verified backup with the expected profile and device
   identity:

   ```text
   mackes backup restore /path/to/backup.bin /path/to/config.json5 reflex serial:A
   mackes backup restore /path/to/backup.bin /path/to/config.json5 reflex serial:A --apply
   ```

   The first command is non-mutating. The second is compatibility-gated and reports an identity
   warning when the backup came from a different device; inspect that warning before proceeding.

If the device remains ambiguous or permission-denied, leave the mapping unchanged, record the
reported reason, and escalate with the identity/subscription snapshot. Do not guess an endpoint,
edit hashes by hand, or restart the daemon as a repair step.
