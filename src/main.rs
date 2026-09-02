mod audio;
mod bot;
mod config;
mod db;
mod error;
mod i18n;
mod memory;
mod metrics;
mod noxis;
mod tts;
mod whisper;
mod worker_pool;
mod ws;

use anyhow::Result;
use std::sync::Arc;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env().add_directive("anubis=info".parse()?),
        )
        .init();

    let cfg = Arc::new(config::Config::load()?);
    info!(
        "ANUBIS Voice Teacher starting — db={} (local-first, open source)",
        cfg.database.path
    );

    let database = db::Database::open(&cfg.database.path, cfg.database.pool_max)?;
    let database = Arc::new(database);

    // TTS router: Piper (CPU) + Kokoro (local sidecar, CPU). Both permissive-licensed.
    let piper = tts::PiperTts::new(cfg.tts.piper_binary.clone(), cfg.tts.voices_dir.clone());
    let mut engines: Vec<Box<dyn tts::engine::TtsEngine>> = vec![Box::new(piper)];
    if !cfg.tts.kokoro_url.trim().is_empty() {
        engines.push(Box::new(tts::kokoro::KokoroTts::new(
            cfg.tts.kokoro_url.clone(),
        )));
    }
    let tts_router = Arc::new(tts::router::TtsRouter::new(engines));
    info!(
        "TTS engines loaded: piper{}",
        if cfg.tts.kokoro_url.trim().is_empty() {
            ""
        } else {
            " + kokoro"
        }
    );

    // Noxis Core — local LLM teacher brain (llama.cpp / ollama compatible).
    let noxis = Arc::new(noxis::NoxisCore::new(cfg.llm.clone()));
    info!(
        "Noxis Core LLM: {}",
        if noxis.enabled() {
            "enabled"
        } else {
            "disabled (set ANUBIS_LLM_URL)"
        }
    );

    let audio = Arc::new(audio::AudioProcessor::new(&cfg.audio.output_dir)?);

    // Whisper local transcription sidecar (voice-to-voice). Optional.
    let whisper = Arc::new(whisper::WhisperClient::new(
        cfg.whisper.url.clone(),
        cfg.whisper.enabled,
    ));
    info!("Whisper voice input: enabled={}", cfg.whisper.enabled);

    // Per-user conversation memory for Noxis Core (persisted in SQLite).
    let memory = Arc::new(memory::ConversationStore::new(database.clone(), 12));

    let worker_pool = Arc::new(worker_pool::WorkerPool::new(
        tts_router,
        audio.clone(),
        worker_pool::WorkerPoolConfig {
            max_synth: cfg.limits.max_concurrent_synth,
            max_convert: 2,
        },
    ));
    let metrics = metrics::Metrics::new();

    let state = bot::AppState {
        db: database,
        noxis,
        audio,
        config: cfg,
        pending: Arc::new(dashmap::DashMap::new()),
        whisper,
        memory,
        last_reply: Arc::new(dashmap::DashMap::new()),
        worker_pool,
        metrics,
    };

    // Real-time opcode WebSocket transport (alongside the Telegram bot).
    let ws_cfg = ws::WsServerConfig {
        bind: std::env::var("ANUBIS_WS_BIND")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| "127.0.0.1:7600".parse().expect("valid bind addr")),
        auth_token: std::env::var("ANUBIS_WS_TOKEN").ok().map(Into::into),
    };
    if ws_cfg.auth_token.is_some() {
        info!("ANUBIS WS server auth: token required");
    }

    let bot_state = state.clone();
    let bot_fut = bot::run(state);
    let ws_fut = ws::run_ws_server(bot_state, ws_cfg);

    tokio::select! {
        r = bot_fut => r?,
        r = ws_fut => r?,
    }
    Ok(())
}
