use super::commands::Command;
use super::{keyboards, AppState, OutMode, PendingAction, PromptKind};
use crate::bot::stats::{
    format_admin_stats, format_daily_active, format_user_list, format_user_stats, StatsEngine,
};
use crate::clone::new_voice_clone;
use crate::security::ratelimit::RateKind;
use crate::security::sanitize_text;
use crate::tts::voices;
use chrono::Utc;
use teloxide::net::Download;
use teloxide::prelude::*;
use teloxide::types::{ChatAction, InlineKeyboardMarkup, InputFile, ParseMode};
use tracing::{error, warn};

// ─── helpers ──────────────────────────────────────────────────────────────────
fn uid(msg: &Message) -> i64 {
    msg.from().map(|u| u.id.0 as i64).unwrap_or(0)
}
fn uname(msg: &Message) -> Option<&str> {
    msg.from().and_then(|u| u.username.as_deref())
}
fn user_lang(state: &AppState, user_id: i64) -> String {
    state
        .db
        .get_user(user_id)
        .ok()
        .flatten()
        .map(|u| u.lang)
        .unwrap_or_else(|| "en".to_string())
}
fn user_voice(state: &AppState, user_id: i64, lang: &str) -> String {
    state
        .db
        .get_user(user_id)
        .ok()
        .flatten()
        .map(|u| u.active_voice)
        .unwrap_or_else(|| voices::default_for_lang(lang).to_string())
}
fn ensure_user(state: &AppState, user_id: i64, username: Option<&str>) -> String {
    let _ = state.db.upsert_user(user_id, username);
    user_lang(state, user_id)
}

/// Animated progress message: edits `msg_id` through the given stage labels one
/// at a time (spinner char rotates) so the user sees real loading instead of a
/// static "generating". `work` runs the actual task.
async fn with_progress<F, Fut>(
    bot: &Bot,
    cid: ChatId,
    msg_id: i32,
    stages: &[&str],
    work: F,
) -> Fut::Output
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future,
{
    let spinners = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
    let stages: Vec<String> = stages.iter().map(|s| s.to_string()).collect();
    let label = stages
        .first()
        .cloned()
        .unwrap_or_else(|| "Working".to_string());
    let mut tick = 0usize;
    let mut last = String::new();
    let handle = {
        let bot = bot.clone();
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(220));
        let start = tokio::time::Instant::now();
        // Drive the spinner in the background while `work` runs.
        tokio::spawn(async move {
            loop {
                let _ = interval.tick().await;
                if start.elapsed().as_secs() > 60 {
                    break; // safety: don't spin forever
                }
                let frame = spinners[tick % spinners.len()];
                let text = format!("{} {}…", frame, label);
                if text != last {
                    let _ = bot
                        .edit_message_text(cid, teloxide::types::MessageId(msg_id), text.clone())
                        .await;
                    last = text;
                }
                tick += 1;
            }
        })
    };

    let result = work().await;
    handle.abort();
    // Settle on a neutral placeholder; the caller overwrites with the real text.
    let _ = bot
        .edit_message_text(cid, teloxide::types::MessageId(msg_id), "⏳ Working…")
        .await;
    result
}

/// Send the ANUBIS banner image (if present) with a MarkdownV2 caption and the
/// command palette. Falls back to a plain text message when the asset is missing.
async fn send_banner(
    bot: Bot,
    cid: ChatId,
    text: &str,
    markup: InlineKeyboardMarkup,
) -> Result<(), teloxide::RequestError> {
    const BANNER: &str = "assets/banner.png";
    if std::path::Path::new(BANNER).exists() {
        bot.send_photo(cid, InputFile::file(BANNER))
            .caption(crate::i18n::md2(text))
            .parse_mode(ParseMode::MarkdownV2)
            .reply_markup(markup)
            .await?;
    } else {
        bot.send_message(cid, crate::i18n::md2(text))
            .parse_mode(ParseMode::MarkdownV2)
            .reply_markup(markup)
            .await?;
    }
    Ok(())
}

/// Centralized voice generation: resolves the active voice, checks cache +
/// credits, synthesizes via the TTS router, converts to OGG and sends the
/// voice message. Used by /speak, /myvoice, ask:voice and the gallery.
#[allow(clippy::too_many_arguments)]
async fn gen_voice(
    bot: &Bot,
    cid: ChatId,
    user_id: i64,
    text: &str,
    voice_id: &str,
    state: &AppState,
    s: &'static crate::i18n::Strings,
    caption: Option<&str>,
) -> Result<(), teloxide::RequestError> {
    let _ = bot.send_chat_action(cid, ChatAction::UploadVoice).await;
    let progress = bot.send_message(cid, s.loading_synth).await?;

    let cache_key = crate::cache::AudioCache::make_key(text, voice_id);
    if let Some(ogg_bytes) = state.cache.get(cache_key) {
        let _ = bot.delete_message(cid, progress.id).await;
        bot.send_voice(cid, InputFile::memory(ogg_bytes)).await?;
        return Ok(());
    }

    let has_credit = state
        .db
        .consume_credit(
            user_id,
            state.config.limits.free_daily_credits,
            state.config.limits.unlimited_mode,
        )
        .unwrap_or(false);
    if !has_credit {
        let _ = bot.delete_message(cid, progress.id).await;
        bot.send_message(cid, s.no_credits).await?;
        return Ok(());
    }
    state
        .db
        .audit(user_id, "speak", &format!("voice={voice_id}"));

    let stages = [s.loading_synth];
    let wav_path = match with_progress(bot, cid, progress.id.0, &stages, || {
        state.tts.synthesize_wav(text, voice_id)
    })
    .await
    {
        Ok(p) => p,
        Err(e) => {
            error!("TTS error for user {user_id}: {e}");
            let _ = bot.delete_message(cid, progress.id).await;
            bot.send_message(cid, s.tts_fail).await?;
            return Ok(());
        }
    };
    let ogg_bytes = match synthesize_to_ogg(state, &wav_path).await {
        Ok(b) => b,
        Err(e) => {
            error!("wav->ogg: {e}");
            crate::tts::remove_wav(&wav_path).await;
            let _ = bot.delete_message(cid, progress.id).await;
            bot.send_message(cid, s.tts_fail).await?;
            return Ok(());
        }
    };
    let _ = bot.delete_message(cid, progress.id).await;
    state.cache.insert(cache_key, ogg_bytes.clone());
    let mut req = bot.send_voice(cid, InputFile::memory(ogg_bytes));
    if let Some(cap) = caption {
        req = req.caption(cap);
    }
    req.await?;
    Ok(())
}

