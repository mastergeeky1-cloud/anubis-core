//! Message / command / callback handlers.
//!
//! ANUBIS Voice Teacher — the user-facing surface is intentionally small and
//! focused: ask your teacher, hear it speak, pick a voice + language, and
//! toggle teacher mode.

use super::{keyboards, AppState, PendingAction, PromptKind};
use crate::bot::commands::Command;
use crate::i18n;
use crate::tts::voices;
use teloxide::prelude::*;
use teloxide::types::{InputFile, Message};
use tracing::warn;

/// Entry point for a parsed command.
pub async fn handle_command(
    bot: Bot,
    msg: Message,
    cmd: Command,
    st: AppState,
) -> anyhow::Result<()> {
    match cmd {
        Command::Start => cmd_start(bot, msg, st).await?,
        Command::Help => cmd_help(bot, msg, st).await?,
        Command::Ask(text) => cmd_ask(bot, msg, text, st).await?,
        Command::Speak(text) => cmd_speak(bot, msg, text, st).await?,
        Command::Voices => cmd_voices(bot, msg, st).await?,
        Command::Lang => cmd_lang(bot, msg, st).await?,
        Command::Teacher(arg) => cmd_teacher(bot, msg, arg, st).await?,
        Command::Reset => cmd_reset(bot, msg, st).await?,
    }
    Ok(())
}

/// Entry point for a plain text message. Unknown slash commands get a
/// "command not found" reply; otherwise text goes to the teacher.
pub async fn handle_message(bot: Bot, msg: Message, st: AppState) -> anyhow::Result<()> {
    let user_id = msg.from().map(|u| u.id.0 as i64).unwrap_or(0);

    if let Some((_, act)) = st.pending.remove(&user_id) {
        match act {
            PendingAction::AwaitingPrompt { kind } => {
                let text = msg.text().unwrap_or("").trim().to_string();
                let lang = st.db.user_lang(user_id);
                if text.is_empty() {
                    let usage = match kind {
                        PromptKind::Ask => i18n::get(&lang).ask_usage,
                        PromptKind::Speak => i18n::get(&lang).speak_usage,
                    };
                    bot.send_message(msg.chat.id, usage).await?;
                    return Ok(());
                }
                match kind {
                    PromptKind::Ask => do_ask(&bot, &msg, &st, &text).await?,
                    PromptKind::Speak => do_speak(&bot, &msg, &st, &text).await?,
                }
                return Ok(());
            }
        }
    }

    if let Some(text) = msg.text() {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Ok(());
        }

        // Unknown slash commands → show help instead of sending to LLM
        if trimmed.starts_with('/') {
            let lang = st.db.user_lang(user_id);
            let s = i18n::get(&lang);
            bot.send_message(msg.chat.id, s.unknown_command).await?;
            bot.send_message(msg.chat.id, s.help).await?;
            return Ok(());
        }

        do_ask(&bot, &msg, &st, trimmed).await?;
    }
    Ok(())
}

