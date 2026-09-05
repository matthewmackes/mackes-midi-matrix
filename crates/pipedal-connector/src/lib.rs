#![allow(clippy::derive_partial_eq_without_eq)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::option_if_let_else)]
#![allow(missing_docs)]

//! Typed, transport-independent `PiPedal` WebSocket protocol contracts.
//!
//! This crate deliberately does not open sockets. The daemon transport can use these
//! bounded message types without placing network I/O on the MIDI dispatch path.

use serde::{Deserialize, Serialize};

/// `PiPedal`'s array-framed WebSocket request envelope.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Request<T> {
    /// Message name.
    pub message: String,
    /// Correlation identifier, when a response is expected.
    #[serde(rename = "replyTo", skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<u64>,
    /// Optional request body.
    pub body: Option<T>,
}

/// Body accepted by `PiPedal`'s `setControl` operation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SetControl {
    /// Client/session identifier.
    #[serde(rename = "clientId")]
    pub client_id: String,
    /// Runtime plugin instance identifier.
    #[serde(rename = "instanceId")]
    pub instance_id: u64,
    /// Plugin control symbol.
    pub symbol: String,
    /// Numeric control value.
    pub value: f32,
}

/// `PiPedal` system or plugin MIDI binding metadata.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MidiBinding {
    pub symbol: String,
    pub channel: i32,
    pub binding_type: i32,
    pub note: i32,
    pub control: i32,
    pub min_control_value: i32,
    pub max_control_value: i32,
    pub min_value: f32,
    pub max_value: f32,
    pub rotary_scale: f32,
    pub linear_control_type: i32,
    pub switch_control_type: i32,
}

/// Encode a `PiPedal` request as its documented two-element JSON array.
pub fn encode_request<T: Serialize>(request: &Request<T>) -> serde_json::Result<Vec<u8>> {
    let header = serde_json::json!({
        "message": request.message,
        "replyTo": request.reply_to,
    });
    match &request.body {
        Some(body) => serde_json::to_vec(&serde_json::json!([header, body])),
        None => serde_json::to_vec(&serde_json::json!([header])),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_control_uses_pipedal_wire_names() {
        let request = Request {
            message: "setControl".into(),
            reply_to: Some(7),
            body: Some(SetControl {
                client_id: "mackes".into(),
                instance_id: 127,
                symbol: "lfLevel".into(),
                value: -3.5,
            }),
        };
        let value: serde_json::Value =
            serde_json::from_slice(&encode_request(&request).expect("encode")).expect("json");
        assert_eq!(value[0]["message"], "setControl");
        assert_eq!(value[0]["replyTo"], 7);
        assert_eq!(value[1]["instanceId"], 127);
        assert_eq!(value[1]["symbol"], "lfLevel");
    }

    #[test]
    fn midi_binding_round_trips_all_fields() {
        let binding = MidiBinding {
            symbol: "lfLevel".into(),
            channel: 0,
            binding_type: 1,
            note: 0,
            control: 74,
            min_control_value: 0,
            max_control_value: 127,
            min_value: -12.0,
            max_value: 12.0,
            rotary_scale: 1.0,
            linear_control_type: 0,
            switch_control_type: 0,
        };
        let encoded = serde_json::to_vec(&binding).expect("encode");
        assert_eq!(serde_json::from_slice::<MidiBinding>(&encoded).expect("decode"), binding);
    }
}
