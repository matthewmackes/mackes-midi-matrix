#!/usr/bin/env python3
"""Verify Studio layer selection is visible, keyboard-accessible, and mutation-free."""
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
    wait.until(lambda d: len(d.find_elements("css selector", "#studio-controller .physical-control")) == 56)
    layers = driver.find_elements("css selector", ".layer-switcher button")
    if len(layers) != 5:
        raise RuntimeError(f"expected five layers, got {len(layers)}")
    layers[1].send_keys(" ")
    wait.until(lambda d: d.find_elements("css selector", ".layer-switcher button")[1].get_attribute("aria-pressed") == "true")
    if driver.find_elements("css selector", ".layer-switcher button")[0].get_attribute("aria-pressed") != "false":
        raise RuntimeError("Base layer remained active after L1 selection")
    if driver.execute_script("return performance.getEntriesByType('resource').filter(e => e.name.includes('/api/v1/assignment')).length"):
        raise RuntimeError("layer selection issued an assignment mutation")
    print("browser-studio-layer: PASS Base_to_L1_without_mutation")
finally:
    driver.quit()
