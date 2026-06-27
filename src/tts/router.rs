use crate::error::Result;
use crate::tts::engine::TtsEngine;
use crate::tts::voices::VoiceMeta;
use std::path::PathBuf;

/// Routes synthesis requests across a chain of TTS engines.
/// Tries each engine in order; the first one that has the requested voice is used.
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

    /// Synthesize with the first engine that reports `voice_id` as available.
    /// Falls back to the first engine if none claim it (lets the engine produce its own error).
    pub async fn synthesize_wav(&self, text: &str, voice_id: &str) -> Result<PathBuf> {
        for engine in &self.engines {
            if engine.available_voices().iter().any(|v| v.id == voice_id) {
                return engine.synthesize_wav(text, voice_id).await;
            }
        }
        // fallback: let the primary engine handle unknown voice (will return VoiceNotFound)
        self.engines[0].synthesize_wav(text, voice_id).await
    }
}
