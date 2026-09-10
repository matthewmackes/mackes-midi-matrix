#!/usr/bin/env python3
"""Verify governed device families resolve to graphical renderers in the installed browser."""

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
    driver.get(f"{origin}/devices")
    WebDriverWait(driver, 15).until(lambda d: d.execute_script("return Boolean(window.MackesDeviceRenderer)"))
    result = driver.execute_script(
        """
        const samples = [
          ['Novation Launch Control XL', 'novation.launch-control-xl'],
          ['Eventide MicroPitch', 'eventide.micropitch'],
          ['Lexicon Reflex', 'lexicon.reflex'],
          ['PiPedal', 'pipedal'],
          ['M-Audio MIDISPORT 4x4', 'm-audio.midisport-4x4'],
          ['RTP-MIDI peer', 'rtp-midi'],
          ['Generic MIDI', 'generic-midi'],
          ['MACKES virtual monitor', 'mackes.virtual-monitor'],
          ['Future endpoint', 'generic.endpoint'],
        ];
        const host = document.createElement('div');
        document.body.append(host);
        const resolved = samples.map(([name, expected]) => {
          const actual = window.MackesDeviceRenderer.rendererFor({name}).key;
          const graphicHost = document.createElement('div');
          window.MackesDeviceRenderer.appendGraphic({name, state: 'unknown'}, graphicHost);
          host.append(graphicHost);
          return {name, expected, actual, svg: Boolean(graphicHost.querySelector('svg.device-graphic')), accessible: graphicHost.querySelectorAll('.device-graphic-accessible-list li').length};
        });
        return {keys: window.MackesDeviceRenderer.keys, resolved};
        """
    )
    expected_keys = {
        "novation.launch-control-xl", "eventide.micropitch", "lexicon.reflex", "pipedal",
        "m-audio.midisport-4x4", "rtp-midi", "generic-midi", "mackes.virtual-monitor", "generic.endpoint",
    }
    assert set(result["keys"]) == expected_keys, result
    assert all(item["actual"] == item["expected"] and item["svg"] and item["accessible"] >= 3 for item in result["resolved"]), result
    print(f"browser-renderer-registry: PASS families={len(result['resolved'])} origin={origin}")
finally:
    driver.quit()
