use crate::error::{AnubisError, Result};
use crate::tts::voices::VoiceMeta;
use async_trait::async_trait;
use std::path::PathBuf;

/// Kokoro TTS via local HTTP sidecar (e.g. a small FastAPI/axum server
/// wrapping Kokoro-82M, Apache-2.0). Zero GPU required — runs on CPU.
/// The sidecar exposes POST /v1/audio/speech with JSON
/// { "text": "...", "voice": "<kokoro_voice_id>" } -> WAV bytes.
pub struct KokoroTts {
    base_url: String,
    /// Map of our catalogue ids -> Kokoro voice ids (subset).
    voice_map: std::collections::HashMap<&'static str, &'static str>,
}

impl KokoroTts {
    pub fn new(base_url: String) -> Self {
        let mut voice_map = std::collections::HashMap::new();
        // Kokoro voices (af_/am_/bf_/bm_ prefixes = female/male, US/UK).
        voice_map.insert("en_US-amy-medium", "af_heart");
        voice_map.insert("en_US-ryan-high", "am_adam");
        voice_map.insert("en_GB-alan-low", "bm_george");
        voice_map.insert("en_US-lessac-medium", "af_alloy");
        Self {
            base_url,
            voice_map,
        }
    }

    fn kokoro_voice(&self, id: &str) -> Option<&'static str> {
        self.voice_map.get(id).copied()
    }
}

#[async_trait]
impl crate::tts::engine::TtsEngine for KokoroTts {
    fn id(&self) -> &str {
        "kokoro"
    }

    fn available_voices(&self) -> Vec<&'static VoiceMeta> {
        crate::tts::voices::VOICES
            .iter()
            .filter(|v| self.voice_map.contains_key(v.id))
            .collect()
    }

    async fn synthesize_wav(&self, text: &str, voice_id: &str) -> Result<PathBuf> {
        let kvoice = self
            .kokoro_voice(voice_id)
            .ok_or_else(|| AnubisError::VoiceNotFound(voice_id.to_string()))?;
        let client = reqwest::Client::new();
        let resp = client
            .post(format!("{}/v1/audio/speech", self.base_url))
            .json(&serde_json::json!({ "text": text, "voice": kvoice, "format": "wav" }))
            .send()
            .await
            .map_err(|e| AnubisError::Tts(format!("kokoro request: {e}")))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AnubisError::Tts(format!("kokoro server {status}: {body}")));
        }
        let bytes = resp
            .bytes()
            .await
            .map_err(|e| AnubisError::Tts(format!("kokoro read: {e}")))?;
        let out = std::env::temp_dir().join(format!("{}.wav", uuid::Uuid::new_v4()));
        tokio::fs::write(&out, &bytes)
            .await
            .map_err(|e| AnubisError::Tts(format!("kokoro write: {e}")))?;
        Ok(out)
    }
}
