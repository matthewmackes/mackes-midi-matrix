#!/usr/bin/env python3
"""Validate the canonical web capability ledger."""

from __future__ import annotations

import pathlib
import re
import json
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
ledger = ROOT / "docs" / "web-feature-coverage.md"
text = ledger.read_text(encoding="utf-8")
ipc = (ROOT / "crates" / "ipc" / "src" / "lib.rs").read_text(encoding="utf-8")
command_block = re.search(r"pub enum Command \{(.*?)\n\}", ipc, re.S)
if not command_block:
    raise SystemExit("IPC Command enum was not found")
commands = set(re.findall(r"^    ([A-Z][A-Za-z0-9_]*),", command_block.group(1), re.M))
rows = []
for line in text.splitlines():
    if not line.startswith("| WEB-"):
        continue
    fields = [field.strip() for field in line.strip("|").split("|")]
    if len(fields) != 9:
        raise SystemExit(f"invalid web ledger row: {line}")
    rows.append(fields)

if not rows:
    raise SystemExit("web capability ledger is empty")

ids = [row[0] for row in rows]
if len(ids) != len(set(ids)):
    raise SystemExit("duplicate web capability ID")
covered_commands = set(re.findall(r"Command::([A-Z][A-Za-z0-9_]*)", "\n".join(row[1] for row in rows)))
missing_commands = sorted(commands - covered_commands)
if missing_commands:
    raise SystemExit(f"IPC commands missing from web ledger: {', '.join(missing_commands)}")
schema = json.loads((ROOT / "schemas" / "config.schema.json").read_text(encoding="utf-8"))
schema_properties: set[str] = set()

def collect_properties(value: object) -> None:
    if not isinstance(value, dict):
        return
    properties = value.get("properties")
    if isinstance(properties, dict):
        schema_properties.update(properties)
        for child in properties.values():
            collect_properties(child)
    for child in value.values():
        if isinstance(child, (dict, list)):
            collect_properties(child)

collect_properties(schema)
missing_fields = sorted(
    field
    for field in schema_properties
    if not re.search(rf"(?<![A-Za-z0-9_]){re.escape(field)}(?![A-Za-z0-9_])", text)
)
if missing_fields:
    raise SystemExit(f"schema fields missing from web classification: {', '.join(missing_fields)}")
tui = (ROOT / "crates" / "tui" / "src" / "lib.rs").read_text(encoding="utf-8")
section_block = re.search(r"pub enum AppSection \{(.*?)\n\}", tui, re.S)
if not section_block:
    raise SystemExit("TUI AppSection enum was not found")
sections = set(re.findall(r"^    ([A-Z][A-Za-z0-9_]*),", section_block.group(1), re.M))
section_labels = {
    "Live": "Live",
    "MapControls": "Map Controls",
    "Scenes": "Scenes",
    "Devices": "Devices",
    "System": "System",
}
missing_sections = sorted(
    label for section, label in section_labels.items() if section in sections and label not in text
)
if missing_sections:
    raise SystemExit(f"TUI sections missing from web ledger: {', '.join(missing_sections)}")
ui_block = re.search(r"pub enum UiCommand \{(.*?)\n\}", tui, re.S)
if not ui_block:
    raise SystemExit("TUI UiCommand enum was not found")
ui_commands = set(re.findall(r"^    ([A-Z][A-Za-z0-9_]*)(?:\([^)]*\))?,", ui_block.group(1), re.M))
covered_ui = set(re.findall(r"UiCommand::([A-Z][A-Za-z0-9_]*)", text))
missing_ui = sorted(ui_commands - covered_ui)
if missing_ui:
    raise SystemExit(f"TUI commands missing from web ledger: {', '.join(missing_ui)}")
for enum_name in ("MappingOperation", "PiPedalOperation", "AssignmentAction"):
    block = re.search(rf"pub enum {enum_name} \{{(.*?)\n\}}", ipc, re.S)
    if not block:
        raise SystemExit(f"IPC {enum_name} enum was not found")
    variants = set(re.findall(r"^    ([A-Z][A-Za-z0-9_]*)(?:\([^)]*\))?,", block.group(1), re.M))
    covered = set(re.findall(rf"{enum_name}::([A-Z][A-Za-z0-9_]*)", text))
    missing = sorted(variants - covered)
    if missing:
        raise SystemExit(f"{enum_name} variants missing from web ledger: {', '.join(missing)}")
profiles = (ROOT / "crates" / "profiles" / "src").glob("*.rs")
profile_text = "\n".join(path.read_text(encoding="utf-8") for path in profiles)
capability_literals = set(
    re.findall(r'provided_capabilities:\s*vec!\[(.*?)\]', profile_text, re.S)
)
profile_ids = {
    literal
    for block in capability_literals
    for literal in re.findall(r'"([a-z0-9_-]+)"\.into\(\)', block)
}
missing_profile_capabilities = sorted(
    capability for capability in profile_ids if f"`{capability}`" not in text
)
if missing_profile_capabilities:
    raise SystemExit(
        "profile capabilities missing from web ledger: " + ", ".join(missing_profile_capabilities)
    )
connector = (ROOT / "crates" / "pipedal-connector" / "src" / "lib.rs").read_text(encoding="utf-8")
operation_block = re.search(r"pub enum Operation \{(.*?)\n\}", connector, re.S)
if not operation_block:
    raise SystemExit("PiPedal Operation enum was not found")
operations = set(re.findall(r"^    ([A-Z][A-Za-z0-9_]*),", operation_block.group(1), re.M))
covered_operations = set(re.findall(r"Operation::([A-Z][A-Za-z0-9_]*)", text))
missing_operations = sorted(operations - covered_operations)
if missing_operations:
    raise SystemExit("PiPedal operations missing from web ledger: " + ", ".join(missing_operations))
planned_ids = {f"W{number:03d}" for number in range(117, 144)}
missing_work_items = sorted(item for item in planned_ids if item not in text)
if missing_work_items:
    raise SystemExit(
        "planned web/device work items missing from dependency ledger: "
        + ", ".join(missing_work_items)
    )
for capability_id, source, availability, semantics, owner, api, control, persistence, evidence in rows:
    if not re.fullmatch(r"WEB-[0-9]{3}", capability_id):
        raise SystemExit(f"invalid capability ID: {capability_id}")
    if not source or source == "TBD":
        raise SystemExit(f"{capability_id}: source is required")
    if availability not in {"implemented", "partial", "planned", "unsupported"}:
        raise SystemExit(f"{capability_id}: invalid availability")
    if semantics not in {"read", "write", "read/write", "derived"}:
        raise SystemExit(f"{capability_id}: invalid semantics")
    if not owner or owner == "TBD":
        raise SystemExit(f"{capability_id}: canonical owner is required")
    if not api or not control or not persistence or not evidence:
        raise SystemExit(f"{capability_id}: incomplete coverage fields")

print(f"web coverage checks passed ({len(rows)} capabilities)")
