use thiserror::Error;

#[derive(Error, Debug)]
pub enum AnubisError {
    #[error("database: {0}")]
    Db(#[from] rusqlite::Error),

    #[error("db pool: {0}")]
    Pool(String),

    #[error("LLM (Noxis Core) failed: {0}")]
    Llm(String),

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

    #[error("voice not found: {0}")]
    VoiceNotFound(String),
}

impl From<r2d2::Error> for AnubisError {
    fn from(e: r2d2::Error) -> Self {
        AnubisError::Pool(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, AnubisError>;
