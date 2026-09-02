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
use teloxide::types::{ChatId, InputFile, LabeledPrice, Message, ParseMode, PreCheckoutQuery};
use tracing::warn;

/// Helper: send an HTML-formatted text message.
async fn send_html(bot: &Bot, chat_id: ChatId, text: &str) -> anyhow::Result<Message> {
    bot.send_message(chat_id, text)
        .parse_mode(ParseMode::Html)
        .await
        .map_err(Into::into)
}

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
        Command::Credits => cmd_credits(bot, msg, st).await?,
        Command::Upgrade => cmd_upgrade(bot, msg, st).await?,
        Command::Mystats => cmd_mystats(bot, msg, st).await?,
    }
    Ok(())
}

/// Entry point for a plain text message. Unknown slash commands get a
/// "command not found" reply; otherwise text goes to the teacher.
pub async fn handle_message(bot: Bot, msg: Message, st: AppState) -> anyhow::Result<()> {
    let user_id = msg.from().map(|u| u.id.0 as i64).unwrap_or(0);

    // If the message is a slash command, ignore any pending prompt and route
    // it to command handling (it was not caught by filter_command because it
    // is unregistered, so it lands here).
    let is_slash = msg
        .text()
        .map(|t| t.trim().starts_with('/'))
        .unwrap_or(false);

    if is_slash {
        let lang = st.db.user_lang(user_id);
        let s = i18n::get(&lang);
        send_html(&bot, msg.chat.id, s.unknown_command).await?;
        send_html(&bot, msg.chat.id, s.help).await?;
        st.pending.remove(&user_id);
        return Ok(());
    }

    if let Some((_, act)) = st.pending.remove(&user_id) {
        match act {
            PendingAction::AwaitingPrompt { kind } => {
                let text = msg.text().unwrap_or("").trim().to_string();
                if text.is_empty() {
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
            send_html(&bot, cid, s.lang_set).await?;
        }
        return Ok(());
    }

    if let Some(voice_id) = data.strip_prefix("voice:") {
        st.db.set_active_voice(user_id, voice_id).ok();
        if let Some(cid) = chat {
            send_html(&bot, cid, s.voice_set).await?;
        }
        return Ok(());
    }

    if data.starts_with("listen:") {
        if let Some(cid) = chat {
            if let Some(reply) = st.last_reply.get(&user_id) {
                let text = reply.clone();
                let wait = send_html(&bot, cid, s.loading_synth).await?;
                match synth_and_send(&bot, cid, &st, &text, user_id).await {
                    Ok(_) => {
                        bot.delete_message(cid, wait.id).await.ok();
                    }
                    Err(e) => {
                        warn!("listen synth error: {e}");
                        bot.edit_message_text(cid, wait.id, s.tts_fail)
                            .parse_mode(ParseMode::Html)
                            .await?;
                    }
                }
            } else {
                send_html(&bot, cid, s.no_voice_data).await?;
            }
        }
        return Ok(());
    }

    if let Some(action) = data.strip_prefix("menu:") {
        if action == "back" {
            if let (Some(cid), Some(mid)) = (chat, msg_id) {
                bot.edit_message_text(cid, mid, s.menu_header)
                    .reply_markup(keyboards::main_menu(&lang))
                    .parse_mode(ParseMode::Html)
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
            }
            "speak" => {
                st.pending.insert(
                    user_id,
                    PendingAction::AwaitingPrompt {
                        kind: PromptKind::Speak,
                    },
                );
            }
            "voices" => {
                if let Some(cid) = chat {
                    let active = st.db.active_voice(user_id);
                    bot.send_message(cid, s.voices_header)
                        .reply_markup(keyboards::voices_keyboard(&lang, &active))
                        .parse_mode(ParseMode::Html)
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
                    send_html(&bot, cid, text).await?;
                }
            }
            "help" => {
                if let Some(cid) = chat {
                    send_html(&bot, cid, s.help).await?;
                }
            }
            "credits" => {
                if let Some(cid) = chat {
                    let text = s
                        .credits_info
                        .replace(
                            "{credits}",
                            &st.db
                                .get_user(user_id)
                                .ok()
                                .flatten()
                                .map(|u| u.credits)
                                .unwrap_or(0)
                                .to_string(),
                        )
                        .replace(
                            "{used}",
                            &st.db
                                .get_user(user_id)
                                .ok()
                                .flatten()
                                .map(|u| u.daily_used)
                                .unwrap_or(0)
                                .to_string(),
                        );
                    send_html(&bot, cid, &text).await?;
                }
            }
            "upgrade" => {
                if let Some(cid) = chat {
                    send_stars_invoice(&bot, cid, s).await?;
                }
            }
            "reset" => {
                if let Some(cid) = chat {
                    st.memory.clear(user_id);
                    send_html(&bot, cid, s.reset_done).await?;
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
        .parse_mode(ParseMode::Html)
        .await?;
    bot.send_message(msg.chat.id, s.choose_lang)
        .reply_markup(keyboards::lang_keyboard())
        .await?;
    Ok(())
}

async fn cmd_help(bot: Bot, msg: Message, st: AppState) -> anyhow::Result<()> {
    let user_id = msg.from().map(|u| u.id.0 as i64).unwrap_or(0);
    let lang = st.db.user_lang(user_id);
    send_html(&bot, msg.chat.id, i18n::get(&lang).help).await?;
    Ok(())
}

async fn cmd_ask(bot: Bot, msg: Message, text: String, st: AppState) -> anyhow::Result<()> {
    let user_id = msg.from().map(|u| u.id.0 as i64).unwrap_or(0);
    if text.trim().is_empty() {
        st.pending.insert(
            user_id,
            PendingAction::AwaitingPrompt {
                kind: PromptKind::Ask,
            },
        );
        return Ok(());
    }
    do_ask(&bot, &msg, &st, text.trim()).await?;
    Ok(())
}

async fn cmd_speak(bot: Bot, msg: Message, text: String, st: AppState) -> anyhow::Result<()> {
    let user_id = msg.from().map(|u| u.id.0 as i64).unwrap_or(0);
    if text.trim().is_empty() {
        st.pending.insert(
            user_id,
            PendingAction::AwaitingPrompt {
                kind: PromptKind::Speak,
            },
        );
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
        .parse_mode(ParseMode::Html)
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
            send_html(&bot, msg.chat.id, s.teacher_on).await?;
        }
        "off" | "disable" | "0" => {
            st.db.set_teacher_mode(user_id, false).ok();
            send_html(&bot, msg.chat.id, s.teacher_off).await?;
        }
        "status" | "" => {
            let status = if st.db.teacher_mode(user_id) {
                s.teacher_status_on
            } else {
                s.teacher_status_off
            };
            send_html(&bot, msg.chat.id, status).await?;
        }
        _ => {
            send_html(&bot, msg.chat.id, s.teacher_usage).await?;
        }
    }
    Ok(())
}

async fn cmd_reset(bot: Bot, msg: Message, st: AppState) -> anyhow::Result<()> {
    let user_id = msg.from().map(|u| u.id.0 as i64).unwrap_or(0);
    st.memory.clear(user_id);
    send_html(
        &bot,
        msg.chat.id,
        i18n::get(&st.db.user_lang(user_id)).reset_done,
    )
    .await?;
    Ok(())
}

// ─── Credits / Payments ─────────────────────────────────────────────────────

async fn cmd_credits(bot: Bot, msg: Message, st: AppState) -> anyhow::Result<()> {
    let user_id = msg.from().map(|u| u.id.0 as i64).unwrap_or(0);
    let lang = st.db.user_lang(user_id);
    let s = i18n::get(&lang);
    // Read current credits + daily usage from DB.
    let user = st.db.get_user(user_id).ok().flatten();
    let (credits, daily_used) = match user {
        Some(u) => (u.credits, u.daily_used),
        None => (0, 0),
    };
    let text = s
        .credits_info
        .replace("{credits}", &credits.to_string())
        .replace("{used}", &daily_used.to_string());
    send_html(&bot, msg.chat.id, &text).await?;
    Ok(())
}

/// Send the 50-credits Telegram Stars invoice. Stars invoices use currency
/// "XTR" with an empty provider token (per Bot API).
async fn send_stars_invoice(bot: &Bot, chat_id: ChatId, s: &i18n::Strings) -> anyhow::Result<()> {
    let stars = 50;
    let credits = 50;
    let payload = format!("credits_{credits}");

    bot.send_invoice(
        chat_id,
        "ANUBIS — 50 Credits",
        s.upgrade_info,
        payload,
        String::new(), // Telegram Stars: provider token must be empty
        "XTR",
        vec![LabeledPrice::new("50 credits", stars)],
    )
    .await?;
    Ok(())
}

async fn cmd_upgrade(bot: Bot, msg: Message, st: AppState) -> anyhow::Result<()> {
    let user_id = msg.from().map(|u| u.id.0 as i64).unwrap_or(0);
    let lang = st.db.user_lang(user_id);
    let s = i18n::get(&lang);

    st.db
        .upsert_user(
            user_id,
            msg.from().and_then(|u| u.username.clone()).as_deref(),
        )
        .ok();

    send_stars_invoice(&bot, msg.chat.id, s).await?;
    Ok(())
}

async fn cmd_mystats(bot: Bot, msg: Message, st: AppState) -> anyhow::Result<()> {
    let user_id = msg.from().map(|u| u.id.0 as i64).unwrap_or(0);
    let lang = st.db.user_lang(user_id);
    let s = i18n::get(&lang);
    let user = st.db.get_user(user_id).ok().flatten();
    let (credits, daily_used) = match user {
        Some(u) => (u.credits, u.daily_used),
        None => (0, 0),
    };
    let credits_line = s
        .credits_info
        .replace("{credits}", &credits.to_string())
        .replace("{used}", &daily_used.to_string());
    let text = format!("{}\n\n{}", s.mystats_header, credits_line);
    send_html(&bot, msg.chat.id, &text).await?;
    Ok(())
}

// ─── Core flows ─────────────────────────────────────────────────────────────

/// Ask the teacher and reply as text with a "Listen" button.
async fn do_ask(bot: &Bot, msg: &Message, st: &AppState, text: &str) -> anyhow::Result<()> {
    let user_id = msg.from().map(|u| u.id.0 as i64).unwrap_or(0);
    let lang = st.db.user_lang(user_id);
    let s = i18n::get(&lang);

    // Free daily quota: 3 per day, unless user has purchased unlimited credits.
    let user = st.db.get_user(user_id).ok().flatten();
    let credits = user.as_ref().map(|u| u.credits).unwrap_or(0);
    let free_daily = st.config.limits.free_daily;
    let unlimited = credits >= 999;
    if !st.db.consume_credit(user_id, free_daily, unlimited)? {
        send_html(bot, msg.chat.id, s.no_credits).await?;
        return Ok(());
    }

    let teacher_mode = st.db.teacher_mode(user_id);
    let history = st.memory.history(user_id);

    let think = send_html(bot, msg.chat.id, s.loading_think).await?;

    let reply = match st.noxis.ask(text, &lang, &history, teacher_mode).await {
        Ok(r) => r,
        Err(e) => {
            warn!("noxis ask error: {e}");
            bot.edit_message_text(msg.chat.id, think.id, s.brain_off)
                .parse_mode(ParseMode::Html)
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
            send_html(
                bot,
                msg.chat.id,
                i18n::get(&st.db.user_lang(user_id)).tts_fail,
            )
            .await?;
        }
    }
    Ok(())
}

/// Use the worker pool to synthesize `text` into a voice note for `chat_id`.
async fn synth_and_send(
    bot: &Bot,
    chat_id: ChatId,
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

// ─── Telegram Stars Payments ────────────────────────────────────────────────

/// Pre-checkout: validate the payment before Telegram charges the user.
pub async fn handle_pre_checkout(
    bot: Bot,
    q: PreCheckoutQuery,
    st: AppState,
) -> anyhow::Result<()> {
    let user_id = q.from.id.0 as i64;
    let lang = st.db.user_lang(user_id);
    let s = i18n::get(&lang);

    // Only accept our own payloads.
    if !q.invoice_payload.starts_with("credits_") {
        bot.answer_pre_checkout_query(q.id, false)
            .error_message(s.payment_failed.to_string())
            .await?;
        return Ok(());
    }

    st.db.upsert_user(user_id, q.from.username.as_deref()).ok();
    bot.answer_pre_checkout_query(q.id, true).await?;
    Ok(())
}

/// Successful payment: grant credits to the user.
pub async fn handle_successful_payment(bot: Bot, msg: Message, st: AppState) -> anyhow::Result<()> {
    let user_id = msg.from().map(|u| u.id.0 as i64).unwrap_or(0);
    let lang = st.db.user_lang(user_id);
    let s = i18n::get(&lang);

    if let Some(payment) = msg.successful_payment() {
        let payload = &payment.invoice_payload;
        let charge_id = &payment.telegram_payment_charge_id;
        let total_stars = payment.total_amount;

        // Parse credits from payload: "credits_N"
        let credits = payload
            .strip_prefix("credits_")
            .and_then(|n| n.parse::<i32>().ok())
            .unwrap_or(50);

        st.db
            .record_payment(user_id, charge_id, payload, credits, total_stars)?;
        st.db.add_credits(user_id, credits, "stars_purchase")?;

        let text = s.payment_success.replace("{credits}", &credits.to_string());
        send_html(&bot, msg.chat.id, &text).await?;
    }

    Ok(())
}
