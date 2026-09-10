#!/usr/bin/env python3
"""Verify workspace refresh restores the focused control."""

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
    driver.get(f"{origin}/routes")
    WebDriverWait(driver, 15).until(lambda d: d.find_element("id", "routing-refresh").is_displayed())
    driver.execute_script("document.getElementById('routing-refresh').focus(); refreshActiveView();")
    WebDriverWait(driver, 15).until(
        lambda d: d.execute_script("return document.activeElement && document.activeElement.id") == "routing-refresh"
    )
    print(f"browser-focus-preservation-smoke: PASS origin={origin}")
finally:
    driver.quit()
