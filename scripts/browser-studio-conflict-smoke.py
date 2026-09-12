#!/usr/bin/env python3
"""Verify a rejected assignment save preserves the draft and exposes a clear conflict state."""
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
    driver.find_element("css selector", ".destination-preview button:nth-child(2)").click()
    wait.until(lambda d: d.find_elements("css selector", "#destination-functions button.function-choice"))[0].click()
    wait.until(lambda d: d.find_element("id", "studio-assignment-preview").get_attribute("hidden") is None)
    driver.execute_script("""
      window.__conflictOriginalFetch = window.fetch;
      window.fetch = (url, options) => String(url).includes('/api/v1/assignment')
        ? Promise.resolve({ok:false, status:409, json:async()=>({reason:'mapping generation conflict'})})
        : window.__conflictOriginalFetch(url, options);
    """)
    driver.find_element("id", "studio-assignment-save").click()
    wait.until(lambda d: "Conflict detected" in d.find_element("id", "studio-assignment-status").text)
    if driver.find_element("id", "studio-assignment-preview").get_attribute("hidden") is not None:
        raise RuntimeError("conflict discarded the assignment preview")
    if driver.execute_script("return !window.MackesStudioState.read().draft"):
        raise RuntimeError("conflict discarded the local draft")
    print("browser-studio-conflict: PASS draft_preserved_after_409")
finally:
    driver.quit()
