//! WebSocket server + opcode dispatch for the real-time transport.
//!
//! Runs an axum server alongside the Telegram bot. Clients connect to
//! `ws://host:PORT/ws`, handshake with `OpCode::Hello`, then stream
//! `Text`/`Voice` frames and receive `TextDelta`/`VoiceChunk`/`Status`
//! frames back. The shared `AppState` (Noxis, TTS, clone, whisper, memory,
//! DB, cache) is reused wholesale — this is purely a transport layer.
//!
//! A single task owns the socket. Every other async flow sends frames over a
//! bounded in-memory channel (`mpsc`), so the LLM stream relay and the socket
//! writer never contend for the same `&mut WebSocket`.

use crate::bot::AppState;
use crate::ws::codec::{OpCode, OpReply, WsFrame, PROTOCOL_VERSION};
use axum::extract::{
    ws::{Message, WebSocket, WebSocketUpgrade},
    State,
};
use bytes::BytesMut;
use futures::{SinkExt, StreamExt};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

/// Server settings taken from the environment / config.
#[derive(Debug, Clone)]
pub struct WsServerConfig {
    /// Bind address, e.g. `127.0.0.1:7600`. Defaults to 127.0.0.1:7600.
    pub bind: SocketAddr,
    /// Optional shared bearer token clients must present in `Hello`.
    pub auth_token: Option<Arc<str>>,
}

impl Default for WsServerConfig {
    fn default() -> Self {
        Self {
            bind: "127.0.0.1:7600".parse().expect("valid default addr"),
            auth_token: None,
        }
    }
}

/// Shared state visible to every WS handler invocation.
struct WsShared {
    state: AppState,
    cfg: Arc<WsServerConfig>,
}

type WsAppState = Arc<WsShared>;

/// Build the axum app that serves the WS endpoint.
pub fn router(state: AppState, cfg: WsServerConfig) -> axum::Router {
    let shared = Arc::new(WsShared {
        state,
        cfg: Arc::new(cfg),
    });
    axum::Router::new()
        .route(crate::ws::DEFAULT_PATH, axum::routing::get(ws_upgrade))
        .route("/metrics", axum::routing::get(metrics_scrape))
        .with_state(shared)
}

/// Prometheus-style metrics scrape endpoint.
async fn metrics_scrape(State(shared): State<WsAppState>) -> axum::response::Response {
    let body = shared.state.metrics.render();
    axum::response::Response::builder()
        .header("content-type", "text/plain; version=0.0.4")
        .body(axum::body::Body::from(body))
        .unwrap()
}

/// Serve forever; blocks the calling task until the listener errors.
pub async fn run_ws_server(state: AppState, cfg: WsServerConfig) -> anyhow::Result<()> {
    let app = router(state, cfg.clone());
    let listener = tokio::net::TcpListener::bind(cfg.bind).await?;
    info!("ANUBIS WS server listening on ws://{}", cfg.bind);
    axum::serve(listener, app).await?;
    Ok(())
}

/// Axum handler: upgrade HTTP → WebSocket, then hand off to `handle_ws`.
async fn ws_upgrade(
    ws: WebSocketUpgrade,
    State(shared): State<WsAppState>,
) -> axum::response::Response {
    ws.on_upgrade(move |socket| handle_ws(socket, shared))
}

/// Thin outbound handle: frames are enqueued and flushed by one writer task.
#[derive(Clone)]
struct Outbound {
    tx: mpsc::UnboundedSender<Message>,
}

impl Outbound {
    fn send(&self, op: OpReply, payload: &[u8]) {
        if let Some(bytes) = WsFrame::encode(op, payload) {
            let _ = self.tx.send(Message::Binary(bytes.freeze().into()));
        }
    }

    fn send_json(&self, op: OpReply, value: &serde_json::Value) {
        self.send(op, value.to_string().as_bytes());
    }

    fn error(&self, code: u16, msg: &str) {
        let v = serde_json::json!({ "code": code, "message": msg });
        self.send_json(OpReply::Error, &v);
    }

    fn status(&self, status: &str) {
        let v = serde_json::json!({ "status": status });
        self.send_json(OpReply::Status, &v);
    }
}

