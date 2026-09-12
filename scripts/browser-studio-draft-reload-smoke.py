#!/usr/bin/env python3
"""Verify an uncommitted assignment draft survives a browser reload."""
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
    driver.set_page_load_timeout(15)
    print("draft-reload: loading", flush=True)
    driver.get(f"{origin}/studio")
    print("draft-reload: loaded", flush=True)
    controls = wait.until(lambda d: d.find_elements("css selector", "#studio-controller .physical-control"))
    controls[0].click()
    driver.find_element("css selector", ".destination-preview button:nth-child(2)").click()
    choice = wait.until(lambda d: d.find_element("css selector", "#destination-functions button.function-choice"))
    choice.click()
    print("draft-reload: draft-created", flush=True)
    before = wait.until(lambda d: d.find_element("id", "studio-assignment-preview").text)
    if "will control" not in before:
        raise RuntimeError(f"draft preview was not shown: {before!r}")
    if driver.execute_script("return window.MackesStudioState.read().draft === null"):
        raise RuntimeError("draft was not stored before reload")
    driver.refresh()
    print("draft-reload: refreshed", flush=True)
    restored = wait.until(lambda d: d.find_element("id", "studio-assignment-preview"))
    if restored.get_attribute("hidden") is not None or "Restored draft" not in restored.text:
        raise RuntimeError(f"draft was not restored after reload: {restored.text!r}")
    if driver.execute_script("return window.performance.getEntriesByType('resource').some(e => e.name.includes('/api/v1/assignment') && e.initiatorType !== 'fetch')"):
        raise RuntimeError("reload attempted an assignment mutation")
    print("browser-studio-draft-reload: PASS draft_restored_without_mutation")
finally:
    driver.quit()
