#!/usr/bin/env python3
"""Verify the installed GUI exposes guided controls rather than code/protocol editors."""

from __future__ import annotations

import sys

from selenium import webdriver
from selenium.webdriver.chrome.options import Options
from selenium.webdriver.chrome.service import Service
from selenium.webdriver.common.by import By
from selenium.webdriver.support.ui import WebDriverWait


origin = sys.argv[1] if len(sys.argv) > 1 else "http://172.20.222.222:8081"
options = Options()
for argument in ("--headless", "--no-sandbox", "--disable-gpu", "--disable-dev-shm-usage", "--no-proxy-server"):
    options.add_argument(argument)
options.add_argument("--window-size=1440,1200")
options.binary_location = "/usr/bin/chromium-browser"
driver = webdriver.Chrome(service=Service("/usr/bin/chromedriver"), options=options)
try:
    driver.get(f"{origin}/devices#browser_smoke=1")
    wait = WebDriverWait(driver, 45)
    wait.until(lambda browser: browser.find_elements(By.CSS_SELECTOR, "#device-board article"))
    forbidden = driver.find_elements(By.CSS_SELECTOR, "textarea, pre, #configuration-json5-draft, #routes-json, #scene-actions-json, #sysex-bytes")
    if forbidden:
        raise RuntimeError(f"forbidden code/protocol surfaces remain: {len(forbidden)}")
    for element_id in ("device-profile", "device-control-name", "device-channel", "device-destination", "pipedal-repair-plugin", "pipedal-repair-symbol", "pipedal-instance-id"):
        element = driver.find_element(By.ID, element_id)
        if element.tag_name.lower() != "select":
            raise RuntimeError(f"{element_id} is not a guided selector")
    if not driver.find_elements(By.CSS_SELECTOR, "#studio-flow, svg.device-graphic"):
        raise RuntimeError("graphical studio representation is absent")
    print(f"browser-novice-surface: PASS origin={origin}")
finally:
    driver.quit()
