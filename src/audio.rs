use crate::error::{AnubisError, Result};
use std::path::{Path, PathBuf};
use tokio::process::Command;
use uuid::Uuid;

/// Local audio processing: WAV <-> OGG (Opus) via ffmpeg, plus duration probe.
/// Pure local; no network. Requires `ffmpeg`/`ffprobe` on PATH.
pub struct AudioProcessor {
    pub output_dir: PathBuf,
}

impl AudioProcessor {
    pub fn new(output_dir: &str) -> Result<Self> {
        let dir = PathBuf::from(output_dir);
        std::fs::create_dir_all(&dir)?;
        Ok(Self { output_dir: dir })
    }

    pub fn tmp_path(&self, ext: &str) -> PathBuf {
        self.output_dir.join(format!("{}.{}", Uuid::new_v4(), ext))
    }

    /// WAV path -> OGG Opus bytes ready for Telegram send_voice.
    pub async fn wav_to_ogg(&self, wav_path: &Path) -> Result<Vec<u8>> {
        let out = self.tmp_path("ogg");
        let status = Command::new("ffmpeg")
            .args([
                "-y",
                "-i",
                wav_path.to_str().unwrap(),
                "-c:a",
                "libopus",
                "-b:a",
                "64k",
                "-vn",
                out.to_str().unwrap(),
            ])
            .stderr(std::process::Stdio::null())
            .status()
            .await?;

        if !status.success() {
            return Err(AnubisError::Audio("ffmpeg wav->ogg failed".into()));
        }
        let bytes = tokio::fs::read(&out).await?;
        tokio::fs::remove_file(&out).await.ok();
        Ok(bytes)
    }
}