/// Stream a question to Noxis Core and edit the "thinking" placeholder live.
/// `out` selects whether the final answer is returned as text (to be sent as a
/// message) or spoken back as a voice message.
#[allow(clippy::too_many_arguments)]
async fn do_ask(
    bot: Bot,
    cid: ChatId,
    user_id: i64,
    text: &str,
    lang: &str,
    state: AppState,
    s: &'static crate::i18n::Strings,
    out: OutMode,
) -> Result<(), teloxide::RequestError> {
    if !state.noxis.enabled() {
        bot.send_message(cid, s.brain_off).await?;
        return Ok(());
    }
    let _ = bot.send_chat_action(cid, ChatAction::Typing).await;
    let thinking = bot.send_message(cid, s.loading_think).await?;
    state.db.audit(user_id, "ask", text);
    let history = state.memory.history(user_id);
    let chat_id = cid;
    let thinking_id = thinking.id;
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    let edit_bot = bot.clone();
    let editor = tokio::spawn(async move {
        let mut rx = rx;
        let mut displayed = String::new();
        let mut last_edit = std::time::Instant::now();
        let mut last_len = 0usize;
        while let Some(delta) = rx.recv().await {
            displayed.push_str(&delta);
            if displayed.len() - last_len >= 120 && last_edit.elapsed().as_millis() >= 250 {
                last_len = displayed.len();
                last_edit = std::time::Instant::now();
                let _ = edit_bot
                    .edit_message_text(chat_id, thinking_id, format!("🧠 {}", displayed))
                    .await;
            }
        }
        let _ = edit_bot
            .edit_message_text(chat_id, thinking_id, displayed)
            .await;
    });
    let send_result = state
        .noxis
        .ask_stream(text, lang, &history, |delta| {
            let _ = tx.send(delta.to_string());
        })
        .await;
    drop(tx);
    let _ = editor.await;
    match send_result {
        Ok(reply) => {
            if reply.trim().is_empty() {
                let _ = bot.delete_message(cid, thinking.id).await;
                bot.send_message(cid, s.tts_fail).await?;
                return Ok(());
            }
            state.memory.push(user_id, text, &reply);
            // Remember this reply so the inline "🔊 Speak this" can re-use it.
            state.last_reply.insert(user_id, reply.clone());
            let _ = bot.delete_message(cid, thinking.id).await;

            match out {
                OutMode::Text => {
                    bot.send_message(cid, reply)
                        .reply_markup(keyboards::reply_voice_keyboard())
                        .await?;
                }
                OutMode::Voice => {
                    let voice_id = user_voice(&state, user_id, lang);
                    gen_voice(&bot, cid, user_id, &reply, &voice_id, &state, s, None).await?;
                }
            }
        }
        Err(e) => {
            error!("noxis ask error: {e}");
            let _ = bot.delete_message(cid, thinking.id).await;
            bot.send_message(
                cid,
                "🧠 Noxis Core could not respond. Check the LLM server.",
            )
            .await?;
        }
    }
    Ok(())
}

