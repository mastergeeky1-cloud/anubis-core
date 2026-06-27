use crate::error::{AnubisError, Result};
use hound::{WavReader, WavSpec, WavWriter};
use std::io::Cursor;

pub struct Watermarker {
    pub enabled: bool,
}

impl Watermarker {
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    /// Embed `user_id` + `timestamp` as LSB steganography into WAV PCM samples.
    /// The first 96 samples each carry one bit: 64 bits of user_id then 32 bits of timestamp.
    pub fn embed(&self, wav_bytes: &[u8], user_id: i64, timestamp: u32) -> Result<Vec<u8>> {
        if !self.enabled {
            return Ok(wav_bytes.to_vec());
        }

        let cursor = Cursor::new(wav_bytes);
        let mut reader = WavReader::new(cursor)
            .map_err(|e| AnubisError::Wav(e.to_string()))?;

        let spec: WavSpec = reader.spec();

        // Only support 16-bit PCM — pass through anything else unchanged
        if spec.sample_format != hound::SampleFormat::Int || spec.bits_per_sample != 16 {
            return Ok(wav_bytes.to_vec());
        }

        // Build 96-bit payload: user_id (8 bytes LE) + timestamp (4 bytes LE)
        let mut payload = [0u8; 12];
        payload[..8].copy_from_slice(&user_id.to_le_bytes());
        payload[8..].copy_from_slice(&timestamp.to_le_bytes());

        let samples: std::result::Result<Vec<i16>, _> =
            reader.samples::<i16>().collect();
        let samples = samples.map_err(|e| AnubisError::Wav(e.to_string()))?;

        let mut out_buf = Cursor::new(Vec::<u8>::new());
        {
            let mut writer = WavWriter::new(&mut out_buf, spec)
                .map_err(|e| AnubisError::Wav(e.to_string()))?;

            for (i, &sample) in samples.iter().enumerate() {
                let watermarked = if i < 96 {
                    let byte_idx = i / 8;
                    let bit_idx  = i % 8;
                    let bit = ((payload[byte_idx] >> bit_idx) & 1) as i16;
                    (sample & !1) | bit
                } else {
                    sample
                };
                writer
                    .write_sample(watermarked)
                    .map_err(|e| AnubisError::Wav(e.to_string()))?;
            }

            writer
                .finalize()
                .map_err(|e| AnubisError::Wav(e.to_string()))?;
        }

        Ok(out_buf.into_inner())
    }
}
