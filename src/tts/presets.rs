/// Named voice presets surfaced in the /presets UI.
#[derive(Debug, Clone, Copy)]
pub struct VoicePreset {
    pub name: &'static str,
    pub voice_id: &'static str,
    pub desc: &'static str,
    pub lang: &'static str,
}

pub const PRESETS: &[VoicePreset] = &[
    VoicePreset {
        name: "Amy (EN)",
        voice_id: "en_US-amy-medium",
        desc: "Clear American female voice",
        lang: "en",
    },
    VoicePreset {
        name: "Ryan (EN)",
        voice_id: "en_US-ryan-high",
        desc: "High-quality American male voice",
        lang: "en",
    },
    VoicePreset {
        name: "Alan (EN)",
        voice_id: "en_GB-alan-low",
        desc: "Crisp British male voice",
        lang: "en",
    },
    VoicePreset {
        name: "Kareem (AR)",
        voice_id: "ar_JO-kareem-medium",
        desc: "Arabic male voice",
        lang: "ar",
    },
    VoicePreset {
        name: "Paola (IT)",
        voice_id: "it_IT-paola-medium",
        desc: "Italian female voice",
        lang: "it",
    },
    VoicePreset {
        name: "Siwis (FR)",
        voice_id: "fr_FR-siwis-medium",
        desc: "French female voice",
        lang: "fr",
    },
    VoicePreset {
        name: "Thorsten (DE)",
        voice_id: "de_DE-thorsten-medium",
        desc: "German male voice",
        lang: "de",
    },
    VoicePreset {
        name: "Irinia (RU)",
        voice_id: "ru_RU-irinia-medium",
        desc: "Russian female voice",
        lang: "ru",
    },
];
