//! Compact LED status decoded from the daemon snapshot.

/// Formats the daemon `led` object into one operator-visible status line.
#[must_use]
pub fn line_from_payload(payload: &serde_json::Value) -> Option<String> {
    let led = payload.get("led")?;
    let attempted = led.get("attempted").and_then(serde_json::Value::as_u64).unwrap_or(0);
    let sent = led.get("sent").and_then(serde_json::Value::as_u64).unwrap_or(0);
    let coalesced = led.get("coalesced").and_then(serde_json::Value::as_u64).unwrap_or(0);
    let failed = led.get("failed").and_then(serde_json::Value::as_u64).unwrap_or(0);
    let template = led.get("template").and_then(serde_json::Value::as_u64).unwrap_or(1);
    let target = led.get("target_id").and_then(serde_json::Value::as_str).unwrap_or("none");
    let error = led.get("last_error").and_then(serde_json::Value::as_str).unwrap_or("none");
    Some(format!(
        "led attempted={attempted} sent={sent} coalesced={coalesced} failed={failed} template={template} target={target} error={error}"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn led_status_line_exposes_zero_send_failures() {
        let payload = serde_json::json!({
            "led": {
                "attempted": 1,
                "sent": 0,
                "coalesced": 0,
                "failed": 1,
                "last_error": "no unique Launch Control XL MIDI output",
                "target_id": null,
                "template": 1
            }
        });
        let line = line_from_payload(&payload).expect("led");
        assert!(line.contains("sent=0"));
        assert!(line.contains("failed=1"));
        assert!(line.contains("no unique Launch Control XL MIDI output"));
    }
}
