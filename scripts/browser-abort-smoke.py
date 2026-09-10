#!/usr/bin/env python3
"""Verify navigation aborts a superseded workspace request."""

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
      window.__abortCount = 0;
      window.__abortOriginalFetch = window.fetch;
      window.fetch = (...args) => {
        if (String(args[0]).endsWith('/api/v1/devices')) {
          const signal = args[1] && args[1].signal;
          if (signal) signal.addEventListener('abort', () => window.__abortCount += 1, {once: true});
          return new Promise((resolve, reject) => {
            const fail = () => reject(new DOMException('aborted', 'AbortError'));
            if (signal?.aborted) return fail();
            signal?.addEventListener('abort', fail, {once: true});
            setTimeout(() => resolve(new Response(JSON.stringify({endpoints: []}), {status: 200})), 5000);
          });
        }
        return window.__abortOriginalFetch(...args);
      };
      load('devices');
      setTimeout(() => navigate('routes'), 100);
    """)
    WebDriverWait(driver, 15).until(lambda d: d.execute_script("return window.__abortCount") > 0)
    WebDriverWait(driver, 15).until(lambda d: d.find_element("id", "view-title").text == "Routing")
    print(f"browser-abort-smoke: PASS origin={origin}")
finally:
    driver.quit()
