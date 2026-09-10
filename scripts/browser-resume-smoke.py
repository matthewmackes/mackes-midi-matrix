#!/usr/bin/env python3
"""Verify visible-tab resume triggers an authoritative health refresh."""

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
    driver.get(f"{origin}/devices/novation")
    WebDriverWait(driver, 15).until(lambda d: "Novation control grid" in d.page_source)
    driver.execute_script("""
      window.__resumeHealthCount = 0;
      window.__resumeOriginalFetch = window.fetch;
      window.fetch = (...args) => {
        if (String(args[0]).endsWith('/api/v1/health')) window.__resumeHealthCount += 1;
        return window.__resumeOriginalFetch(...args);
      };
      document.dispatchEvent(new Event('visibilitychange'));
    """)
    WebDriverWait(driver, 15).until(lambda d: d.execute_script("return window.__resumeHealthCount") > 0)
    print(f"browser-resume-smoke: PASS origin={origin}")
finally:
    driver.quit()
