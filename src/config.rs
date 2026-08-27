use anyhow::{bail, Result};
use serde::Deserialize;

/// Configuration is loaded from config.toml BUT every secret / endpoint is
/// overridable (and preferred) via environment variables. Secrets are never
/// required to live in a file.
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub telegram: TelegramConfig,
    pub llm: LlmConfig,
    pub tts: TtsConfig,
    pub clone: CloneConfig,
    pub database: DatabaseConfig,
    pub limits: LimitsConfig,
    pub audio: AudioConfig,
    pub security: SecurityConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TelegramConfig {
    pub token: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LlmConfig {
    pub base_url: String,
    pub model: String,
    pub max_tokens: u32,
    pub system_prompt: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TtsConfig {
    pub piper_binary: String,
    pub voices_dir: String,
    pub kokoro_url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CloneConfig {
    pub enabled: bool,
    pub url: String,
    pub clones_dir: String,
    pub ref_text: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub path: String,
    pub pool_max: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LimitsConfig {
    pub max_audio_duration_secs: u32,
    pub max_text_chars: usize,
    pub free_daily_credits: i32,
    pub unlimited_mode: bool,
    pub cache_capacity: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AudioConfig {
    pub output_dir: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SecurityConfig {
    pub admin_ids: Vec<i64>,
    pub watermark_enabled: bool,
    pub rate_speak_per_min: u32,
    pub rate_clone_per_hr: u32,
    pub require_consent: bool,
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = std::env::var("ANUBIS_CONFIG").unwrap_or_else(|_| "config.toml".to_string());
        let raw = std::fs::read_to_string(&path).unwrap_or_default();
        let mut cfg: Config = if raw.trim().is_empty() {
            Config::defaults()
        } else {
            toml::from_str(&raw)?
        };

        // ── Secrets / endpoints: env overrides file ──
        if let Ok(tok) = std::env::var("ANUBIS_TELEGRAM_TOKEN") {
            cfg.telegram.token = tok;
        }
        if let Ok(u) = std::env::var("ANUBIS_LLM_URL") {
            cfg.llm.base_url = u;
        }
        if let Ok(u) = std::env::var("ANUBIS_CLONE_URL") {
            cfg.clone.url = u;
        }
        if let Ok(u) = std::env::var("ANUBIS_KOKORO_URL") {
            cfg.tts.kokoro_url = u;
        }

        if cfg.telegram.token.trim().is_empty() {
            bail!(
                "Telegram token missing. Set ANUBIS_TELEGRAM_TOKEN (or config.toml telegram.token). \
                 Refusing to start with a hardcoded/empty token."
            );
        }
        Ok(cfg)
    }

    pub fn is_admin(&self, user_id: i64) -> bool {
        self.security.admin_ids.contains(&user_id)
    }

    fn defaults() -> Self {
        Self {
            telegram: TelegramConfig {
                token: String::new(),
            },
            llm: LlmConfig {
                base_url: "http://127.0.0.1:8080".into(),
                model: "local".into(),
                max_tokens: 512,
                system_prompt:
                    "You are Noxis Core, the secure local assistant inside ANUBIS. Be concise."
                        .into(),
            },
            tts: TtsConfig {
                piper_binary: "piper".into(),
                voices_dir: "./voices".into(),
                kokoro_url: "http://127.0.0.1:8880".into(),
            },
            clone: CloneConfig {
                enabled: true,
                url: "http://127.0.0.1:8008".into(),
                clones_dir: "./clones".into(),
                ref_text: String::new(),
            },
            database: DatabaseConfig {
                path: "./anubis.db".into(),
                pool_max: 8,
            },
            limits: LimitsConfig {
                max_audio_duration_secs: 60,
                max_text_chars: 1000,
                free_daily_credits: 30,
                unlimited_mode: false,
                cache_capacity: 512,
            },
            audio: AudioConfig {
                output_dir: "./audio_output".into(),
            },
            security: SecurityConfig {
                admin_ids: vec![],
                watermark_enabled: true,
                rate_speak_per_min: 30,
                rate_clone_per_hr: 5,
                require_consent: true,
            },
        }
    }
}
