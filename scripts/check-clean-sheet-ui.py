#!/usr/bin/env python3
"""Static qualification guard for the clean-sheet Studio shell."""
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
static = ROOT / "apps" / "mackes-web" / "static"
web_main = (ROOT / "apps" / "mackes-web" / "src" / "main.rs").read_text()
handoff = (ROOT / "docs" / "clean-sheet-luna-handoff.md").read_text()
html = (static / "studio.html").read_text()
controller = (static / "studio_controller.js").read_text()
state = (static / "studio_state.js").read_text()
assignment = (static / "studio_assignment.js").read_text()
css = (static / "studio.css").read_text()
assets = [static / name for name in ("studio.html", "studio.css", "studio.js", "studio_state.js", "studio_controller.js", "studio_catalog.js", "studio_assignment.js", "studio_behavior.js", "studio_views.js", "device_renderer.js")]
required = ["PiPedal", "Eventide", "Lexicon", "studio-supporting-view", "studio_behavior.js"]
missing = [marker for marker in required if marker not in html]
if missing:
    raise SystemExit(f"clean-sheet shell missing: {', '.join(missing)}")
root_route = web_main.split('(\"GET\", \"/\")', 1)[1].split('(\"GET\", \"/studio\"', 1)[0] if '(\"GET\", \"/\")' in web_main else ''
if 'include_bytes!(\"../static/studio.html\")' not in root_route:
    raise SystemExit("canonical root route does not serve the clean-sheet Studio shell")
asset_text = "\n".join(asset.read_text().lower() for asset in assets)
if "/assets/app.js" in asset_text or "carbon" in asset_text or "json5" in asset_text:
    raise SystemExit("clean-sheet assets contain a retired dependency or editor")
for marker in ("W180", "W181", "W182", "W183", "W184", "W185", "cutover"):
    if marker not in handoff:
        raise SystemExit(f"Luna handoff missing: {marker}")
if html.count('class="assignment-panel"') != 1 or any(tag in html.lower() for tag in ("<textarea", "<pre", "contenteditable=")):
    raise SystemExit("clean-sheet shell contains duplicate ownership or a raw editor")
script_order = [html.find(f"/assets/{name}") for name in ("studio_state.js", "studio_controller.js", "studio_catalog.js", "studio_assignment.js", "studio.js", "studio_views.js")]
if any(index < 0 for index in script_order) or script_order != sorted(script_order):
    raise SystemExit("clean-sheet scripts are not loaded in dependency order")
if any("http://" in asset.read_text() or "https://" in asset.read_text() for asset in assets):
    raise SystemExit("clean-sheet assets contain an external network dependency")
for marker in ["count: 24", "count: 16", "count: 8", "controlId", "button.type = 'button'", "studio-control-selected", "Device", "Mute", "Solo", "Record", "Right"]:
    if marker not in controller:
        raise SystemExit(f"controller contract missing: {marker}")
for marker in ("LED capable", "no LED"):
    if marker not in controller:
        raise SystemExit(f"LED accessibility contract missing: {marker}")
for marker in ["localStorage.getItem", "localStorage.setItem", "localStorage.removeItem"]:
    if marker not in state:
        raise SystemExit(f"draft persistence contract missing: {marker}")
if "restoredDraft" not in assignment or "state.read().draft" not in assignment:
    raise SystemExit("assignment preview does not restore persisted drafts")
for marker in ["@media (max-width: 640px)", "@media (max-width: 980px)", "prefers-reduced-motion", "min-height: 56px", "min-height: 44px", "focus-visible"]:
    if marker not in css:
        raise SystemExit(f"responsive/accessibility contract missing: {marker}")
for marker in ["aria-live=\"polite\"", "aria-label", "role=\"status\""]:
    if marker not in html:
        raise SystemExit(f"accessible shell marker missing: {marker}")
print("clean-sheet UI qualification guard passed")
