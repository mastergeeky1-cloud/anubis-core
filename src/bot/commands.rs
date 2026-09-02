use teloxide::utils::command::BotCommands;

#[derive(BotCommands, Clone, Debug)]
#[command(
    rename_rule = "lowercase",
    description = "🔱 ANUBIS Voice Bot — command list:"
)]
pub enum Command {
    // ── Getting started ─────────────────────────────────────────────────────
    #[command(description = "🚀 Start the bot and show the main menu")]
    Start,

    #[command(description = "📋 Show the main command-center menu")]
    Menu,

    #[command(description = "❓ Show this help message")]
    Help,

    // ── AI & speech ────────────────────────────────────────────────────────
    #[command(description = "🧠 Chat with the local Noxis Core brain: /ask <text>")]
    Ask(String),

    #[command(description = "🔊 Generate speech from text: /speak <text>")]
    Speak(String),

    #[command(description = "🧬 Speak in your cloned voice: /myvoice <text>")]
    Myvoice(String),

    // ── Voice cloning ──────────────────────────────────────────────────────
    #[command(description = "🎤 Clone your voice (send a 30–60s voice message after)")]
    Clone,

    #[command(description = "🗂 Manage your voice clones")]
    Clones,

    // ── Voices & presets ───────────────────────────────────────────────────
    #[command(description = "🎙 Browse and pick a TTS voice")]
    Voices,

    #[command(description = "⚙️ Set active TTS voice: /setvoice <id>")]
    Setvoice(String),

    #[command(description = "✨ Browse curated voice presets")]
    Presets,

    #[command(description = "🛍 Voice Pack Marketplace: install curated voice packs")]
    Shop,

    // ── Teacher mode ──────────────────────────────────────────────────────────
    #[command(description = "🎓 Teacher mode: /teacher on|off|status")]
    Teacher(String),

    // ── Settings & account ─────────────────────────────────────────────────
    #[command(description = "🌐 Change interface language")]
    Lang,

    #[command(description = "💳 Show your credit balance")]
    Credits,

    #[command(description = "📊 Show your usage statistics")]
    MyStats,

    #[command(description = "⚙️ Show your current settings (voice, language, credits)")]
    Settings,

    #[command(description = "🔄 Clear conversation memory for /ask")]
    Reset,

    #[command(description = "⭐ Upgrade and buy credits via Telegram Stars")]
    Upgrade,

    // ── Admin ──────────────────────────────────────────────────────────────
    #[command(description = "🛡 Ban a user: /ban <user_id>")]
    Ban(String),

    #[command(description = "🛡 Unban a user: /unban <user_id>")]
    Unban(String),

    #[command(description = "🛡 Grant credits: /grant <user_id> <amount>")]
    Grant(String),

    #[command(description = "📊 Show bot statistics (admin)")]
    Stats,

    #[command(description = "👥 List all users (admin)")]
    Users,

    #[command(description = "📈 Show daily active users (admin)")]
    DailyActive,
}
