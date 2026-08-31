//! Bot handlers module
//!
//! Split into submodules for maintainability:
//! - `commands`: Command parsing (BotCommands derive)
//! - `keyboards`: Inline keyboard builders
//! - `handlers`: Message/command/callback handlers
//! - `stats`: User/admin statistics handlers

pub mod commands;
pub mod handlers;
pub mod keyboards;
pub mod stats;

use crate::audio::AudioProcessor;
use crate::cache::AudioCache;
use crate::clone::ChatterboxCloner;
use crate::config::Config;
use crate::db::Database;
use crate::memory::ConversationStore;
use crate::noxis::NoxisCore;
use crate::security::{RateLimiter, Watermarker};
use crate::tts::router::TtsRouter;
use crate::whisper::WhisperClient;
use dashmap::DashMap;
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutMode {
    Text,
    Voice,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptKind {
    Ask,
    Speak,
    MyVoice,
}

#[derive(Debug, Clone)]
pub enum PendingAction {
    AwaitingVoiceForClone,
    /// Bot asked the user to type free-form input for a command (tap-from-menu
    /// flow), which arrives as the next plain-text message. `mode` selects the
    /// output format (text chat vs. spoken voice).
    AwaitingPrompt {
        kind: PromptKind,
        mode: OutMode,
    },
}

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
    pub tts: Arc<TtsRouter>,
    pub clone_engine: Arc<ChatterboxCloner>,
    pub noxis: Arc<NoxisCore>,
    pub whisper: Arc<WhisperClient>,
    pub memory: Arc<ConversationStore>,
    pub audio: Arc<AudioProcessor>,
    pub config: Arc<Config>,
    pub pending: Arc<DashMap<i64, PendingAction>>,
    pub rate_limiter: Arc<RateLimiter>,
    pub cache: Arc<AudioCache>,
    pub watermark: Arc<Watermarker>,
    /// Last AI reply text per user — lets the "🔊 Speak this" / "🎨 Voice
    /// Gallery" inline buttons re-synthesize the previous answer on demand.
    pub last_reply: Arc<DashMap<i64, String>>,
}

pub async fn run(state: AppState) -> anyhow::Result<()> {
    use commands::Command;
    use handlers::{
        handle_callback, handle_command, handle_message, handle_pre_checkout,
        handle_successful_payment,
    };

    let bot = Bot::new(state.config.telegram.token.clone());
    bot.set_my_commands(Command::bot_commands()).await?;

    let handler = dptree::entry()
        .branch(
            Update::filter_message()
                .filter_command::<Command>()
                .endpoint(
                    |bot: Bot, msg: Message, cmd: Command, st: AppState| async move {
                        handle_command(bot, msg, cmd, st).await
                    },
                ),
        )
        .branch(Update::filter_pre_checkout_query().endpoint(
            |bot: Bot, q: PreCheckoutQuery, st: AppState| async move {
                handle_pre_checkout(bot, q, st).await
            },
        ))
        .branch(
            Update::filter_message()
                .filter(|msg: Message| msg.successful_payment().is_some())
                .endpoint(|bot: Bot, msg: Message, st: AppState| async move {
                    handle_successful_payment(bot, msg, st).await
                }),
        )
        .branch(
            Update::filter_message().endpoint(|bot: Bot, msg: Message, st: AppState| async move {
                handle_message(bot, msg, st).await
            }),
        )
        .branch(
            Update::filter_callback_query().endpoint(
                |bot: Bot, q: CallbackQuery, st: AppState| async move {
                    handle_callback(bot, q, st).await
                },
            ),
        );

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![state])
        .default_handler(|_| async {})
        .error_handler(LoggingErrorHandler::with_custom_text(
            "ANUBIS dispatcher error",
        ))
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn out_mode_discriminates_text_vs_voice() {
        // The AI-chat voice pipeline keys off this: Text = reply as message,
        // Voice = synthesize + send as voice note.
        let text = OutMode::Text;
        let voice = OutMode::Voice;
        assert_ne!(
            core::mem::discriminant(&text),
            core::mem::discriminant(&voice)
        );
    }

    #[test]
    fn awaiting_prompt_carries_kind_and_mode() {
        let a = PendingAction::AwaitingPrompt {
            kind: PromptKind::Ask,
            mode: OutMode::Voice,
        };
        let b = PendingAction::AwaitingPrompt {
            kind: PromptKind::Speak,
            mode: OutMode::Text,
        };
        // Two AwaitingPrompt values with different inner fields must differ
        // when compared structurally (kind/mode), even though they share the
        // same enum discriminant.
        assert_ne!(
            (matches!(&a, PendingAction::AwaitingPrompt { kind, mode } if *kind == PromptKind::Ask && *mode == OutMode::Voice)),
            (matches!(&b, PendingAction::AwaitingPrompt { kind, mode } if *kind == PromptKind::Ask && *mode == OutMode::Voice)),
        );
        // Clone-vs-ask paths are a distinct variant.
        let clone = PendingAction::AwaitingVoiceForClone;
        assert!(core::mem::discriminant(&a) != core::mem::discriminant(&clone));
    }
}
