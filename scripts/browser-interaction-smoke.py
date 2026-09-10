#!/usr/bin/env python3
"""Offline Selenium smoke for pointer and keyboard Novation control selection."""

from __future__ import annotations

import os
import json
import sys
import time

from selenium import webdriver
from selenium.webdriver.chrome.options import Options
from selenium.webdriver.chrome.service import Service
from selenium.webdriver.common.by import By
from selenium.webdriver.common.keys import Keys
from selenium.webdriver.support.ui import WebDriverWait


origin = sys.argv[1] if len(sys.argv) > 1 else "http://172.20.222.222:8081"
artifact = sys.argv[2] if len(sys.argv) > 2 else "/tmp/mackes-accessibility-tree.json"
logs_artifact = sys.argv[3] if len(sys.argv) > 3 else "/tmp/mackes-browser-logs.json"
options = Options()
options.add_argument("--headless")
options.add_argument("--no-sandbox")
options.add_argument("--disable-gpu")
options.add_argument("--disable-dev-shm-usage")
options.add_argument("--no-proxy-server")
options.add_argument("--window-size=1440,1200")
options.set_capability("goog:loggingPrefs", {"browser": "ALL", "performance": "ALL"})
options.binary_location = os.environ.get("MACKES_CHROME_BINARY", "/usr/bin/chromium-browser")
driver = webdriver.Chrome(service=Service("/usr/bin/chromedriver"), options=options)
try:
    driver.get(f"{origin}/devices/novation#browser_smoke=1")
    wait = WebDriverWait(driver, 15)
    controls = wait.until(lambda d: d.find_elements(By.CSS_SELECTOR, "[role='button'][aria-label^='Select']"))
    assert len(controls) >= 56, f"expected 56 controls, found {len(controls)}"
    first = controls[0]
    first.click()
    wait.until(lambda d: "selected" in d.find_element(By.ID, "workspace-inspector").text.lower())
    first.send_keys(Keys.ENTER)
    wait.until(lambda d: "selected" in d.find_element(By.ID, "workspace-inspector").text.lower())
    tree = driver.execute_cdp_cmd("Accessibility.getFullAXTree", {})
    nodes = tree.get("nodes", [])
    named_buttons = [node for node in nodes if node.get("role", {}).get("value") == "button" and node.get("name", {}).get("value")]
    assert len(named_buttons) >= 56, f"expected 56 named accessibility buttons, found {len(named_buttons)}"
    with open(artifact, "w", encoding="utf-8") as handle:
        json.dump(tree, handle, indent=2)
    with open(logs_artifact, "w", encoding="utf-8") as handle:
        json.dump({"browser": driver.get_log("browser"), "performance": driver.get_log("performance")}, handle, indent=2)
    print(f"browser-interaction-smoke: PASS controls={len(controls)} accessibility_buttons={len(named_buttons)} artifact={artifact} logs={logs_artifact} origin={origin}")
finally:
    driver.quit()
