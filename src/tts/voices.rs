/// Static voice catalogue.
///
/// Two engine families:
///   • Piper  — CPU, MIT/Apache onnx models in `<voices_dir>/<lang>/<id>.onnx`
///   • Kokoro — local neural sidecar (Apache-2.0) for `af_/am_/bf_/bm_/…` ids
///
/// `source` tells the UI which engine a voice belongs to (for grouping and so
/// a voice that is listed but not yet downloaded is shown as "install via /setup").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceSource {
    Piper,
    Kokoro,
}

#[derive(Debug, Clone, Copy)]
pub struct VoiceMeta {
    pub id: &'static str,
    pub lang: &'static str,
    pub name: &'static str,
    pub gender: &'static str,
    pub quality: &'static str,
    pub source: VoiceSource,
}

impl VoiceMeta {
    /// Human label used in the voice gallery (icon + name + quality).
    pub fn label(&self) -> String {
        let icon = if self.gender == "female" {
            "♀"
        } else {
            "♂"
        };
        format!("{} {} ({})", icon, self.name, self.quality)
    }

    /// Flag/emoji for the language group header in the gallery.
    pub fn flag(&self) -> &'static str {
        lang_flag(self.lang)
    }

    /// Engine name this voice is synthesized with (shown in the gallery).
    pub fn source(&self) -> &'static str {
        source_name(self.source)
    }
}

pub fn lang_flag(lang: &str) -> &'static str {
    match lang {
        "en" => "🇬🇧",
        "ar" => "🇸🇦",
        "it" => "🇮🇹",
        "fr" => "🇫🇷",
        "es" => "🇪🇸",
        "de" => "🇩🇪",
        "ru" => "🇷🇺",
        "hi" => "🇮🇳",
        "tr" => "🇹🇷",
        "pt" => "🇧🇷",
        _ => "🌐",
    }
}

pub fn source_name(src: VoiceSource) -> &'static str {
    match src {
        VoiceSource::Piper => "Piper",
        VoiceSource::Kokoro => "Kokoro",
    }
}

