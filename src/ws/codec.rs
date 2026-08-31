//! Opcode namespace + compact binary frame codec for the ANUBIS WS protocol.

use bytes::{BufMut, BytesMut};

/// Protocol version negotiated at connect time (part of the first frame we
/// expect, see `OpCode::Hello`). Bump on any breaking change to the frame
/// layout or opcode namespace.
pub const PROTOCOL_VERSION: u8 = 1;

/// Opcodes a **client → server**.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum OpCode {
    /// Handshake: 1 byte version + auth token (SSE-style newline optional).
    Hello = 0x01,
    /// UTF-8 text prompt for the brain.
    Text = 0x02,
    /// A chunk of audio to transcribe (OPUS currently; whisper expects WAV,
    /// so chunks are batched and decoded by the server).
    Voice = 0x03,
    /// Update runtime settings: `{ "voice": "...", "lang": "en" }`.
    Config = 0x04,
    /// Client heartbeat — server replies with `Pong`.
    Ping = 0x05,
    /// Request conversation history for the session.
    History = 0x06,
}

/// Opcodes **server → client**.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum OpReply {
    /// ACK for a completed handshake; carries protocol version + session id.
    Hello = 0x81,
    /// One incremental text delta of the LLM answer.
    TextDelta = 0x82,
    /// One chunk of synthesized voice (OPUS) ready to play.
    VoiceChunk = 0x83,
    /// State transition notification ("thinking", "synthesizing", "done").
    Status = 0x84,
    /// Structured error (code + message).
    Error = 0x85,
    /// Metadata update (credits, latency, active voice, etc).
    Meta = 0x86,
    /// Reply to a client `Ping`; echoes the client timestamp for RTT.
    Pong = 0x87,
    /// Full conversation history (JSON array of turns).
    History = 0x88,
    /// Final marker for a completed text reply (client can finalise UI).
    TextEnd = 0x89,
}

impl OpCode {
    /// Parse a raw opcode byte against a known set.
    pub fn from_byte(b: u8) -> Option<OpCode> {
        use OpCode::*;
        match b {
            0x01 => Some(Hello),
            0x02 => Some(Text),
            0x03 => Some(Voice),
            0x04 => Some(Config),
            0x05 => Some(Ping),
            0x06 => Some(History),
            _ => None,
        }
    }
}

/// The on-wire frame: `[opcode:u8][length:u32 BE][payload]`.
///
/// Payload length is capped at 4 MiB to keep malformed clients from forcing
/// unbounded allocations. The codec owns a reusable buffer so hot paths don't
/// reallocate.
pub struct WsFrame {
    pub op: u8,
    pub payload: BytesMut,
}

impl WsFrame {
    /// Encode a response frame. Returns `None` if the payload exceeds the
    /// frame-size cap (caller should fall back to chunking).
    pub fn encode(op: OpReply, payload: &[u8]) -> Option<BytesMut> {
        if payload.len() > MAX_PAYLOAD {
            return None;
        }
        let mut out = BytesMut::with_capacity(5 + payload.len());
        out.put_u8(op as u8);
        out.put_u32(payload.len() as u32);
        out.put_slice(payload);
        Some(out)
    }

    /// Decode a single frame from a byte slice. Returns the frame plus the
    /// number of bytes consumed, allowing pipelined buffers.
    pub fn decode(buf: &[u8]) -> Option<(WsFrame, usize)> {
        if buf.len() < 5 {
            return None; // need the header first
        }
        let op = buf[0];
        let len = u32::from_be_bytes([buf[1], buf[2], buf[3], buf[4]]) as usize;
        if len > MAX_PAYLOAD {
            return None;
        }
        let total = 5 + len;
        if buf.len() < total {
            return None; // payload not fully buffered yet
        }
        let payload = BytesMut::from(&buf[5..total]);
        Some((WsFrame { op, payload }, total))
    }
}

/// Max on-wire payload (4 MiB) — larger data must be chunked by the sender.
pub const MAX_PAYLOAD: usize = 4 * 1024 * 1024;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_roundtrips() {
        let payload = b"hello world";
        let enc = WsFrame::encode(OpReply::TextDelta, payload).unwrap();
        let (frame, consumed) = WsFrame::decode(&enc).unwrap();
        assert_eq!(frame.op, OpReply::TextDelta as u8);
        assert_eq!(consumed, 5 + payload.len());
        assert_eq!(frame.payload.as_ref(), payload);

        // Client opcodes also round-trip through decode.
        let text = WsFrame::encode(OpReply::Status, b"thinking").unwrap();
        let (f2, c2) = WsFrame::decode(&text).unwrap();
        assert_eq!(f2.op, OpReply::Status as u8);
        assert_eq!(c2, text.len());
    }

    #[test]
    fn frame_rejects_jumbo_length() {
        // A forged 8-byte header claiming a huge payload must be rejected.
        let mut buf = BytesMut::new();
        buf.put_u8(0x02);
        buf.put_u32(u32::MAX); // more than the 4 MiB cap
        buf.put_slice(b"x");
        assert!(WsFrame::decode(&buf).is_none());
    }

    #[test]
    fn frame_decodes_partial_and_full() {
        let full = WsFrame::encode(OpReply::Pong, b"pong").unwrap();
        // Feeding it header+0 bytes: not decodable yet.
        assert!(WsFrame::decode(&full[..5]).is_none());
        // Full frame decodes.
        let (_, consumed) = WsFrame::decode(&full).unwrap();
        assert_eq!(consumed, full.len());
    }
}
