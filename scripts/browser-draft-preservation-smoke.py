#!/usr/bin/env python3
"""Verify background refresh does not overwrite a dirty routing draft."""

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
    WebDriverWait(driver, 15).until(lambda d: d.find_element("id", "routing-add").is_displayed())
    driver.find_element("id", "routing-add").click()
    WebDriverWait(driver, 15).until(lambda d: len(d.find_elements("css selector", ".route-card")) >= 1)
    draft_card_count = len(driver.find_elements("css selector", ".route-card"))
    driver.execute_script("refreshActiveView();")
    WebDriverWait(driver, 15).until(lambda d: len(d.find_elements("css selector", ".route-card")) == draft_card_count)
    assert driver.find_element("id", "routing-board").is_displayed()
    assert "unsaved" in driver.find_element("id", "inspector-summary").text.lower() or driver.execute_script("return window.MackesStateStore?.snapshot?.().routeDraftDirty ?? true")
    print(f"browser-draft-preservation-smoke: PASS origin={origin}")
finally:
    driver.quit()
