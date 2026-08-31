use crate::bot::PromptKind;
use crate::i18n::LANGUAGES;
use crate::tts::presets::{VoicePreset, PRESETS};
use crate::tts::voices::{VoiceMeta, VOICES};
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

/// Language selector — two buttons per row.
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

pub const VOICES_PER_PAGE: usize = 8;

/// Build a paginated voice gallery for a specific language.
///
/// `installed` is the set of voice ids that are actually downloaded and
/// usable; voices not yet installed are shown but marked "🔧 install".
/// `active_id` is the user's currently selected voice (gets a ✅).
pub fn voices_keyboard(
    lang: &str,
    installed: &[&'static VoiceMeta],
    active_id: &str,
    page: usize,
) -> InlineKeyboardMarkup {
    let installed_ids: Vec<&str> = installed.iter().map(|v| v.id).collect();
    let lang_voices: Vec<&VoiceMeta> = VOICES.iter().filter(|v| v.lang == lang).collect();

    let total_pages = lang_voices.len().div_ceil(VOICES_PER_PAGE).max(1);
    let page = page.min(total_pages - 1);
    let start = page * VOICES_PER_PAGE;
    let page_voices = &lang_voices[start..(start + VOICES_PER_PAGE).min(lang_voices.len())];

    let mut rows: Vec<Vec<InlineKeyboardButton>> = page_voices
        .iter()
        .map(|v| {
            let is_active = v.id == active_id;
            let is_installed = installed_ids.contains(&v.id);
            let marker = if is_active { "✅ " } else { "" };
            let status = if is_installed { "" } else { " 🔧" };
            vec![InlineKeyboardButton::callback(
                format!(
                    "{} {} {}{} · {}",
                    v.flag(),
                    marker,
                    v.label(),
                    status,
                    v.source()
                ),
                format!("voice:{}", v.id),
            )]
        })
        .collect();

    // Pagination row (only if more than one page).
    if total_pages > 1 {
        let mut nav = Vec::new();
        if page > 0 {
            nav.push(InlineKeyboardButton::callback(
                "◀️",
                format!("vpage:{}:{}", lang, page - 1),
            ));
        }
        nav.push(InlineKeyboardButton::callback(
            format!("📄 {}/{}", page + 1, total_pages),
            "vpage:noop".to_string(),
        ));
        if page + 1 < total_pages {
            nav.push(InlineKeyboardButton::callback(
                "▶️",
                format!("vpage:{}:{}", lang, page + 1),
            ));
        }
        rows.push(nav);
    }

    rows.push(vec![InlineKeyboardButton::callback(
        "🔙 Back",
        "menu:voices_back",
    )]);
    InlineKeyboardMarkup::new(rows)
}

/// "Install more Arabic voices" keyboard — lists the community Arabic packs
/// with a download action. Shown inside the Arabic voice gallery.
pub fn arabic_install_keyboard() -> InlineKeyboardMarkup {
    let ar: Vec<&VoiceMeta> = VOICES.iter().filter(|v| v.lang == "ar").collect();
    let rows: Vec<Vec<InlineKeyboardButton>> = ar
        .iter()
        .map(|v| {
            vec![InlineKeyboardButton::callback(
                format!("⬇️ Install {}", v.name),
                format!("dlvoice:{}", v.id),
            )]
        })
        .collect();
    let mut rows = rows;
    rows.push(vec![InlineKeyboardButton::callback(
        "🔙 Back",
        "menu:voices_back",
    )]);
    InlineKeyboardMarkup::new(rows)
}

/// Voice presets — one button per preset, filtered to the user's language.
pub fn presets_keyboard(lang: &str) -> InlineKeyboardMarkup {
    let matched: Vec<&VoicePreset> = PRESETS.iter().filter(|p| p.lang == lang).collect();
    let presets: Vec<&VoicePreset> = if matched.is_empty() {
        PRESETS.iter().collect()
    } else {
        matched
    };
    let buttons: Vec<Vec<InlineKeyboardButton>> = presets
        .chunks(2)
        .map(|pair| {
            pair.iter()
                .map(|p| {
                    InlineKeyboardButton::callback(
                        format!("{} — {}", p.name, p.desc),
                        format!("voice:{}", p.voice_id),
                    )
                })
                .collect()
        })
        .collect();
    let mut buttons = buttons;
    buttons.push(vec![InlineKeyboardButton::callback(
        "🔙 Back",
        "menu:voices_back",
    )]);
    InlineKeyboardMarkup::new(buttons)
}

/// Clones management — one Delete button per clone.
pub fn clones_keyboard(clones: &[crate::db::VoiceClone]) -> InlineKeyboardMarkup {
    let buttons: Vec<Vec<InlineKeyboardButton>> = clones
        .iter()
        .map(|c| {
            vec![InlineKeyboardButton::callback(
                format!("🗑 Delete: {}", c.name),
                format!("del_clone:{}", c.id),
            )]
        })
        .collect();
    InlineKeyboardMarkup::new(buttons)
}

/// Main command-center menu shown after /start and /menu. Each button is an
/// action that either runs directly or opens a Text/Voice choice.
pub fn main_menu(_s: &crate::i18n::Strings) -> InlineKeyboardMarkup {
    let rows = vec![
        vec![
            InlineKeyboardButton::callback("🧠 Chat with AI", "act:ask"),
            InlineKeyboardButton::callback("🔊 Text → Speech", "act:speak"),
        ],
        vec![
            InlineKeyboardButton::callback("🧬 My Cloned Voice", "act:myvoice"),
            InlineKeyboardButton::callback("🎤 Clone a Voice", "act:clone"),
        ],
        vec![
            InlineKeyboardButton::callback("🎙 Browse Voices", "act:voices"),
            InlineKeyboardButton::callback("⚙️ Settings", "act:settings"),
        ],
        vec![
            InlineKeyboardButton::callback("💳 Credits", "act:credits"),
            InlineKeyboardButton::callback("📊 My Stats", "act:stats"),
        ],
        vec![
            InlineKeyboardButton::callback("⭐ Upgrade", "act:upgrade"),
            InlineKeyboardButton::callback("❓ Help", "act:help"),
        ],
        vec![InlineKeyboardButton::callback(
            "🔄 Reset Memory",
            "act:reset",
        )],
    ];
    InlineKeyboardMarkup::new(rows)
}

/// "Text or Voice?" choice shown after tapping an action that can answer
/// either way (e.g. Chat with AI).
pub fn mode_keyboard(kind: PromptKind) -> InlineKeyboardMarkup {
    let tag = match kind {
        PromptKind::Ask => "ask",
        PromptKind::Speak => "speak",
        PromptKind::MyVoice => "myvoice",
    };
    InlineKeyboardMarkup::new(vec![vec![
        InlineKeyboardButton::callback("💬 As Text", format!("mode:{}:text", tag)),
        InlineKeyboardButton::callback("🔊 As Voice", format!("mode:{}:voice", tag)),
    ]])
}

/// Attached to every AI text reply — lets the user instantly hear the answer
/// in their active voice or open the full voice gallery.
pub fn reply_voice_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![
            InlineKeyboardButton::callback("🔊 Speak this", "speaklast:active"),
            InlineKeyboardButton::callback("🎨 Voice Gallery", "speaklast:gallery"),
        ],
        vec![InlineKeyboardButton::callback(
            "🔁 Regenerate",
            "speaklast:regen",
        )],
    ])
}

