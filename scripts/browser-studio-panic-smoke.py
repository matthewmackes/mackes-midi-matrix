#!/usr/bin/env python3
"""Verify Panic is always visible and confirmation-gated before any operation request."""
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
    driver.get(f"{origin}/studio")
    WebDriverWait(driver, 30).until(lambda d: d.find_element("id", "studio-panic").is_displayed())
    driver.execute_script("window.confirm=()=>false; document.querySelector('#studio-panic').click();")
    if driver.execute_script("return performance.getEntriesByType('resource').filter(e => e.name.includes('/api/v1/operations')).length"):
        raise RuntimeError("Panic request bypassed confirmation")
    captured = driver.execute_async_script("""
      const done = arguments[0];
      window.confirm = () => true;
      window.fetch = (url, options) => { window.__panicRequest = {url, options}; return Promise.resolve({ok:true, json:async()=>({ok:true})}); };
      document.querySelector('#studio-panic').click();
      setTimeout(() => done(window.__panicRequest || null), 25);
    """)
    if not captured or captured["url"] != "/api/v1/operations" or '"panic"' not in captured["options"]["body"] or '"confirm":true' not in captured["options"]["body"]:
        raise RuntimeError(f"affirmative Panic payload was malformed: {captured!r}")
    print("browser-studio-panic: PASS confirmation_gate_and_payload")
finally:
    driver.quit()
