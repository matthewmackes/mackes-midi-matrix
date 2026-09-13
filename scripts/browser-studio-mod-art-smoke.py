#!/usr/bin/env python3
"""Verify the installed MOD-inspired Devices surface renders its governed visual layers."""
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
    driver.get(f"{origin}/studio/devices")
    wait = WebDriverWait(driver, 30)
    wait.until(lambda d: d.find_elements("css selector", ".pipedal-canvas-card"))
    if not driver.find_elements("css selector", ".pipedal-node"):
        raise RuntimeError("no authoritative PiPedal target nodes rendered")
    if not driver.find_elements("css selector", ".pipedal-library-card"):
        raise RuntimeError("PiPedal visual library missing")
    if not driver.find_elements("css selector", ".pipedal-topology-list"):
        raise RuntimeError("accessible topology list missing")
    assets = driver.execute_script("return performance.getEntriesByType('resource').map(e => e.name).filter(name => name.includes('/assets/vendor/mod-art/'))")
    if not assets:
        raise RuntimeError("no governed MOD artwork loaded")
    severe = [entry for entry in driver.get_log("browser") if entry.get("level") == "SEVERE"]
    if severe:
        raise RuntimeError(f"Devices visual console errors: {severe!r}")
    print(f"browser-studio-mod-art: PASS nodes={len(driver.find_elements('css selector', '.pipedal-node'))} art_requests={len(assets)}")
finally:
    driver.quit()
