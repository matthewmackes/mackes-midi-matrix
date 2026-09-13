#!/usr/bin/env python3
"""Verify multiple destinations and behavior metadata remain visible without a mutation."""
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
    controls = wait.until(lambda d: d.find_elements("css selector", "#studio-controller .physical-control"))
    # Ignore the read-only bootstrap requests; inspection must not add a mutation afterward.
    driver.execute_script("performance.clearResourceTimings()")
    driver.execute_script("""
      window.MackesStudioState.publish({assignments: [
        {physical_control_id:'knob-r1-c1', destination_profile:'pipedal', destination_parameter:'gain', enabled:true,
         behavior:{source_range:[2,120], destination_range:[-12,18], curve:'ease-in', invert:true}},
        {physical_control_id:'knob-r1-c1', destination_profile:'eventide.micropitch', destination_parameter:'mix', enabled:true,
         behavior:{source_range:[0,127], destination_range:[0,127], curve:'linear', invert:false}}
      ]});
    """)
    controls[0].click()
    wait.until(lambda d: len(d.find_elements("css selector", "#studio-destination-cards .destination-card")) == 2)
    cards = driver.find_elements("css selector", "#studio-destination-cards .destination-card")
    text = " ".join(card.text for card in cards)
    if "Gain" not in text or "Mix" not in text or "ease-in" not in text or "linear" not in text:
        raise RuntimeError(f"multi-destination detail was not preserved: {text!r}")
    if driver.execute_script("return performance.getEntriesByType('resource').filter(e => e.name.includes('/api/v1/assignment') || e.name.includes('/api/v1/mappings')).some(e => e.initiatorType === 'fetch')"):
        raise RuntimeError("multi-destination inspection issued a mutation")
    print(f"browser-studio-multi-destination: PASS destinations={len(cards)}")
finally:
    driver.quit()
