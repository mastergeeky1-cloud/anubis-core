//! Real-time opcode WebSocket transport for ANUBIS.
//!
//! Alongside the Telegram long-poll surface, ANUBIS exposes a raw WebSocket
//! endpoint that supports full-duplex, streaming conversation. Clients speak a
//! tiny binary protocol: every message is `[1B opcode][4B length][payload]`.
//!
//! This gives a web app / desktop client / game engine the ability to:
//!   • stream LLM text deltas as they generate (not after the fact),
//!   • receive TTS audio frames as they are synthesized (start hearing
//!     before synthesis finishes),
//!   • send continuous opus voice and have it transcribed on the fly,
//!   • get live status ("thinking", "synthesizing") instead of a spinner.
//!
//! The heavy lifting (Noxis LLM, TTS router, clone engine, whisper) is all
//! reused from the shared `AppState` — this transport is just a new front
//! door.

pub mod codec;
pub mod handler;

pub use handler::{run_ws_server, WsServerConfig};

/// Key the underlying axum router is built on.
pub const DEFAULT_PATH: &str = "/ws";
