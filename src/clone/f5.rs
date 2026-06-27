use crate::error::{AnubisError, Result};
use reqwest::multipart;
use std::path::PathBuf;

/// Wraps a local F5-TTS HTTP server.
///
/// Expected endpoint:
///   POST /tts
///   Multipart fields:
///     ref_audio  — WAV bytes of the reference speaker sample
///     ref_text   — transcript of the reference audio (may be empty)
///     gen_text   — text to synthesize
///   Returns: WAV audio bytes
pub struct F5Cloner {
    client:     reqwest::Client,
    base_url:   String,
    clones_dir: PathBuf,
    pub enabled: bool,
}

impl F5Cloner {
    pub fn new(base_url: String, clones_dir: &str, enabled: bool) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(180))
                .build()
                .expect("reqwest client"),
            base_url,
            clones_dir: PathBuf::from(clones_dir),
            enabled,
        }
    }

    /// Save a WAV reference sample for `user_id`.
    /// Returns (clone_id, saved_path).
    pub async fn save_sample(&self, user_id: i64, wav_path: &std::path::Path) -> Result<(String, PathBuf)> {
        let clone_id = uuid::Uuid::new_v4().to_string();
        let user_dir = self.clones_dir.join("f5").join(user_id.to_string());
        tokio::fs::create_dir_all(&user_dir).await?;
        let dest = user_dir.join(format!("{}.wav", clone_id));
        tokio::fs::copy(wav_path, &dest).await?;
        Ok((clone_id, dest))
    }

    /// Synthesize `text` using the reference voice at `wav_path`.
    /// Returns raw WAV bytes.
    pub async fn synthesize(
        &self,
        text: &str,
        wav_path: &std::path::Path,
        ref_text: &str,
    ) -> Result<Vec<u8>> {
        if !self.enabled {
            return Err(AnubisError::Clone("F5-TTS is disabled in config".into()));
        }

        let wav_bytes = tokio::fs::read(wav_path).await?;

        let ref_audio_part = multipart::Part::bytes(wav_bytes)
            .file_name("ref.wav")
            .mime_str("audio/wav")
            .map_err(|e| AnubisError::Clone(format!("mime: {e}")))?;

        let form = multipart::Form::new()
            .part("ref_audio", ref_audio_part)
            .text("ref_text", ref_text.to_string())
            .text("gen_text", text.to_string());

        let response = self
            .client
            .post(format!("{}/tts", self.base_url))
            .multipart(form)
            .send()
            .await
            .map_err(|e| AnubisError::Clone(format!("F5-TTS request: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AnubisError::Clone(format!("F5-TTS server {status}: {body}")));
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| AnubisError::Clone(format!("F5-TTS read body: {e}")))?;

        Ok(bytes.to_vec())
    }
}
