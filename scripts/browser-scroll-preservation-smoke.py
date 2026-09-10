#!/usr/bin/env python3
"""Verify workspace refresh does not reset the operator's scroll position."""

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
options.add_argument("--window-size=768,1024")
options.binary_location = "/usr/bin/chromium-browser"
driver = webdriver.Chrome(service=Service("/usr/bin/chromedriver"), options=options)
try:
    driver.get(f"{origin}/devices/novation")
    WebDriverWait(driver, 15).until(lambda d: "Novation control grid" in d.page_source)
    driver.execute_script("window.scrollTo(0, 420); refreshActiveView();")
    WebDriverWait(driver, 15).until(lambda d: d.execute_script("return window.scrollY") >= 380)
    print(f"browser-scroll-preservation-smoke: PASS origin={origin}")
finally:
    driver.quit()
