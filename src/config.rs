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
    pub whisper: WhisperConfig,
    pub database: DatabaseConfig,
    pub limits: LimitsConfig,
    pub audio: AudioConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WhisperConfig {
    pub enabled: bool,
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct TelegramConfig {
    pub token: String,
    /// "poll" (default) or "webhook". Webhook scales to multiple replicas
    /// behind a reverse proxy (set via ANUBIS_TELEGRAM_MODE=webhook).
    pub mode: String,
    /// Only used when mode = "webhook".
    pub webhook_url: String,
    pub webhook_listen: String,
}

impl Default for TelegramConfig {
    fn default() -> Self {
        Self {
            token: String::new(),
            mode: "poll".into(),
            webhook_url: String::new(),
            webhook_listen: "127.0.0.1:8443".into(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct LlmConfig {
    pub base_url: String,
    pub model: String,
    pub max_tokens: u32,
    pub system_prompt: String,
    /// Bearer token for hosted OpenAI-compatible endpoints (e.g. omniroute).
    /// Loaded from ANUBIS_LLM_KEY. Never stored in config.toml.
    pub api_key: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TtsConfig {
    pub piper_binary: String,
    pub voices_dir: String,
    pub kokoro_url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub path: String,
    pub pool_max: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct LimitsConfig {
    pub max_text_chars: usize,
    pub max_concurrent_synth: usize,
}

impl Default for LimitsConfig {
    fn default() -> Self {
        Self {
            max_text_chars: 1000,
            max_concurrent_synth: 2,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct AudioConfig {
    pub output_dir: String,
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
        if let Ok(mode) = std::env::var("ANUBIS_TELEGRAM_MODE") {
            cfg.telegram.mode = mode;
        }
        if let Ok(u) = std::env::var("ANUBIS_WEBHOOK_URL") {
            cfg.telegram.webhook_url = u;
        }
        if let Ok(l) = std::env::var("ANUBIS_WEBHOOK_LISTEN") {
            cfg.telegram.webhook_listen = l;
        }
        if let Ok(u) = std::env::var("ANUBIS_LLM_URL") {
            cfg.llm.base_url = u;
        }
        if let Ok(k) = std::env::var("ANUBIS_LLM_KEY") {
            cfg.llm.api_key = k;
        }
        if let Ok(m) = std::env::var("ANUBIS_LLM_MODEL") {
            cfg.llm.model = m;
        }
        if let Ok(u) = std::env::var("ANUBIS_KOKORO_URL") {
            cfg.tts.kokoro_url = u;
        }
        if let Ok(u) = std::env::var("ANUBIS_WHISPER_URL") {
            let u = u.trim().to_string();
            cfg.whisper.url = u.clone();
            cfg.whisper.enabled = !u.is_empty();
        }

        if cfg.telegram.token.trim().is_empty() {
            bail!(
                "Telegram token missing. Set ANUBIS_TELEGRAM_TOKEN (or config.toml telegram.token). \
                 Refusing to start with a hardcoded/empty token."
            );
        }
        Ok(cfg)
    }

    fn defaults() -> Self {
        Self {
            telegram: TelegramConfig {
                token: String::new(),
                mode: "poll".into(),
                webhook_url: String::new(),
                webhook_listen: "127.0.0.1:8443".into(),
            },
            llm: LlmConfig {
                base_url: "http://127.0.0.1:8080".into(),
                model: "local".into(),
                max_tokens: 512,
                api_key: String::new(),
                system_prompt:
                    "You are Noxis Core, the secure local assistant inside ANUBIS. Be concise."
                        .into(),
            },
            tts: TtsConfig {
                piper_binary: "piper".into(),
                voices_dir: "./voices".into(),
                kokoro_url: "http://127.0.0.1:8880".into(),
            },
            whisper: WhisperConfig {
                enabled: false,
                url: String::new(),
            },
            database: DatabaseConfig {
                path: "./anubis.db".into(),
                pool_max: 8,
            },
            limits: LimitsConfig {
                max_text_chars: 1000,
                max_concurrent_synth: 2,
            },
            audio: AudioConfig {
                output_dir: "./audio_output".into(),
            },
        }
    }
}
