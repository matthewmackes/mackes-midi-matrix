#!/usr/bin/env bash
set -euo pipefail

# Deterministic Novation emulator qualification. The testkit virtual MIDI pair
# is the software emulator used for controller input, LED output, reconnect,
# assignment, and bounded-traffic checks; no physical USB write is performed.
echo "novation-emulator: virtual Launch Control XL Mk2"
cargo test -p mackes-testkit virtual_launch_control --all-features
cargo test -p mackes-profiles factory1_led_feedback_golden_bytes --all-features
cargo test -p mackesd led_surface::tests:: --all-features
for cycle in $(seq 1 10); do
  cargo test -p mackesd reconnect_preserves_assignment_and_output_requests_led_replay --all-features
done
echo "novation-emulator: PASS"