// ─── command handler ──────────────────────────────────────────────────────────
pub async fn handle_command(
    bot: Bot,
    msg: Message,
    cmd: Command,
    state: AppState,
) -> Result<(), teloxide::RequestError> {
    let user_id = uid(&msg);
    let lang = ensure_user(&state, user_id, uname(&msg));
    let s = crate::i18n::get(&lang);
    let cid = msg.chat.id;

    if state.db.is_banned(user_id) {
        bot.send_message(cid, "Your account has been suspended.")
            .await?;
        return Ok(());
    }

    match cmd {
        Command::Start => {
            send_banner(bot.clone(), cid, s.welcome, keyboards::main_menu(s)).await?;
        }
        Command::Settings => {
            show_settings(bot, cid, user_id, &state, s).await?;
        }
        Command::Menu => {
            send_banner(bot.clone(), cid, s.menu_header, keyboards::main_menu(s)).await?;
        }
        Command::Upgrade => {
            bot.send_message(
                cid,
                crate::i18n::md2(&format!(
                    "{}\n\n{}\n\n{}",
                    s.upgrade_header, s.upgrade_info, s.payment_secure
                )),
            )
            .parse_mode(ParseMode::MarkdownV2)
            .reply_markup(keyboards::upgrade_menu())
            .await?;
        }
        Command::Help => {
            bot.send_message(cid, crate::i18n::md2(s.help))
                .parse_mode(ParseMode::MarkdownV2)
                .reply_markup(keyboards::main_menu(s))
                .await?;
        }
        Command::Lang => {
            bot.send_message(cid, s.choose_lang)
                .reply_markup(keyboards::lang_keyboard())
                .await?;
        }
        Command::Credits => {
            let (credits, daily_used) = state
                .db
                .get_user(user_id)
                .ok()
                .flatten()
                .map(|u| (u.credits, u.daily_used))
                .unwrap_or((0, 0));
            let free_max = state.config.limits.free_daily_credits;
            let text = s
                .credits_info
                .replace("{credits}", &credits.to_string())
                .replace("{free}", &daily_used.to_string())
                .replace("{max}", &free_max.to_string());
            bot.send_message(cid, text).await?;
        }
        Command::Voices => {
            let installed = state.tts.available_voices();
            let active = user_voice(&state, user_id, &lang);
            if lang == "ar" {
                // Arabic users also get the install-more pack shortcut.
                bot.send_message(
                    cid,
                    crate::i18n::md2(&format!("{}\n\n{}", s.voices_header, s.arabic_more)),
                )
                .parse_mode(ParseMode::MarkdownV2)
                .reply_markup(keyboards::voices_keyboard(&lang, &installed, &active, 0))
                .await?;
                bot.send_message(cid, "⬇️ Install more Arabic voices:")
                    .reply_markup(keyboards::arabic_install_keyboard())
                    .await?;
            } else {
                bot.send_message(cid, crate::i18n::md2(s.voices_header))
                    .parse_mode(ParseMode::MarkdownV2)
                    .reply_markup(keyboards::voices_keyboard(&lang, &installed, &active, 0))
                    .await?;
            }
        }
        Command::Presets => {
            bot.send_message(cid, "Choose a voice preset:")
                .reply_markup(keyboards::presets_keyboard(&lang))
                .await?;
        }
        Command::Setvoice(id) => {
            let id = id.trim().to_string();
            if voices::find(&id).is_none() {
                bot.send_message(
                    cid,
                    crate::i18n::md2(&format!("Unknown voice: `{}`. Use /voices.", id)),
                )
                .parse_mode(ParseMode::MarkdownV2)
                .await?;
                return Ok(());
            }
            let _ = state.db.set_active_voice(user_id, &id);
            let name = voices::find(&id).map(|v| v.name).unwrap_or("voice");
            bot.send_message(
                cid,
                format!("{}\n{}: {}", s.voice_set, s.voice_active, name),
            )
            .await?;
        }
        Command::Clone => {
            if !state.clone_engine.enabled {
                bot.send_message(cid, s.cloning_disabled).await?;
                return Ok(());
            }
            if state.config.security.require_consent && !state.db.has_consent(user_id) {
                bot.send_message(cid, s.consent_prompt)
                    .reply_markup(keyboards::consent_keyboard())
                    .await?;
                return Ok(());
            }
            if !state.rate_limiter.check(user_id, RateKind::Clone) {
                bot.send_message(cid, s.rate_limited).await?;
                return Ok(());
            }
            state
                .pending
                .insert(user_id, PendingAction::AwaitingVoiceForClone);
            bot.send_message(cid, s.clone_prompt).await?;
        }
        Command::Clones => {
            let clones = state.db.get_clones(user_id).unwrap_or_default();
            if clones.is_empty() {
                bot.send_message(cid, s.no_clones).await?;
            } else {
                bot.send_message(cid, crate::i18n::md2(s.my_clones))
                    .parse_mode(ParseMode::MarkdownV2)
                    .reply_markup(keyboards::clones_keyboard(&clones))
                    .await?;
            }
        }
        Command::Speak(text) => {
            let text = sanitize_text(&text);
            if text.is_empty() {
                state.pending.insert(
                    user_id,
                    PendingAction::AwaitingPrompt {
                        kind: PromptKind::Speak,
                        mode: OutMode::Voice,
                    },
                );
                bot.send_message(cid, "✍️ Type the text you want me to speak, then send it:")
                    .await?;
                return Ok(());
            }
            if !state.rate_limiter.check(user_id, RateKind::Speak) {
                bot.send_message(cid, s.rate_limited).await?;
                return Ok(());
            }
            do_speak(bot, cid, user_id, &text, &lang, state, s).await?;
        }
        Command::Ask(text) => {
            let text = sanitize_text(&text);
            if text.is_empty() {
                state.pending.insert(
                    user_id,
                    PendingAction::AwaitingPrompt {
                        kind: PromptKind::Ask,
                        mode: OutMode::Text,
                    },
                );
                bot.send_message(cid, "✍️ Type your question for Noxis Core, then send it:")
                    .await?;
                return Ok(());
            }
            do_ask(bot, cid, user_id, &text, &lang, state, s, OutMode::Text).await?;
        }
        Command::Myvoice(text) => {
            let text = sanitize_text(&text);
            if text.is_empty() {
                state.pending.insert(
                    user_id,
                    PendingAction::AwaitingPrompt {
                        kind: PromptKind::MyVoice,
                        mode: OutMode::Voice,
                    },
                );
                bot.send_message(
                    cid,
                    format!(
                        "{}\n\n✍️ Then send me the text to speak in your voice.",
                        s.myvoice_usage
                    ),
                )
                .await?;
                return Ok(());
            }
            if !state.clone_engine.enabled {
                bot.send_message(cid, s.cloning_disabled).await?;
                return Ok(());
            }
            if !state.rate_limiter.check(user_id, RateKind::Speak) {
                bot.send_message(cid, s.rate_limited).await?;
                return Ok(());
            }
            do_myvoice(bot, cid, user_id, &text, &lang, state, s).await?;
        }
        // ── user stats ────────────────────────────────────────────────────────
        Command::MyStats => {
            let stats_engine = StatsEngine::new(state.db.clone());
            match stats_engine
                .user_stats(user_id, state.config.limits.free_daily_credits)
                .await
            {
                Ok(stats) => {
                    let text = format_user_stats(&stats, s);
                    bot.send_message(cid, text).await?;
                }
                Err(e) => {
                    error!("user stats error: {e}");
                    bot.send_message(cid, "Failed to fetch stats").await?;
                }
            }
        }
        Command::Reset => {
            state.memory.clear(user_id);
            state.last_reply.remove(&user_id);
            state.db.audit(user_id, "reset_memory", "");
            bot.send_message(cid, s.reset_done).await?;
        }
        // ── admin ─────────────────────────────────────────────────────────────
        Command::Ban(arg) => {
            admin_guard(&state, user_id, &bot, cid, &arg, |st, t| {
                st.db.ban_user(t, true)
            })
            .await?
        }
        Command::Unban(arg) => {
            admin_guard(&state, user_id, &bot, cid, &arg, |st, t| {
                st.db.ban_user(t, false)
            })
            .await?
        }
        Command::Grant(arg) => {
            if !state.config.is_admin(user_id) {
                bot.send_message(cid, "Not authorized.").await?;
                return Ok(());
            }
            let parts: Vec<&str> = arg.trim().splitn(2, ' ').collect();
            if parts.len() != 2 {
                bot.send_message(cid, "Usage: /grant <user_id> <amount>")
                    .await?;
                return Ok(());
            }
            match (parts[0].parse::<i64>(), parts[1].parse::<i32>()) {
                (Ok(t), Ok(a)) => match state.db.add_credits(t, a, "admin_grant") {
                    Ok(_) => {
                        bot.send_message(cid, format!("Granted {} credits to user {}.", a, t))
                            .await?;
                    }
                    Err(e) => {
                        bot.send_message(cid, format!("Error: {e}")).await?;
                    }
                },
                _ => {
                    bot.send_message(cid, "Usage: /grant <user_id> <amount>")
                        .await?;
                }
            }
        }
        Command::Stats => {
            if !state.config.is_admin(user_id) {
                bot.send_message(cid, "Not authorized.").await?;
                return Ok(());
            }
            let stats_engine = StatsEngine::new(state.db.clone());
            match stats_engine.admin_stats().await {
                Ok(stats) => {
                    let text = format_admin_stats(&stats, s);
                    bot.send_message(cid, text).await?;
                }
                Err(e) => {
                    error!("admin stats error: {e}");
                    bot.send_message(cid, "Failed to fetch stats").await?;
                }
            }
        }
        Command::Users => {
            if !state.config.is_admin(user_id) {
                bot.send_message(cid, "Not authorized.").await?;
                return Ok(());
            }
            let stats_engine = StatsEngine::new(state.db.clone());
            match stats_engine.all_users().await {
                Ok(users) => {
                    let text = format_user_list(&users, s);
                    bot.send_message(cid, text).await?;
                }
                Err(e) => {
                    error!("users list error: {e}");
                    bot.send_message(cid, "Failed to fetch users").await?;
                }
            }
        }
        Command::DailyActive => {
            if !state.config.is_admin(user_id) {
                bot.send_message(cid, "Not authorized.").await?;
                return Ok(());
            }
            let stats_engine = StatsEngine::new(state.db.clone());
            match stats_engine.daily_active(30).await {
                Ok(data) => {
                    let text = format_daily_active(&data, s);
                    bot.send_message(cid, text).await?;
                }
                Err(e) => {
                    error!("daily active error: {e}");
                    bot.send_message(cid, "Failed to fetch daily active users")
                        .await?;
                }
            }
        }
    }
    Ok(())
}

/// Render the user's current settings (voice, language, credits, clone) and
/// offer quick-change buttons. This is what makes the AI chat "aware" of all
/// settings and voices.
async fn show_settings(
    bot: Bot,
    cid: ChatId,
    user_id: i64,
    state: &AppState,
    s: &'static crate::i18n::Strings,
) -> Result<(), teloxide::RequestError> {
    let user = state.db.get_user(user_id).ok().flatten();
    let voice_id = user
        .as_ref()
        .map(|u| u.active_voice.clone())
        .unwrap_or_else(|| voices::default_for_lang("en").to_string());
    let lang = user
        .as_ref()
        .map(|u| u.lang.clone())
        .unwrap_or_else(|| "en".to_string());
    let credits = user.as_ref().map(|u| u.credits).unwrap_or(0);
    let daily_used = user.as_ref().map(|u| u.daily_used).unwrap_or(0);
    let daily_max = state.config.limits.free_daily_credits;
    let voice_name = voices::find(&voice_id)
        .map(|v| v.name.to_string())
        .unwrap_or_else(|| voice_id.clone());
    let flag = voices::lang_flag(&lang);
    let lang_label = crate::i18n::LANGUAGES
        .iter()
        .find(|l| l.code == lang)
        .map(|l| l.label.to_string())
        .unwrap_or_else(|| lang.clone());
    let clone_count = state.db.get_clones(user_id).unwrap_or_default().len();

    let text = format!(
        "{}\n\n{} {}: `{}` _{}_\n{}: {}\n{}: {} ({}/{} 🆓 today)\n{}: {} 🧬",
        s.settings_title,
        s.settings_voice,
        flag,
        voice_name,
        voice_id,
        s.settings_lang,
        lang_label,
        s.settings_credits,
        credits,
        daily_used,
        daily_max,
        s.settings_clone,
        clone_count,
    );
    bot.send_message(cid, crate::i18n::md2(&text))
        .parse_mode(ParseMode::MarkdownV2)
        .reply_markup(keyboards::settings_keyboard())
        .await?;
    Ok(())
}

