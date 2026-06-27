/// Static voice catalogue — maps voice IDs to Piper model file names.
/// Model files are stored at: <voices_dir>/<lang>/<id>.onnx
#[derive(Debug, Clone, Copy)]
pub struct VoiceMeta {
    pub id: &'static str,
    pub lang: &'static str,
    pub name: &'static str,
    pub gender: &'static str,
    pub quality: &'static str,
}

pub const VOICES: &[VoiceMeta] = &[
    // English
    VoiceMeta { id: "en_US-amy-medium",       lang: "en", name: "Amy",      gender: "female", quality: "medium" },
    VoiceMeta { id: "en_US-ryan-high",         lang: "en", name: "Ryan",     gender: "male",   quality: "high"   },
    VoiceMeta { id: "en_GB-alan-low",          lang: "en", name: "Alan",     gender: "male",   quality: "low"    },
    VoiceMeta { id: "en_US-lessac-medium",     lang: "en", name: "Lessac",   gender: "female", quality: "medium" },
    // Arabic
    VoiceMeta { id: "ar_JO-kareem-medium",     lang: "ar", name: "Kareem",   gender: "male",   quality: "medium" },
    // Italian
    VoiceMeta { id: "it_IT-riccardo-x_low",   lang: "it", name: "Riccardo", gender: "male",   quality: "low"    },
    VoiceMeta { id: "it_IT-paola-medium",      lang: "it", name: "Paola",    gender: "female", quality: "medium" },
    // French
    VoiceMeta { id: "fr_FR-siwis-medium",      lang: "fr", name: "Siwis",    gender: "female", quality: "medium" },
    VoiceMeta { id: "fr_FR-upmc-medium",       lang: "fr", name: "UPMC",     gender: "male",   quality: "medium" },
    // Spanish
    VoiceMeta { id: "es_ES-mls_10246-low",     lang: "es", name: "MLS",      gender: "female", quality: "low"    },
    VoiceMeta { id: "es_ES-carlfm-x_low",      lang: "es", name: "Carlos",   gender: "male",   quality: "low"    },
    // German
    VoiceMeta { id: "de_DE-thorsten-medium",   lang: "de", name: "Thorsten", gender: "male",   quality: "medium" },
    VoiceMeta { id: "de_DE-eva_k-x_low",       lang: "de", name: "Eva",      gender: "female", quality: "low"    },
    // Russian
    VoiceMeta { id: "ru_RU-irinia-medium",     lang: "ru", name: "Irinia",   gender: "female", quality: "medium" },
    VoiceMeta { id: "ru_RU-ruslan-medium",     lang: "ru", name: "Ruslan",   gender: "male",   quality: "medium" },
    // Hindi
    VoiceMeta { id: "hi_IN-deepika-medium",    lang: "hi", name: "Deepika",  gender: "female", quality: "medium" },
    // Turkish
    VoiceMeta { id: "tr_TR-dfki-medium",       lang: "tr", name: "DFKI",     gender: "female", quality: "medium" },
    // Portuguese
    VoiceMeta { id: "pt_BR-faber-medium",      lang: "pt", name: "Faber",    gender: "male",   quality: "medium" },
    VoiceMeta { id: "pt_PT-tugao-medium",      lang: "pt", name: "Tugão",    gender: "male",   quality: "medium" },
];

pub fn find(id: &str) -> Option<&'static VoiceMeta> {
    VOICES.iter().find(|v| v.id == id)
}

#[allow(dead_code)]
pub fn for_lang(lang: &str) -> Vec<&'static VoiceMeta> {
    VOICES.iter().filter(|v| v.lang == lang).collect()
}

pub fn default_for_lang(lang: &str) -> &'static str {
    VOICES
        .iter()
        .find(|v| v.lang == lang)
        .map(|v| v.id)
        .unwrap_or("en_US-amy-medium")
}
