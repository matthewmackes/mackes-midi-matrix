#!/usr/bin/env python3
"""Validate the stable, bounded web API v1 operation envelope schema."""

from __future__ import annotations

import json
import pathlib
import re

ROOT = pathlib.Path(__file__).resolve().parents[1]
schema = json.loads((ROOT / "schemas" / "web-api-v1.schema.json").read_text(encoding="utf-8"))
contract = (ROOT / "crates" / "web-contract" / "src" / "lib.rs").read_text(encoding="utf-8")


def contract_bound(name: str) -> int:
    match = re.search(rf"pub const {name}: usize = (\d+);", contract)
    if match is None:
        raise SystemExit(f"web contract constant {name} is missing")
    return int(match.group(1))


contract_bounds = {
    "id": contract_bound("MAX_ID_LENGTH"),
    "operation": contract_bound("MAX_OPERATION_LENGTH"),
    "error_code": contract_bound("MAX_ERROR_CODE_LENGTH"),
    "event_kind": contract_bound("MAX_EVENT_KIND_LENGTH"),
    "error": contract_bound("MAX_ERROR_LENGTH"),
}
web_source = (ROOT / "apps" / "mackes-web" / "src" / "main.rs").read_text(encoding="utf-8")
for route in (
    "/api/v1/capabilities", "/api/v1/state", "/api/v1/health", "/api/v1/endpoints",
    "/api/v1/routes", "/api/v1/scenes", "/api/v1/devices", "/api/v1/assignment", "/api/v1/monitor",
    "/api/v1/backups", "/api/v1/configuration", "/api/v1/validation", "/api/v1/operations",
    "/api/v1/mappings", "/api/v1/pipedal", "/api/v1/sysex", "/api/v1/events", "/api/v1/diagnostics", "/api/v1/diagnostics/bundle",
):
    if route not in web_source:
        raise SystemExit(f"web service route is missing: {route}")
if schema.get("$id") != "https://mackes.invalid/schema/web-api-v1.schema.json":
    raise SystemExit("web API schema has an unexpected identity")
if schema.get("type") != "object" or schema.get("additionalProperties") is not False:
    raise SystemExit("web API envelope must be a closed object")
required = set(schema.get("required", []))
if required != {"request_id", "operation", "generation"}:
    raise SystemExit("web API envelope required fields changed")
properties = schema.get("properties", {})
for name, maximum in (("request_id", contract_bounds["id"]), ("operation", contract_bounds["operation"])):
    field = properties.get(name, {})
    if field.get("type") != "string" or field.get("minLength") != 1 or field.get("maxLength") != maximum:
        raise SystemExit(f"web API {name} bounds are missing or changed")
if properties.get("generation", {}).get("type") != "integer":
    raise SystemExit("web API generation must be an integer")
if properties.get("confirm", {}).get("type") != "boolean":
    raise SystemExit("web API confirmation must be boolean")
if properties.get("confirm", {}).get("default") is not False:
    raise SystemExit("web API confirmation must default to false")
definitions = schema.get("$defs", {})
for definition, required in {
    "operation_response": {"operation_id", "generation", "accepted"},
    "assignment_request": {"generation", "action"},
    "backup_request": {"action"},
    "device_control_payload": {"profile_id", "control", "channel", "value", "destination"},
    "error": {"code", "message"},
    "state_event": {"sequence", "generation", "kind", "payload"},
}.items():
    value = definitions.get(definition, {})
    if value.get("type") != "object" or value.get("additionalProperties") is not False:
        raise SystemExit(f"web API {definition} must be a closed object")
    if set(value.get("required", [])) != required:
        raise SystemExit(f"web API {definition} required fields changed")
assignment_actions = definitions["assignment_request"]["properties"]["action"].get("enum", [])
if len(assignment_actions) != 16 or "Snapshot" not in assignment_actions:
    raise SystemExit("web API assignment action catalog is incomplete")
backup = definitions["backup_request"]
backup_actions = backup["properties"]["action"].get("enum", [])
if backup_actions != ["create", "restore", "export", "portable_export", "portable_import"]:
    raise SystemExit("web API backup action catalog is incomplete")
if backup["properties"].get("content", {}).get("maxLength") != 1024 * 1024:
    raise SystemExit("web API portable backup content bound diverges")
if backup["properties"].get("name", {}).get("maxLength") != 256:
    raise SystemExit("web API backup name bound diverges")
control = definitions["device_control_payload"]["properties"]
if control["channel"].get("maximum") != 15 or control["value"].get("maximum") != 65535:
    raise SystemExit("web API device-control bounds diverge")
rules = schema.get("allOf", [])
if not any(
    rule.get("if", {}).get("properties", {}).get("operation", {}).get("const") == "device_control"
    and rule.get("then", {}).get("properties", {}).get("payload", {}).get("$ref")
    == "#/$defs/device_control_payload"
    and rule.get("then", {}).get("properties", {}).get("confirm", {}).get("const") is True
    for rule in rules
):
    raise SystemExit("web API device-control conditional confirmation/payload rule is missing")
response_id = definitions["operation_response"]["properties"].get("operation_id", {})
if definitions["operation_response"]["properties"].get("daemon", {}).get("type") != ["object", "null"]:
    raise SystemExit("web API operation response must expose the daemon result object")
if response_id.get("maxLength") != contract_bounds["id"]:
    raise SystemExit("web API operation_id bound diverges from Rust contract")
error_properties = definitions["error"]["properties"]
if error_properties.get("code", {}).get("maxLength") != contract_bounds["error_code"]:
    raise SystemExit("web API error code bound diverges from Rust contract")
if error_properties.get("message", {}).get("maxLength") != contract_bounds["error"]:
    raise SystemExit("web API error message bound diverges from Rust contract")
if error_properties.get("field", {}).get("maxLength") != contract_bounds["id"]:
    raise SystemExit("web API error field bound diverges from Rust contract")
if definitions["state_event"]["properties"].get("kind", {}).get("maxLength") != contract_bounds["event_kind"]:
    raise SystemExit("web API event kind bound diverges from Rust contract")
print("web API schema checks passed")
