#![allow(clippy::derive_partial_eq_without_eq)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::option_if_let_else)]
#![allow(clippy::match_same_arms)]
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

/// Maximum encoded PiPedal control frame accepted by the connector.
pub const MAX_FRAME_BYTES: usize = 1_048_576;

/// Bounded accumulator for fragmented WebSocket text messages.
#[derive(Debug, Default)]
pub struct TextAssembler {
    pending: Vec<u8>,
}

/// A decoded server-to-client WebSocket frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerFrame {
    /// FIN bit indicating the final fragment.
    pub final_fragment: bool,
    /// WebSocket opcode (1=text, 0=continuation, 8=close, 9=ping, 10=pong).
    pub opcode: u8,
    /// Unmasked payload bytes.
    pub payload: Vec<u8>,
}

/// Encode a masked client text frame for PiPedal.
pub fn encode_client_text(payload: &[u8], mask: [u8; 4]) -> Result<Vec<u8>, String> {
    if payload.len() > MAX_FRAME_BYTES {
        return Err("PiPedal client payload exceeds configured limit".into());
    }
    let mut output = Vec::with_capacity(payload.len() + 14);
    output.push(0x81);
    match payload.len() {
        0..=125 => output.push(
            0x80 | u8::try_from(payload.len()).map_err(|_| "invalid short payload".to_string())?,
        ),
        126..=65_535 => {
            output.push(0xFE);
            output.extend_from_slice(
                &u16::try_from(payload.len())
                    .map_err(|_| "invalid extended payload".to_string())?
                    .to_be_bytes(),
            );
        }
        _ => {
            output.push(0xFF);
            output.extend_from_slice(&(payload.len() as u64).to_be_bytes());
        }
    }
    output.extend_from_slice(&mask);
    output.extend(payload.iter().enumerate().map(|(i, byte)| byte ^ mask[i % 4]));
    Ok(output)
}

/// Decode one complete unmasked server WebSocket frame.
pub fn decode_server_frame(input: &[u8]) -> Result<ServerFrame, String> {
    if input.len() < 2 {
        return Err("PiPedal frame header is truncated".into());
    }
    let final_fragment = input[0] & 0x80 != 0;
    let opcode = input[0] & 0x0f;
    if input[1] & 0x80 != 0 {
        return Err("server WebSocket frame must not be masked".into());
    }
    let (length, offset) = match input[1] & 0x7f {
        n @ 0..=125 => (n as usize, 2),
        126 if input.len() >= 4 => (u16::from_be_bytes([input[2], input[3]]) as usize, 4),
        127 if input.len() >= 10 => {
            let mut bytes = [0_u8; 8];
            bytes.copy_from_slice(&input[2..10]);
            let length = usize::try_from(u64::from_be_bytes(bytes))
                .map_err(|_| "PiPedal frame length overflows platform size".to_string())?;
            (length, 10)
        }
        _ => return Err("PiPedal frame length is truncated".into()),
    };
    if length > MAX_FRAME_BYTES || input.len() != offset + length {
        return Err("PiPedal frame length is invalid or exceeds limit".into());
    }
    Ok(ServerFrame { final_fragment, opcode, payload: input[offset..].to_vec() })
}

impl TextAssembler {
    /// Add one WebSocket payload fragment, returning a complete message at `final_fragment`.
    pub fn push(
        &mut self,
        fragment: &[u8],
        final_fragment: bool,
    ) -> Result<Option<Vec<u8>>, String> {
        if self.pending.len().saturating_add(fragment.len()) > MAX_FRAME_BYTES {
            self.pending.clear();
            return Err("PiPedal fragmented message exceeds configured limit".into());
        }
        self.pending.extend_from_slice(fragment);
        if final_fragment {
            Ok(Some(std::mem::take(&mut self.pending)))
        } else {
            Ok(None)
        }
    }
}

/// Header returned by PiPedal for replies and asynchronous events.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MessageHeader {
    /// Message or event name.
    pub message: String,
    /// Correlation identifier for a request response.
    #[serde(rename = "replyTo")]
    pub reply_to: Option<u64>,
}

/// PiPedal error reply body.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ErrorBody {
    /// Human-readable server error.
    pub message: String,
}

/// Ordered phases of a PiPedal control session.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionPhase {
    /// No WebSocket handshake has completed.
    Disconnected,
    /// WebSocket is open and `hello` is next.
    Connected,
    /// Client identity was accepted; `version` is next.
    Identified,
    /// Server version was read; catalog loading is in progress.
    LoadingCatalog,
    /// Required startup catalog/state has been loaded.
    Ready,
}

