#!/usr/bin/env python3
"""Verify a confirmed assignment exposes Undo and sends one typed undo request."""
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
    driver.find_element("css selector", "[data-control-id='knob-r1-c1']").click()
    driver.find_element("css selector", ".destination-preview button:nth-child(2)").click()
    wait.until(lambda d: d.find_elements("css selector", "#destination-functions button.function-choice"))[0].click()
    wait.until(lambda d: d.find_element("id", "studio-assignment-preview").get_attribute("hidden") is None)
    driver.execute_script("""
      window.confirm = () => true; window.__undoCalls = [];
      window.__undoOriginalFetch = window.fetch;
      window.fetch = (url, options) => {
        const text = String(url); if (text.includes('/api/v1/assignment')) return Promise.resolve({ok:true, status:200, json:async()=>({applied:true, generation:8})});
        if (text.includes('/api/v1/mappings') && options?.method === 'POST') { window.__undoCalls.push(JSON.parse(options.body)); return Promise.resolve({ok:true, status:200, json:async()=>({outcome:'Applied', generation:9})}); }
        return window.__undoOriginalFetch(url, options);
      };
    """)
    driver.find_element("id", "studio-assignment-save").click()
    wait.until(lambda d: d.find_element("id", "studio-assignment-undo").is_displayed())
    driver.find_element("id", "studio-assignment-undo").click()
    wait.until(lambda d: len(d.execute_script("return window.__undoCalls")) == 1)
    request = driver.execute_script("return window.__undoCalls[0]")
    if request.get("operation") != "Undo" or not isinstance(request.get("generation"), int):
        raise RuntimeError(f"invalid undo request: {request!r}")
    print("browser-studio-undo: PASS confirmed_save_then_typed_undo")
finally:
    driver.quit()
