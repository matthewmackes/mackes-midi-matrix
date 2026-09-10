#!/usr/bin/env bash
set -euo pipefail

origin=${1:-http://172.20.222.222:8081}
requests=${2:-10}
host=${MACKES_HEALTH_HOST:-172.20.222.222:8081}
probe_dir=$(mktemp -d /tmp/mackes-feature-isolation.XXXXXX)
trap 'rm -rf "$probe_dir"' EXIT

for endpoint in health novation mappings; do
  seq 1 "$requests" | xargs -P10 -I{} sh -c \
    'curl --silent --show-error --fail --max-time 6 -o /dev/null -w "%{http_code} %{time_total}\n" -H "Host: '"$host"'" "'"$origin"'/api/v1/'"$endpoint"'"' \
    >"$probe_dir/$endpoint"
  awk -v endpoint="$endpoint" '{if ($1 == 200) ok++; else fail++; if ($2 > max) max = $2} END {printf "%s requests=%d success=%d failure=%d max_seconds=%.6f\n", endpoint, ok+fail, ok, fail, max; if (fail) exit 2}' "$probe_dir/$endpoint"
done

