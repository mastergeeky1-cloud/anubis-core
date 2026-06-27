use crate::i18n::LANGUAGES;
use crate::tts::presets::PRESETS;
use crate::tts::voices;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

/// Language selector keyboard — two buttons per row.
pub fn lang_keyboard() -> InlineKeyboardMarkup {
    let buttons: Vec<Vec<InlineKeyboardButton>> = LANGUAGES
        .chunks(2)
        .map(|pair| {
            pair.iter()
                .map(|l| InlineKeyboardButton::callback(l.label, format!("lang:{}", l.code)))
                .collect()
        })
        .collect();
    InlineKeyboardMarkup::new(buttons)
}

/// Voice selector keyboard filtered by language.
pub fn voices_keyboard(lang: &str, installed: &[&voices::VoiceMeta]) -> InlineKeyboardMarkup {
    let lang_voices: Vec<_> = installed.iter().filter(|v| v.lang == lang).collect();

    let source: Vec<_> = if lang_voices.is_empty() {
        installed.iter().collect()
    } else {
        lang_voices
    };

    let buttons: Vec<Vec<InlineKeyboardButton>> = source
        .iter()
        .map(|v| {
            let gender_icon = if v.gender == "female" { "♀" } else { "♂" };
            vec![InlineKeyboardButton::callback(
                format!("{} {} ({})", gender_icon, v.name, v.quality),
                format!("voice:{}", v.id),
            )]
        })
        .collect();

    InlineKeyboardMarkup::new(buttons)
}

/// Voice presets keyboard — one button per preset.
pub fn presets_keyboard() -> InlineKeyboardMarkup {
    let buttons: Vec<Vec<InlineKeyboardButton>> = PRESETS
        .chunks(2)
        .map(|pair| {
            pair.iter()
                .map(|p| InlineKeyboardButton::callback(p.name, format!("voice:{}", p.voice_id)))
                .collect()
        })
        .collect();
    InlineKeyboardMarkup::new(buttons)
}

/// Clones management keyboard — one Delete button per clone.
pub fn clones_keyboard(clones: &[crate::db::VoiceClone]) -> InlineKeyboardMarkup {
    let buttons: Vec<Vec<InlineKeyboardButton>> = clones
        .iter()
        .map(|c| {
            vec![InlineKeyboardButton::callback(
                format!("Delete: {}", c.name),
                format!("del_clone:{}", c.id),
            )]
        })
        .collect();
    InlineKeyboardMarkup::new(buttons)
}

/// Consent prompt keyboard.
pub fn consent_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![vec![
        InlineKeyboardButton::callback("I Agree", "consent:yes"),
        InlineKeyboardButton::callback("Cancel",  "consent:no"),
    ]])
}
