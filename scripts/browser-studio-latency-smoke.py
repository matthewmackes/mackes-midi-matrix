#!/usr/bin/env python3
"""Verify clean-sheet control selection produces a visible response within 100ms."""
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
    latency = driver.execute_script("""
      const controls = Array.from(document.querySelectorAll('#studio-controller .physical-control')).slice(0, 20);
      const heading = document.querySelector('#assignment-title');
      const samples = controls.map((control, index) => { const expected = `Knobs ${index + 1}`; const started = performance.now(); control.click(); return {ms: performance.now() - started, expected, actual: heading.textContent}; });
      return {samples, heading: heading.textContent};
    """)
    ordered = sorted(item["ms"] for item in latency["samples"])
    p95 = ordered[max(0, int(len(ordered) * 0.95) - 1)]
    if latency["heading"] != "Knobs 20" or p95 > 100 or any(item["expected"] != item["actual"] for item in latency["samples"]):
        raise RuntimeError(f"visible selection p95 exceeded budget: p95={p95:.2f} samples={latency['samples']!r}")
    feedback = driver.execute_script("""
      const control = document.querySelector('[data-control-id="knob-r1-c1"]');
      const started = performance.now();
      window.MackesStudioHandleEvent({payload:{physical_control_id:'knob-r1-c1', observed_value:73}});
      return {ms: performance.now() - started, text: control.querySelector('.control-value').textContent};
    """)
    if feedback["ms"] > 500 or not feedback["text"].startswith("Observed"):
        raise RuntimeError(f"live feedback convergence exceeded budget: {feedback!r}")
    print(f"browser-studio-latency: PASS samples={len(ordered)} p95_ms={p95:.2f} feedback_ms={feedback['ms']:.2f}")
finally:
    driver.quit()
