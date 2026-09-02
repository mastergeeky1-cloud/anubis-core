#[derive(Debug, Clone, Copy)]
pub struct VoiceMeta {
    pub id: &'static str,
    pub lang: &'static str,
    pub name: &'static str,
    pub gender: &'static str,
}

pub const VOICES: &[VoiceMeta] = &[
    // ── English (Piper) ───────────────────────────────────────────────────
    VoiceMeta {
        id: "en_US-amy-medium",
        lang: "en",
        name: "Amy",
        gender: "female",
    },
    VoiceMeta {
        id: "en_US-ryan-high",
        lang: "en",
        name: "Ryan",
        gender: "male",
    },
    VoiceMeta {
        id: "en_US-ryan-medium",
        lang: "en",
        name: "Ryan",
        gender: "male",
    },
    VoiceMeta {
        id: "en_GB-alan-low",
        lang: "en",
        name: "Alan",
        gender: "male",
    },
    VoiceMeta {
        id: "en_US-lessac-medium",
        lang: "en",
        name: "Lessac",
        gender: "female",
    },
    VoiceMeta {
        id: "en_US-lessac-high",
        lang: "en",
        name: "Lessac",
        gender: "female",
    },
    VoiceMeta {
        id: "en_GB-cori-high",
        lang: "en",
        name: "Cori",
        gender: "female",
    },
    VoiceMeta {
        id: "en_GB-northern_english_male-medium",
        lang: "en",
        name: "Northern",
        gender: "male",
    },
    // ── Arabic (Piper, official) ──────────────────────────────────────────
    VoiceMeta {
        id: "ar_JO-kareem-low",
        lang: "ar",
        name: "Kareem",
        gender: "male",
    },
    VoiceMeta {
        id: "ar_JO-kareem-medium",
        lang: "ar",
        name: "Kareem",
        gender: "male",
    },
    // ── Italian ───────────────────────────────────────────────────────────
    VoiceMeta {
        id: "it_IT-riccardo-x_low",
        lang: "it",
        name: "Riccardo",
        gender: "male",
    },
    VoiceMeta {
        id: "it_IT-paola-medium",
        lang: "it",
        name: "Paola",
        gender: "female",
    },
    // ── French ────────────────────────────────────────────────────────────
    VoiceMeta {
        id: "fr_FR-siwis-low",
        lang: "fr",
        name: "Siwis",
        gender: "female",
    },
    VoiceMeta {
        id: "fr_FR-siwis-medium",
        lang: "fr",
        name: "Siwis",
        gender: "female",
    },
    VoiceMeta {
        id: "fr_FR-upmc-medium",
        lang: "fr",
        name: "UPMC",
        gender: "male",
    },
    // ── Spanish ───────────────────────────────────────────────────────────
    VoiceMeta {
        id: "es_ES-mls_10246-low",
        lang: "es",
        name: "MLS",
        gender: "female",
    },
    VoiceMeta {
        id: "es_ES-carlfm-x_low",
        lang: "es",
        name: "Carlos",
        gender: "male",
    },
    VoiceMeta {
        id: "es_ES-davefx-medium",
        lang: "es",
        name: "Dave",
        gender: "male",
    },
    // ── German ────────────────────────────────────────────────────────────
    VoiceMeta {
        id: "de_DE-thorsten-medium",
        lang: "de",
        name: "Thorsten",
        gender: "male",
    },
    VoiceMeta {
        id: "de_DE-eva_k-x_low",
        lang: "de",
        name: "Eva",
        gender: "female",
    },
    VoiceMeta {
        id: "de_DE-thorsten-high",
        lang: "de",
        name: "Thorsten",
        gender: "male",
    },
    // ── Russian ───────────────────────────────────────────────────────────
    VoiceMeta {
        id: "ru_RU-irina-medium",
        lang: "ru",
        name: "Irina",
        gender: "female",
    },
    VoiceMeta {
        id: "ru_RU-ruslan-medium",
        lang: "ru",
        name: "Ruslan",
        gender: "male",
    },
    VoiceMeta {
        id: "ru_RU-denis-medium",
        lang: "ru",
        name: "Denis",
        gender: "male",
    },
    // ── Hindi ─────────────────────────────────────────────────────────────
    VoiceMeta {
        id: "hi_IN-pratham-medium",
        lang: "hi",
        name: "Pratham",
        gender: "male",
    },
    VoiceMeta {
        id: "hi_IN-priyamvada-medium",
        lang: "hi",
        name: "Priyamvada",
        gender: "female",
    },
    // ── Turkish ────────────────────────────────────────────────────────────
    VoiceMeta {
        id: "tr_TR-dfki-medium",
        lang: "tr",
        name: "DFKI",
        gender: "female",
    },
    // ── Portuguese ────────────────────────────────────────────────────────
    VoiceMeta {
        id: "pt_BR-faber-medium",
        lang: "pt",
        name: "Faber",
        gender: "male",
    },
    // ── Kokoro (Apache-2.0) local neural TTS — full catalogue ─────────────
    // American English — female
    VoiceMeta {
        id: "af_heart",
        lang: "en",
        name: "Heart (Kokoro)",
        gender: "female",
    },
    VoiceMeta {
        id: "af_bella",
        lang: "en",
        name: "Bella (Kokoro)",
        gender: "female",
    },
    VoiceMeta {
        id: "af_sky",
        lang: "en",
        name: "Sky (Kokoro)",
        gender: "female",
    },
    VoiceMeta {
        id: "af_nova",
        lang: "en",
        name: "Nova (Kokoro)",
        gender: "female",
    },
    VoiceMeta {
        id: "af_sarah",
        lang: "en",
        name: "Sarah (Kokoro)",
        gender: "female",
    },
    VoiceMeta {
        id: "af_alloy",
        lang: "en",
        name: "Alloy (Kokoro)",
        gender: "female",
    },
    VoiceMeta {
        id: "af_nicole",
        lang: "en",
        name: "Nicole (Kokoro)",
        gender: "female",
    },
    VoiceMeta {
        id: "af_jessica",
        lang: "en",
        name: "Jessica (Kokoro)",
        gender: "female",
    },
    VoiceMeta {
        id: "af_river",
        lang: "en",
        name: "River (Kokoro)",
        gender: "female",
    },
    VoiceMeta {
        id: "af_aoede",
        lang: "en",
        name: "Aoede (Kokoro)",
        gender: "female",
    },
    VoiceMeta {
        id: "af_kore",
        lang: "en",
        name: "Kore (Kokoro)",
        gender: "female",
    },
    // American English — male
    VoiceMeta {
        id: "am_adam",
        lang: "en",
        name: "Adam (Kokoro)",
        gender: "male",
    },
    VoiceMeta {
        id: "am_michael",
        lang: "en",
        name: "Michael (Kokoro)",
        gender: "male",
    },
    VoiceMeta {
        id: "am_onyx",
        lang: "en",
        name: "Onyx (Kokoro)",
        gender: "male",
    },
    VoiceMeta {
        id: "am_echo",
        lang: "en",
        name: "Echo (Kokoro)",
        gender: "male",
    },
    VoiceMeta {
        id: "am_eric",
        lang: "en",
        name: "Eric (Kokoro)",
        gender: "male",
    },
    VoiceMeta {
        id: "am_liam",
        lang: "en",
        name: "Liam (Kokoro)",
        gender: "male",
    },
    VoiceMeta {
        id: "am_puck",
        lang: "en",
        name: "Puck (Kokoro)",
        gender: "male",
    },
    VoiceMeta {
        id: "am_fenrir",
        lang: "en",
        name: "Fenrir (Kokoro)",
        gender: "male",
    },
    VoiceMeta {
        id: "am_santa",
        lang: "en",
        name: "Santa (Kokoro)",
        gender: "male",
    },
    // British English — female
    VoiceMeta {
        id: "bf_emma",
        lang: "en",
        name: "Emma (UK)",
        gender: "female",
    },
    VoiceMeta {
        id: "bf_alice",
        lang: "en",
        name: "Alice (UK)",
        gender: "female",
    },
    VoiceMeta {
        id: "bf_isabella",
        lang: "en",
        name: "Isabella (UK)",
        gender: "female",
    },
    VoiceMeta {
        id: "bf_lily",
        lang: "en",
        name: "Lily (UK)",
        gender: "female",
    },
    // British English — male
    VoiceMeta {
        id: "bm_george",
        lang: "en",
        name: "George (UK)",
        gender: "male",
    },
    VoiceMeta {
        id: "bm_daniel",
        lang: "en",
        name: "Daniel (UK)",
        gender: "male",
    },
    VoiceMeta {
        id: "bm_fred",
        lang: "en",
        name: "Fred (UK)",
        gender: "male",
    },
    VoiceMeta {
        id: "bm_leo",
        lang: "en",
        name: "Leo (UK)",
        gender: "male",
    },
    VoiceMeta {
        id: "bm_pierre",
        lang: "fr",
        name: "Pierre (FR)",
        gender: "male",
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

/// All Arabic voices (official + Kokoro). Surfaced under the 🇸🇦 header and
/// used by the tests.
#[allow(dead_code)]
pub fn arabic_voices() -> Vec<&'static VoiceMeta> {
    for_lang("ar")
}

/// Best (highest-quality, most natural) voice id per supported language.
pub const BEST_VOICE_PER_LANG: &[(&str, &str)] = &[
    ("en", "en_US-ryan-high"),
    ("ar", "ar_JO-kareem-medium"),
    ("it", "it_IT-paola-medium"),
    ("fr", "fr_FR-siwis-medium"),
    ("es", "es_ES-davefx-medium"),
    ("de", "de_DE-thorsten-high"),
    ("ru", "ru_RU-irina-medium"),
    ("hi", "hi_IN-priyamvada-medium"),
    ("tr", "tr_TR-dfki-medium"),
    ("pt", "pt_BR-faber-medium"),
];

/// Highest-quality natural voice for a language (hardcoded best pick).
pub fn best_voice_for_lang(lang: &str) -> &'static str {
    BEST_VOICE_PER_LANG
        .iter()
        .find(|(l, _)| *l == lang)
        .map(|(_, v)| *v)
        .unwrap_or("en_US-amy-medium")
}

