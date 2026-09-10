#!/usr/bin/env python3
"""Verify an assigned Novation control exposes authoritative inspector detail."""

from __future__ import annotations

import sys

from selenium import webdriver
from selenium.webdriver.chrome.options import Options
from selenium.webdriver.chrome.service import Service
from selenium.webdriver.support.ui import WebDriverWait


origin = sys.argv[1] if len(sys.argv) > 1 else "http://172.20.222.222:8081"
options = Options()
for argument in ("--headless", "--no-sandbox", "--disable-gpu", "--disable-dev-shm-usage", "--no-proxy-server"):
    options.add_argument(argument)
options.add_argument("--window-size=1440,1200")
options.binary_location = "/usr/bin/chromium-browser"
driver = webdriver.Chrome(service=Service("/usr/bin/chromedriver"), options=options)
try:
    wait = WebDriverWait(driver, 20)
    driver.get(f"{origin}/devices#browser_smoke=1")
    wait = WebDriverWait(driver, 45)
    wait.until(lambda d: "active of" in d.find_element("id", "inspector-summary").text)
    control = wait.until(lambda d: next(
        (item for item in d.find_elements("css selector", "#faceplate-controls [data-physical-control-id]")
         if item.get_attribute("data-physical-control-id") == "knob-r1-c1"
         and "; assigned;" in (item.get_attribute("aria-label") or "")), None))
    # SVG <g> controls are keyboard/pointer targets but Chromium may reject a
    # synthetic WebDriver click on the group itself; dispatch the same DOM
    # event path used by the browser surface.
    driver.execute_script("arguments[0].dispatchEvent(new MouseEvent('click', {bubbles: true}));", control)
    summary = wait.until(lambda d: d.find_element("id", "inspector-summary").text)
    required = ("Destination:", "Source:", "Behavior:")
    missing = [marker for marker in required if marker not in summary]
    if missing:
        raise RuntimeError(f"assigned control inspector missing {missing}: {summary!r}")
    if "mapping id:" in summary or "runtime" in summary.lower():
        raise RuntimeError(f"inspector exposed an internal identity: {summary!r}")
    print(f"browser-assignment-inspector: PASS origin={origin} summary={summary.splitlines()[0]}")
finally:
    driver.quit()
