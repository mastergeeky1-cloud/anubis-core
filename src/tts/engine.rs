use crate::error::Result;
use crate::tts::voices::VoiceMeta;
use async_trait::async_trait;
use std::path::PathBuf;

#[async_trait]
pub trait TtsEngine: Send + Sync {
    fn available_voices(&self) -> Vec<&'static VoiceMeta>;
    async fn synthesize_wav(&self, text: &str, voice_id: &str) -> Result<PathBuf>;
}