/// Entry point for an inline callback (menu buttons, voice pickers …).
pub async fn handle_callback(bot: Bot, q: CallbackQuery, st: AppState) -> anyhow::Result<()> {
    let user = q.from.clone();
    let user_id = user.id.0 as i64;
    st.db.upsert_user(user_id, user.username.as_deref()).ok();
    let Some(data) = q.data.clone() else {
        return Ok(());
    };
    let lang = st.db.user_lang(user_id);
    let s = i18n::get(&lang);
    let chat = q.message.as_ref().map(|m| m.chat.id);
    let msg_id = q.message.as_ref().map(|m| m.id);

    bot.answer_callback_query(&q.id).await.ok();

    if let Some(code) = data.strip_prefix("lang:") {
        st.db.set_lang(user_id, code).ok();
        let best = voices::best_voice_for_lang(code).to_string();
        st.db.set_active_voice(user_id, &best).ok();
        if let Some(cid) = chat {
            bot.send_message(cid, s.lang_set).await?;
        }
        return Ok(());
    }

    if let Some(voice_id) = data.strip_prefix("voice:") {
        st.db.set_active_voice(user_id, voice_id).ok();
        if let Some(cid) = chat {
            bot.send_message(cid, s.voice_set).await?;
        }
        return Ok(());
    }

    if data.starts_with("listen:") {
        if let Some(cid) = chat {
            if let Some(reply) = st.last_reply.get(&user_id) {
                let text = reply.clone();
                match synth_and_send(&bot, cid, &st, &text, user_id).await {
                    Ok(_) => {}
                    Err(e) => {
                        warn!("listen synth error: {e}");
                        bot.send_message(cid, s.tts_fail).await?;
                    }
                }
            } else {
                bot.send_message(cid, s.no_voice_data).await?;
            }
        }
        return Ok(());
    }

    if let Some(action) = data.strip_prefix("menu:") {
        if action == "back" {
            if let (Some(cid), Some(mid)) = (chat, msg_id) {
                bot.edit_message_text(cid, mid, s.menu_header)
                    .reply_markup(keyboards::main_menu(&lang))
                    .await?;
            }
        }
        return Ok(());
    }

    if let Some(action) = data.strip_prefix("act:") {
        match action {
            "ask" => {
                st.pending.insert(
                    user_id,
                    PendingAction::AwaitingPrompt {
                        kind: PromptKind::Ask,
                    },
                );
                if let Some(cid) = chat {
                    bot.send_message(cid, s.ask_usage).await?;
                }
            }
            "speak" => {
                st.pending.insert(
                    user_id,
                    PendingAction::AwaitingPrompt {
                        kind: PromptKind::Speak,
                    },
                );
                if let Some(cid) = chat {
                    bot.send_message(cid, s.speak_usage).await?;
                }
            }
            "voices" => {
                if let Some(cid) = chat {
                    let active = st.db.active_voice(user_id);
                    bot.send_message(cid, s.voices_header)
                        .reply_markup(keyboards::voices_keyboard(&lang, &active))
                        .await?;
                }
            }
            "lang" => {
                if let Some(cid) = chat {
                    bot.send_message(cid, s.choose_lang)
                        .reply_markup(keyboards::lang_keyboard())
                        .await?;
                }
            }
            "teacher" => {
                if let Some(cid) = chat {
                    let cur = st.db.teacher_mode(user_id);
                    st.db.set_teacher_mode(user_id, !cur).ok();
                    let text = if !cur { s.teacher_on } else { s.teacher_off };
                    bot.send_message(cid, text).await?;
                }
            }
            "help" => {
                if let Some(cid) = chat {
                    bot.send_message(cid, s.help).await?;
                }
            }
            "reset" => {
                if let Some(cid) = chat {
                    st.memory.clear(user_id);
                    bot.send_message(cid, s.reset_done).await?;
                }
            }
            _ => {}
        }
        return Ok(());
    }

    Ok(())
}

// ─── Command implementations ────────────────────────────────────────────────

async fn cmd_start(bot: Bot, msg: Message, st: AppState) -> anyhow::Result<()> {
    let user_id = msg.from().map(|u| u.id.0 as i64).unwrap_or(0);
    let username = msg.from().and_then(|u| u.username.clone());
    st.db.upsert_user(user_id, username.as_deref()).ok();
    let lang = st.db.user_lang(user_id);
    let s = i18n::get(&lang);
    bot.send_message(msg.chat.id, s.welcome)
        .reply_markup(keyboards::main_menu(&lang))
        .await?;
    bot.send_message(msg.chat.id, s.choose_lang)
        .reply_markup(keyboards::lang_keyboard())
        .await?;
    Ok(())
}

async fn cmd_help(bot: Bot, msg: Message, st: AppState) -> anyhow::Result<()> {
    let user_id = msg.from().map(|u| u.id.0 as i64).unwrap_or(0);
    let lang = st.db.user_lang(user_id);
    bot.send_message(msg.chat.id, i18n::get(&lang).help).await?;
    Ok(())
}

async fn cmd_ask(bot: Bot, msg: Message, text: String, st: AppState) -> anyhow::Result<()> {
    let user_id = msg.from().map(|u| u.id.0 as i64).unwrap_or(0);
    let lang = st.db.user_lang(user_id);
    if text.trim().is_empty() {
        bot.send_message(msg.chat.id, i18n::get(&lang).ask_usage)
            .await?;
        return Ok(());
    }
    do_ask(&bot, &msg, &st, text.trim()).await?;
    Ok(())
}

async fn cmd_speak(bot: Bot, msg: Message, text: String, st: AppState) -> anyhow::Result<()> {
    let user_id = msg.from().map(|u| u.id.0 as i64).unwrap_or(0);
    let lang = st.db.user_lang(user_id);
    if text.trim().is_empty() {
        bot.send_message(msg.chat.id, i18n::get(&lang).speak_usage)
            .await?;
        return Ok(());
    }
    do_speak(&bot, &msg, &st, text.trim()).await?;
    Ok(())
}

async fn cmd_voices(bot: Bot, msg: Message, st: AppState) -> anyhow::Result<()> {
    let user_id = msg.from().map(|u| u.id.0 as i64).unwrap_or(0);
    let lang = st.db.user_lang(user_id);
    let active = st.db.active_voice(user_id);
    bot.send_message(msg.chat.id, i18n::get(&lang).voices_header)
        .reply_markup(keyboards::voices_keyboard(&lang, &active))
        .await?;
    Ok(())
}

