#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 || "$1" != /* ]]; then
  printf 'usage: %s /absolute/output-directory\n' "$0" >&2
  exit 64
fi
output_dir="$1"
install -d -m 0750 "$output_dir"

capture() {
  local name="$1"
  shift
  timeout 10s "$@" >"$output_dir/$name" 2>&1 || {
    status=$?
    printf 'command failed or timed out (exit %s): %s\n' "$status" "$*" >>"$output_dir/$name"
  }
}

capture metadata.txt date -u '+captured_at_utc=%Y-%m-%dT%H:%M:%SZ'
printf 'operator=%s\n' "${USER:-unknown}" >>"$output_dir/metadata.txt"
printf 'host=%s\n' "$(hostname 2>/dev/null || printf 'unknown')" >>"$output_dir/metadata.txt"

capture lsusb.txt lsusb
capture amidi.txt amidi -l
capture aconnect.txt aconnect -l
capture service.txt systemctl show mackes-midi-matrix.service \
  -p ActiveState -p SubState -p User -p Group -p Restart -p NRestarts -p ExecStart
capture console-service.txt systemctl show mackes-midi-matrix-tui.service \
  -p ActiveState -p SubState -p User -p Group -p Restart -p NRestarts -p ExecStart
capture status.json /usr/local/bin/mackes-midi-matrix status --json
capture status-no-env.json env -u MACKES_CONFIG -u MACKES_SOCKET \
  /usr/local/bin/mackes-midi-matrix status --json
capture artifact-hashes.txt bash -c '
  for path in /usr/local/libexec/mackes-midi-matrix/mackes-midi-matrixd \
    /usr/local/libexec/mackes-midi-matrix/mackes-midi-matrix-cli \
    /usr/local/bin/mackes-midi-matrix \
    /usr/local/bin/mackes-midi-matrix-local; do
    if [[ -f "$path" ]]; then sha256sum "$path"; else
      printf "missing %s\\n" "$path"
    fi
  done
'
repo_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
capture repository-revision.txt git -C "$repo_dir" rev-parse HEAD
printf 'qualification baseline captured in %s\n' "$output_dir"
