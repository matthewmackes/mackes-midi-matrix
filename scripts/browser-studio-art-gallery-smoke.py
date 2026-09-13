#!/usr/bin/env python3
"""Verify the installed semantic MOD art gallery renders its governed examples."""
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
    driver.get(f"{origin}/studio/gallery")
    wait = WebDriverWait(driver, 30)
    items = wait.until(lambda d: d.find_elements("css selector", ".mod-art-gallery-item"))
    states = driver.find_elements("css selector", ".mod-art-state")
    images = driver.find_elements("css selector", ".mod-art-gallery-item img")
    if len(items) < 10 or len(states) < 10 or len(images) < 8:
        raise RuntimeError(f"incomplete semantic gallery items={len(items)} states={len(states)} images={len(images)}")
    if any(item.size["width"] < 44 or item.size["height"] < 44 for item in items):
        raise RuntimeError("gallery target is below 44 CSS pixels")
    print(f"browser-studio-art-gallery: PASS items={len(items)} states={len(states)} images={len(images)}")
finally:
    driver.quit()
