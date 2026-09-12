#!/usr/bin/env python3
"""Verify named destination choices are filtered by the selected physical control role."""
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
    wait = WebDriverWait(driver, 30)
    driver.get(f"{origin}/studio")
    controls = wait.until(lambda d: d.find_elements("css selector", "#studio-controller .physical-control"))
    controls[40].click()  # first fader, a continuous control
    driver.find_element("css selector", ".destination-preview button:nth-child(2)").click()
    choices = wait.until(lambda d: d.find_elements("css selector", "#destination-functions button.function-choice"))
    labels = [choice.text for choice in choices]
    if not labels or not any("Expression" in label or "Pitch" in label or "Mix" in label for label in labels):
        raise RuntimeError("continuous Eventide choices were not offered to the fader")
    if any("Tap trigger" in label or "Active / bypass" in label for label in labels):
        raise RuntimeError("button-only Eventide action was offered to the fader")
    if driver.execute_script("return performance.getEntriesByType('resource').filter(e => e.name.includes('/api/v1/assignment')).length"):
        raise RuntimeError("opening the compatibility browser issued an assignment mutation")
    print(f"browser-studio-compatibility: PASS fader_choices={len(labels)}")
finally:
    driver.quit()
