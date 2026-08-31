mod audio;
mod bot;
mod cache;
mod clone;
mod config;
mod db;
mod error;
mod i18n;
mod memory;
mod noxis;
mod security;
mod tts;
mod whisper;

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
        "ANUBIS Core starting — db={} (local-first, open source)",
        cfg.database.path
    );

    let database = db::Database::open(&cfg.database.path, cfg.database.pool_max)?;

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

    // Voice clone engine (Chatterbox, MIT) — replaces old XTTS/F5.
    let clone_engine = Arc::new(clone::ChatterboxCloner::new(
        cfg.clone.url.clone(),
        &cfg.clone.clones_dir,
        cfg.clone.enabled,
    ));
    info!("Clone engine: chatterbox enabled={}", cfg.clone.enabled);

    // Noxis Core — local LLM brain (llama.cpp / ollama compatible).
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

    // Per-user conversation memory for Noxis Core.
    let memory = Arc::new(memory::ConversationStore::new(12));

    let rate_limiter = Arc::new(security::RateLimiter::new(
        cfg.security.rate_speak_per_min,
        cfg.security.rate_clone_per_hr,
    ));
    let watermark = Arc::new(security::Watermarker::new(cfg.security.watermark_enabled));
    let cache = Arc::new(cache::AudioCache::new(cfg.limits.cache_capacity));

    let state = bot::AppState {
        db: Arc::new(database),
        tts: tts_router,
        clone_engine,
        noxis,
        audio,
        config: cfg,
        pending: Arc::new(dashmap::DashMap::new()),
        rate_limiter,
        cache,
        watermark,
        whisper,
        memory,
        last_reply: Arc::new(dashmap::DashMap::new()),
    };

    bot::run(state).await
}
