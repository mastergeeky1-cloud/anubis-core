//! Local Whisper voice transcription client.
//!
//! Talks to a small whisper.cpp-compatible sidecar (e.g. `whisper-server` from
//! whisper.cpp, or a tiny Python wrapper) running entirely on the machine so
//! no audio leaves the host. This powers voice-to-voice conversation.
//!
//! Expected endpoint:
//!   POST {base_url}/transcribe
//!   multipart form: file=<wav bytes> (audio/wav), optional model/language
//!   Response (JSON): { "text": "..." }
//!
//! The `service_url` may also point at a plain OpenAI-compatible
//! `/audio/transcriptions` endpoint; in that case set `openai_compat = true`.

use crate::error::{AnubisError, Result};
use std::path::Path;

pub struct WhisperClient {
    client: reqwest::Client,
    base_url: String,
    pub enabled: bool,
}

impl WhisperClient {
    pub fn new(base_url: String, enabled: bool) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .expect("reqwest client"),
            base_url,
            enabled,
        }
    }

    /// Transcribe a local WAV file to text.
    pub async fn transcribe(&self, wav_path: &Path, lang: Option<&str>) -> Result<String> {
        if !self.enabled {
            return Err(AnubisError::Audio("whisper not configured".into()));
        }
        let bytes = tokio::fs::read(wav_path).await?;
        let part = reqwest::multipart::Part::bytes(bytes)
            .file_name("audio.wav")
            .mime_str("audio/wav")
            .map_err(|e| AnubisError::Audio(format!("whisper mime: {e}")))?;

        let mut form = reqwest::multipart::Form::new().part("file", part);
        if let Some(lang) = lang {
            if !lang.is_empty() {
                form = form.text("language", lang.to_string());
            }
        }

        let response = self
            .client
            .post(format!("{}/audio/transcriptions", self.base_url))
            .multipart(form)
            .send()
            .await
            .map_err(|e| AnubisError::Audio(format!("whisper request: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let b = response.text().await.unwrap_or_default();
            return Err(AnubisError::Audio(format!("whisper server {status}: {b}")));
        }
        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| AnubisError::Audio(format!("whisper json: {e}")))?;
        let text = json
            .get("text")
            .and_then(|t| t.as_str())
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
            .ok_or_else(|| AnubisError::Audio("whisper returned no text".into()))?;
        Ok(text)
    }
}
