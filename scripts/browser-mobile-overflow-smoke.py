#!/usr/bin/env python3
"""Verify narrow workspaces do not widen the document viewport."""

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
options.add_argument("--window-size=320,900")
options.binary_location = "/usr/bin/chromium-browser"
driver = webdriver.Chrome(service=Service("/usr/bin/chromedriver"), options=options)
try:
    driver.get(f"{origin}/studio")
    WebDriverWait(driver, 30).until(lambda d: len(d.find_elements("css selector", "#studio-controller .physical-control")) == 56)
    width = driver.execute_script("return [document.documentElement.scrollWidth, window.innerWidth]")
    if width[0] > width[1]:
        raise RuntimeError(f"document overflow: scrollWidth={width[0]} innerWidth={width[1]}")
    print(f"browser-mobile-overflow: PASS origin={origin} width={width[1]}")
finally:
    driver.quit()
