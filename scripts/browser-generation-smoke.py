#!/usr/bin/env python3
"""Verify older asynchronous view responses cannot overwrite newer generations."""

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
    driver.get(f"{origin}/devices/novation#browser_smoke=1")
    WebDriverWait(driver, 15).until(lambda d: "Novation control grid" in d.page_source)
    driver.execute_script("""
      window.__generationFetchCount = 0;
      window.__generationOriginalFetch = window.fetch;
      window.fetch = (...args) => {
        if (String(args[0]).endsWith('/api/v1/devices')) {
          window.__generationFetchCount += 1;
          const generation = window.__generationFetchCount;
          const response = new Response(JSON.stringify({generation, endpoints: [], devices: []}), {status: 200, headers: {'Content-Type': 'application/json'}});
          return generation === 1 ? new Promise(resolve => setTimeout(() => resolve(response), 1000)) : Promise.resolve(response);
        }
        return window.__generationOriginalFetch(...args);
      };
    """)
    driver.execute_script("load('devices'); load('devices');")
    WebDriverWait(driver, 15).until(lambda d: d.execute_script("return window.__generationFetchCount") >= 2)
    WebDriverWait(driver, 15).until(lambda d: "Generation: 2" in d.find_element("id", "state").text)
    assert "Generation: 1" not in driver.find_element("id", "state").text
    print(f"browser-generation-smoke: PASS origin={origin}")
finally:
    driver.quit()
