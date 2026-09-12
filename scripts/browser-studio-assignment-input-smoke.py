#!/usr/bin/env python3
"""Verify pointer, keyboard, and touch-style activation on the clean-sheet assignment surface."""

from __future__ import annotations

import sys

from selenium import webdriver
from selenium.webdriver.chrome.options import Options
from selenium.webdriver.chrome.service import Service
from selenium.webdriver.common.keys import Keys
from selenium.webdriver.support.ui import WebDriverWait


origin = sys.argv[1] if len(sys.argv) > 1 else "http://172.20.222.222:8081"
options = Options()
for argument in ("--headless", "--no-sandbox", "--disable-gpu", "--disable-dev-shm-usage", "--no-proxy-server"):
    options.add_argument(argument)
options.add_argument("--window-size=1440,1200")
options.binary_location = "/usr/bin/chromium-browser"
driver = webdriver.Chrome(service=Service("/usr/bin/chromedriver"), options=options)
try:
    wait = WebDriverWait(driver, 20)
    driver.get(f"{origin}/studio")
    controls = wait.until(lambda d: d.find_elements("css selector", "#studio-controller .physical-control"))
    if len(controls) != 56:
        raise RuntimeError(f"expected 56 controls, got {len(controls)}")
    controls[0].click()
    wait.until(lambda d: d.find_element("id", "assignment-title").text)
    pointer_title = driver.find_element("id", "assignment-title").text
    controls[1].send_keys(Keys.ENTER)
    keyboard_title = driver.find_element("id", "assignment-title").text
    driver.execute_script("arguments[0].dispatchEvent(new PointerEvent('pointerup', {bubbles:true, pointerType:'touch'})); arguments[0].click();", controls[2])
    touch_title = driver.find_element("id", "assignment-title").text
    if not pointer_title or not keyboard_title or not touch_title:
        raise RuntimeError("one input path did not select a control")
    if driver.execute_script("return performance.getEntriesByType('resource').filter(e => e.name.includes('/api/v1/assignment')).length"):
        raise RuntimeError("selection input issued an assignment mutation")
    print(f"browser-studio-assignment-input: PASS pointer={pointer_title!r} keyboard={keyboard_title!r} touch={touch_title!r}")
finally:
    driver.quit()
