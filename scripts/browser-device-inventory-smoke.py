#!/usr/bin/env python3
"""Verify the live Devices workspace renders authoritative inventory rows."""

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
    for attempt in range(2):
        driver.get(f"{origin}/devices#browser_smoke=1&attempt={attempt + 1}")
        wait = WebDriverWait(driver, 45)
        try:
            wait.until(lambda d: d.find_element("id", "device-board").get_attribute("hidden") is None)
            wait.until(lambda d: len(d.find_elements("css selector", "#device-board article")) > 0)
            break
        except Exception:
            if attempt == 1:
                raise
            driver.delete_all_cookies()
    board = driver.find_element("id", "device-board")
    cards = [card.text for card in board.find_elements("css selector", "article")]
    if not any("Novation" in card or "Launch Control XL" in card for card in cards):
        raise RuntimeError(f"Novation inventory row missing from {len(cards)} device cards")
    print(f"browser-device-inventory: PASS origin={origin} cards={len(cards)}")
finally:
    driver.quit()
