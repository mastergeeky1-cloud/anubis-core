use crate::error::{AnubisError, Result};
use hound::{WavReader, WavSpec, WavWriter};
use std::io::Cursor;

/// Robust LSB steganography for cloned-voice provenance.
///
/// Embeds `user_id` (i64) + `timestamp` (u32) + a CRC32 checksum into the
/// low bit of 16-bit PCM samples, **redundantly** across multiple blocks.
/// The redundancy lets `decode` recover the tag even if some samples are
/// modified (e.g. a naive volume/bit-perturbation removal attempt), and the
/// CRC catches corruption so we never report a wrong id.
///
/// Wire layout per block (12 payload bytes + 4 checksum bytes = 128 bits):
///   bits 0..64   user_id  (LE)
///   bits 64..96  timestamp (LE)
///   bits 96..128 crc32 of the 12-byte payload
/// The 128 bits are repeated `REPETITIONS` times across the first N samples,
/// and the decoded value is chosen by majority vote per bit.
///
/// This is a stronger baseline than plain single-bit LSB; for truly robust
/// anti-removal, pair it with a neural watermark (e.g. Chatterbox "Perth").
pub struct Watermarker {
    pub enabled: bool,
}

/// Number of redundant copies of the 128-bit payload block.
const REPETITIONS: usize = 3;
const PAYLOAD_BYTES: usize = 16; // 12 payload + 4 crc
const BITS_PER_BLOCK: usize = PAYLOAD_BYTES * 8; // 128

fn crc32(data: &[u8]) -> u32 {
    data.iter().fold(0xFFFFFFFFu32, |crc, &byte| {
        let mut crc = crc ^ (byte as u32);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xEDB88320
            } else {
                crc >> 1
            };
        }
        crc
    })
}