async fn cmd_lang(bot: Bot, msg: Message, st: AppState) -> anyhow::Result<()> {
    let user_id = msg.from().map(|u| u.id.0 as i64).unwrap_or(0);
    let lang = st.db.user_lang(user_id);
    bot.send_message(msg.chat.id, i18n::get(&lang).choose_lang)
        .reply_markup(keyboards::lang_keyboard())
        .await?;
    Ok(())
}

async fn cmd_teacher(bot: Bot, msg: Message, arg: String, st: AppState) -> anyhow::Result<()> {
    let user_id = msg.from().map(|u| u.id.0 as i64).unwrap_or(0);
    let s = i18n::get(&st.db.user_lang(user_id));
    match arg.trim().to_ascii_lowercase().as_str() {
        "on" | "enable" | "1" => {
            st.db.set_teacher_mode(user_id, true).ok();
            bot.send_message(msg.chat.id, s.teacher_on).await?;
        }
        "off" | "disable" | "0" => {
            st.db.set_teacher_mode(user_id, false).ok();
            bot.send_message(msg.chat.id, s.teacher_off).await?;
        }
        "status" | "" => {
            let status = if st.db.teacher_mode(user_id) {
                s.teacher_status_on
            } else {
                s.teacher_status_off
            };
            bot.send_message(msg.chat.id, status).await?;
        }
        _ => {
            bot.send_message(msg.chat.id, s.teacher_usage).await?;
        }
    }
    Ok(())
}

async fn cmd_reset(bot: Bot, msg: Message, st: AppState) -> anyhow::Result<()> {
    let user_id = msg.from().map(|u| u.id.0 as i64).unwrap_or(0);
    st.memory.clear(user_id);
    bot.send_message(msg.chat.id, i18n::get(&st.db.user_lang(user_id)).reset_done)
        .await?;
    Ok(())
}

// ─── Core flows ─────────────────────────────────────────────────────────────

/// Ask the teacher and reply as text with a "Listen" button.
async fn do_ask(bot: &Bot, msg: &Message, st: &AppState, text: &str) -> anyhow::Result<()> {
    let user_id = msg.from().map(|u| u.id.0 as i64).unwrap_or(0);
    let lang = st.db.user_lang(user_id);
    let teacher_mode = st.db.teacher_mode(user_id);
    let s = i18n::get(&lang);
    let history = st.memory.history(user_id);

    let think = bot.send_message(msg.chat.id, s.loading_think).await?;

    let reply = match st.noxis.ask(text, &lang, &history, teacher_mode).await {
        Ok(r) => r,
        Err(e) => {
            warn!("noxis ask error: {e}");
            bot.edit_message_text(msg.chat.id, think.id, s.brain_off)
                .await?;
            return Ok(());
        }
    };

    st.memory.push(user_id, text, &reply);
    st.last_reply.insert(user_id, reply.clone());
    bot.delete_message(msg.chat.id, think.id).await.ok();
    bot.send_message(msg.chat.id, reply)
        .reply_markup(keyboards::reply_voice_keyboard(&lang))
        .await?;
    Ok(())
}

/// Synthesize `text` with the user's active voice and send as a voice note.
async fn do_speak(bot: &Bot, msg: &Message, st: &AppState, text: &str) -> anyhow::Result<()> {
    let user_id = msg.from().map(|u| u.id.0 as i64).unwrap_or(0);
    match synth_and_send(bot, msg.chat.id, st, text, user_id).await {
        Ok(_) => {}
        Err(e) => {
            warn!("speak synth error: {e}");
            bot.send_message(msg.chat.id, i18n::get(&st.db.user_lang(user_id)).tts_fail)
                .await?;
        }
    }
    Ok(())
}

/// Use the worker pool to synthesize `text` into a voice note for `chat_id`.
async fn synth_and_send(
    bot: &Bot,
    chat_id: teloxide::types::ChatId,
    st: &AppState,
    text: &str,
    user_id: i64,
) -> anyhow::Result<()> {
    let voice_id = st.db.active_voice(user_id);
    let wav = st
        .worker_pool
        .synthesize_tts(text, &voice_id)
        .await
        .map_err(|e| anyhow::anyhow!("tts: {e}"))?;
    let ogg = st
        .worker_pool
        .convert_wav_to_ogg(&wav)
        .await
        .map_err(|e| anyhow::anyhow!("ogg: {e}"))?;
    crate::tts::remove_wav(&wav).await;
    bot.send_voice(chat_id, InputFile::memory(ogg)).await?;
    Ok(())
}
