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
    let phase = led.get("phase").and_then(serde_json::Value::as_str).unwrap_or("unknown");
    let desired = led.get("desired_indices").and_then(serde_json::Value::as_u64).unwrap_or(0);
    let pending = led.get("pending_indices").and_then(serde_json::Value::as_u64).unwrap_or(0);
    Some(format!(
        "led phase={phase} desired={desired} pending={pending} attempted={attempted} sent={sent} coalesced={coalesced} failed={failed} template={template} target={target} error={error}"
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
        assert!(line.contains("phase=unknown"));
    }

    #[test]
    fn led_status_line_exposes_authoritative_surface_progress() {
        let payload = serde_json::json!({
            "led": {
                "phase": "initializing",
                "desired_indices": 48,
                "pending_indices": 23,
                "attempted": 2,
                "sent": 1,
                "coalesced": 4,
                "failed": 1,
                "template": 8,
                "target_id": "launch-control-xl-1",
                "last_error": "transport timeout"
            }
        });
        let line = line_from_payload(&payload).expect("led");
        assert!(line.contains("phase=initializing"));
        assert!(line.contains("desired=48"));
        assert!(line.contains("pending=23"));
        assert!(line.contains("template=8"));
        assert!(line.contains("target=launch-control-xl-1"));
    }
}
