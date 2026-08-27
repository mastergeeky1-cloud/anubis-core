use super::commands::Command;
use super::{keyboards, AppState, PendingAction};
use crate::clone::new_voice_clone;
use crate::security::ratelimit::RateKind;
use crate::security::sanitize_text;
use crate::tts::voices;
use chrono::Utc;
use teloxide::net::Download;
use teloxide::prelude::*;
use teloxide::types::{InputFile, ParseMode};
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
fn ensure_user(state: &AppState, user_id: i64, username: Option<&str>) -> String {
    let _ = state.db.upsert_user(user_id, username);
    user_lang(state, user_id)
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
            bot.send_message(cid, s.welcome)
                .parse_mode(ParseMode::MarkdownV2)
                .await?;
        }
        Command::Help => {
            bot.send_message(cid, s.help)
                .parse_mode(ParseMode::MarkdownV2)
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
            bot.send_message(cid, s.voices_header)
                .parse_mode(ParseMode::MarkdownV2)
                .reply_markup(keyboards::voices_keyboard(&lang, &installed))
                .await?;
        }
        Command::Presets => {
            bot.send_message(cid, "Choose a voice preset:")
                .reply_markup(keyboards::presets_keyboard())
                .await?;
        }
        Command::Setvoice(id) => {
            let id = id.trim().to_string();
            if voices::find(&id).is_none() {
                bot.send_message(cid, format!("Unknown voice: `{}`. Use /voices.", id))
                    .parse_mode(ParseMode::MarkdownV2)
                    .await?;
                return Ok(());
            }
            let _ = state.db.set_active_voice(user_id, &id);
            bot.send_message(cid, s.voice_set).await?;
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
                bot.send_message(cid, s.my_clones)
                    .parse_mode(ParseMode::MarkdownV2)
                    .reply_markup(keyboards::clones_keyboard(&clones))
                    .await?;
            }
        }
        Command::Speak(text) => {
            let text = sanitize_text(&text);
            if text.is_empty() {
                bot.send_message(cid, s.speak_usage).await?;
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
                bot.send_message(cid, s.ask_usage).await?;
                return Ok(());
            }
            if !state.noxis.enabled() {
                bot.send_message(cid, s.brain_off).await?;
                return Ok(());
            }
            let thinking = bot.send_message(cid, s.generating).await?;
            state.db.audit(user_id, "ask", &text);
            match state.noxis.ask(&text, &lang).await {
                Ok(reply) => {
                    let _ = bot.delete_message(cid, thinking.id).await;
                    bot.send_message(cid, reply).await?;
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
        }
        Command::Myvoice(text) => {
            let text = sanitize_text(&text);
            if text.is_empty() {
                bot.send_message(cid, s.speak_usage).await?;
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
        // ── admin ──
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
            let (users, gens) = state.db.stats().unwrap_or((0, 0));
            bot.send_message(
                cid,
                format!("Users: {}\nTotal generations: {}", users, gens),
            )
            .await?;
        }
    }
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
    let voice_id = state
        .db
        .get_user(user_id)
        .ok()
        .flatten()
        .map(|u| u.active_voice)
        .unwrap_or_else(|| voices::default_for_lang(lang).to_string());

    let cache_key = crate::cache::AudioCache::make_key(text, &voice_id);
    if let Some(ogg_bytes) = state.cache.get(cache_key) {
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
        bot.send_message(cid, s.no_credits).await?;
        return Ok(());
    }
    state
        .db
        .audit(user_id, "speak", &format!("voice={voice_id}"));

    let thinking = bot.send_message(cid, s.generating).await?;
    let wav_path = match state.tts.synthesize_wav(text, &voice_id).await {
        Ok(p) => p,
        Err(e) => {
            error!("TTS error for user {user_id}: {e}");
            let _ = bot.delete_message(cid, thinking.id).await;
            bot.send_message(cid, s.tts_fail).await?;
            return Ok(());
        }
    };
    let ogg_bytes = match synthesize_to_ogg(&state, &wav_path).await {
        Ok(b) => b,
        Err(e) => {
            error!("wav->ogg: {e}");
            crate::tts::remove_wav(&wav_path).await;
            let _ = bot.delete_message(cid, thinking.id).await;
            bot.send_message(cid, s.tts_fail).await?;
            return Ok(());
        }
    };
    let _ = bot.delete_message(cid, thinking.id).await;
    state.cache.insert(cache_key, ogg_bytes.clone());
    bot.send_voice(cid, InputFile::memory(ogg_bytes)).await?;
    Ok(())
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

    let thinking = bot.send_message(cid, s.generating).await?;
    let wav_bytes = match state
        .clone_engine
        .synthesize(
            text,
            std::path::Path::new(&clone.wav_path),
            &lang,
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

/// Shared WAV->OGG step used by /speak.
async fn synthesize_to_ogg(
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
    let Some(PendingAction::AwaitingVoiceForClone) = state.pending.get(&user_id).map(|r| r.clone())
    else {
        if msg.text().is_some() {
            bot.send_message(cid, s.unknown_cmd).await?;
        }
        return Ok(());
    };

    let Some(voice) = msg.voice() else {
        bot.send_message(cid, s.send_voice_to_clone).await?;
        return Ok(());
    };
    if voice.duration > state.config.limits.max_audio_duration_secs {
        bot.send_message(cid, s.voice_too_long).await?;
        state.pending.remove(&user_id);
        return Ok(());
    }

    let thinking = bot.send_message(cid, s.generating).await?;
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
    bot.send_message(cid, s.clone_success).await?;
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
        bot.answer_callback_query(&q.id).text(s.voice_set).await?;
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
