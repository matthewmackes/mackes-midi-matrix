# Studio human qualification checklist

Use this checklist on the installed host at `http://172.20.222.222:8081/` after the automated
release gate passes. Record observations in the table before closing W201/W202.

| Scenario | Expected observation | Result / evidence |
| --- | --- | --- |
| Connect Launch Control XL | Controller appears with 24 knobs, 16 buttons, 8 faders, and 8 utility controls; no invented fader LEDs | |
| Move a physical knob | Matching on-screen control updates within the visible-response budget | |
| Change a supported device value | Studio value and state update without an echo loop | |
| Assign a knob to PiPedal | Named function selection, preview, confirmation, and saved feedback are clear | |
| Assign a fader to Lexicon | Only compatible continuous functions are offered | |
| Assign a button to Eventide | Button-compatible functions are offered with truthful readback wording | |
| Trigger pickup | Control says `Move to pickup` until the physical value crosses the pickup tolerance | |
| Disconnect a device | Affected controls show stale/unavailable state; unrelated assignments remain usable | |
| Reconnect a device | State resynchronizes without replaying uncertain writes | |
| Recall a scene | Confirmation is explicit and partial/unconfirmed device state is visible | |
| Press Panic | Confirmation is required and all mapped output stops | |
| Use keyboard and touch | Every primary action is reachable, focused, and readable without hover | |
| Use light theme and 200% zoom | Labels, status, focus, and controls remain legible with no essential overflow | |

Before sign-off, attach screenshots or a short screen recording for the controller, assignment preview,
device capability cards, scenes, recovery, and panic flows. Record hardware model/serial, firmware,
browser version, viewport, and timestamp. A failed scenario stays open with a disposition; do not mark
the work item complete from automated fixtures alone.

Automated prerequisites:

```text
scripts/release-gate.sh
python3 scripts/browser-root-studio-smoke.py
python3 scripts/browser-studio-console-smoke.py
python3 scripts/browser-studio-accessibility-tree-smoke.py
python3 scripts/browser-studio-latency-smoke.py
```
