#!/usr/bin/env bash
set -euo pipefail

check_only=false
if [[ "$#" -gt 1 || ("$#" -eq 1 && "$1" != "--check") ]]; then
  echo "usage: $0 [--check]" >&2
  exit 64
fi
[[ "$#" -eq 1 ]] && check_only=true

if [[ "$(id -u)" -ne 0 ]]; then
  echo "run as root" >&2
  exit 77
fi
if [[ "$(uname -m)" != "x86_64" ]]; then
  echo "mackes-midi-matrix requires x86_64" >&2
  exit 78
fi
root_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
for required in "$root_dir/target/release/mackes-midi-matrix" "$root_dir/target/release/mackes-midi-matrixd"; do
  if [[ ! -x "$required" ]]; then
    echo "missing executable: $required (run cargo build --release first)" >&2
    exit 79
  fi
done
for tool in install getent groupadd useradd usermod systemctl ldconfig; do
  command -v "$tool" >/dev/null || { echo "missing required tool: $tool" >&2; exit 80; }
done
alsa_libraries="$(ldconfig -p 2>/dev/null || true)"
if [[ "$alsa_libraries" != *"libasound.so.2"* ]]; then
  echo "missing ALSA runtime: install alsa-lib" >&2
  exit 81
fi
if "$check_only"; then
  echo "preflight checks passed"
  exit 0
fi
bin_dir=/usr/local/bin
libexec_dir=/usr/local/libexec/mackes-midi-matrix
config_dir=/etc/mackes-midi-matrix
state_dir=/var/lib/mackes-midi-matrix
run_dir=/run/mackes-midi-matrix
install -d -m 0755 "$bin_dir" "$libexec_dir" "$config_dir" "$state_dir" "$run_dir"
config_entries=()
while IFS= read -r -d '' entry; do config_entries+=("$entry"); done < <(find "$config_dir" -mindepth 1 -maxdepth 1 -print0)
if [[ "${#config_entries[@]}" -gt 0 ]]; then
  if [[ "${MACKES_CONFIRM_CONFIG_BACKUP:-}" != "1" ]]; then
    echo "existing configuration found; set MACKES_CONFIRM_CONFIG_BACKUP=1 to back it up before upgrade" >&2
    exit 82
  fi
  backup_dir="$state_dir/config-backups/$(date -u +%Y%m%dT%H%M%SZ)"
  install -d -m 0750 "$backup_dir"
  cp -a "${config_entries[@]}" "$backup_dir/"
  echo "configuration backed up to $backup_dir"
fi
getent group mackes-control >/dev/null || groupadd --system mackes-control
getent passwd mackes >/dev/null || useradd --system --home-dir "$state_dir" --shell /sbin/nologin mackes
if getent group audio >/dev/null; then
  usermod --append --groups audio mackes
fi
install -m 0755 "$root_dir/target/release/mackes-midi-matrix" "$bin_dir/mackes-midi-matrix"
install -m 0755 "$root_dir/scripts/mackes-midi-matrix-local" "$bin_dir/mackes-midi-matrix-local"
install -m 0755 "$root_dir/target/release/mackes-midi-matrixd" "$libexec_dir/mackes-midi-matrixd"
install -m 0644 "$root_dir/packaging/mackes.service" /etc/systemd/system/mackes-midi-matrix.service
chown -R mackes:mackes "$state_dir" "$run_dir"
chmod 0750 "$config_dir" "$state_dir"
systemctl daemon-reload
systemctl enable --now mackes-midi-matrix.service
systemctl restart mackes-midi-matrix.service
echo "installed and started; service is enabled for boot: mackes-midi-matrix.service"
