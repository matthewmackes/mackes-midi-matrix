#!/usr/bin/env python3
"""Verify the installed root route is the clean-sheet Studio and deep links remain intact."""
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
    for path in ("/", "/studio"):
        driver.get(f"{origin}{path}")
        wait.until(lambda d: len(d.find_elements("css selector", "#studio-controller .physical-control")) == 56)
        if "MACKES Studio" not in driver.find_element("tag name", "body").text:
            raise RuntimeError(f"{path} did not render the Studio shell")
    print("browser-root-studio: PASS root_and_studio_deep_link")
finally:
    driver.quit()
