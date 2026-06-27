use anyhow::{bail, Result};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub telegram: TelegramConfig,
    pub tts:      TtsConfig,
    pub clone:    CloneConfig,
    pub database: DatabaseConfig,
    pub limits:   LimitsConfig,
    pub audio:    AudioConfig,
    pub security: SecurityConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TelegramConfig {
    pub token: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TtsConfig {
    pub piper_binary: String,
    pub voices_dir:   String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CloneConfig {
    pub xtts_url:   String,
    pub f5_url:     String,
    pub clones_dir: String,
    pub enabled:    bool,
    pub f5_enabled: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub path:     String,
    pub pool_max: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LimitsConfig {
    pub max_audio_duration_secs: u32,
    pub max_text_chars:          usize,
    pub free_daily_credits:      i32,
    pub unlimited_mode:          bool,
    pub cache_capacity:          usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AudioConfig {
    pub output_dir: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SecurityConfig {
    pub admin_ids:          Vec<i64>,
    pub watermark_enabled:  bool,
    pub rate_speak_per_min: u32,
    pub rate_clone_per_hr:  u32,
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = std::env::var("ANUBIS_CONFIG").unwrap_or_else(|_| "config.toml".to_string());
        let raw  = std::fs::read_to_string(&path).unwrap_or_default();

        let mut cfg: Config = if raw.trim().is_empty() {
            Config::defaults()
        } else {
            toml::from_str(&raw)?
        };

        if let Ok(tok) = std::env::var("TELEGRAM_BOT_TOKEN") {
            cfg.telegram.token = tok;
        }

        if cfg.telegram.token.trim().is_empty() {
            bail!("TELEGRAM_BOT_TOKEN is not set");
        }

        Ok(cfg)
    }

    pub fn is_admin(&self, user_id: i64) -> bool {
        self.security.admin_ids.contains(&user_id)
    }

    fn defaults() -> Self {
        Self {
            telegram: TelegramConfig { token: String::new() },
            tts: TtsConfig {
                piper_binary: "piper".into(),
                voices_dir:   "./voices".into(),
            },
            clone: CloneConfig {
                xtts_url:   "http://127.0.0.1:8020".into(),
                f5_url:     "http://127.0.0.1:9050".into(),
                clones_dir: "./clones".into(),
                enabled:    true,
                f5_enabled: false,
            },
            database: DatabaseConfig {
                path:     "./anubis.db".into(),
                pool_max: 8,
            },
            limits: LimitsConfig {
                max_audio_duration_secs: 60,
                max_text_chars:          500,
                free_daily_credits:      3,
                unlimited_mode:          false,
                cache_capacity:          256,
            },
            audio: AudioConfig { output_dir: "./audio_output".into() },
            security: SecurityConfig {
                admin_ids:          vec![],
                watermark_enabled:  true,
                rate_speak_per_min: 30,
                rate_clone_per_hr:  3,
            },
        }
    }
}