/// Per-connection session state.
struct Session {
    authed: bool,
    lang: String,
    voice_id: String,
}

impl Session {
    fn new() -> Self {
        Self {
            authed: false,
            lang: "en".into(),
            voice_id: crate::tts::voices::default_for_lang("en").to_string(),
        }
    }
}

// ─── Core event loop ─────────────────────────────────────────────────────────

async fn handle_ws(ws: WebSocket, shared: WsAppState) {
    shared.state.metrics.inc("ws_connections_total");
    // Split the socket: one task owns the writer, this task owns the reader.
    let (mut writer, mut reader) = ws.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();
    let out = Outbound { tx };

    // Writer task: drain the outbound channel into the socket.
    let writer_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if writer.send(msg).await.is_err() {
                break;
            }
        }
        let _ = writer.close().await;
        // Dropping `writer` closes the socket.
        drop(writer);
    });

    let mut buf = BytesMut::with_capacity(8192);
    let mut session = Session::new();

    // Greet the client so it knows the protocol version immediately.
    let hello_ack = serde_json::json!({
        "version": PROTOCOL_VERSION,
        "server": "anubis-core",
    });
    out.send_json(OpReply::Hello, &hello_ack);

    while let Some(msg) = reader.next().await {
        let msg = match msg {
            Ok(Message::Binary(b)) => b,
            Ok(Message::Text(t)) => t.into_bytes(),
            Ok(Message::Close(_)) => break,
            _ => continue,
        };

        buf.extend_from_slice(&msg);

        while let Some((frame, consumed)) = WsFrame::decode(&buf) {
            let _ = buf.split_to(consumed);
            process_frame(&out, &mut session, &shared, frame).await;
        }
    }

    // Connection closed — stop the writer task.
    drop(out);
    let _ = writer_task.await;
}

async fn process_frame(out: &Outbound, session: &mut Session, shared: &WsShared, frame: WsFrame) {
    let op = match OpCode::from_byte(frame.op) {
        Some(op) => op,
        None => {
            out.error(400, "unknown opcode");
            return;
        }
    };

    match op {
        OpCode::Hello => handle_hello(out, session, shared, &frame.payload),
        OpCode::Text => handle_text(out, session, shared, &frame.payload).await,
        OpCode::Voice => handle_voice(out, shared, &frame.payload).await,
        OpCode::Config => handle_config(out, session, &frame.payload),
        OpCode::Ping => handle_ping(out),
        OpCode::History => handle_history(out, session, shared),
    }
}

// ─── Opcode handlers ─────────────────────────────────────────────────────────

fn handle_hello(out: &Outbound, session: &mut Session, shared: &WsShared, payload: &[u8]) {
    let v: serde_json::Value = serde_json::from_slice(payload).unwrap_or(serde_json::Value::Null);

    if let Some(ref required) = shared.cfg.auth_token {
        let provided = v.get("token").and_then(|t| t.as_str()).unwrap_or("");
        if &**required != provided {
            out.error(401, "invalid token");
            return;
        }
    }

    session.authed = true;
    if let Some(lang) = v.get("lang").and_then(|l| l.as_str()) {
        session.lang = lang.to_string();
    }
    if let Some(voice) = v.get("voice").and_then(|v| v.as_str()) {
        session.voice_id = voice.to_string();
    }

    let session_id = uuid::Uuid::new_v4().to_string();
    let ack = serde_json::json!({
        "version": PROTOCOL_VERSION,
        "session_id": session_id,
        "voices": crate::tts::voices::VOICES.iter().map(|v| v.id).collect::<Vec<_>>(),
        "languages": crate::i18n::LANGUAGES.iter().map(|l| l.code).collect::<Vec<_>>(),
    });
    out.send_json(OpReply::Hello, &ack);
}

