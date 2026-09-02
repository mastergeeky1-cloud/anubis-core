pub mod engine;
pub mod kokoro;
pub mod router;
pub mod voices;

use crate::error::{AnubisError, Result};
use async_trait::async_trait;
use engine::TtsEngine;
use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

/// Piper local TTS engine. Pure CPU, fast, MIT-licensed models.
pub struct PiperTts {
    binary: String,
    voices_dir: PathBuf,
}

impl PiperTts {
    pub fn new(binary: String, voices_dir: String) -> Self {
        Self {
            binary,
            voices_dir: PathBuf::from(voices_dir),
        }
    }

    fn model_path(&self, voice_id: &str, lang: &str) -> PathBuf {
        // Models are nested: <voices_dir>/<lang>/<...>/<voice_id>.onnx
        // Search recursively so we don't depend on the exact subfolder layout.
        let wanted = format!("{}.onnx", voice_id);
        let mut found = None;
        if let Ok(entries) = std::fs::read_dir(&self.voices_dir) {
            for e in entries.flatten() {
                let p = e.path();
                if p.is_dir() {
                    if let Some(m) = find_model_recursive(&p, &wanted) {
                        found = Some(m);
                        break;
                    }
                } else if p.file_name().map(|n| n == wanted.as_str()).unwrap_or(false) {
                    found = Some(p);
                    break;
                }
            }
        }
        found.unwrap_or_else(|| self.voices_dir.join(lang).join(&wanted))
    }
}

fn find_model_recursive(dir: &std::path::Path, wanted: &str) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            if let Some(m) = find_model_recursive(&p, wanted) {
                return Some(m);
            }
        } else if p.file_name().map(|n| n == wanted).unwrap_or(false) {
            return Some(p);
        }
    }
    None
}

#[async_trait]
impl TtsEngine for PiperTts {
    fn available_voices(&self) -> Vec<&'static voices::VoiceMeta> {
        voices::VOICES
            .iter()
            .filter(|v| self.model_path(v.id, v.lang).exists())
            .collect()
    }

    async fn synthesize_wav(&self, text: &str, voice_id: &str) -> Result<PathBuf> {
        let meta = voices::find(voice_id)
            .ok_or_else(|| AnubisError::VoiceNotFound(voice_id.to_string()))?;
        let model_path = self.model_path(voice_id, meta.lang);
        if !model_path.exists() {
            return Err(AnubisError::Tts(format!(
                "model file not found: {}",
                model_path.display()
            )));
        }
        let wav_out = std::env::temp_dir().join(format!("{}.wav", uuid::Uuid::new_v4()));
        let mut child = Command::new(&self.binary)
            .args([
                "--model",
                model_path.to_str().unwrap(),
                "--output_file",
                wav_out.to_str().unwrap(),
            ])
            .stdin(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| AnubisError::Tts(format!("failed to spawn piper: {e}")))?;

        if let Some(stdin) = child.stdin.take() {
            let mut stdin = tokio::io::BufWriter::new(stdin);
            stdin
                .write_all(text.as_bytes())
                .await
                .map_err(|e| AnubisError::Tts(format!("stdin write: {e}")))?;
            stdin
                .shutdown()
                .await
                .map_err(|e| AnubisError::Tts(format!("stdin shutdown: {e}")))?;
        }
        let status = child
            .wait()
            .await
            .map_err(|e| AnubisError::Tts(format!("piper wait: {e}")))?;
        if !status.success() {
            return Err(AnubisError::Tts("piper exited with non-zero status".into()));
        }
        if !wav_out.exists() {
            return Err(AnubisError::Tts("piper did not produce output file".into()));
        }
        Ok(wav_out)
    }
}

/// Helper: clean up a temporary WAV file, ignoring errors.
pub async fn remove_wav(path: &Path) {
    tokio::fs::remove_file(path).await.ok();
}
