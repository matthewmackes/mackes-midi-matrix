#!/usr/bin/env python3
"""Verify Studio scene save and confirmation-gated recall requests."""
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
    driver.get(f"{origin}/studio/scenes")
    wait.until(lambda d: d.find_element("id", "studio-supporting-view").get_attribute("hidden") is None)
    driver.execute_script("""
      window.confirm = () => true; window.__sceneCalls = [];
      window.__sceneFetch = window.fetch;
      window.fetch = (url, options) => {
        const text = String(url);
        if (text.endsWith('/api/v1/scenes') && (!options || options.method !== 'POST')) return Promise.resolve({ok:true, status:200, json:async()=>({scenes:[{id:'scene-a', name:'Scene A'}], active_scene:'Scene A'})});
        if (text.endsWith('/api/v1/scenes') && options?.method === 'POST') { window.__sceneCalls.push(JSON.parse(options.body)); return Promise.resolve({ok:true, status:200, json:async()=>({ok:true, active_scene:'Scene A'})}); }
        return window.__sceneFetch(url, options);
      };
    """)
    driver.execute_script("window.MackesStudioViewsRefresh()")
    wait.until(lambda d: d.find_element("css selector", ".scene-actions input").is_displayed())
    name = driver.find_element("css selector", ".scene-actions input"); name.send_keys("My Setup")
    driver.find_element("css selector", ".scene-actions button.add-assignment").click()
    wait.until(lambda d: len(d.execute_script("return window.__sceneCalls")) == 1)
    wait.until(lambda d: d.find_elements("css selector", ".scene-actions button.quiet-button"))
    driver.find_element("css selector", ".scene-actions button.quiet-button").click()
    wait.until(lambda d: len(d.execute_script("return window.__sceneCalls")) == 2)
    calls = driver.execute_script("return window.__sceneCalls")
    if calls[0].get('scene') != 'My Setup' or calls[1].get('execute_scene') != 'scene-a':
        raise RuntimeError(f"unexpected scene requests: {calls!r}")
    print("browser-studio-scenes-actions: PASS save_then_confirmed_recall")
finally:
    driver.quit()