async fn admin_guard<F>(
    state: &AppState,
    user_id: i64,
    bot: &Bot,
    cid: ChatId,
    arg: &str,
    action: F,
) -> Result<(), teloxide::RequestError>
where
    F: FnOnce(&AppState, i64) -> std::result::Result<(), crate::error::AnubisError>,
{
    if !state.config.is_admin(user_id) {
        bot.send_message(cid, "Not authorized.").await?;
        return Ok(());
    }
    match arg.trim().parse::<i64>() {
        Ok(target) => {
            let _ = action(state, target);
            bot.send_message(cid, format!("User {} updated.", target))
                .await?;
        }
        Err(_) => {
            bot.send_message(cid, "Usage: /ban <user_id> or /unban <user_id>")
                .await?;
        }
    }
    Ok(())
}

// ─── TTS generation ───────────────────────────────────────────────────────────
async fn do_speak(
    bot: Bot,
    cid: ChatId,
    user_id: i64,
    text: &str,
    lang: &str,
    state: AppState,
    s: &'static crate::i18n::Strings,
) -> Result<(), teloxide::RequestError> {
    if text.chars().count() > state.config.limits.max_text_chars {
        bot.send_message(cid, s.text_too_long).await?;
        return Ok(());
    }
    let voice_id = user_voice(&state, user_id, lang);
    gen_voice(&bot, cid, user_id, text, &voice_id, &state, s, None).await
}

async fn do_myvoice(
    bot: Bot,
    cid: ChatId,
    user_id: i64,
    text: &str,
    lang: &str,
    state: AppState,
    s: &'static crate::i18n::Strings,
) -> Result<(), teloxide::RequestError> {
    if text.chars().count() > state.config.limits.max_text_chars {
        bot.send_message(cid, s.text_too_long).await?;
        return Ok(());
    }
    let has_credit = state
        .db
        .consume_credit(
            user_id,
            state.config.limits.free_daily_credits,
            state.config.limits.unlimited_mode,
        )
        .unwrap_or(false);
    if !has_credit {
        bot.send_message(cid, s.no_credits).await?;
        return Ok(());
    }
    let clone = match state.db.get_latest_clone(user_id).unwrap_or(None) {
        Some(c) => c,
        None => {
            bot.send_message(cid, s.no_clone).await?;
            return Ok(());
        }
    };
    state.db.audit(user_id, "myvoice", "clone speak");

    let _ = bot.send_chat_action(cid, ChatAction::UploadVoice).await;
    let thinking = bot.send_message(cid, s.loading_synth).await?;
    let wav_bytes = match state
        .clone_engine
        .synthesize(
            text,
            std::path::Path::new(&clone.wav_path),
            lang,
            &clone.ref_text,
        )
        .await
    {
        Ok(b) => b,
        Err(e) => {
            error!("clone synthesis error user {user_id}: {e}");
            let _ = bot.delete_message(cid, thinking.id).await;
            bot.send_message(cid, s.tts_fail).await?;
            return Ok(());
        }
    };

    // Watermark for provenance (user id + timestamp embedded in WAV samples).
    let ts = Utc::now().timestamp() as u32;
    let wav_bytes = state
        .watermark
        .embed(&wav_bytes, user_id, ts)
        .unwrap_or(wav_bytes);

    let tmp_wav = state.audio.tmp_path("wav");
    if let Err(e) = tokio::fs::write(&tmp_wav, &wav_bytes).await {
        error!("write tmp wav: {e}");
        let _ = bot.delete_message(cid, thinking.id).await;
        bot.send_message(cid, s.tts_fail).await?;
        return Ok(());
    }
    let ogg_bytes = match state.audio.wav_to_ogg(&tmp_wav).await {
        Ok(b) => b,
        Err(e) => {
            error!("wav->ogg (clone): {e}");
            tokio::fs::remove_file(&tmp_wav).await.ok();
            let _ = bot.delete_message(cid, thinking.id).await;
            bot.send_message(cid, s.tts_fail).await?;
            return Ok(());
        }
    };
    tokio::fs::remove_file(&tmp_wav).await.ok();
    let _ = bot.delete_message(cid, thinking.id).await;
    bot.send_voice(cid, InputFile::memory(ogg_bytes)).await?;
    Ok(())
}

/// Shared WAV->OGG step used by /speak and gen_voice.
pub(crate) async fn synthesize_to_ogg(
    state: &AppState,
    wav_path: &std::path::Path,
) -> crate::error::Result<Vec<u8>> {
    let wav_bytes = tokio::fs::read(wav_path).await?;
    crate::tts::remove_wav(wav_path).await;
    let tmp_wav = state.audio.tmp_path("wav");
    tokio::fs::write(&tmp_wav, &wav_bytes).await?;
    let ogg = state.audio.wav_to_ogg(&tmp_wav).await?;
    tokio::fs::remove_file(&tmp_wav).await.ok();
    Ok(ogg)
}

