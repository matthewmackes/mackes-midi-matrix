#!/usr/bin/env python3
"""Remove editor metadata from approved SVG art and reject executable/external content."""
import re
import sys
import xml.etree.ElementTree as ET
from pathlib import Path


for name in sys.argv[1:]:
    path = Path(name)
    text = path.read_text(encoding="utf-8")
    lowered = text.lower()
    for forbidden in ("<script", " onload=", " onclick=", "javascript:", "<iframe", "<foreignobject"):
        if forbidden in lowered:
            raise SystemExit(f"unsafe SVG content {forbidden!r}: {path}")
    if re.search(r'(?:href|xlink:href)=["\'](?:https?:|//)', text, re.I):
        raise SystemExit(f"external SVG reference: {path}")
    root = ET.fromstring(text)
    for child in list(root):
        if child.tag.rsplit("}", 1)[-1].lower() in {"metadata", "namedview"}:
            root.remove(child)
    ET.register_namespace("", "http://www.w3.org/2000/svg")
    ET.ElementTree(root).write(path, encoding="utf-8", xml_declaration=True, short_empty_elements=True)
    print(f"sanitized {path}")