impl Watermarker {
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    pub fn embed(&self, wav_bytes: &[u8], user_id: i64, timestamp: u32) -> Result<Vec<u8>> {
        if !self.enabled {
            return Ok(wav_bytes.to_vec());
        }

        let cursor = Cursor::new(wav_bytes);
        let mut reader = WavReader::new(cursor).map_err(|e| AnubisError::Wav(e.to_string()))?;
        let spec: WavSpec = reader.spec();

        if spec.sample_format != hound::SampleFormat::Int || spec.bits_per_sample != 16 {
            return Ok(wav_bytes.to_vec());
        }

        let mut payload = [0u8; PAYLOAD_BYTES];
        payload[..8].copy_from_slice(&user_id.to_le_bytes());
        payload[8..12].copy_from_slice(&timestamp.to_le_bytes());
        let csum = crc32(&payload[..12]);
        payload[12..].copy_from_slice(&csum.to_le_bytes());

        let samples: std::result::Result<Vec<i16>, _> = reader.samples::<i16>().collect();
        let samples = samples.map_err(|e| AnubisError::Wav(e.to_string()))?;

        // Lay out the repeated payload.
        let total_bits = BITS_PER_BLOCK * REPETITIONS;
        let mut repeated = Vec::with_capacity(total_bits);
        for _ in 0..REPETITIONS {
            for &byte in payload.iter() {
                for bit in 0..8 {
                    repeated.push(((byte >> bit) & 1) as i16);
                }
            }
        }

        let mut out_buf = Cursor::new(Vec::<u8>::new());
        {
            let mut writer =
                WavWriter::new(&mut out_buf, spec).map_err(|e| AnubisError::Wav(e.to_string()))?;
            for (i, &sample) in samples.iter().enumerate() {
                let watermarked = if i < total_bits {
                    (sample & !1) | repeated[i]
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

    /// Recover `(user_id, timestamp)` from a watermarked WAV, or `None` if the
    /// tag is absent/corrupt. Uses majority vote across redundant blocks and a
    /// CRC check. Forensic/provenance tool — used by tests and admins alike.
    #[allow(dead_code)]
    pub fn decode(&self, wav_bytes: &[u8]) -> Option<(i64, u32)> {
        let cursor = Cursor::new(wav_bytes);
        let mut reader = WavReader::new(cursor).ok()?;
        let spec = reader.spec();
        if spec.sample_format != hound::SampleFormat::Int || spec.bits_per_sample != 16 {
            return None;
        }
        let samples: std::result::Result<Vec<i16>, _> = reader.samples::<i16>().collect();
        let samples = samples.ok()?;
        let total_bits = BITS_PER_BLOCK * REPETITIONS;
        if samples.len() < total_bits {
            return None;
        }

        // Majority-vote each of the 128 payload bits across the REPETITIONS copies.
        let mut voted = [0u8; PAYLOAD_BYTES];
        for bit_pos in 0..BITS_PER_BLOCK {
            let mut ones = 0;
            for rep in 0..REPETITIONS {
                let idx = rep * BITS_PER_BLOCK + bit_pos;
                ones += (samples[idx] & 1) as usize;
            }
            let bit = if ones * 2 >= REPETITIONS { 1u8 } else { 0u8 };
            voted[bit_pos / 8] |= bit << (bit_pos % 8);
        }

        // Validate checksum.
        let csum = u32::from_le_bytes([voted[12], voted[13], voted[14], voted[15]]);
        if crc32(&voted[..12]) != csum {
            return None;
        }

        let user_id = i64::from_le_bytes(voted[..8].try_into().expect("8 bytes"));
        let timestamp = u32::from_le_bytes([voted[8], voted[9], voted[10], voted[11]]);
        Some((user_id, timestamp))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hound::{SampleFormat, WavSpec, WavWriter};

    fn make_wav(sample_count: usize) -> Vec<u8> {
        let spec = WavSpec {
            channels: 1,
            sample_rate: 8000,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };
        let mut buf = Cursor::new(Vec::new());
        {
            let mut w = WavWriter::new(&mut buf, spec).unwrap();
            for i in 0..sample_count {
                w.write_sample(((i as i32 * 37) % 32000 - 16000) as i16)
                    .unwrap();
            }
        }
        buf.into_inner()
    }

    #[test]
    fn roundtrip_recovers_metadata() {
        let w = make_wav(2000);
        let wm = Watermarker::new(true);
        let tagged = wm.embed(&w, 123456789, 987654321).unwrap();
        let r = wm.decode(&tagged).unwrap();
        assert_eq!(r, (123456789, 987654321));
    }

    #[test]
    fn disabled_returns_original() {
        let w = make_wav(500);
        let out = Watermarker::new(false).embed(&w, 1, 2).unwrap();
        assert_eq!(out, w);
    }

    #[test]
    fn survives_bit_flips() {
        let w = make_wav(2000);
        let wm = Watermarker::new(true);
        let mut tagged = wm.embed(&w, 42, 99).unwrap();

        // Corrupt ~10% of the watermarked sample LSBs.
        // WAV header is 44 bytes; samples start at offset 44.
        let header_size = 44usize;
        let mut rng = 0x12345678u64;
        let in_scope = BITS_PER_BLOCK * REPETITIONS;
        let corrupt_count = in_scope / 10; // 10%
        for _ in 0..corrupt_count {
            rng = rng
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let sample_idx = (rng as usize) % in_scope;
            // Each sample is 2 bytes; LSB is the first byte of the pair.
            let byte_offset = header_size + sample_idx * 2;
            if byte_offset + 1 < tagged.len() {
                tagged[byte_offset] ^= 1;
            }
        }

        // With 3x redundancy + majority vote, the id should survive.
        let r = wm.decode(&tagged).unwrap();
        assert_eq!(r.0, 42);
    }

    #[test]
    fn empty_or_corrupt_returns_none() {
        let w = make_wav(500);
        let wm = Watermarker::new(true);
        // Original (no tag) — low chance of a valid CRC collision.
        assert!(wm.decode(&w).is_none());
        // Truncated — too few samples.
        let tagged = wm.embed(&w, 7, 8).unwrap();
        assert!(wm.decode(&tagged[..100]).is_none());
    }
}
