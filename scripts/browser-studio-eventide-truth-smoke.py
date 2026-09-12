#!/usr/bin/env python3
"""Verify Eventide choices expose send-only feedback truth on the installed Studio surface."""
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
    controls[0].click()
    driver.find_element("css selector", ".destination-preview button:nth-child(2)").click()
    choices = wait.until(lambda d: d.find_elements("css selector", "#destination-functions button.function-choice"))
    labels = [choice.text for choice in choices]
    if len(labels) != 14 or not all("Sent — device does not confirm" in label for label in labels):
        raise RuntimeError(f"Eventide truth labels incomplete: {labels!r}")
    if driver.execute_script("return performance.getEntriesByType('resource').filter(e => e.name.includes('/api/v1/assignment')).length"):
        raise RuntimeError("opening Eventide choices issued an assignment mutation")
    print(f"browser-studio-eventide-truth: PASS choices={len(labels)}")
finally:
    driver.quit()
