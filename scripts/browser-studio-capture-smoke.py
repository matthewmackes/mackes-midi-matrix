#!/usr/bin/env python3
"""Verify an external capture event selects and updates a stable control without writing."""
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
    driver.execute_script("window.MackesStudioHandleEvent({kind:'ControlCaptured', payload:{physical_control_id:'knob-r1-c2', observed_value:73, action:'ControlCaptured'}});")
    wait.until(lambda d: d.find_element("id", "assignment-title").text == "Knobs 2")
    control = driver.find_element("css selector", "[data-control-id='knob-r1-c2']")
    if "Observed 73" not in control.text:
        raise RuntimeError("capture event did not produce observed selection feedback")
    if driver.execute_script("return performance.getEntriesByType('resource').filter(e => e.name.includes('/api/v1/assignment')).length"):
        raise RuntimeError("capture event issued an assignment mutation")
    print(f"browser-studio-capture: PASS control={control.get_attribute('data-control-id')} value={control.text.splitlines()[2]}")
finally:
    driver.quit()
