//! Bounded worker pool for CPU-heavy / rate-limited synthesis tasks.
//!
//! Prevents too many concurrent TTS / clone / ffmpeg jobs from overwhelming
//  sidecars or the event loop. Handlers acquire a permit before starting
//! work; excess requests queue behind the semaphore.

use crate::audio::AudioProcessor;
use crate::clone::ChatterboxCloner;
use crate::error::Result;
use crate::tts::router::TtsRouter;
use std::sync::Arc;
use tokio::sync::Semaphore;

/// Worker pool configuration.
#[derive(Debug, Clone)]
pub struct WorkerPoolConfig {
    /// Max concurrent synthesis jobs (TTS + clone).
    pub max_synth: usize,
    /// Max concurrent ffmpeg conversions.
    pub max_convert: usize,
}

impl Default for WorkerPoolConfig {
    fn default() -> Self {
        Self {
            max_synth: 2,
            max_convert: 2,
        }
    }
}

/// The worker pool.
pub struct WorkerPool {
    tts: Arc<TtsRouter>,
    clone_engine: Arc<ChatterboxCloner>,
    audio: Arc<AudioProcessor>,
    sem_synth: Arc<Semaphore>,
    sem_convert: Arc<Semaphore>,
}

impl WorkerPool {
    pub fn new(
        tts: Arc<TtsRouter>,
        clone_engine: Arc<ChatterboxCloner>,
        audio: Arc<AudioProcessor>,
        cfg: WorkerPoolConfig,
    ) -> Self {
        Self {
            tts,
            clone_engine,
            audio,
            sem_synth: Arc::new(Semaphore::new(cfg.max_synth)),
            sem_convert: Arc::new(Semaphore::new(cfg.max_convert)),
        }
    }

    /// Run a TTS synthesis job, returning the WAV file path.
    pub async fn synthesize_tts(&self, text: &str, voice_id: &str) -> Result<std::path::PathBuf> {
        let _permit = self.sem_synth.acquire().await.unwrap();
        self.tts.synthesize_wav(text, voice_id).await
    }

    /// Run a voice clone synthesis job, returning WAV bytes.
    pub async fn synthesize_clone(
        &self,
        text: &str,
        wav_path: &str,
        lang: &str,
        ref_text: &str,
    ) -> Result<Vec<u8>> {
        let _permit = self.sem_synth.acquire().await.unwrap();
        self.clone_engine
            .synthesize(text, std::path::Path::new(wav_path), lang, ref_text)
            .await
    }

    /// Convert WAV to OGG via ffmpeg, returning OGG bytes.
    pub async fn convert_wav_to_ogg(&self, wav_path: &std::path::Path) -> Result<Vec<u8>> {
        let _permit = self.sem_convert.acquire().await.unwrap();
        self.audio.wav_to_ogg(wav_path).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_defaults() {
        let cfg = WorkerPoolConfig::default();
        assert_eq!(cfg.max_synth, 2);
        assert_eq!(cfg.max_convert, 2);
    }
}
