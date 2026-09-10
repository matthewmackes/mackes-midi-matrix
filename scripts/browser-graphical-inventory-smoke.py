#!/usr/bin/env python3
"""Verify that live and researched endpoint cards are graphical, not text-only."""

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
    wait.until(lambda browser: browser.find_elements(By.CSS_SELECTOR, "#feature-board article"))
    device_cards = driver.find_elements(By.CSS_SELECTOR, "#device-board article")
    feature_cards = driver.find_elements(By.CSS_SELECTOR, "#feature-board article")
    graphical_devices = [card for card in device_cards if card.find_elements(By.CSS_SELECTOR, "svg.device-graphic")]
    graphical_features = [card for card in feature_cards if card.find_elements(By.CSS_SELECTOR, "svg.device-graphic")]
    if len(graphical_devices) != len(device_cards):
        raise RuntimeError(f"text-only live device cards: {len(device_cards) - len(graphical_devices)}")
    if len(graphical_features) != len(feature_cards):
        raise RuntimeError(f"text-only researched feature cards: {len(feature_cards) - len(graphical_features)}")
    print(f"browser-graphical-inventory: PASS devices={len(device_cards)} features={len(feature_cards)} origin={origin}")
finally:
    driver.quit()
