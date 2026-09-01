//! Voice Pack Marketplace.
//!
//! A curated catalog of voice "packs" — themed bundles that map one voice id
//! per language. Users browse the catalog in the bot (`/shop`), install a pack
//! (which activates its default voice for the user's language), and can
//! uninstall back to the built-in default voice. A pack is a lightweight way to
//! rebrand / pre-pick a voice selection without managing raw voice ids.

use crate::tts::voices;

pub struct VoicePack {
    pub id: &'static str,
    pub name: &'static str,
    pub emoji: &'static str,
    pub blurb: &'static str,
    /// Pack default voice per language (id -> voice id).
    pub voices: &'static [(&'static str, &'static str)],
}

pub static PACKS: &[VoicePack] = &[
    VoicePack {
        id: "amy",
        name: "Amy (Default)",
        emoji: "🌐",
        blurb: "Balanced all-rounder, default on new accounts.",
        voices: &[("en", "en_US-amy-medium")],
    },
    VoicePack {
        id: "news",
        name: "News Anchor",
        emoji: "📰",
        blurb: "Clear, neutral broadcast voices.",
        voices: &[
            ("en", "en_US-amy-medium"),
            ("ar", "ar_JO-kareem-medium"),
            ("fr", "fr_FR-siwis-medium"),
            ("es", "es_ES-mls_10246-low"),
        ],
    },
    VoicePack {
        id: "companion",
        name: "Companion",
        emoji: "🎧",
        blurb: "Warm, approachable everyday voices.",
        voices: &[
            ("en", "en_US-lessac-medium"),
            ("ar", "ar-zayd0-diacritized"),
            ("fr", "fr_FR-upmc-medium"),
            ("es", "es_ES-carlfm-x_low"),
        ],
    },
    VoicePack {
        id: "arabic_elite",
        name: "Arabic Elite",
        emoji: "🇸🇦",
        blurb: "Premium Arabic voices, including the Emirati female pack.",
        voices: &[("ar", "ar_AE-emirati-female"), ("en", "en_US-amy-medium")],
    },
];

/// Pick the default voice id for a pack given the user's language.
pub fn pack_voice(pack_id: &str, lang: &str) -> Option<&'static str> {
    let pack = PACKS.iter().find(|p| p.id == pack_id)?;
    pack.voices
        .iter()
        .find(|(l, _)| *l == lang)
        .map(|(_, v)| *v)
        .or_else(|| Some(pack.voices[0].1))
}

/// Return the pack (if any) that maps the given voice id as its language entry.
pub fn pack_for_voice(voice_id: &str) -> Option<&'static VoicePack> {
    PACKS
        .iter()
        .find(|p| p.voices.iter().any(|(_, v)| *v == voice_id))
}

fn _voice_ids_exist() -> Result<(), String> {
    for p in PACKS {
        for (_, v) in p.voices {
            if voices::find(v).is_none() {
                return Err(format!("pack '{}' references missing voice '{v}'", p.id));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_pack_voice_resolves() {
        assert!(matches!(super::_voice_ids_exist(), Ok(())));
    }

    #[test]
    fn pack_voice_falls_back_to_first_lang() {
        assert_eq!(pack_voice("news", "de"), Some("en_US-amy-medium"));
        assert_eq!(
            pack_voice("arabic_elite", "ar"),
            Some("ar_AE-emirati-female")
        );
    }

    #[test]
    fn unknown_pack_returns_none() {
        assert!(pack_voice("nope", "en").is_none());
    }
}
