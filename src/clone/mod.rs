pub mod f5;

use crate::error::{AnubisError, Result};
use chrono::Utc;
use std::path::{Path, PathBuf};

pub use f5::F5Cloner;

/// Wraps the local XTTS-v2 REST server (xtts-api-server).
///
/// Start the server with:
///   xtts-api-server --port 8020 --model_source local --model_path /path/to/xtts_v2
///
/// API used:
///   POST /tts_to_audio/
///   JSON { "text": "…", "speaker_wav": "/abs/path/to/sample.wav", "language": "en" }
///   Returns: WAV audio bytes
pub struct XttsCloner {
    client:     reqwest::Client,
    base_url:   String,
    clones_dir: PathBuf,
    pub enabled: bool,
}

impl XttsCloner {
    pub fn new(base_url: String, clones_dir: &str, enabled: bool) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .expect("reqwest client"),
            base_url,
            clones_dir: PathBuf::from(clones_dir),
            enabled,
        }
    }

    /// Save a WAV sample for `user_id`.
    /// Returns (clone_id, saved_path).
    pub async fn save_sample(&self, user_id: i64, wav_path: &Path) -> Result<(String, PathBuf)> {
        let clone_id = uuid::Uuid::new_v4().to_string();
        let user_dir = self.clones_dir.join(user_id.to_string());
        tokio::fs::create_dir_all(&user_dir).await?;

        let dest = user_dir.join(format!("{}.wav", clone_id));
        tokio::fs::copy(wav_path, &dest).await?;

        Ok((clone_id, dest))
    }

    /// Call XTTS to synthesize `text` using the cloned voice at `wav_path`.
    /// Returns raw WAV bytes.
    pub async fn synthesize(&self, text: &str, wav_path: &Path, lang: &str) -> Result<Vec<u8>> {
        if !self.enabled {
            return Err(AnubisError::Clone("voice cloning is disabled in config".into()));
        }

        let body = serde_json::json!({
            "text": text,
            "speaker_wav": wav_path.to_str().unwrap_or(""),
            "language": lang
        });

        let response = self
            .client
            .post(format!("{}/tts_to_audio/", self.base_url))
            .json(&body)
            .send()
            .await
            .map_err(|e| AnubisError::Clone(format!("XTTS request: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AnubisError::Clone(format!("XTTS server {status}: {body}")));
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| AnubisError::Clone(format!("XTTS read body: {e}")))?;

        Ok(bytes.to_vec())
    }
}

/// Metadata stored in the database for a voice clone.
pub fn new_voice_clone(
    user_id: i64,
    clone_id: String,
    wav_path: PathBuf,
) -> crate::db::VoiceClone {
    crate::db::VoiceClone {
        id:         clone_id,
        user_id,
        name:       format!("Clone {}", Utc::now().format("%Y-%m-%d %H:%M")),
        wav_path:   wav_path.to_string_lossy().to_string(),
        ref_text:   String::new(),
        created_at: Utc::now().to_rfc3339(),
    }
}
