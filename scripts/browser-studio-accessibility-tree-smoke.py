#!/usr/bin/env python3
"""Verify the clean-sheet controller exposes complete accessible controls and target sizes."""
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
    wait.until(lambda d: len(d.find_elements("css selector", "#studio-controller button.physical-control")) == 56)
    report = driver.execute_script("""
      return Array.from(document.querySelectorAll('#studio-controller button.physical-control')).map(item => {
        const rect = item.getBoundingClientRect();
        return {name: item.getAttribute('aria-label'), role: item.getAttribute('role') || item.tagName.toLowerCase(), width: rect.width, height: rect.height};
      });
    """)
    if not all(item["name"] and item["role"] == "button" for item in report):
        raise RuntimeError("one or more controls lacks an accessible name or button role")
    too_small = [item for item in report if item["width"] < 44 or item["height"] < 44]
    if too_small:
        raise RuntimeError(f"touch targets below 44px: {too_small[:2]!r}")
    print(f"browser-studio-accessibility-tree: PASS controls={len(report)} min_target=44px")
finally:
    driver.quit()
