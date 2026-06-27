mod audio;
mod bot;
mod cache;
mod clone;
mod config;
mod db;
mod error;
mod i18n;
mod security;
mod tts;

use anyhow::Result;
use std::sync::Arc;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("anubis=info".parse()?),
        )
        .init();

    let cfg = Arc::new(config::Config::load()?);
    info!("ANUBIS starting — db={}", cfg.database.path);

    // Database (r2d2 pool)
    let database = db::Database::open(&cfg.database.path, cfg.database.pool_max)?;

    // TTS router (Piper)
    let piper = tts::PiperTts::new(cfg.tts.piper_binary.clone(), cfg.tts.voices_dir.clone());
    let tts_router = Arc::new(tts::router::TtsRouter::new(vec![Box::new(piper)]));
    info!(
        "Piper TTS: binary={} voices_dir={}",
        cfg.tts.piper_binary, cfg.tts.voices_dir
    );

    // Voice clone engines
    let clone_engine = Arc::new(clone::XttsCloner::new(
        cfg.clone.xtts_url.clone(),
        &cfg.clone.clones_dir,
        cfg.clone.enabled,
    ));
    let f5_cloner = Arc::new(clone::F5Cloner::new(
        cfg.clone.f5_url.clone(),
        &cfg.clone.clones_dir,
        cfg.clone.f5_enabled,
    ));
    info!(
        "Clone engines: xtts={} f5={}",
        cfg.clone.enabled, cfg.clone.f5_enabled
    );

    // Audio processor
    let audio = Arc::new(audio::AudioProcessor::new(&cfg.audio.output_dir)?);

    // Security
    let rate_limiter = Arc::new(security::RateLimiter::new(
        cfg.security.rate_speak_per_min,
        cfg.security.rate_clone_per_hr,
    ));
    let watermark = Arc::new(security::Watermarker::new(cfg.security.watermark_enabled));

    // Cache
    let cache = Arc::new(cache::AudioCache::new(cfg.limits.cache_capacity));

    let state = bot::AppState {
        db: Arc::new(database),
        tts: tts_router,
        clone_engine,
        f5_cloner,
        audio,
        config: cfg,
        pending: Arc::new(dashmap::DashMap::new()),
        rate_limiter,
        cache,
        watermark,
    };

    bot::run(state).await
}
