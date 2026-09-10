#!/usr/bin/env python3
"""Verify background refresh does not overwrite a dirty routing draft."""

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
    driver.get(f"{origin}/routes")
    WebDriverWait(driver, 15).until(lambda d: d.find_element("id", "routes-json").is_displayed())
    draft = '[{"source":"draft-source","destination":"draft-destination","enabled":true}]'
    driver.execute_script("""
      const textarea = document.getElementById('routes-json');
      textarea.value = arguments[0];
      textarea.dispatchEvent(new Event('input', {bubbles: true}));
      refreshActiveView();
    """, draft)
    assert driver.find_element("id", "routes-json").get_attribute("value") == draft
    print(f"browser-draft-preservation-smoke: PASS origin={origin}")
finally:
    driver.quit()
