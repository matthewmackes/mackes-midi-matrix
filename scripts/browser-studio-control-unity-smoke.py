#!/usr/bin/env python3
"""Verify the clean-sheet Studio control surface contract without hardware."""

from __future__ import annotations

import json
import sys
import time

from selenium import webdriver
from selenium.webdriver.chrome.options import Options
from selenium.webdriver.chrome.service import Service
from selenium.webdriver.common.by import By
from selenium.webdriver.support.ui import WebDriverWait


origin = sys.argv[1] if len(sys.argv) > 1 else "http://172.20.222.222:8081"
options = Options()
for argument in ("--headless", "--no-sandbox", "--disable-gpu", "--disable-dev-shm-usage", "--no-proxy-server"):
    options.add_argument(argument)
options.add_argument("--window-size=1440,1200")
options.set_capability("goog:loggingPrefs", {"performance": "ALL"})
options.binary_location = "/usr/bin/chromium-browser"
driver = webdriver.Chrome(service=Service("/usr/bin/chromedriver"), options=options)
try:
    driver.get(f"{origin}/studio#browser_smoke=1")
    wait = WebDriverWait(driver, 20)
    wait.until(lambda page: len(page.find_elements(By.CSS_SELECTOR, "#studio-controller [data-control-id]")) == 56)
    controls = driver.find_elements(By.CSS_SELECTOR, "#studio-controller [data-control-id]")
    ids = [control.get_attribute("data-control-id") for control in controls]
    assert len(set(ids)) == 56 and ids[0] == "knob-r1-c1", ids[:3]
    faders = driver.find_elements(By.CSS_SELECTOR, "#studio-controller [data-control-type='fader']")
    assert len(faders) == 8
    assert all("no LED" in (item.get_attribute("aria-label") or "") for item in faders)
    driver.find_element(By.CSS_SELECTOR, "#studio-controller [data-control-id='knob-r1-c1']").click()
    wait.until(lambda page: "Knobs 1" in page.find_element(By.ID, "assignment-title").text)
    driver.find_element(By.CSS_SELECTOR, ".layer-switcher button:nth-of-type(2)").click()
    wait.until(lambda page: page.find_element(By.CSS_SELECTOR, ".layer-switcher .layer-active").text == "L1")
    time.sleep(0.4)
    posts = []
    for entry in driver.get_log("performance"):
        message = json.loads(entry["message"])["message"]
        if message.get("method") == "Network.requestWillBeSent":
            request = message["params"]["request"]
            if request.get("method") == "POST":
                posts.append(request.get("url", ""))
    assert not any("/api/v1/" in url for url in posts), posts
    print(f"browser-studio-control-unity: PASS controls={len(ids)} faders={len(faders)} mutation_posts=0 origin={origin}")
finally:
    driver.quit()
