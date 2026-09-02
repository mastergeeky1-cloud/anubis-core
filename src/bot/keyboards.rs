use crate::i18n;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

pub fn main_menu(lang: &str) -> InlineKeyboardMarkup {
    let s = i18n::get(lang);
    InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::callback(
            s.btn_ask.to_string(),
            "act:ask",
        )],
        vec![InlineKeyboardButton::callback(
            s.btn_speak.to_string(),
            "act:speak",
        )],
        vec![
            InlineKeyboardButton::callback(s.btn_voices.to_string(), "act:voices"),
            InlineKeyboardButton::callback(s.btn_lang.to_string(), "act:lang"),
        ],
        vec![
            InlineKeyboardButton::callback(s.btn_teacher.to_string(), "act:teacher"),
            InlineKeyboardButton::callback(s.btn_help.to_string(), "act:help"),
        ],
        vec![
            InlineKeyboardButton::callback(s.btn_credits.to_string(), "act:credits"),
            InlineKeyboardButton::callback(s.btn_upgrade.to_string(), "act:upgrade"),
        ],
        vec![InlineKeyboardButton::callback(
            s.btn_reset.to_string(),
            "act:reset",
        )],
    ])
}

pub fn lang_keyboard() -> InlineKeyboardMarkup {
    let buttons: Vec<Vec<InlineKeyboardButton>> = crate::i18n::LANGUAGES
        .chunks(2)
        .map(|pair| {
            pair.iter()
                .map(|l| InlineKeyboardButton::callback(l.label, format!("lang:{}", l.code)))
                .collect()
        })
        .collect();
    InlineKeyboardMarkup::new(buttons)
}

pub fn voices_keyboard(lang: &str, active_id: &str) -> InlineKeyboardMarkup {
    let voices = crate::tts::voices::for_lang(lang);
    let mut rows: Vec<Vec<InlineKeyboardButton>> = voices
        .chunks(2)
        .map(|pair| {
            pair.iter()
                .map(|v| {
                    let marker = if v.id == active_id { " ✅" } else { "" };
                    let icon = if v.gender == "female" { "♀" } else { "♂" };
                    InlineKeyboardButton::callback(
                        format!("{} {}{}", icon, v.name, marker),
                        format!("voice:{}", v.id),
                    )
                })
                .collect()
        })
        .collect();
    rows.push(vec![InlineKeyboardButton::callback("🔙", "menu:back")]);
    InlineKeyboardMarkup::new(rows)
}

pub fn reply_voice_keyboard(lang: &str) -> InlineKeyboardMarkup {
    let s = i18n::get(lang);
    InlineKeyboardMarkup::new(vec![vec![InlineKeyboardButton::callback(
        s.listen_hint.to_string(),
        "listen:last",
    )]])
}
