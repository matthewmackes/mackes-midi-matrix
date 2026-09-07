#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 3 || "$1" != /* || ! "$2" =~ ^[0-9]+$ || ! "$3" =~ ^[0-9]+$ ]]; then
  printf 'usage: %s /absolute/output-directory duration-seconds interval-seconds\n' "$0" >&2
  exit 64
fi
output_dir="$1"
duration="$2"
interval="$3"
(( interval > 0 && interval <= 300 && duration > 0 && duration <= 28800 )) || {
  printf 'duration must be 1..28800 seconds and interval must be 1..300 seconds\n' >&2
  exit 64
}
install -d -m 0750 "$output_dir"

printf 'captured_at_utc,daemon_active,console_active,daemon_pid,daemon_cpu_percent,daemon_rss_kib,status_ok,received,sent,dropped,nrestarts,daemon_log_lines\n' \
  >"$output_dir/samples.csv"
deadline=$((SECONDS + duration))
while (( SECONDS < deadline )); do
  timestamp="$(date -u '+%Y-%m-%dT%H:%M:%SZ')"
  daemon_active=0
  console_active=0
  systemctl is-active --quiet mackes-midi-matrix.service && daemon_active=1 || true
  systemctl is-active --quiet mackes-midi-matrix-tui.service && console_active=1 || true
  daemon_pid="$(systemctl show -p MainPID --value mackes-midi-matrix.service 2>/dev/null || true)"
  cpu=''; rss=''
  if [[ "$daemon_pid" =~ ^[1-9][0-9]*$ ]]; then
    read -r cpu rss < <(ps -p "$daemon_pid" -o %cpu=,rss= 2>/dev/null || true)
  fi
  status_ok=1
  if ! status_json="$(timeout 10s env -u MACKES_CONFIG -u MACKES_SOCKET \
    /usr/local/bin/mackes-midi-matrix status --json 2>/dev/null)"; then
    status_ok=0
    status_json=''
  fi
  received="$(jq -r '.received // ""' <<<"$status_json" 2>/dev/null || true)"
  sent="$(jq -r '.sent // ""' <<<"$status_json" 2>/dev/null || true)"
  dropped="$(jq -r '.dropped // ""' <<<"$status_json" 2>/dev/null || true)"
  restarts="$(systemctl show -p NRestarts --value mackes-midi-matrix.service 2>/dev/null || true)"
  log_lines="$(timeout 10s journalctl -u mackes-midi-matrix.service --no-pager -n 10000 2>/dev/null | wc -l || true)"
  printf '%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s\n' "$timestamp" "$daemon_active" "$console_active" \
    "$daemon_pid" "$cpu" "$rss" "$status_ok" "$received" "$sent" "$dropped" "$restarts" \
    "$log_lines" >>"$output_dir/samples.csv"
  remaining=$((deadline - SECONDS))
  (( remaining <= 0 )) || sleep "$(( interval < remaining ? interval : remaining ))"
done
awk -F, '
  NR == 1 { next }
  NR == 2 { min_rss=$6; max_rss=$6; min_cpu=$5; max_cpu=$5; first_log=$12; last_log=$12; first_restart=$11; last_restart=$11 }
  NR > 1 {
    samples++
    if ($6 < min_rss) min_rss=$6
    if ($6 > max_rss) max_rss=$6
    if ($5 < min_cpu) min_cpu=$5
    if ($5 > max_cpu) max_cpu=$5
    if ($12 < first_log) first_log=$12
    if ($12 > last_log) last_log=$12
    if ($11 < first_restart) first_restart=$11
    if ($11 > last_restart) last_restart=$11
    if ($10 > max_dropped) max_dropped=$10
    if ($7 != 1) status_failures++
  }
  END {
    printf "samples=%d\nmin_cpu_percent=%s\nmax_cpu_percent=%s\nmin_rss_kib=%s\nmax_rss_kib=%s\nmax_dropped=%s\nstatus_failures=%d\nrestart_count_first=%s\nrestart_count_last=%s\njournal_lines_first=%s\njournal_lines_last=%s\n", samples, min_cpu, max_cpu, min_rss, max_rss, max_dropped + 0, status_failures + 0, first_restart, last_restart, first_log, last_log
  }
' "$output_dir/samples.csv" >"$output_dir/summary.txt"
printf 'qualification soak samples captured in %s\n' "$output_dir"
