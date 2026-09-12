#!/usr/bin/env python3
"""Verify the Devices workspace renders typed capability truth without protocol identifiers."""
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
    driver.get(f"{origin}/studio/devices")
    wait.until(lambda d: d.find_element("id", "studio-supporting-view").get_attribute("hidden") is None)
    driver.execute_script("""
      const nativeFetch = window.fetch;
      window.fetch = (url, options) => {
        const text = String(url);
        if (text.endsWith('/api/v1/studio/capabilities')) return Promise.resolve({ok:true, status:200, json:async()=>({devices:[{label:'Eventide MicroPitch', renderer:'eventide.micropitch', lifecycle:'ready', features:[{label:'Pitch A', writable:true, readable:false},{label:'Mix', writable:true, readable:true}]}]})});
        if (text.endsWith('/api/v1/health')) return Promise.resolve({ok:true, status:200, json:async()=>({})});
        if (text.endsWith('/api/v1/mappings')) return Promise.resolve({ok:true, status:200, json:async()=>({active:[]})});
        return nativeFetch(url, options);
      };
    """)
    driver.execute_script("window.MackesStudioViewsRefresh()")
    wait.until(lambda d: "2 named functions" in d.find_element("css selector", ".device-capability-list").text)
    text = driver.find_element("css selector", ".device-capability-list").text
    if "2 adjustable" not in text or "1 with live readback" not in text or "Pitch A: send only" not in text:
        raise RuntimeError(f"capability truth missing: {text!r}")
    if "eventide.micropitch" in text.lower() or "stable_id" in text.lower():
        raise RuntimeError("protocol identity leaked into capability cards")
    print("browser-studio-capability-view: PASS ready_counts_and_protocol_hiding")
finally:
    driver.quit()