/// Settings panel: shows current state and offers quick changes.
pub fn settings_keyboard() -> InlineKeyboardMarkup {
    let rows = vec![
        vec![
            InlineKeyboardButton::callback("🎙 Change Voice", "act:voices"),
            InlineKeyboardButton::callback("🌐 Language", "act:lang"),
        ],
        vec![
            InlineKeyboardButton::callback("🧬 Voice Clone", "act:clone"),
            InlineKeyboardButton::callback("💳 Credits", "act:credits"),
        ],
        vec![InlineKeyboardButton::callback(
            "🔙 Back to Menu",
            "menu:home",
        )],
    ];
    InlineKeyboardMarkup::new(rows)
}

/// Upgrade / payment menu with Telegram Stars invoice button.
pub fn upgrade_menu() -> InlineKeyboardMarkup {
    let rows = vec![
        vec![InlineKeyboardButton::callback(
            "⭐ 100 Credits — 1★",
            "pay:100",
        )],
        vec![InlineKeyboardButton::callback(
            "💎 500 Credits — 4★",
            "pay:500",
        )],
        vec![InlineKeyboardButton::callback(
            "👑 1500 Credits — 10★",
            "pay:1500",
        )],
        vec![InlineKeyboardButton::callback("🔙 Back", "menu:home")],
    ];
    InlineKeyboardMarkup::new(rows)
}

