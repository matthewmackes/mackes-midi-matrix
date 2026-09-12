#!/usr/bin/env python3
"""Verify a PiPedal-originated observation updates namespaced Studio state without echo."""
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
    wait.until(lambda d: len(d.find_elements("css selector", "#studio-controller .physical-control")) == 56)
    driver.execute_script("window.MackesStudioHandleEvent({kind:'pipedal.control', payload:{symbol:'gain', value:0.75, device:'PiPedal'}});")
    observed = driver.execute_script("return window.MackesStudioState.read().observations['pipedal:gain'];")
    if not observed or observed.get("value") != 0.75 or observed.get("source") != "external":
        raise RuntimeError(f"PiPedal observation was not reconciled: {observed!r}")
    driver.execute_script("window.MackesStudioHandleEvent({kind:'pipedal.meter', payload:{control:'output-level', observed_value:0.42, device:'PiPedal'}});")
    meter = driver.execute_script("return window.MackesStudioState.read().observations['pipedal:output-level'];")
    if not meter or meter.get("value") != 0.42 or meter.get("source") != "external":
        raise RuntimeError(f"PiPedal meter was not reconciled: {meter!r}")
    if driver.execute_script("return performance.getEntriesByType('resource').filter(e => e.name.includes('/api/v1/assignment')).length"):
        raise RuntimeError("PiPedal observation issued an assignment mutation")
    print(f"browser-studio-pipedal-event: PASS feature=pipedal:gain value={observed['value']} meter={meter['value']}")
finally:
    driver.quit()
