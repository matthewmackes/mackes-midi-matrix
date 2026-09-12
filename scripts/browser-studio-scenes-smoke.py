#!/usr/bin/env python3
"""Verify the Scenes workspace presents authoritative scene and assignment context."""
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
    driver.get(f"{origin}/studio/scenes")
    wait = WebDriverWait(driver, 45)
    wait.until(lambda d: "SAVED SCENES" in d.find_element("id", "studio-supporting-view").text)
    text = driver.find_element("id", "studio-supporting-view").text
    if "ASSIGNMENTS" not in text or "Scene changes preserve destination details" not in text:
        raise RuntimeError(f"scene context is incomplete: {text!r}")
    if any(token in text.lower() for token in ("mapping id", "runtime", "uri", "json")):
        raise RuntimeError(f"scene view exposed implementation detail: {text!r}")
    print(f"browser-studio-scenes: PASS origin={origin} saved_state={text.splitlines()[4]!r}")
finally:
    driver.quit()
