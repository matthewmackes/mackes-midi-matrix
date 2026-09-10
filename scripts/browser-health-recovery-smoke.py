#!/usr/bin/env python3
"""Deterministic browser health failure/recovery fixture."""

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
    WebDriverWait(driver, 15).until(lambda d: d.find_element("id", "health"))
    driver.execute_script("""
      window.__healthFailures = 0;
      window.__healthOriginalFetch = window.fetch;
      window.fetch = (...args) => {
        if (String(args[0]).includes('/api/v1/health') && window.__healthFailures < 3) {
          window.__healthFailures += 1;
          return Promise.reject(new Error('fixture health failure'));
        }
        return window.__healthOriginalFetch(...args);
      };
    """)
    for _ in range(3):
        driver.execute_async_script("""const done = arguments[0]; pollHealth().finally(done);""")
    assert "offline" in driver.find_element("id", "health").text.lower()
    driver.execute_async_script("""const done = arguments[0]; pollHealth().finally(done);""")
    assert "offline" not in driver.find_element("id", "health").text.lower()
    print(f"browser-health-recovery: PASS origin={origin}")
finally:
    driver.quit()
