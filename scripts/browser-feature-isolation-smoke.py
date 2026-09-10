#!/usr/bin/env python3
"""Verify a slow/failed feature request does not falsely mark backend health offline."""

from __future__ import annotations

import sys
import time

from selenium import webdriver
from selenium.webdriver.chrome.options import Options
from selenium.webdriver.chrome.service import Service


origin = sys.argv[1] if len(sys.argv) > 1 else "http://172.20.222.222:8081"


def run(scenario: str) -> None:
    options = Options()
    options.add_argument("--headless")
    options.add_argument("--no-sandbox")
    options.add_argument("--disable-gpu")
    options.add_argument("--disable-dev-shm-usage")
    options.add_argument("--no-proxy-server")
    options.add_argument("--window-size=1440,1200")
    options.binary_location = "/usr/bin/chromium-browser"
    driver = webdriver.Chrome(service=Service("/usr/bin/chromedriver"), options=options)
    try:
        delay = "new Promise(resolve => setTimeout(() => resolve(original(...args)), 8000))"
        body = "Promise.reject(new Error('fixture feature failure'))"
        fixture = f"""
        (() => {{
          const original = window.fetch;
          window.fetch = (...args) => {{
            const url = String(args[0]);
            if (url.includes('/api/v1/mappings')) return {delay if scenario == 'delayed' else body};
            return original(...args);
          }};
        }})();
        """
        driver.execute_cdp_cmd("Page.addScriptToEvaluateOnNewDocument", {"source": fixture})
        driver.get(f"{origin}/devices/novation#browser_smoke=1")
        time.sleep(2)
        health = driver.find_element("id", "health").text.lower()
        assert "offline" not in health, f"{scenario} feature request falsely marked health offline: {health}"
        print(f"browser-feature-isolation: PASS scenario={scenario} health={health}")
    finally:
        driver.quit()


for scenario in ("delayed", "failed"):
    run(scenario)