async fn handle_text(out: &Outbound, session: &mut Session, shared: &WsShared, payload: &[u8]) {
    if !session.authed {
        out.error(403, "send Hello first");
        return;
    }

    let text = match std::str::from_utf8(payload) {
        Ok(t) => t.trim().to_string(),
        Err(_) => {
            out.error(400, "invalid UTF-8");
            return;
        }
    };
    if text.is_empty() {
        out.error(400, "empty text");
        return;
    }

    out.status("thinking");

    // Transient user id: 0 for anonymous WS clients. Persisting is opt-in.
    let user_id = 0i64;
    let state = &shared.state;
    state.db.audit(user_id, "ws_ask", &text);

    let history = state.memory.history(user_id);

    // Stream the LLM reply: the sync callback enqueues TextDelta frames on the
    // unbounded channel — the socket writer task flushes them in real time.
    let out_delta = out.clone();
    let reply = match state
        .noxis
        .ask_stream(&text, &session.lang, &history, |delta| {
            if let Some(bytes) = WsFrame::encode(OpReply::TextDelta, delta.as_bytes()) {
                let _ = out_delta.tx.send(Message::Binary(bytes.freeze().into()));
            }
        })
        .await
    {
        Ok(r) => r,
        Err(e) => {
            error!("ws noxis error: {e}");
            out_delta.error(502, &format!("LLM error: {e}"));
            return;
        }
    };

    state.memory.push(user_id, &text, &reply);
    state.last_reply.insert(user_id, reply.clone());
    out.send_json(OpReply::TextEnd, &serde_json::json!({ "len": reply.len() }));

    // Optional: synthesize the reply as speech and stream opus chunks.
    if crate::tts::voices::find(&session.voice_id).is_some() {
        out.status("synthesizing");
        match state
            .worker_pool
            .synthesize_tts(&reply, &session.voice_id)
            .await
        {
            Ok(wav_path) => match state.worker_pool.convert_wav_to_ogg(&wav_path).await {
                Ok(ogg) => {
                    for chunk in ogg.chunks(32 * 1024) {
                        out.send(OpReply::VoiceChunk, chunk);
                    }
                    out.send_json(OpReply::Status, &serde_json::json!({ "status": "done" }));
                }
                Err(e) => warn!("ws wav->ogg failed: {e}"),
            },
            Err(e) => warn!("ws tts failed: {e}"),
        }
    }
}

async fn handle_voice(out: &Outbound, shared: &WsShared, payload: &[u8]) {
    let state = &shared.state;

    if !state.whisper.enabled {
        out.error(501, "whisper not configured");
        return;
    }

    let tmp = state.audio.tmp_path("wav");
    if let Err(e) = tokio::fs::write(&tmp, payload).await {
        out.error(500, &format!("file write error: {e}"));
        return;
    }

    out.status("transcribing");

    match state.whisper.transcribe(&tmp, None).await {
        Ok(text) => {
            tokio::fs::remove_file(&tmp).await.ok();
            out.send(OpReply::TextDelta, text.as_bytes());
        }
        Err(e) => {
            tokio::fs::remove_file(&tmp).await.ok();
            out.error(500, &format!("whisper error: {e}"));
        }
    }
}

fn handle_config(out: &Outbound, session: &mut Session, payload: &[u8]) {
    let v: serde_json::Value = match serde_json::from_slice(payload) {
        Ok(v) => v,
        Err(_) => {
            out.error(400, "invalid JSON");
            return;
        }
    };
    if let Some(lang) = v.get("lang").and_then(|l| l.as_str()) {
        session.lang = lang.to_string();
    }
    if let Some(voice) = v.get("voice").and_then(|v| v.as_str()) {
        session.voice_id = voice.to_string();
    }
    out.send_json(
        OpReply::Meta,
        &serde_json::json!({ "voice": session.voice_id, "lang": session.lang }),
    );
}

fn handle_ping(out: &Outbound) {
    out.send_json(
        OpReply::Pong,
        &serde_json::json!({ "ts": chrono::Utc::now().to_rfc3339() }),
    );
}

fn handle_history(out: &Outbound, _session: &mut Session, shared: &WsShared) {
    let history = shared.state.memory.history(0); // anonymous user
    let turns: Vec<serde_json::Value> = history
        .iter()
        .map(|(role, content)| serde_json::json!({ "role": role, "content": content }))
        .collect();
    out.send_json(OpReply::History, &serde_json::json!({ "turns": turns }));
}
