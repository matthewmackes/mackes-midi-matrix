#!/usr/bin/env python3
"""Walk the primary graphical workspaces as a nontechnical user."""

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


def visible(element) -> bool:
    return element.is_displayed() and element.get_attribute("hidden") is None


try:
    workspaces = {
        "studio": ("/studio", "#studio-controller"),
        "devices": ("/studio/devices", "#studio-supporting-view"),
        "routing": ("/studio/routing", "#studio-supporting-view"),
        "scenes": ("/studio/scenes", "#studio-supporting-view"),
        "system": ("/studio/system", "#studio-supporting-view"),
    }
    forbidden = {"textarea", "pre", "[contenteditable='true']"}
    for view, (path, selector) in workspaces.items():
        driver.get(f"{origin}{path}")
        wait = WebDriverWait(driver, 45)
        wait.until(lambda d: any(visible(item) for item in d.find_elements("css selector", selector)) or "Unavailable" in d.find_element("tag name", "body").text)
        if any(driver.find_elements("css selector", item) for item in forbidden):
            raise RuntimeError(f"{view} exposes a code-like editor surface")
        visible_text = driver.execute_script("""
          return Array.from(document.querySelectorAll('body *'))
            .filter(item => item.offsetParent !== null)
            .map(item => item.textContent || '').join(' ');
        """).lower()
        if any(marker in visible_text for marker in ("raw json5", "enter sysex", "state dump", "runtime id")):
            raise RuntimeError(f"{view} exposes implementation language in visible text")
    print(f"browser-novice-walkthrough: PASS origin={origin} workspaces={len(workspaces)}")
finally:
    driver.quit()
