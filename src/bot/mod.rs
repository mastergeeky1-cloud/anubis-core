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

use crate::audio::AudioProcessor;
use crate::config::Config;
use crate::db::Database;
use crate::memory::ConversationStore;
use crate::noxis::NoxisCore;
use crate::whisper::WhisperClient;
use dashmap::DashMap;
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;
use tracing::info;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptKind {
    Ask,
    Speak,
}

#[derive(Debug, Clone)]
pub enum PendingAction {
    /// Bot asked the user to type free-form input for a command (tap-from-menu
    /// flow), which arrives as the next plain-text message. `kind` selects the
    /// action (chat vs. spoken output).
    AwaitingPrompt { kind: PromptKind },
}

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
    pub noxis: Arc<NoxisCore>,
    pub whisper: Arc<WhisperClient>,
    pub memory: Arc<ConversationStore>,
    pub audio: Arc<AudioProcessor>,
    pub config: Arc<Config>,
    pub pending: Arc<DashMap<i64, PendingAction>>,
    /// Last AI reply text per user — lets the "🔊 Listen" inline button
    /// re-synthesize the previous answer on demand.
    pub last_reply: Arc<DashMap<i64, String>>,
    /// Bounded worker pool for synthesis tasks.
    pub worker_pool: Arc<crate::worker_pool::WorkerPool>,
    /// Runtime observability counters.
    pub metrics: Arc<crate::metrics::Metrics>,
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
        )
        .branch(Update::filter_pre_checkout_query().endpoint(
            |bot: Bot, q: PreCheckoutQuery, st: AppState| async move {
                handle_pre_checkout(bot, q, st).await
            },
        ));

    let mode = &state.config.telegram.mode;
    let mode = mode.trim().to_ascii_lowercase();

    if mode == "webhook" {
        let url = state.config.telegram.webhook_url.clone();
        if url.trim().is_empty() {
            anyhow::bail!(
                "webhook mode requires ANUBIS_WEBHOOK_URL (e.g. https://anubis.example.com:8443/webhook)"
            );
        }
        use teloxide::dispatching::update_listeners::webhooks;

        let listen: std::net::SocketAddr = state
            .config
            .telegram
            .webhook_listen
            .parse()
            .map_err(|e| anyhow::anyhow!("invalid ANUBIS_WEBHOOK_LISTEN: {e}"))?;
        let public_url = url::Url::parse(&url)
            .map_err(|e| anyhow::anyhow!("invalid ANUBIS_WEBHOOK_URL: {e}"))?;

        info!("Telegram via webhook :: {url} (listen {listen})");

        let listener =
            webhooks::axum(bot.clone(), webhooks::Options::new(listen, public_url)).await?;

        let mut dispatcher = Dispatcher::builder(bot, handler)
            .dependencies(dptree::deps![state])
            .enable_ctrlc_handler()
            .build();
        dispatcher
            .dispatch_with_listener(
                listener,
                std::sync::Arc::new(|e: std::convert::Infallible| async move {
                    tracing::error!("webhook listener error: {e:?}");
                }),
            )
            .await;
        return Ok(());
    }

    info!("Telegram via long-polling");
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
    fn awaiting_prompt_carries_kind() {
        let a = PendingAction::AwaitingPrompt {
            kind: PromptKind::Ask,
        };
        let b = PendingAction::AwaitingPrompt {
            kind: PromptKind::Speak,
        };
        // Two AwaitingPrompt values with different inner fields must differ
        // when compared structurally (kind), even though they share the same
        // enum discriminant.
        assert_ne!(
            (matches!(&a, PendingAction::AwaitingPrompt { kind } if *kind == PromptKind::Ask)),
            (matches!(&b, PendingAction::AwaitingPrompt { kind } if *kind == PromptKind::Ask)),
        );
    }
}