/// Consent prompt.
pub fn consent_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![vec![
        InlineKeyboardButton::callback("✅ I Agree", "consent:yes"),
        InlineKeyboardButton::callback("❌ Cancel", "consent:no"),
    ]])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tts::voices::VOICES;
    use teloxide::types::InlineKeyboardButtonKind;

    /// Count selectable (voice:) callback buttons on a rendered markup,
    /// excluding the pagination/back rows.
    fn total_voice_buttons(markup: &InlineKeyboardMarkup) -> usize {
        markup
            .inline_keyboard
            .iter()
            .flatten()
            .filter(|b| {
                matches!(&b.kind, InlineKeyboardButtonKind::CallbackData(d) if d.starts_with("voice:"))
            })
            .count()
    }

    /// Extract the callback data string from a button, if it's a callback button.
    fn cb_data(b: &teloxide::types::InlineKeyboardButton) -> Option<&str> {
        match &b.kind {
            InlineKeyboardButtonKind::CallbackData(d) => Some(d),
            _ => None,
        }
    }

    #[test]
    fn english_gallery_paginates() {
        let installed: Vec<&VoiceMeta> = VOICES.iter().filter(|v| v.lang == "en").collect();
        let total = installed.len();
        assert!(total > VOICES_PER_PAGE, "need >1 page to test pagination");

        let pages = total.div_ceil(VOICES_PER_PAGE);
        // First page is full.
        let p0 = voices_keyboard("en", &installed, "en_US-amy-medium", 0);
        assert_eq!(total_voice_buttons(&p0), VOICES_PER_PAGE);

        // Last page holds the remainder.
        let last = voices_keyboard("en", &installed, "en_US-amy-medium", pages - 1);
        let rem = total - (pages - 1) * VOICES_PER_PAGE;
        assert_eq!(total_voice_buttons(&last), rem);

        // Every voice is reachable across the pages (no duplicates, no gaps).
        let mut seen = std::collections::HashSet::new();
        for page in 0..pages {
            let m = voices_keyboard("en", &installed, "en_US-amy-medium", page);
            for b in m.inline_keyboard.iter().flatten() {
                if let Some(data) = cb_data(b) {
                    if let Some(id) = data.strip_prefix("voice:") {
                        assert!(
                            seen.insert(id.to_string()),
                            "duplicate voice id across pages: {id}"
                        );
                    }
                }
            }
        }
        assert_eq!(seen.len(), total, "some voices missing across pages");
    }

    #[test]
    fn page_index_is_clamped() {
        let installed: Vec<&VoiceMeta> = VOICES.iter().filter(|v| v.lang == "en").collect();
        let pages = installed.len().div_ceil(VOICES_PER_PAGE);
        // Requesting an out-of-range page must not panic and must not exceed bounds.
        let over = voices_keyboard("en", &installed, "en_US-amy-medium", pages + 5);
        // Clamped to last real page: still renders at least 1 voice button.
        assert!(total_voice_buttons(&over) > 0);
    }

    #[test]
    fn active_voice_is_marked() {
        let installed: Vec<&VoiceMeta> = VOICES.iter().filter(|v| v.lang == "en").collect();
        let active = "en_US-amy-medium";
        let m = voices_keyboard("en", &installed, active, 0);
        let marked: Vec<&str> = m
            .inline_keyboard
            .iter()
            .flatten()
            .map(|b| b.text.as_str())
            .filter(|t| t.contains('✅'))
            .collect();
        assert_eq!(marked.len(), 1, "exactly one active marker expected");
        assert!(
            marked[0].contains(active) || marked[0].contains("Amy"),
            "active marker on right voice"
        );
    }

    #[test]
    fn mode_keyboard_tags_are_distinct() {
        let ask = mode_keyboard(PromptKind::Ask);
        let speak = mode_keyboard(PromptKind::Speak);
        let ask_data: Vec<&str> = ask
            .inline_keyboard
            .iter()
            .flatten()
            .filter_map(|b| cb_data(b))
            .collect();
        let speak_data: Vec<&str> = speak
            .inline_keyboard
            .iter()
            .flatten()
            .filter_map(|b| cb_data(b))
            .collect();
        assert!(ask_data.iter().any(|d| d.starts_with("mode:ask:")));
        assert!(speak_data.iter().any(|d| d.starts_with("mode:speak:")));
        assert!(!speak_data.iter().any(|d| d.starts_with("mode:ask:")));
    }
}
