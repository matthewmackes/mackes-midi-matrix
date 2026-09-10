#!/usr/bin/env python3
"""Verify Routing refuses mutation when endpoint IDs exceed JS-safe precision."""

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
options.set_capability("pageLoadStrategy", "eager")
driver = webdriver.Chrome(service=Service("/usr/bin/chromedriver"), options=options)
try:
    driver.get(f"{origin}/routes#browser_smoke=1")
    def lossless_summary(browser):
        text = browser.find_element("id", "inspector-summary").text
        return text if "view-only" in text else False

    summary = WebDriverWait(driver, 45).until(lossless_summary)
    if "view-only" not in summary or "too large" not in summary:
        raise RuntimeError(f"lossless route guard not visible: {summary!r}")
    apply_button = driver.find_element("id", "routing-apply")
    driver.execute_script("arguments[0].click();", apply_button)
    operation = driver.find_element("id", "operation").text
    if "lossless numeric contract" not in operation:
        raise RuntimeError(f"route apply was not blocked: {operation!r}")
    print(f"browser-route-lossless-guard: PASS origin={origin}")
finally:
    driver.quit()
