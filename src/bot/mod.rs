pub mod commands;
pub mod handlers;
pub mod keyboards;

use crate::audio::AudioProcessor;
use crate::cache::AudioCache;
use crate::clone::ChatterboxCloner;
use crate::config::Config;
use crate::db::Database;
use crate::noxis::NoxisCore;
use crate::security::{RateLimiter, Watermarker};
use crate::tts::router::TtsRouter;
use dashmap::DashMap;
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;

#[derive(Debug, Clone)]
pub enum PendingAction {
    AwaitingVoiceForClone,
}

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
    pub tts: Arc<TtsRouter>,
    pub clone_engine: Arc<ChatterboxCloner>,
    pub noxis: Arc<NoxisCore>,
    pub audio: Arc<AudioProcessor>,
    pub config: Arc<Config>,
    pub pending: Arc<DashMap<i64, PendingAction>>,
    pub rate_limiter: Arc<RateLimiter>,
    pub cache: Arc<AudioCache>,
    pub watermark: Arc<Watermarker>,
}

pub async fn run(state: AppState) -> anyhow::Result<()> {
    use commands::Command;
    use handlers::{handle_callback, handle_command, handle_message};

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
