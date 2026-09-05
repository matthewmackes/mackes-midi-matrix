# Fedora installation

Build the x86_64 release bundle from the repository root:

```text
cargo build --release
sudo scripts/install-fedora.sh --check
sudo scripts/install-fedora.sh
```

When upgrading an installation that already contains `/etc/mackes-midi-matrix` entries, provide
an explicit backup acknowledgement:

```text
sudo MACKES_CONFIRM_CONFIG_BACKUP=1 scripts/install-fedora.sh
```

For an existing `/etc/mackes-midi-matrix`, the installer requires an explicit backup acknowledgement:

```text
sudo MACKES_CONFIRM_CONFIG_BACKUP=1 scripts/install-fedora.sh
```

The installer places `mackes-midi-matrix` in `/usr/local/bin`, `mackes-midi-matrixd` in
`/usr/local/libexec/mackes-midi-matrix`, configuration in `/etc/mackes-midi-matrix`, state in
`/var/lib/mackes-midi-matrix`, and the runtime socket directory in `/run/mackes-midi-matrix`. It creates the
system account `mackes` and group `mackes-control`. When Fedora's `audio` group is
available, the installer adds the daemon account to it so ALSA MIDI devices can be opened.
It never adds an operator to `mackes-control` implicitly.

The installer enables and starts the service automatically:

```text
systemctl status mackes-midi-matrix.service
```

For local testing from a shell that has not yet refreshed its group membership, launch the
TUI through the installed session wrapper:

```text
mackes-midi-matrix-local
```

The service validates `/etc/mackes-midi-matrix/config.json5` before accepting IPC. For a development
instance, use an explicit configuration path: `mackesd --config PATH`.

The CLI queries daemon health through `/run/mackes-midi-matrix/control.sock` by default. For a
temporary or development daemon, override the path with `MACKES_SOCKET`:

```text
MACKES_SOCKET=/tmp/mackes/control.sock mackes-midi-matrix status --json
```

The local control socket is restricted to its owner and `mackes-control` group. Add a
trusted operator only after reviewing local access:

```text
sudo usermod --append --groups mackes-control "$USER"
```

Log out and back in after group enrollment. Network MIDI peers are not granted local
IPC authority; configure and review RTP-MIDI trust separately. Do not put preshared keys
or device serials in shared fixtures or logs. Use `mackes doctor` before connecting
hardware, and retain configuration backups before upgrades.

Qualification commands are available from the repository root:

```text
scripts/qualify-hardware.sh
scripts/installer-smoke.sh
scripts/benchmark-routing.sh
scripts/soak-routing.sh 3600
scripts/release-gate.sh
scripts/integration-suite.sh
```

The hardware report is observation-only and never sends MIDI. Physical vendor-map
validation and long-duration soak evidence must be recorded separately before release.

Qualification commands are observation/test procedures:

```text
scripts/qualify-hardware.sh
scripts/installer-smoke.sh
scripts/benchmark-routing.sh
scripts/soak-routing.sh 3600
scripts/release-gate.sh
scripts/physical-write-guard.sh path/to/verified-map.record
```

The hardware report never sends MIDI. Physical vendor-map validation and long-duration
soak evidence must be recorded separately before production release.
The current per-device qualification matrix is maintained in
`docs/hardware-qualification.md`.

For MIDISPORT 4x4 qualification on Fedora, install the firmware loader and
ALSA diagnostic tools before connecting or re-triggering the device:

```text
sudo dnf install fxload midisport-firmware alsa-utils
```

The production daemon only requires the ALSA runtime library; these packages
are qualification prerequisites and are not silently installed by MACKES.
Physical writes additionally require a map record containing `status=verified` and
`physical_test=pass`, plus `MACKES_CONFIRM_PHYSICAL_WRITE=1`.

## Native ALSA recovery

The production service owns one native ALSA Sequencer client. Verify ownership and the
configured input registrations with:

```text
systemctl status mackes-midi-matrix.service --no-pager
/usr/local/bin/mackes-midi-matrix status --json
aconnect -l
journalctl -u mackes-midi-matrix.service -b --no-pager
```

The expected service identity is `User=mackes`, `Group=mackes-control`, with `audio`
supplementary access and `/dev/snd/seq` read/write access. Do not run the daemon as root or
change the control socket to world-writable. If a controller is reconnected, restart the
service only after confirming the device has returned in `aconnect -l`; mappings are retained
and native subscriptions are reconciled by the daemon.

For a failed native migration, restore the previously installed daemon binary from the retained
release artifact, restart the service, and confirm `health=ready` plus the expected input count.
Snapshots report `native_backend` as `alsa-seq` for the production Linux path or
`midir-rollback` only when the daemon is built without `alsa-seq-backend`; those backends are
mutually exclusive for hardware input. The rollback is software-only and does not reset the
controller or overwrite processor presets.