impl SessionPhase {
    /// Advance the session after a successful response.
    pub fn accept(self, message: &str) -> Result<Self, String> {
        match (self, message) {
            (Self::Connected, "ehlo") => Ok(Self::Identified),
            (Self::Identified, "version") => Ok(Self::LoadingCatalog),
            (Self::LoadingCatalog, "getSystemMidiBindings") => Ok(Self::Ready),
            (
                Self::LoadingCatalog,
                "plugins" | "currentPedalboard" | "pluginClasses" | "getPresets" | "getBankIndex"
                | "getFavorites" | "imageList",
            ) => Ok(Self::LoadingCatalog),
            (Self::Ready, _) => Ok(Self::Ready),
            (phase, message) => {
                Err(format!("unexpected PiPedal message {message} during {phase:?}"))
            }
        }
    }
}

/// Decode a bounded PiPedal array-framed message.
pub fn decode_message(input: &[u8]) -> Result<(MessageHeader, Option<serde_json::Value>), String> {
    if input.len() > MAX_FRAME_BYTES {
        return Err("PiPedal frame exceeds configured limit".into());
    }
    let value: serde_json::Value = serde_json::from_slice(input).map_err(|e| e.to_string())?;
    let array = value.as_array().ok_or_else(|| "PiPedal frame is not an array".to_string())?;
    if !(1..=2).contains(&array.len()) {
        return Err("PiPedal frame must contain one header and at most one body".into());
    }
    let header: MessageHeader =
        serde_json::from_value(array[0].clone()).map_err(|e| e.to_string())?;
    Ok((header, array.get(1).cloned()))
}

/// Decode a message body into a caller-selected typed value.
pub fn decode_body<T: for<'de> Deserialize<'de>>(
    body: Option<serde_json::Value>,
) -> Result<T, String> {
    body.ok_or_else(|| "PiPedal message has no body".to_string())
        .and_then(|value| serde_json::from_value(value).map_err(|e| e.to_string()))
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

    #[test]
    fn decoder_rejects_invalid_shape_and_accepts_event_body() {
        assert!(decode_message(br"{}").is_err());
        assert!(decode_message(br#"[{"message":"x"},{},{}]"#).is_err());
        let (header, body) =
            decode_message(br#"[{"message":"onPedalboardChanged"},{"generation":3}]"#)
                .expect("decode");
        assert_eq!(header.message, "onPedalboardChanged");
        assert_eq!(body.expect("body")["generation"], 3);
    }

    #[test]
    fn decoder_rejects_oversized_frames() {
        let input = vec![b' '; MAX_FRAME_BYTES + 1];
        assert!(decode_message(&input).is_err());
    }

    #[test]
    fn typed_body_decode_reports_missing_body_and_errors() {
        assert!(decode_body::<ErrorBody>(None).is_err());
        let body = serde_json::json!({"message":"invalid control"});
        assert_eq!(decode_body::<ErrorBody>(Some(body)).expect("body").message, "invalid control");
    }

    #[test]
    fn session_requires_hello_and_version_before_catalog_ready() {
        assert_eq!(SessionPhase::Connected.accept("ehlo").expect("ehlo"), SessionPhase::Identified);
        assert!(SessionPhase::Connected.accept("hello").is_err());
        assert!(SessionPhase::Connected.accept("getSystemMidiBindings").is_err());
        let phase = SessionPhase::Identified.accept("version").expect("version");
        let phase = phase.accept("plugins").expect("plugins");
        assert_eq!(phase.accept("getSystemMidiBindings").expect("bindings"), SessionPhase::Ready);
    }

    #[test]
    fn text_assembler_reassembles_fragments_and_bounds_growth() {
        let mut assembler = TextAssembler::default();
        assert_eq!(assembler.push(br"[{", false).expect("fragment"), None);
        assert_eq!(assembler.push(br"}]", true).expect("complete"), Some(b"[{}]".to_vec()));
        let mut assembler = TextAssembler::default();
        assert!(assembler.push(&vec![b'x'; MAX_FRAME_BYTES], false).is_ok());
        assert!(assembler.push(b"x", true).is_err());
    }

    #[test]
    fn server_frame_decoder_validates_header_and_payload() {
        let frame = decode_server_frame(&[0x81, 3, b'o', b'k', b'!']).expect("frame");
        assert_eq!(
            frame,
            ServerFrame { final_fragment: true, opcode: 1, payload: b"ok!".to_vec() }
        );
        assert!(decode_server_frame(&[0x81, 0x80]).is_err());
        assert!(decode_server_frame(&[0x81, 4, b'o']).is_err());
    }

    #[test]
    fn client_encoder_masks_text_and_supports_extended_lengths() {
        let frame = encode_client_text(b"ok", [1, 2, 3, 4]).expect("frame");
        assert_eq!(&frame[..6], &[0x81, 0x82, 1, 2, 3, 4]);
        assert_eq!(&frame[6..], &[110, 105]);
        assert!(encode_client_text(&vec![0; MAX_FRAME_BYTES + 1], [0; 4]).is_err());
    }
}
