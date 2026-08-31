use crate::error::Result;
use crate::tts::engine::TtsEngine;
use crate::tts::voices::VoiceMeta;
use std::path::PathBuf;

/// Routes synthesis requests across a chain of TTS engines (Piper + Kokoro).
/// Tries each engine in order; the first one that has the requested voice wins.
pub struct TtsRouter {
    engines: Vec<Box<dyn TtsEngine>>,
}

impl TtsRouter {
    pub fn new(engines: Vec<Box<dyn TtsEngine>>) -> Self {
        Self { engines }
    }

    /// Union of all available voices across all engines (no duplicates).
    pub fn available_voices(&self) -> Vec<&'static VoiceMeta> {
        let mut seen = std::collections::HashSet::new();
        let mut all = Vec::new();
        for engine in &self.engines {
            for v in engine.available_voices() {
                if seen.insert(v.id) {
                    all.push(v);
                }
            }
        }
        all
    }

    pub async fn synthesize_wav(&self, text: &str, voice_id: &str) -> Result<PathBuf> {
        for engine in &self.engines {
            if engine.available_voices().iter().any(|v| v.id == voice_id) {
                return engine.synthesize_wav(text, voice_id).await;
            }
        }
        // No engine reported the voice as available. If we have engines at all,
        // let the first one attempt it (it may still resolve an alias); but a
        // router with no engines cannot synthesize anything.
        self.engines
            .first()
            .ok_or_else(|| crate::error::AnubisError::Tts("no TTS engine configured".into()))?
            .synthesize_wav(text, voice_id)
            .await
    }
}