pub fn default_for_lang(lang: &str) -> &'static str {
    best_voice_for_lang(lang)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arabic_has_real_voices() {
        let ar = arabic_voices();
        assert!(
            ar.len() >= 2,
            "expected >=2 Arabic voices, got {}",
            ar.len()
        );
        let ids: Vec<&str> = ar.iter().map(|v| v.id).collect();
        assert!(ids.contains(&"ar_JO-kareem-low"));
        assert!(ids.contains(&"ar_JO-kareem-medium"));
        assert!(ar.iter().all(|v| !v.id.is_empty() && !v.name.is_empty()));
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

    #[test]
    fn best_voice_per_lang_resolves_to_real_voice() {
        for (lang, _) in BEST_VOICE_PER_LANG {
            let best = best_voice_for_lang(lang);
            assert!(
                find(best).is_some(),
                "best voice {best} for lang {lang} is not in catalogue"
            );
        }
    }

    #[test]
    fn italian_has_female_and_male() {
        let it = for_lang("it");
        assert!(it.iter().any(|v| v.gender == "female"));
        assert!(it.iter().any(|v| v.gender == "male"));
        assert!(it.iter().any(|v| v.id.contains("riccardo")));
        assert!(it.iter().any(|v| v.id.contains("paola")));
    }
}
