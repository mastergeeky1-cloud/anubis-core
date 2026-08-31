use crate::error::{AnubisError, Result};
use chrono::Utc;
use std::path::{Path, PathBuf};

/// Local Chatterbox-Multilingual voice-clone server (MIT license).
/// Replaces the old non-commercial XTTS-v2 / F5-TTS path.
///
/// Expected endpoint:
///   POST /tts
///   multipart form:
///     reference_audio — WAV bytes of the user's cloned voice sample
///     reference_text  — transcript of the reference audio
///     text            — text to synthesize in the cloned voice
///     language        — BCP-47 lang tag (e.g. "en")
///   Returns: WAV audio bytes
pub struct ChatterboxCloner {
    client: reqwest::Client,
    base_url: String,
    clones_dir: PathBuf,
    pub enabled: bool,
}

impl ChatterboxCloner {
    pub fn new(base_url: String, clones_dir: &str, enabled: bool) -> Self {
        Self {
            client: reqwest::Client::builder()
                // Clone synthesis on CPU can take a few minutes (longer on
                // first warm request while the model loads), so allow generous
                // time before timing out.
                .timeout(std::time::Duration::from_secs(420))
                .build()
                .expect("reqwest client"),
            base_url,
            clones_dir: PathBuf::from(clones_dir),
            enabled,
        }
    }

    /// Persist a WAV sample for `user_id`. Returns (clone_id, saved_path).
    pub async fn save_sample(&self, user_id: i64, wav_path: &Path) -> Result<(String, PathBuf)> {
        let clone_id = uuid::Uuid::new_v4().to_string();
        let user_dir = self.clones_dir.join(user_id.to_string());
        tokio::fs::create_dir_all(&user_dir).await?;
        let dest = user_dir.join(format!("{}.wav", clone_id));
        tokio::fs::copy(wav_path, &dest).await?;
        Ok((clone_id, dest))
    }

    /// Synthesize `text` in the cloned voice at `wav_path`.
    pub async fn synthesize(
        &self,
        text: &str,
        wav_path: &Path,
        lang: &str,
        ref_text: &str,
    ) -> Result<Vec<u8>> {
        if !self.enabled {
            return Err(AnubisError::Clone(
                "voice cloning is disabled in config".into(),
            ));
        }
        let wav_bytes = tokio::fs::read(wav_path).await?;
        let ref_audio_part = reqwest::multipart::Part::bytes(wav_bytes)
            .file_name("ref.wav")
            .mime_str("audio/wav")
            .map_err(|e| AnubisError::Clone(format!("mime: {e}")))?;

        let form = reqwest::multipart::Form::new()
            .part("reference_audio", ref_audio_part)
            .text("reference_text", ref_text.to_string())
            .text("text", text.to_string())
            .text("language", lang.to_string());

        let response = self
            .client
            .post(format!("{}/tts", self.base_url))
            .multipart(form)
            .send()
            .await
            .map_err(|e| AnubisError::Clone(format!("chatterbox request: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AnubisError::Clone(format!(
                "chatterbox server {status}: {body}"
            )));
        }
        let bytes = response
            .bytes()
            .await
            .map_err(|e| AnubisError::Clone(format!("chatterbox read: {e}")))?;
        Ok(bytes.to_vec())
    }
}

/// Metadata stored in the database for a voice clone.
pub fn new_voice_clone(
    user_id: i64,
    clone_id: String,
    wav_path: PathBuf,
    ref_text: &str,
) -> crate::db::VoiceClone {
    crate::db::VoiceClone {
        id: clone_id,
        user_id,
        name: format!("Clone {}", Utc::now().format("%Y-%m-%d %H:%M")),
        wav_path: wav_path.to_string_lossy().to_string(),
        ref_text: ref_text.to_string(),
        created_at: Utc::now().to_rfc3339(),
    }
}
