#!/usr/bin/env python3
"""Verify each workspace exposes only its canonical editor panels."""

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
    expected = {
        "devices": {"device-control": True, "assignment-controls": True, "routing-controls": False, "scene-controls": False, "system-board": False},
        "mappings": {"device-control": False, "assignment-controls": True, "routing-controls": False, "scene-controls": False, "system-board": False},
        "routes": {"device-control": False, "assignment-controls": False, "routing-controls": True, "scene-controls": False, "system-board": False},
        "scenes": {"device-control": False, "assignment-controls": False, "routing-controls": False, "scene-controls": True, "system-board": False},
        "system": {"device-control": False, "assignment-controls": False, "routing-controls": False, "scene-controls": False, "system-board": True},
    }
    wait = WebDriverWait(driver, 15)
    for view, panels in expected.items():
        driver.get(f"{origin}/{view}#browser_smoke=1")
        wait.until(lambda d: d.find_element("id", "view-title").is_displayed())
        for panel, visible in panels.items():
            actual = driver.find_element("id", panel).get_attribute("hidden") is None
            if actual != visible:
                raise RuntimeError(f"{view}: {panel} expected visible={visible}, got {actual}")
    print(f"browser-task-ownership: PASS origin={origin}")
finally:
    driver.quit()
