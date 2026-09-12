#!/usr/bin/env python3
"""Verify assignment preview and cancellation preserve authoritative state."""
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
    choice = wait.until(lambda d: d.find_element("css selector", "#destination-functions button.function-choice"))
    choice.click()
    preview = wait.until(lambda d: d.find_element("id", "studio-assignment-preview"))
    if preview.get_attribute("hidden") is not None or "will control" not in preview.text:
        raise RuntimeError(f"assignment preview was not shown: {preview.text!r}")
    driver.find_element("id", "studio-assignment-cancel").click()
    if driver.find_element("id", "studio-assignment-preview").get_attribute("hidden") is None:
        raise RuntimeError("cancel left the assignment preview open")
    if driver.execute_script("return window.MackesStudioState.read().draft") is not None:
        raise RuntimeError("cancel did not clear the local draft")
    if driver.execute_script("return performance.getEntriesByType('resource').filter(e => e.name.includes('/api/v1/assignment')).length"):
        raise RuntimeError("preview/cancel issued an assignment mutation")
    print("browser-studio-draft: PASS preview_cancel_without_mutation")
finally:
    driver.quit()