// ─── voice message (clone sample) handler ─────────────────────────────────────
pub async fn handle_message(
    bot: Bot,
    msg: Message,
    state: AppState,
) -> Result<(), teloxide::RequestError> {
    let user_id = uid(&msg);
    let lang = ensure_user(&state, user_id, uname(&msg));
    let s = crate::i18n::get(&lang);
    let cid = msg.chat.id;

    if state.db.is_banned(user_id) {
        return Ok(());
    }

    // Tap-from-menu prompt: the user previously tapped an action with no text,
    // so the next plain message is that command's input. `mode` selects whether
    // the result comes back as text (chat) or as a voice message.
    if let Some(PendingAction::AwaitingPrompt { kind, mode }) =
        state.pending.get(&user_id).map(|r| r.clone())
    {
        match msg.text() {
            Some(txt) => {
                let t = sanitize_text(txt);
                state.pending.remove(&user_id);
                if t.trim().is_empty() {
                    bot.send_message(cid, "Please send some text first.")
                        .await?;
                    return Ok(());
                }
                match (kind, mode) {
                    (PromptKind::Ask, OutMode::Text) => {
                        do_ask(bot, cid, user_id, &t, &lang, state, s, OutMode::Text).await?
                    }
                    (PromptKind::Speak, _) => {
                        do_speak(bot, cid, user_id, &t, &lang, state, s).await?
                    }
                    (PromptKind::MyVoice, _) => {
                        do_myvoice(bot, cid, user_id, &t, &lang, state, s).await?
                    }
                    (PromptKind::Ask, OutMode::Voice) => {
                        do_ask(bot, cid, user_id, &t, &lang, state, s, OutMode::Voice).await?
                    }
                }
            }
            None => {
                bot.send_message(cid, "Please send text, not a file.")
                    .await?;
            }
        }
        return Ok(());
    }

    let awaiting_clone = matches!(
        state.pending.get(&user_id).map(|r| r.clone()),
        Some(PendingAction::AwaitingVoiceForClone)
    );
    if !awaiting_clone {
        // No pending clone action. If the user sent a voice message and
        // whisper voice-input is enabled, treat it as a spoken question for
        // the Noxis brain (voice-to-voice conversation).
        if msg.voice().is_some() && state.whisper.enabled {
            return handle_voice_conversation(bot, msg, state).await;
        }
        if msg.text().is_some() {
            bot.send_message(cid, s.unknown_cmd).await?;
        }
        return Ok(());
    }

    let Some(voice) = msg.voice() else {
        bot.send_message(cid, s.send_voice_to_clone).await?;
        return Ok(());
    };
    if voice.duration > state.config.limits.max_audio_duration_secs {
        bot.send_message(cid, s.voice_too_long).await?;
        state.pending.remove(&user_id);
        return Ok(());
    }

    let _ = bot.send_chat_action(cid, ChatAction::UploadVoice).await;
    let thinking = bot.send_message(cid, s.loading_synth).await?;
    let tg_file = match bot.get_file(voice.file.id.to_string()).await {
        Ok(f) => f,
        Err(e) => {
            error!("get_file: {e}");
            let _ = bot.delete_message(cid, thinking.id).await;
            bot.send_message(cid, s.clone_fail).await?;
            state.pending.remove(&user_id);
            return Ok(());
        }
    };
    let tmp_ogg = state.audio.tmp_path("ogg");
    let download_ok = async {
        let mut dst = tokio::fs::File::create(&tmp_ogg).await?;
        bot.download_file(&tg_file.path, &mut dst).await?;
        Ok::<(), teloxide::DownloadError>(())
    }
    .await;
    if download_ok.is_err() {
        error!("download_file failed");
        tokio::fs::remove_file(&tmp_ogg).await.ok();
        let _ = bot.delete_message(cid, thinking.id).await;
        bot.send_message(cid, s.clone_fail).await?;
        state.pending.remove(&user_id);
        return Ok(());
    }
    let ogg_bytes = match tokio::fs::read(&tmp_ogg).await {
        Ok(b) => b,
        Err(e) => {
            error!("read ogg: {e}");
            tokio::fs::remove_file(&tmp_ogg).await.ok();
            let _ = bot.delete_message(cid, thinking.id).await;
            bot.send_message(cid, s.clone_fail).await?;
            state.pending.remove(&user_id);
            return Ok(());
        }
    };
    tokio::fs::remove_file(&tmp_ogg).await.ok();
    let wav_path = match state.audio.ogg_to_wav(&ogg_bytes).await {
        Ok(p) => p,
        Err(e) => {
            error!("ogg->wav: {e}");
            let _ = bot.delete_message(cid, thinking.id).await;
            bot.send_message(cid, s.clone_fail).await?;
            state.pending.remove(&user_id);
            return Ok(());
        }
    };
    let (clone_id, saved_path) = match state.clone_engine.save_sample(user_id, &wav_path).await {
        Ok(r) => r,
        Err(e) => {
            error!("save_sample: {e}");
            tokio::fs::remove_file(&wav_path).await.ok();
            let _ = bot.delete_message(cid, thinking.id).await;
            bot.send_message(cid, s.clone_fail).await?;
            state.pending.remove(&user_id);
            return Ok(());
        }
    };
    tokio::fs::remove_file(&wav_path).await.ok();

    let vc = new_voice_clone(user_id, clone_id, saved_path, &state.config.clone.ref_text);
    let _ = state.db.save_clone(&vc);
    state.db.audit(user_id, "clone", "voice sample stored");

    state.pending.remove(&user_id);
    let _ = bot.delete_message(cid, thinking.id).await;
    bot.send_message(cid, s.clone_success)
        .reply_markup(keyboards::main_menu(s))
        .await?;
    Ok(())
}