pub const VOICES: &[VoiceMeta] = &[
    // ── English (Piper) ───────────────────────────────────────────────────
    VoiceMeta {
        id: "en_US-amy-medium",
        lang: "en",
        name: "Amy",
        gender: "female",
        quality: "medium",
        source: VoiceSource::Piper,
    },
    VoiceMeta {
        id: "en_US-amy-high",
        lang: "en",
        name: "Amy",
        gender: "female",
        quality: "high",
        source: VoiceSource::Piper,
    },
    VoiceMeta {
        id: "en_US-ryan-high",
        lang: "en",
        name: "Ryan",
        gender: "male",
        quality: "high",
        source: VoiceSource::Piper,
    },
    VoiceMeta {
        id: "en_US-ryan-medium",
        lang: "en",
        name: "Ryan",
        gender: "male",
        quality: "medium",
        source: VoiceSource::Piper,
    },
    VoiceMeta {
        id: "en_GB-alan-low",
        lang: "en",
        name: "Alan",
        gender: "male",
        quality: "low",
        source: VoiceSource::Piper,
    },
    VoiceMeta {
        id: "en_US-lessac-medium",
        lang: "en",
        name: "Lessac",
        gender: "female",
        quality: "medium",
        source: VoiceSource::Piper,
    },
    VoiceMeta {
        id: "en_US-lessac-high",
        lang: "en",
        name: "Lessac",
        gender: "female",
        quality: "high",
        source: VoiceSource::Piper,
    },
    VoiceMeta {
        id: "en_US-hubert-high",
        lang: "en",
        name: "Hubert",
        gender: "male",
        quality: "high",
        source: VoiceSource::Piper,
    },
    VoiceMeta {
        id: "en_GB-cori-high",
        lang: "en",
        name: "Cori",
        gender: "female",
        quality: "high",
        source: VoiceSource::Piper,
    },
    VoiceMeta {
        id: "en_GB-northern_english_male-medium",
        lang: "en",
        name: "Northern",
        gender: "male",
        quality: "medium",
        source: VoiceSource::Piper,
    },
    // ── Arabic (Piper, official) ──────────────────────────────────────────
    VoiceMeta {
        id: "ar_JO-kareem-low",
        lang: "ar",
        name: "Kareem",
        gender: "male",
        quality: "low",
        source: VoiceSource::Piper,
    },
    VoiceMeta {
        id: "ar_JO-kareem-medium",
        lang: "ar",
        name: "Kareem",
        gender: "male",
        quality: "medium",
        source: VoiceSource::Piper,
    },
    // ── Arabic (community Piper packs — install via scripts/setup.sh) ──────
    VoiceMeta {
        id: "ar-zayd0-diacritized",
        lang: "ar",
        name: "Zayd0 (Diacritized)",
        gender: "male",
        quality: "medium",
        source: VoiceSource::Piper,
    },
    VoiceMeta {
        id: "ar_AE-emirati-female",
        lang: "ar",
        name: "Emirati (Female)",
        gender: "female",
        quality: "medium",
        source: VoiceSource::Piper,
    },
    // ── Italian ───────────────────────────────────────────────────────────
    VoiceMeta {
        id: "it_IT-riccardo-x_low",
        lang: "it",
        name: "Riccardo",
        gender: "male",
        quality: "low",
        source: VoiceSource::Piper,
    },
    VoiceMeta {
        id: "it_IT-paola-medium",
        lang: "it",
        name: "Paola",
        gender: "female",
        quality: "medium",
        source: VoiceSource::Piper,
    },
    // ── French ────────────────────────────────────────────────────────────
    VoiceMeta {
        id: "fr_FR-siwis-medium",
        lang: "fr",
        name: "Siwis",
        gender: "female",
        quality: "medium",
        source: VoiceSource::Piper,
    },
    VoiceMeta {
        id: "fr_FR-upmc-medium",
        lang: "fr",
        name: "UPMC",
        gender: "male",
        quality: "medium",
        source: VoiceSource::Piper,
    },
    // ── Spanish ───────────────────────────────────────────────────────────
    VoiceMeta {
        id: "es_ES-mls_10246-low",
        lang: "es",
        name: "MLS",
        gender: "female",
        quality: "low",
        source: VoiceSource::Piper,
    },
    VoiceMeta {
        id: "es_ES-carlfm-x_low",
        lang: "es",
        name: "Carlos",
        gender: "male",
        quality: "low",
        source: VoiceSource::Piper,
    },
    // ── German ────────────────────────────────────────────────────────────
    VoiceMeta {
        id: "de_DE-thorsten-medium",
        lang: "de",
        name: "Thorsten",
        gender: "male",
        quality: "medium",
        source: VoiceSource::Piper,
    },
    VoiceMeta {
        id: "de_DE-eva_k-x_low",
        lang: "de",
        name: "Eva",
        gender: "female",
        quality: "low",
        source: VoiceSource::Piper,
    },
    VoiceMeta {
        id: "de_DE-thorsten-high",
        lang: "de",
        name: "Thorsten",
        gender: "male",
        quality: "high",
        source: VoiceSource::Piper,
    },
    // ── Russian ───────────────────────────────────────────────────────────
    VoiceMeta {
        id: "ru_RU-irinia-medium",
        lang: "ru",
        name: "Irinia",
        gender: "female",
        quality: "medium",
        source: VoiceSource::Piper,
    },
    VoiceMeta {
        id: "ru_RU-ruslan-medium",
        lang: "ru",
        name: "Ruslan",
        gender: "male",
        quality: "medium",
        source: VoiceSource::Piper,
    },
    // ── Hindi ─────────────────────────────────────────────────────────────
    VoiceMeta {
        id: "hi_IN-deepika-medium",
        lang: "hi",
        name: "Deepika",
        gender: "female",
        quality: "medium",
        source: VoiceSource::Piper,
    },
    // ── Turkish ────────────────────────────────────────────────────────────
    VoiceMeta {
        id: "tr_TR-dfki-medium",
        lang: "tr",
        name: "DFKI",
        gender: "female",
        quality: "medium",
        source: VoiceSource::Piper,
    },
    // ── Portuguese ────────────────────────────────────────────────────────
    VoiceMeta {
        id: "pt_BR-faber-medium",
        lang: "pt",
        name: "Faber",
        gender: "male",
        quality: "medium",
        source: VoiceSource::Piper,
    },
    VoiceMeta {
        id: "pt_PT-tugao-medium",
        lang: "pt",
        name: "Tugão",
        gender: "male",
        quality: "medium",
        source: VoiceSource::Piper,
    },
    // ── Kokoro (Apache-2.0) local neural TTS — full catalogue ─────────────
    // American English — female
    VoiceMeta {
        id: "af_heart",
        lang: "en",
        name: "Heart (Kokoro)",
        gender: "female",
        quality: "high",
        source: VoiceSource::Kokoro,
    },
    VoiceMeta {
        id: "af_bella",
        lang: "en",
        name: "Bella (Kokoro)",
        gender: "female",
        quality: "high",
        source: VoiceSource::Kokoro,
    },
    VoiceMeta {
        id: "af_sky",
        lang: "en",
        name: "Sky (Kokoro)",
        gender: "female",
        quality: "high",
        source: VoiceSource::Kokoro,
    },
    VoiceMeta {
        id: "af_nova",
        lang: "en",
        name: "Nova (Kokoro)",
        gender: "female",
        quality: "high",
        source: VoiceSource::Kokoro,
    },
    VoiceMeta {
        id: "af_sarah",
        lang: "en",
        name: "Sarah (Kokoro)",
        gender: "female",
        quality: "high",
        source: VoiceSource::Kokoro,
    },
    VoiceMeta {
        id: "af_alloy",
        lang: "en",
        name: "Alloy (Kokoro)",
        gender: "female",
        quality: "high",
        source: VoiceSource::Kokoro,
    },
    VoiceMeta {
        id: "af_nicole",
        lang: "en",
        name: "Nicole (Kokoro)",
        gender: "female",
        quality: "high",
        source: VoiceSource::Kokoro,
    },
    VoiceMeta {
        id: "af_jessica",
        lang: "en",
        name: "Jessica (Kokoro)",
        gender: "female",
        quality: "high",
        source: VoiceSource::Kokoro,
    },
    VoiceMeta {
        id: "af_river",
        lang: "en",
        name: "River (Kokoro)",
        gender: "female",
        quality: "high",
        source: VoiceSource::Kokoro,
    },
    VoiceMeta {
        id: "af_aoede",
        lang: "en",
        name: "Aoede (Kokoro)",
        gender: "female",
        quality: "high",
        source: VoiceSource::Kokoro,
    },
    VoiceMeta {
        id: "af_kore",
        lang: "en",
        name: "Kore (Kokoro)",
        gender: "female",
        quality: "high",
        source: VoiceSource::Kokoro,
    },
    // American English — male
    VoiceMeta {
        id: "am_adam",
        lang: "en",
        name: "Adam (Kokoro)",
        gender: "male",
        quality: "high",
        source: VoiceSource::Kokoro,
    },
    VoiceMeta {
        id: "am_michael",
        lang: "en",
        name: "Michael (Kokoro)",
        gender: "male",
        quality: "high",
        source: VoiceSource::Kokoro,
    },
    VoiceMeta {
        id: "am_onyx",
        lang: "en",
        name: "Onyx (Kokoro)",
        gender: "male",
        quality: "high",
        source: VoiceSource::Kokoro,
    },
    VoiceMeta {
        id: "am_echo",
        lang: "en",
        name: "Echo (Kokoro)",
        gender: "male",
        quality: "high",
        source: VoiceSource::Kokoro,
    },
    VoiceMeta {
        id: "am_eric",
        lang: "en",
        name: "Eric (Kokoro)",
        gender: "male",
        quality: "high",
        source: VoiceSource::Kokoro,
    },
    VoiceMeta {
        id: "am_liam",
        lang: "en",
        name: "Liam (Kokoro)",
        gender: "male",
        quality: "high",
        source: VoiceSource::Kokoro,
    },
    VoiceMeta {
        id: "am_puck",
        lang: "en",
        name: "Puck (Kokoro)",
        gender: "male",
        quality: "high",
        source: VoiceSource::Kokoro,
    },
    VoiceMeta {
        id: "am_fenrir",
        lang: "en",
        name: "Fenrir (Kokoro)",
        gender: "male",
        quality: "high",
        source: VoiceSource::Kokoro,
    },
    VoiceMeta {
        id: "am_santa",
        lang: "en",
        name: "Santa (Kokoro)",
        gender: "male",
        quality: "high",
        source: VoiceSource::Kokoro,
    },
    // British English — female
    VoiceMeta {
        id: "bf_emma",
        lang: "en",
        name: "Emma (UK)",
        gender: "female",
        quality: "high",
        source: VoiceSource::Kokoro,
    },
    VoiceMeta {
        id: "bf_alice",
        lang: "en",
        name: "Alice (UK)",
        gender: "female",
        quality: "high",
        source: VoiceSource::Kokoro,
    },
    VoiceMeta {
        id: "bf_isabella",
        lang: "en",
        name: "Isabella (UK)",
        gender: "female",
        quality: "high",
        source: VoiceSource::Kokoro,
    },
    VoiceMeta {
        id: "bf_lily",
        lang: "en",
        name: "Lily (UK)",
        gender: "female",
        quality: "high",
        source: VoiceSource::Kokoro,
    },
    // British English — male
    VoiceMeta {
        id: "bm_george",
        lang: "en",
        name: "George (UK)",
        gender: "male",
        quality: "high",
        source: VoiceSource::Kokoro,
    },
    VoiceMeta {
        id: "bm_daniel",
        lang: "en",
        name: "Daniel (UK)",
        gender: "male",
        quality: "high",
        source: VoiceSource::Kokoro,
    },
    VoiceMeta {
        id: "bm_fred",
        lang: "en",
        name: "Fred (UK)",
        gender: "male",
        quality: "high",
        source: VoiceSource::Kokoro,
    },
    VoiceMeta {
        id: "bm_leo",
        lang: "en",
        name: "Leo (UK)",
        gender: "male",
        quality: "high",
        source: VoiceSource::Kokoro,
    },
    VoiceMeta {
        id: "bm_pierre",
        lang: "fr",
        name: "Pierre (FR)",
        gender: "male",
        quality: "high",
        source: VoiceSource::Kokoro,
    },
];

