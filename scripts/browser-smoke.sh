#!/usr/bin/env bash
set -euo pipefail

origin=${1:-http://172.20.222.222:8081}
out_dir=${2:-$(mktemp -d /tmp/mackes-browser-smoke.XXXXXX)}
mkdir -p "$out_dir"
browser=$(command -v chromium || command -v chromium-browser)
window_size=${MACKES_BROWSER_WINDOW_SIZE:-1440,1200}
base_args=(--headless --no-sandbox --disable-gpu --disable-dev-shm-usage --no-proxy-server --disable-background-networking --disable-component-update --hide-scrollbars --window-size="$window_size")
for view in devices mappings routes scenes recovery system monitor; do
  timeout --signal=TERM 25s "$browser" "${base_args[@]}" --virtual-time-budget=10000 --screenshot="$out_dir/$view.png" \
    --dump-dom "$origin/$view#browser_smoke=1" >"$out_dir/$view.html" || true
done
if [[ ! -s "$out_dir/devices.html" ]]; then
  timeout --signal=TERM 60s "$browser" "${base_args[@]}" --virtual-time-budget=15000 --screenshot="$out_dir/devices-retry.png" \
    --dump-dom "$origin/devices#browser_smoke=1" >"$out_dir/devices.html" || true
fi
if [[ ! -s "$out_dir/mappings.html" ]]; then
  timeout --signal=TERM 60s "$browser" "${base_args[@]}" --virtual-time-budget=15000 --screenshot="$out_dir/mappings-retry.png" \
    --dump-dom "$origin/mappings#browser_smoke=1" >"$out_dir/mappings.html" || true
fi
rg -q 'Novation control grid|faceplate-controls|workspace-sidebar' "$out_dir/devices.html"
rg -q 'Current assignments \([0-9]+\)' "$out_dir/devices.html"
rg -q 'value unavailable|no authoritative readback' "$out_dir/devices.html"
rg -q 'CHANNEL BUTTONS|UTILITY|Device|Mute|Solo|Record|Up|Down|Left|Right' "$out_dir/devices.html"
rg -q 'role="button"|aria-label="Select' "$out_dir/devices.html"
python3 - "$out_dir/devices.html" "$out_dir/system.html" <<'PY'
import re
import sys

devices, system = (open(path, encoding="utf-8").read() for path in sys.argv[1:])
hardware = r'<details[^>]*id="hardware-actions"[^>]*>'
device_match = re.search(hardware, devices)
system_match = re.search(hardware, system)
if not device_match or 'hidden' not in device_match.group(0):
    raise SystemExit('browser-smoke: Hardware must be hidden on Devices')
if not system_match or 'hidden' in system_match.group(0):
    raise SystemExit('browser-smoke: Hardware must be visible on System')
PY
rg -q 'workspace-inspector|Backend|Daemon' "$out_dir"/*.html
rg -q 'data-view="recovery"|Recovery' "$out_dir/recovery.html"
deep_link_html="$out_dir/novation-deep-link.html"
timeout --signal=TERM 25s "$browser" "${base_args[@]}" --virtual-time-budget=10000 --dump-dom \
  "$origin/devices/novation#browser_smoke=1" >"$deep_link_html" || true
test -s "$deep_link_html"
rg -q 'Novation control grid|Current assignments' "$deep_link_html"
for responsive_size in 320,900 768,1024; do
  safe_size=${responsive_size//,/x}
  responsive_html="$out_dir/novation-${safe_size}.html"
  timeout --signal=TERM 25s "$browser" "${base_args[@]}" --window-size="$responsive_size" --virtual-time-budget=10000 --dump-dom \
    "$origin/devices/novation#browser_smoke=1" >"$responsive_html" || true
  test -s "$responsive_html"
  rg -q 'Novation control grid|Current assignments' "$responsive_html"
done
light_html="$out_dir/novation-light.html"
timeout --signal=TERM 25s "$browser" "${base_args[@]}" --dump-dom \
  "$origin/devices/novation#theme=light&browser_smoke=1" >"$light_html" || true
test -s "$light_html"
rg -q 'class="light"' "$light_html"
rg -q 'Novation control grid|Current assignments' "$light_html"
for asset in navigation.js health.js feature_catalog.js feature_renderer.js state_store.js app.js app.css; do
  local_hash=$(sha256sum "apps/mackes-web/static/$asset" | awk '{print $1}')
  live_hash=$(curl --silent --show-error --fail --max-time 10 -H "Host: ${origin#http://}" "$origin/assets/$asset" | sha256sum | awk '{print $1}')
  [[ "$local_hash" == "$live_hash" ]] || {
    echo "browser-smoke: installed asset hash mismatch for $asset" >&2
    exit 1
  }
  printf 'asset-hash %s %s\n' "$asset" "$live_hash" >"$out_dir/$asset.sha256"
done
printf 'browser-smoke: PASS origin=%s artifacts=%s\n' "$origin" "$out_dir"
