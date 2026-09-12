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
rg -q 'studio-controller|MACKES Studio' "$out_dir/devices.html"
rg -q 'studio-assignment-preview|Choose a function' "$out_dir/devices.html"
rg -q 'Value unknown|Sent — device does not confirm|Live sync' "$out_dir/devices.html"
rg -q 'CHANNEL BUTTONS|UTILITY|Device|Mute|Solo|Record|Up|Down|Left|Right' "$out_dir/devices.html"
rg -q 'role="listbox"|aria-label="Novation Launch Control XL control surface"' "$out_dir/devices.html"
rg -q 'studio-supporting-view|Connected devices' "$out_dir/devices.html"
rg -q 'MACKES Studio|Daemon|Connected' "$out_dir"/*.html
rg -q 'studio-controller|MACKES Studio' "$out_dir/recovery.html"
rg -q 'studio-controller|MACKES Studio' "$out_dir/devices.html"
for asset in navigation.js health.js feature_catalog.js feature_renderer.js device_renderer.js state_store.js app.js app.css; do
  local_hash=$(sha256sum "apps/mackes-web/static/$asset" | awk '{print $1}')
  live_hash=$(curl --silent --show-error --fail --max-time 10 -H "Host: ${origin#http://}" "$origin/assets/$asset" | sha256sum | awk '{print $1}')
  [[ "$local_hash" == "$live_hash" ]] || {
    echo "browser-smoke: installed asset hash mismatch for $asset" >&2
    exit 1
  }
  printf 'asset-hash %s %s\n' "$asset" "$live_hash" >"$out_dir/$asset.sha256"
done
printf 'browser-smoke: PASS origin=%s artifacts=%s\n' "$origin" "$out_dir"
