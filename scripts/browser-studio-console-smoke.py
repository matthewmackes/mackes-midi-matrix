#!/usr/bin/env python3
"""Verify the clean-sheet Studio route loads without browser console errors."""
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
options.add_argument("--log-level=3")
options.set_capability("goog:loggingPrefs", {"browser": "ALL"})
options.binary_location = "/usr/bin/chromium-browser"
driver = webdriver.Chrome(service=Service("/usr/bin/chromedriver"), options=options)
try:
    for path in ("/", "/studio"):
        driver.get(f"{origin}{path}")
        WebDriverWait(driver, 30).until(lambda d: len(d.find_elements("css selector", "#studio-controller .physical-control")) == 56)
        severe = [entry for entry in driver.get_log("browser") if entry.get("level") == "SEVERE"]
        if severe:
            raise RuntimeError(f"Studio console errors at {path}: {severe!r}")
    print(f"browser-studio-console: PASS root_and_studio origin={origin}")
finally:
    driver.quit()
