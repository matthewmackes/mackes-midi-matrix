#!/usr/bin/env bash
set -euo pipefail

root_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT
mkdir -p "$tmpdir/usr/local/libexec/mackes-midi-matrix"
install -m 0755 /bin/true "$tmpdir/usr/local/libexec/mackes-midi-matrix/mackes-web"
mkdir -p "$tmpdir/mackes-midi-matrix.service.d"
cp "$root_dir/packaging/mackes.service" "$tmpdir/mackes-midi-matrix.service"
cp "$root_dir/packaging/10-appliance.conf" \
  "$tmpdir/mackes-midi-matrix.service.d/10-appliance.conf"
cp "$root_dir/packaging/mackes-midi-matrix-tui.service" \
  "$tmpdir/mackes-midi-matrix-tui.service"
cp "$root_dir/packaging/mackes-web.service" "$tmpdir/mackes-web.service"
sed -i "s|/usr/local/libexec/mackes-midi-matrix/mackes-web|$tmpdir/usr/local/libexec/mackes-midi-matrix/mackes-web|" "$tmpdir/mackes-web.service"
grep -q '^User=mackes$' "$tmpdir/mackes-midi-matrix.service"
grep -q '^Group=mackes-control$' "$tmpdir/mackes-midi-matrix.service"
grep -q '^WantedBy=multi-user.target$' "$tmpdir/mackes-midi-matrix.service"
grep -q '^Restart=always$' "$tmpdir/mackes-midi-matrix.service.d/10-appliance.conf"
grep -q '^RestartSec=3s$' "$tmpdir/mackes-midi-matrix.service.d/10-appliance.conf"
grep -q '^Wants=pipedald.service$' "$tmpdir/mackes-midi-matrix.service.d/10-appliance.conf"
grep -q '^PartOf=pipedald.service$' "$tmpdir/mackes-midi-matrix.service.d/10-appliance.conf"
grep -q '^After=pipedald.service alsa-restore.service$' "$tmpdir/mackes-midi-matrix.service.d/10-appliance.conf"
grep -q '^Requires=mackes-midi-matrix.service$' "$tmpdir/mackes-midi-matrix-tui.service"
grep -q '^WantedBy=multi-user.target$' "$tmpdir/mackes-midi-matrix-tui.service"
grep -q '^Environment=MACKES_CONFIG=' "$tmpdir/mackes-midi-matrix-tui.service"
grep -q '^User=mackes$' "$tmpdir/mackes-web.service"
grep -q '^Group=mackes-control$' "$tmpdir/mackes-web.service"
grep -q '^WantedBy=multi-user.target$' "$tmpdir/mackes-web.service"
grep -q '^Restart=always$' "$tmpdir/mackes-web.service"
grep -q '^Environment=MACKES_WEB_BIND=' "$tmpdir/mackes-web.service"
grep -q '^Environment=MACKES_WEB_ORIGIN=' "$tmpdir/mackes-web.service"
systemd-analyze verify \
  "$tmpdir/mackes-midi-matrix.service" \
  "$tmpdir/mackes-midi-matrix-tui.service" \
  "$tmpdir/mackes-web.service"
echo "systemd-unit-verification: PASS"
