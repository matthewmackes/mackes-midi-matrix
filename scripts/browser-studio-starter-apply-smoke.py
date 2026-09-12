#!/usr/bin/env python3
"""Verify Quick Start previews first and sends one atomic typed batch on Apply."""
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
    driver.execute_script("""
      window.confirm = () => true;
      window.__starterBatch = null;
      window.__starterConflict = false;
      const nativeFetch = window.fetch;
      window.fetch = async (input, init) => {
        const url = String(input);
        if (url.endsWith('/api/v1/studio/capabilities')) return new Response(JSON.stringify({generation:4, devices:[{stable_id:'eventide-1', label:'Eventide MicroPitch', renderer:'eventide.micropitch', lifecycle:'ready', features:[{key:'pitch-a', label:'Pitch A', writable:true, readable:false, led_feedback:false}]}]}), {status:200, headers:{'Content-Type':'application/json'}});
        if (url.endsWith('/api/v1/mappings') && (!init || init.method !== 'POST')) return new Response(JSON.stringify({generation:4, active:[{physical_control_id:'knob-r1-c1', enabled:true, destination_parameter:'existing'}]}), {status:200, headers:{'Content-Type':'application/json'}});
        if (url.endsWith('/api/v1/mappings') && init?.method === 'POST') {
          window.__starterBatch = JSON.parse(init.body);
          if (window.__starterConflict) return new Response(JSON.stringify({reason:'mapping generation conflict'}), {status:409, headers:{'Content-Type':'application/json'}});
          return new Response(JSON.stringify({generation:5, outcome:'Applied'}), {status:200, headers:{'Content-Type':'application/json'}});
        }
        return nativeFetch(input, init);
      };
    """)
    driver.find_element("id", "studio-starter-review").click()
    wait.until(lambda d: d.find_element("id", "studio-starter-apply").is_displayed())
    if driver.find_elements("css selector", "#studio-starter-proposals [data-control='knob-r1-c1']"):
        raise RuntimeError("starter review proposed over an existing assignment")
    if driver.execute_script("return window.__starterBatch") is not None:
        raise RuntimeError("review unexpectedly sent a mutation")
    driver.find_element("id", "studio-starter-apply").click()
    wait.until(lambda d: d.execute_script("return window.__starterBatch !== null"))
    batch = driver.execute_script("return window.__starterBatch")
    if batch.get("payload", {}).get("kind") != "Batch" or batch.get("operation") != "Activate" or not batch["payload"].get("mappings"):
        raise RuntimeError(f"invalid starter batch: {batch!r}")
    driver.execute_script("window.__starterConflict = true; window.MackesStudioState.publish({connection:'ready'}); document.getElementById('studio-starter-review').click();")
    wait.until(lambda d: d.find_element("id", "studio-starter-apply").is_displayed())
    driver.find_element("id", "studio-starter-apply").click()
    wait.until(lambda d: "Setup was not changed" in d.find_element("id", "studio-starter-status").text)
    if driver.find_element("id", "studio-starter-proposals").get_attribute("hidden") is not None:
        raise RuntimeError("conflict discarded reviewed starter proposals")
    print(f"browser-studio-starter-apply: PASS mappings={len(batch['payload']['mappings'])} atomic=Batch conflict_preserved=true")
finally:
    driver.quit()
