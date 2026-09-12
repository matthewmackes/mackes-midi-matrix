#!/usr/bin/env python3
"""Verify theme, reduced-motion, zoom, and non-color graphical state cues."""

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
options.set_capability("pageLoadStrategy", "eager")
driver = webdriver.Chrome(service=Service("/usr/bin/chromedriver"), options=options)
try:
    driver.execute_cdp_cmd("Emulation.setEmulatedMedia", {"features": [{"name": "prefers-reduced-motion", "value": "reduce"}]})
    driver.get(f"{origin}/studio")
    wait = WebDriverWait(driver, 45)
    wait.until(lambda d: len(d.find_elements("css selector", "#studio-controller .physical-control")) == 56)
    driver.find_element("id", "studio-theme").click()
    theme = driver.find_element("tag name", "body").get_attribute("class")
    if "light-theme" not in theme.split():
        raise RuntimeError(f"light theme not applied: {theme!r}")
    reduced_motion = driver.execute_script("return matchMedia('(prefers-reduced-motion: reduce)').matches")
    if not reduced_motion:
        raise RuntimeError("reduced-motion preference was not applied")
    labels = driver.execute_script("return Array.from(document.querySelectorAll('#studio-controller .physical-control[aria-label]')).map(item => item.getAttribute('aria-label'))")
    if len(labels) != 56 or not all(any(marker in label for marker in ("assigned", "unassigned", "disabled")) for label in labels):
        raise RuntimeError("graphical controls lack non-color assignment state labels")
    driver.execute_script("document.body.style.zoom = '200%'")
    zoom = driver.execute_script("return getComputedStyle(document.body).zoom")
    controls_after_zoom = len(driver.find_elements("css selector", "#studio-controller .physical-control"))
    if zoom not in ("2", "2.0") or controls_after_zoom != 56:
        raise RuntimeError(f"zoomed graphical layout is not usable: zoom={zoom!r} controls={controls_after_zoom}")
    print(f"browser-visual-accessibility: PASS origin={origin} controls={len(labels)} zoom={zoom}")
finally:
    driver.quit()