// ─── callback query handler ───────────────────────────────────────────────────
pub async fn handle_callback(
    bot: Bot,
    q: CallbackQuery,
    state: AppState,
) -> Result<(), teloxide::RequestError> {
    let user_id = q.from.id.0 as i64;
    let Some(data) = q.data.as_deref() else {
        bot.answer_callback_query(q.id).await?;
        return Ok(());
    };
    let lang = user_lang(&state, user_id);
    let s = crate::i18n::get(&lang);

    // ── Main menu navigation ────────────────────────────────────────────────
    if let Some(action) = data.strip_prefix("menu:") {
        match action {
            "home" | "back" => {
                bot.answer_callback_query(&q.id).await?;
                if let Some(msg) = q.message {
                    bot.edit_message_text(msg.chat.id, msg.id, crate::i18n::md2(s.menu_header))
                        .parse_mode(ParseMode::MarkdownV2)
                        .reply_markup(keyboards::main_menu(s))
                        .await?;
                }
                return Ok(());
            }
            "voices_back" => {
                bot.answer_callback_query(&q.id).await?;
                if let Some(msg) = q.message {
                    let installed = state.tts.available_voices();
                    let active = user_voice(&state, user_id, &lang);
                    bot.edit_message_text(msg.chat.id, msg.id, crate::i18n::md2(s.voices_header))
                        .parse_mode(ParseMode::MarkdownV2)
                        .reply_markup(keyboards::voices_keyboard(&lang, &installed, &active, 0))
                        .await?;
                }
                return Ok(());
            }
            "speak" => {
                bot.answer_callback_query(&q.id)
                    .text(s.speak_usage)
                    .show_alert(true)
                    .await?;
                return Ok(());
            }
            "ask" => {
                bot.answer_callback_query(&q.id)
                    .text(s.ask_usage)
                    .show_alert(true)
                    .await?;
                return Ok(());
            }
            "clone" => {
                bot.answer_callback_query(&q.id).await?;
                if let Some(msg) = q.message {
                    bot.edit_message_text(msg.chat.id, msg.id, s.clone_prompt)
                        .reply_markup(keyboards::consent_keyboard())
                        .await?;
                }
                state
                    .pending
                    .insert(user_id, PendingAction::AwaitingVoiceForClone);
                return Ok(());
            }
            "voices" => {
                bot.answer_callback_query(&q.id).await?;
                let installed = state.tts.available_voices();
                let active = user_voice(&state, user_id, &lang);
                if let Some(msg) = q.message {
                    let header = if lang == "ar" {
                        format!("{}\n\n{}", s.voices_header, s.arabic_more)
                    } else {
                        s.voices_header.to_string()
                    };
                    bot.edit_message_text(msg.chat.id, msg.id, crate::i18n::md2(&header))
                        .parse_mode(ParseMode::MarkdownV2)
                        .reply_markup(keyboards::voices_keyboard(&lang, &installed, &active, 0))
                        .await?;
                }
                return Ok(());
            }
            "credits" => {
                bot.answer_callback_query(&q.id).await?;
                let (credits, daily_used) = state
                    .db
                    .get_user(user_id)
                    .ok()
                    .flatten()
                    .map(|u| (u.credits, u.daily_used))
                    .unwrap_or((0, 0));
                let free_max = state.config.limits.free_daily_credits;
                let text = s
                    .credits_info
                    .replace("{credits}", &credits.to_string())
                    .replace("{free}", &daily_used.to_string())
                    .replace("{max}", &free_max.to_string());
                if let Some(msg) = q.message {
                    bot.edit_message_text(msg.chat.id, msg.id, text).await?;
                }
                return Ok(());
            }
            "stats" => {
                bot.answer_callback_query(&q.id).await?;
                let stats_engine = StatsEngine::new(state.db.clone());
                if let Some(msg) = q.message {
                    match stats_engine
                        .user_stats(user_id, state.config.limits.free_daily_credits)
                        .await
                    {
                        Ok(stats) => {
                            bot.edit_message_text(
                                msg.chat.id,
                                msg.id,
                                format_user_stats(&stats, s),
                            )
                            .await?;
                        }
                        Err(e) => {
                            error!("user stats error: {e}");
                            bot.edit_message_text(msg.chat.id, msg.id, "Failed to fetch stats")
                                .await?;
                        }
                    }
                }
                return Ok(());
            }
            "lang" => {
                bot.answer_callback_query(&q.id).await?;
                if let Some(msg) = q.message {
                    bot.edit_message_text(msg.chat.id, msg.id, s.choose_lang)
                        .reply_markup(keyboards::lang_keyboard())
                        .await?;
                }
                return Ok(());
            }
            "settings" => {
                bot.answer_callback_query(&q.id).await?;
                if let Some(msg) = q.message {
                    let _ = bot.delete_message(msg.chat.id, msg.id).await;
                }
                show_settings(
                    bot,
                    teloxide::types::ChatId(q.from.id.0 as i64),
                    user_id,
                    &state,
                    s,
                )
                .await?;
                return Ok(());
            }
            "upgrade" => {
                bot.answer_callback_query(&q.id).await?;
                if let Some(msg) = q.message {
                    bot.edit_message_text(
                        msg.chat.id,
                        msg.id,
                        crate::i18n::md2(&format!(
                            "{}\n\n{}\n\n{}",
                            s.upgrade_header, s.upgrade_info, s.payment_secure
                        )),
                    )
                    .parse_mode(ParseMode::MarkdownV2)
                    .reply_markup(keyboards::upgrade_menu())
                    .await?;
                }
                return Ok(());
            }
            _ => {}
        }
    }

    // ── Action buttons (new inline menu) ─────────────────────────────────────
    if let Some(act) = data.strip_prefix("act:") {
        bot.answer_callback_query(&q.id).await?;
        let cmd_msg = match q.message.clone() {
            Some(m) => m,
            None => return Ok(()),
        };
        match act {
            "ask" => {
                if let Some(m) = q.message {
                    bot.edit_message_text(m.chat.id, m.id,
                        "How should ANUBIS reply? Tap 💬 for a written answer or 🔊 for a spoken voice message.")
                        .reply_markup(keyboards::mode_keyboard(PromptKind::Ask))
                        .await?;
                }
            }
            "speak" => {
                handle_command(
                    bot.clone(),
                    cmd_msg.clone(),
                    Command::Speak(String::new()),
                    state.clone(),
                )
                .await?
            }
            "myvoice" => {
                handle_command(
                    bot.clone(),
                    cmd_msg.clone(),
                    Command::Myvoice(String::new()),
                    state.clone(),
                )
                .await?
            }
            "clone" => {
                handle_command(bot.clone(), cmd_msg.clone(), Command::Clone, state.clone()).await?
            }
            "voices" => {
                handle_command(bot.clone(), cmd_msg.clone(), Command::Voices, state.clone()).await?
            }
            "lang" => {
                handle_command(bot.clone(), cmd_msg.clone(), Command::Lang, state.clone()).await?
            }
            "credits" => {
                handle_command(
                    bot.clone(),
                    cmd_msg.clone(),
                    Command::Credits,
                    state.clone(),
                )
                .await?
            }
            "settings" => {
                handle_command(
                    bot.clone(),
                    cmd_msg.clone(),
                    Command::Settings,
                    state.clone(),
                )
                .await?
            }
            "stats" => {
                handle_command(
                    bot.clone(),
                    cmd_msg.clone(),
                    Command::MyStats,
                    state.clone(),
                )
                .await?
            }
            "help" => {
                handle_command(bot.clone(), cmd_msg.clone(), Command::Help, state.clone()).await?
            }
            "reset" => {
                handle_command(bot.clone(), cmd_msg.clone(), Command::Reset, state.clone()).await?
            }
            "upgrade" => {
                handle_command(
                    bot.clone(),
                    cmd_msg.clone(),
                    Command::Upgrade,
                    state.clone(),
                )
                .await?
            }
            _ => {}
        }
        return Ok(());
    }

    // ── Text/Voice mode choice ───────────────────────────────────────────────
    if let Some(rest) = data.strip_prefix("mode:") {
        bot.answer_callback_query(&q.id).await?;
        let (kind, mode) = match rest {
            "ask:text" => (PromptKind::Ask, OutMode::Text),
            "ask:voice" => (PromptKind::Ask, OutMode::Voice),
            _ => (PromptKind::Ask, OutMode::Text),
        };
        state
            .pending
            .insert(user_id, PendingAction::AwaitingPrompt { kind, mode });
        if let Some(m) = q.message {
            let prompt = match mode {
                OutMode::Text => "✍️ Type your question — ANUBIS will reply as text, then send it:",
                OutMode::Voice => {
                    "✍️ Type your question — ANUBIS will reply with a voice message, then send it:"
                }
            };
            bot.edit_message_text(m.chat.id, m.id, prompt).await?;
        }
        return Ok(());
    }

    // ── Voice gallery pagination ────────────────────────────────────────────
    if let Some(rest) = data.strip_prefix("vpage:") {
        bot.answer_callback_query(&q.id).await?;
        if rest == "noop" {
            return Ok(());
        }
        // rest = "<lang>:<page>"
        if let Some((vlang, page_str)) = rest.split_once(':') {
            if let Ok(page) = page_str.parse::<usize>() {
                let installed = state.tts.available_voices();
                let active = user_voice(&state, user_id, &lang);
                if let Some(msg) = q.message {
                    bot.edit_message_text(msg.chat.id, msg.id, crate::i18n::md2(s.voices_header))
                        .parse_mode(ParseMode::MarkdownV2)
                        .reply_markup(keyboards::voices_keyboard(vlang, &installed, &active, page))
                        .await?;
                }
            }
        }
        return Ok(());
    }

    // ── Download (install) a community Arabic voice ──────────────────────────
    if let Some(voice_id) = data.strip_prefix("dlvoice:") {
        bot.answer_callback_query(&q.id)
            .text(s.gallery_install)
            .show_alert(true)
            .await?;
        // Kick off the installer in the background (it downloads the requested
        // voice into ./voices/ar). Fire-and-forget; the user gets a toast.
        let vid = voice_id.to_string();
        tokio::spawn(async move {
            let _ = tokio::process::Command::new("./scripts/setup.sh")
                .arg("ar")
                .output()
                .await;
            tracing::info!("community Arabic voice install finished: {vid}");
        });
        return Ok(());
    }

    // ── "Speak this" from an AI reply ────────────────────────────────────────
    if let Some(rest) = data.strip_prefix("speaklast:") {
        bot.answer_callback_query(&q.id).await?;
        let last = state.last_reply.get(&user_id).map(|r| r.clone());
        let Some(reply) = last else {
            if let Some(msg) = q.message {
                bot.edit_message_text(msg.chat.id, msg.id, s.no_last_reply)
                    .await?;
            }
            return Ok(());
        };
        match rest {
            "active" => {
                let voice_id = user_voice(&state, user_id, &lang);
                gen_voice(
                    &bot,
                    teloxide::types::ChatId(q.from.id.0 as i64),
                    user_id,
                    &reply,
                    &voice_id,
                    &state,
                    s,
                    Some(s.speak_this),
                )
                .await?;
            }
            "gallery" => {
                // Open the full voice gallery; choosing a voice speaks `reply`.
                state.pending.insert(
                    user_id,
                    PendingAction::AwaitingPrompt {
                        kind: PromptKind::Ask,
                        mode: OutMode::Voice,
                    },
                );
                let installed = state.tts.available_voices();
                let active = user_voice(&state, user_id, &lang);
                if let Some(msg) = q.message {
                    bot.edit_message_text(
                        msg.chat.id,
                        msg.id,
                        crate::i18n::md2(&format!("{}\n\n{}", s.gallery_title, s.gallery_pick)),
                    )
                    .parse_mode(ParseMode::MarkdownV2)
                    .reply_markup(keyboards::voices_keyboard(&lang, &installed, &active, 0))
                    .await?;
                }
            }
            "regen" => {
                let bot2 = bot.clone();
                do_ask(
                    bot2,
                    teloxide::types::ChatId(q.from.id.0 as i64),
                    user_id,
                    &reply,
                    &lang,
                    state.clone(),
                    s,
                    OutMode::Text,
                )
                .await?;
                bot.answer_callback_query(&q.id).text(s.regen_done).await?;
            }
            _ => {}
        }
        return Ok(());
    }

    // ── Payment (Telegram Stars) ────────────────────────────────────────────
    if let Some(amount_str) = data.strip_prefix("pay:") {
        bot.answer_callback_query(&q.id).await?;
        if let Ok(amount) = amount_str.parse::<i32>() {
            let stars = stars_for(amount);
            let title = format!("ANUBIS — {} Credits", amount);
            let description = "Top up your ANUBIS voice generation credits.";
            let prices = vec![teloxide::types::LabeledPrice {
                label: format!("{} credits", amount),
                amount: stars,
            }];
            // Telegram Stars uses currency "XTR" and an empty provider token.
            if let Some(msg) = q.message {
                match bot
                    .send_invoice(
                        msg.chat.id,
                        title,
                        description.to_string(),
                        format!("anubis_credits:{}:{}", user_id, amount),
                        "".to_string(),    // provider_token (empty for XTR)
                        "XTR".to_string(), // currency
                        prices,
                    )
                    .await
                {
                    Ok(_) => {}
                    Err(e) => {
                        error!("send_invoice error: {e}");
                        bot.send_message(msg.chat.id, s.payment_failed).await?;
                    }
                }
            }
        }
        return Ok(());
    }

    if let Some(code) = data.strip_prefix("lang:") {
        let _ = state.db.set_lang(user_id, code);
        bot.answer_callback_query(&q.id)
            .text(crate::i18n::get(code).lang_set)
            .await?;
        if let Some(msg) = q.message {
            let _ = bot.delete_message(msg.chat.id, msg.id).await;
        }
        return Ok(());
    }
    if let Some(voice_id) = data.strip_prefix("voice:") {
        let _ = state.db.set_active_voice(user_id, voice_id);
        let name = voices::find(voice_id).map(|v| v.name).unwrap_or("voice");
        bot.answer_callback_query(&q.id)
            .text(format!("{} — {}: {}", s.preset_set, s.voice_active, name))
            .await?;
        if let Some(msg) = q.message {
            let _ = bot.delete_message(msg.chat.id, msg.id).await;
        }
        return Ok(());
    }
    if let Some(clone_id) = data.strip_prefix("del_clone:") {
        let wav_path = state
            .db
            .get_clones(user_id)
            .unwrap_or_default()
            .into_iter()
            .find(|c| c.id == clone_id)
            .map(|c| c.wav_path);
        let deleted = state.db.delete_clone(clone_id, user_id).unwrap_or(false);
        if deleted {
            if let Some(p) = wav_path {
                tokio::fs::remove_file(&p).await.ok();
            }
            state.db.audit(user_id, "delete_clone", clone_id);
            bot.answer_callback_query(&q.id)
                .text(s.clone_deleted)
                .await?;
        } else {
            bot.answer_callback_query(&q.id).text("Not found.").await?;
        }
        if let Some(msg) = q.message {
            let _ = bot.delete_message(msg.chat.id, msg.id).await;
        }
        return Ok(());
    }
    if let Some(answer) = data.strip_prefix("consent:") {
        if answer == "yes" {
            let _ = state.db.set_consent(user_id);
            state.db.audit(user_id, "consent", "voice cloning agreed");
            bot.answer_callback_query(&q.id)
                .text("Consent recorded. Use /clone to start.")
                .await?;
        } else {
            bot.answer_callback_query(&q.id).text("Cancelled.").await?;
        }
        if let Some(msg) = q.message {
            let _ = bot.delete_message(msg.chat.id, msg.id).await;
        }
        return Ok(());
    }
    warn!("unhandled callback data: {data}");
    bot.answer_callback_query(q.id).await?;
    Ok(())
}

