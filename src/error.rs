use thiserror::Error;

#[derive(Error, Debug)]
pub enum AnubisError {
    #[error("database: {0}")]
    Db(#[from] rusqlite::Error),

    #[error("db pool: {0}")]
    Pool(String),

    #[error("TTS failed: {0}")]
    Tts(String),

    #[error("voice clone failed: {0}")]
    Clone(String),

    #[error("audio processing failed: {0}")]
    Audio(String),

    #[error("WAV codec: {0}")]
    Wav(String),

    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("insufficient credits")]
    NoCredits,

    #[error("voice not found: {0}")]
    VoiceNotFound(String),

    #[error("no cloned voice — use /clone first")]
    NoClone,

    #[error("text too long ({chars} chars, max {max})")]
    TextTooLong { chars: usize, max: usize },

    #[error("audio too long ({secs}s, max {max}s)")]
    AudioTooLong { secs: u32, max: u32 },

    #[error("unsupported language: {0}")]
    BadLang(String),
}

impl From<r2d2::Error> for AnubisError {
    fn from(e: r2d2::Error) -> Self {
        AnubisError::Pool(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, AnubisError>;
