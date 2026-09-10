#!/usr/bin/env bash
set -euo pipefail

origin=${1:-http://172.20.222.222:8081}
cycles=${2:-1000}
host=${MACKES_HEALTH_HOST:-172.20.222.222:8081}
tmp_dir=$(mktemp -d /tmp/mackes-health-probe.XXXXXX)
trap 'rm -rf "$tmp_dir"' EXIT

for ((index = 1; index <= cycles; index += 1)); do
  curl --silent --show-error --fail --max-time 6 -H "Host: $host" \
    -o "$tmp_dir/response" -w '%{http_code} %{time_total}\n' "$origin/api/v1/health" >>"$tmp_dir/results" || echo '000 6.000000' >>"$tmp_dir/results"
done

awk '
  { count += 1; if ($1 == 200) { ok += 1; latency[++n] = $2 } }
  END {
    if (count == 0) exit 1
    asort(latency)
    p95 = latency[int(n * 0.95 + 0.999)]
    if (p95 == "") p95 = 0
    printf "health-probe cycles=%d success=%d failure=%d p95_seconds=%.6f max_seconds=%.6f\n", count, ok, count-ok, p95, latency[n]
    if (ok != count) exit 2
  }
' "$tmp_dir/results"

