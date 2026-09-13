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
    driver.execute_script("[0.75,0.76,0.77].forEach(value => window.MackesStudioHandleEvent({kind:'pipedal.control', payload:{symbol:'gain', value, device:'PiPedal'}}));")
    observed = driver.execute_script("return window.MackesStudioState.read().observations['pipedal:gain'];")
    if not observed or observed.get("value") != 0.77 or observed.get("source") != "external":
        raise RuntimeError(f"PiPedal observation was not reconciled: {observed!r}")
    driver.execute_script("window.MackesStudioHandleEvent({kind:'pipedal.control', payload:{symbol:'gain', value:0.5, device:'PiPedal', stale_instance:true}});")
    stale = driver.execute_script("return window.MackesStudioState.read().observations['pipedal:gain'];")
    if not stale or stale.get("value") != 0.5 or stale.get("freshness") != "stale":
        raise RuntimeError(f"stale PiPedal instance was not marked stale: {stale!r}")
    driver.execute_script("window.MackesStudioHandleEvent({kind:'pipedal.control', payload:{symbol:'mix', value:0.31, device:'PiPedal', freshness:'stale'}});")
    driver.execute_script("window.MackesStudioHandleEvent({kind:'pipedal.meter', payload:{control:'output-level', observed_value:0.42, device:'PiPedal'}});")
    driver.execute_async_script("const done = arguments[0]; queueMicrotask(done);")
    meter = driver.execute_script("return window.MackesStudioState.read().observations['pipedal:output-level'];")
    if not meter or meter.get("value") != 0.42 or meter.get("source") != "external":
        raise RuntimeError(f"PiPedal meter was not reconciled: {meter!r}")
    driver.execute_script("window.MackesStudioState.reconcile({generation:9, sequence:20, connection:'reconnected', observations:{'pipedal:gain':{value:0.77, source:'external', freshness:'observed'}, 'pipedal:output-level':{value:0.42, source:'external', freshness:'observed'}, 'pipedal:mix':{value:0.31, source:'external', freshness:'stale'}}});")
    restored = driver.execute_script("return window.MackesStudioState.read().observations;")
    if restored.get("pipedal:gain", {}).get("value") != 0.77 or restored.get("pipedal:output-level", {}).get("value") != 0.42 or restored.get("pipedal:mix", {}).get("freshness") != "stale":
        raise RuntimeError(f"PiPedal reconnect snapshot lost observations: {restored!r}")
    burst_ms = driver.execute_script("const start = performance.now(); for (let i = 0; i < 1000; i += 1) window.MackesStudioHandleEvent({kind:'pipedal.meter', payload:{control:'output-level', observed_value:(i % 100) / 100, device:'PiPedal'}}); return performance.now() - start;")
    if burst_ms >= 100:
        raise RuntimeError(f"PiPedal event burst exceeded 100ms UI budget: {burst_ms:.2f}ms")
    if driver.execute_script("return performance.getEntriesByType('resource').filter(e => e.name.includes('/api/v1/assignment')).length"):
        raise RuntimeError("PiPedal observation issued an assignment mutation")
    print(f"browser-studio-pipedal-event: PASS burst=pipedal:gain value={observed['value']} meter={meter['value']} reconnect=preserved performance=1000/{burst_ms:.2f}ms")
finally:
    driver.quit()