/// Handle a successful Telegram Stars payment: grant the purchased credits.
/// Idempotent — replaying the same Telegram charge cannot double-credit,
/// because we key crediting on the unique `telegram_payment_charge_id`.
pub async fn handle_successful_payment(
    bot: Bot,
    msg: Message,
    state: AppState,
) -> Result<(), teloxide::RequestError> {
    let Some(payment) = msg.successful_payment() else {
        return Ok(());
    };
    let user_id = msg.from().map(|u| u.id.0 as i64).unwrap_or(0);
    let s = crate::i18n::get(&user_lang(&state, user_id));
    // payload = "anubis_credits:<user_id>:<amount>"
    let parts: Vec<&str> = payment.invoice_payload.split(':').collect();
    let mut ok = false;
    if parts.len() == 3 && parts[0] == "anubis_credits" {
        if let (Ok(payload_uid), Ok(amount)) = (parts[1].parse::<i64>(), parts[2].parse::<i32>()) {
            // Validate that the amount matches the tier and the payload user
            // is the one who actually paid (the invoice was sent to them).
            if payload_uid == user_id && amount > 0 && stars_for(amount) == payment.total_amount {
                let charge_id = payment.telegram_payment_charge_id.as_str();
                // Idempotent: record_payment returns false on replay.
                let credited = state.db.record_payment(
                    charge_id,
                    &payment.invoice_payload,
                    user_id,
                    amount,
                    payment.total_amount,
                );
                if credited {
                    state.db.audit(
                        user_id,
                        "purchase",
                        &format!(
                            "stars={} credits={} charge={}",
                            payment.total_amount, amount, charge_id
                        ),
                    );
                    ok = true;
                } else {
                    // Already credited (replay) — still acknowledge.
                    warn!("duplicate payment charge ignored: {charge_id}");
                    ok = true;
                }
            }
        }
    }
    let _ = bot
        .send_message(
            msg.chat.id,
            if ok {
                s.payment_success
            } else {
                s.payment_failed
            },
        )
        .await;
    Ok(())
}