pub fn find(id: &str) -> Option<&'static VoiceMeta> {
    VOICES.iter().find(|v| v.id == id)
}

/// Voices for one language, in catalogue order.
#[allow(dead_code)]
pub fn for_lang(lang: &str) -> Vec<&'static VoiceMeta> {
    VOICES.iter().filter(|v| v.lang == lang).collect()
}

/// All Arabic voices (official + community). Surfaced under the 🇸🇦 header and
/// used by the tests.
#[allow(dead_code)]
pub fn arabic_voices() -> Vec<&'static VoiceMeta> {
    for_lang("ar")
}

pub fn default_for_lang(lang: &str) -> &'static str {
    VOICES
        .iter()
        .find(|v| v.lang == lang)
        .map(|v| v.id)
        .unwrap_or("en_US-amy-medium")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arabic_has_multiple_real_voices() {
        let ar = arabic_voices();
        // Official Kareem (low + medium) plus the two community packs.
        assert!(
            ar.len() >= 4,
            "expected >=4 Arabic voices, got {}",
            ar.len()
        );
        let ids: Vec<&str> = ar.iter().map(|v| v.id).collect();
        assert!(ids.contains(&"ar_JO-kareem-low"));
        assert!(ids.contains(&"ar_JO-kareem-medium"));
        assert!(ids.contains(&"ar-zayd0-diacritized"));
        assert!(ids.contains(&"ar_AE-emirati-female"));
        // No fake/empty ids.
        assert!(ar.iter().all(|v| !v.id.is_empty() && !v.name.is_empty()));
    }

    #[test]
    fn flag_covers_all_catalogue_langs() {
        for v in VOICES {
            let f = lang_flag(v.lang);
            assert_ne!(f, "🌐", "voice {} has unmapped lang {}", v.id, v.lang);
        }
    }

    #[test]
    fn label_includes_gender_icon_and_quality() {
        let v = find("en_US-amy-medium").unwrap();
        let l = v.label();
        assert!(l.contains("Amy"));
        assert!(l.contains("medium"));
        assert!(l.contains('♀') || l.contains('♂'));
    }

    #[test]
    fn source_name_roundtrips() {
        assert_eq!(source_name(VoiceSource::Piper), "Piper");
        assert_eq!(source_name(VoiceSource::Kokoro), "Kokoro");
    }

    #[test]
    fn default_falls_back_to_english() {
        assert_eq!(default_for_lang("zz"), "en_US-amy-medium");
        assert!(find(default_for_lang("ar")).is_some());
        assert!(find(default_for_lang("fr")).is_some());
    }

    #[test]
    fn every_voice_id_is_unique() {
        let mut seen = std::collections::HashSet::new();
        for v in VOICES {
            assert!(seen.insert(v.id), "duplicate voice id: {}", v.id);
        }
    }
}
