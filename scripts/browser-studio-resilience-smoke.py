#!/usr/bin/env python3
"""Verify Studio keeps known controls and selection visible across refresh failure."""
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
    controls[0].click()
    wait.until(lambda d: d.find_element("id", "assignment-title").text == "Knobs 1")
    driver.execute_script("window.__studioOriginalFetch=window.fetch; window.fetch=(...args)=>Promise.reject(new Error('resilience fixture failure')); window.MackesStudioRefresh();")
    wait.until(lambda d: "Stale" in d.find_element("css selector", "[data-control-id='knob-r1-c1']").text)
    if len(driver.find_elements("css selector", "#studio-controller .physical-control")) != 56:
        raise RuntimeError("refresh failure removed known controls")
    if driver.find_element("id", "assignment-title").text != "Knobs 1":
        raise RuntimeError("refresh failure lost the selected control")
    driver.execute_script("window.MackesStudioState.reconcile({generation:4, sequence:12, observations:{'knob-r1-c1':{value:91, freshness:'observed'}}}); window.MackesStudioState.reconcile({generation:4, sequence:11, observations:{'knob-r1-c1':{value:12, freshness:'stale'}}});")
    observed = driver.execute_script("return window.MackesStudioState.read().observations['knob-r1-c1'].value;")
    if observed != 91:
        raise RuntimeError(f"late event regressed observed value: {observed!r}")
    driver.execute_script("window.MackesStudioHandleEvent({payload:{physical_control_id:'knob-r1-c1', observed_value:88, freshness:'stale'}})")
    stale_text = driver.find_element("css selector", "[data-control-id='knob-r1-c1'] .control-value").text
    if not stale_text.startswith("Stale"):
        raise RuntimeError(f"stale observation was presented as current: {stale_text!r}")
    driver.execute_script("window.MackesStudioHandleEvent({payload:{last_activity:{physical_control_id:'knob-r1-c1', observed_value:64}}})")
    nested_text = driver.find_element("css selector", "[data-control-id='knob-r1-c1'] .control-value").text
    if nested_text != "Observed 64":
        raise RuntimeError(f"nested live activity did not update control: {nested_text!r}")
    nested_control = driver.find_element("css selector", "[data-control-id='knob-r1-c1']")
    if "is-live-active" not in nested_control.get_attribute("class").split():
        raise RuntimeError("observed physical knob was not highlighted")
    angle = nested_control.value_of_css_property("--control-angle") or nested_control.get_attribute("style")
    if "1.062992125984" not in angle and "--control-angle" not in angle:
        raise RuntimeError(f"observed physical knob did not rotate: {angle!r}")
    print("browser-studio-resilience: PASS known_controls_and_selection_preserved")
finally:
    driver.quit()