/// Answer pre-checkout queries — required for payments to succeed.
pub async fn handle_pre_checkout(
    bot: Bot,
    q: PreCheckoutQuery,
    _state: AppState,
) -> Result<(), teloxide::RequestError> {
    bot.answer_pre_checkout_query(q.id, true).await?;
    Ok(())
}

/// Map credit amount to Telegram Stars price (1★ per 100 credits, min 1★).
fn stars_for(credits: i32) -> i32 {
    ((credits as f64) / 100.0).ceil().max(1.0) as i32
}

/// React to a user voice message as a spoken question for Noxis Core:
/// transcribe (local whisper) → run the brain (with conversation memory) →
/// synthesize the reply → send it back as a voice message with the transcript.
async fn handle_voice_conversation(
    bot: Bot,
    msg: Message,
    state: AppState,
) -> Result<(), teloxide::RequestError> {
    let user_id = uid(&msg);
    let lang = user_lang(&state, user_id);
    let s = crate::i18n::get(&lang);
    let cid = msg.chat.id;

    if !state.noxis.enabled() {
        bot.send_message(cid, s.brain_off).await?;
        return Ok(());
    }
    let Some(voice) = msg.voice() else {
        return Ok(());
    };
    if voice.duration > state.config.limits.max_audio_duration_secs {
        bot.send_message(cid, s.voice_too_long).await?;
        return Ok(());
    }
    if !state.rate_limiter.check(user_id, RateKind::Speak) {
        bot.send_message(cid, s.rate_limited).await?;
        return Ok(());
    }
    let _ = bot.send_chat_action(cid, ChatAction::RecordVoice).await;
    let thinking = bot.send_message(cid, s.loading_transcribe).await?;

    // 1) Download the OGG voice message.
    let tg_file = match bot.get_file(voice.file.id.to_string()).await {
        Ok(f) => f,
        Err(e) => {
            error!("voice conv get_file: {e}");
            let _ = bot.delete_message(cid, thinking.id).await;
            bot.send_message(cid, s.tts_fail).await?;
            return Ok(());
        }
    };
    let tmp_ogg = state.audio.tmp_path("ogg");
    let dl = async {
        let mut dst = tokio::fs::File::create(&tmp_ogg).await?;
        bot.download_file(&tg_file.path, &mut dst).await?;
        Ok::<(), teloxide::DownloadError>(())
    }
    .await;
    if dl.is_err() {
        tokio::fs::remove_file(&tmp_ogg).await.ok();
        let _ = bot.delete_message(cid, thinking.id).await;
        bot.send_message(cid, s.tts_fail).await?;
        return Ok(());
    }
    let ogg_bytes = match tokio::fs::read(&tmp_ogg).await {
        Ok(b) => b,
        Err(e) => {
            error!("voice conv read ogg: {e}");
            tokio::fs::remove_file(&tmp_ogg).await.ok();
            let _ = bot.delete_message(cid, thinking.id).await;
            bot.send_message(cid, s.tts_fail).await?;
            return Ok(());
        }
    };
    tokio::fs::remove_file(&tmp_ogg).await.ok();

    // 2) Convert OGG -> WAV (16-bit PCM mono, 22.05 kHz) for whisper.
    let wav_path = match state.audio.ogg_to_wav(&ogg_bytes).await {
        Ok(p) => p,
        Err(e) => {
            error!("voice conv ogg->wav: {e}");
            let _ = bot.delete_message(cid, thinking.id).await;
            bot.send_message(cid, s.tts_fail).await?;
            return Ok(());
        }
    };

    // 3) Transcribe locally with whisper.
    let text = match state.whisper.transcribe(&wav_path, Some(&lang)).await {
        Ok(t) => {
            let t = sanitize_text(&t);
            tokio::fs::remove_file(&wav_path).await.ok();
            t
        }
        Err(e) => {
            error!("whisper transcribe error: {e}");
            tokio::fs::remove_file(&wav_path).await.ok();
            let _ = bot.delete_message(cid, thinking.id).await;
            bot.send_message(cid, s.tts_fail).await?;
            return Ok(());
        }
    };
    if text.is_empty() {
        let _ = bot.delete_message(cid, thinking.id).await;
        bot.send_message(cid, s.tts_fail).await?;
        return Ok(());
    }
    state.db.audit(user_id, "voice_ask", &text);

    // 4) Ask Noxis with conversation memory (animated thinking).
    let _ = bot
        .edit_message_text(cid, thinking.id, s.loading_think)
        .await;
    let history = state.memory.history(user_id);
    let reply = match state.noxis.ask(&text, &lang, &history).await {
        Ok(r) => r,
        Err(e) => {
            error!("voice conv ask error: {e}");
            let _ = bot.delete_message(cid, thinking.id).await;
            bot.send_message(cid, s.tts_fail).await?;
            return Ok(());
        }
    };
    state.memory.push(user_id, &text, &reply);
    state.last_reply.insert(user_id, reply.clone());

    // 5) Synthesize the reply into the user's active voice and send as voice.
    let has_credit = state
        .db
        .consume_credit(
            user_id,
            state.config.limits.free_daily_credits,
            state.config.limits.unlimited_mode,
        )
        .unwrap_or(false);
    if !has_credit {
        let _ = bot.delete_message(cid, thinking.id).await;
        bot.send_message(cid, s.no_credits).await?;
        return Ok(());
    }
    let voice_id = user_voice(&state, user_id, &lang);
    let _ = bot
        .edit_message_text(cid, thinking.id, s.loading_synth)
        .await;
    let wav = match state.tts.synthesize_wav(&reply, &voice_id).await {
        Ok(p) => p,
        Err(e) => {
            error!("voice conv tts error: {e}");
            let _ = bot.delete_message(cid, thinking.id).await;
            bot.send_message(cid, s.tts_fail).await?;
            return Ok(());
        }
    };
    let ogg = match synthesize_to_ogg(&state, &wav).await {
        Ok(b) => b,
        Err(e) => {
            error!("voice conv wav->ogg: {e}");
            let _ = bot.delete_message(cid, thinking.id).await;
            bot.send_message(cid, s.tts_fail).await?;
            return Ok(());
        }
    };
    let _ = bot.delete_message(cid, thinking.id).await;
    bot.send_voice(cid, InputFile::memory(ogg))
        .caption(format!(
            "🗣 You: {}\n\n🤖 {}\n\n{}",
            text, reply, s.listen_hint
        ))
        .reply_markup(keyboards::reply_voice_keyboard())
        .await?;
    Ok(())
}
