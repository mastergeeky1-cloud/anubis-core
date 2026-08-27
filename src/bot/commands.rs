use teloxide::utils::command::BotCommands;

#[derive(BotCommands, Clone, Debug)]
#[command(rename_rule = "lowercase", description = "ANUBIS Voice Bot commands:")]
pub enum Command {
    #[command(description = "Start the bot")]
    Start,

    #[command(description = "Show all commands")]
    Help,

    #[command(description = "Generate speech: /speak <text>")]
    Speak(String),

    #[command(description = "Ask the local Noxis Core brain: /ask <text>")]
    Ask(String),

    #[command(description = "Speak in your cloned voice: /myvoice <text>")]
    Myvoice(String),

    #[command(description = "Clone your voice (send a 30–60s voice message after)")]
    Clone,

    #[command(description = "List and manage your voice clones")]
    Clones,

    #[command(description = "List available TTS voices")]
    Voices,

    #[command(description = "Set active TTS voice: /setvoice <id>")]
    Setvoice(String),

    #[command(description = "Browse voice presets")]
    Presets,

    #[command(description = "Change interface language")]
    Lang,

    #[command(description = "Show your credit balance")]
    Credits,

    // ── admin commands ──────────────────────────────────────────────────────
    #[command(description = "[admin] Ban a user: /ban <user_id>")]
    Ban(String),

    #[command(description = "[admin] Unban a user: /unban <user_id>")]
    Unban(String),

    #[command(description = "[admin] Grant credits: /grant <user_id> <amount>")]
    Grant(String),

    #[command(description = "[admin] Show bot statistics")]
    Stats,
}
