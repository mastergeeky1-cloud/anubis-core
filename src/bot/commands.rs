use teloxide::utils::command::BotCommands;

#[derive(BotCommands, Clone, Debug)]
#[command(
    rename_rule = "lowercase",
    description = "🎓 ANUBIS Voice Teacher — command list:"
)]
pub enum Command {
    // ── Getting started ─────────────────────────────────────────────────────
    #[command(description = "🎓 Start the voice teacher")]
    Start,

    #[command(description = "❓ Show this help message")]
    Help,

    // ── Learning ────────────────────────────────────────────────────────────
    #[command(description = "🧠 Ask your teacher a question: /ask <text>")]
    Ask(String),

    #[command(description = "🔊 Hear a response spoken aloud: /speak <text>")]
    Speak(String),

    // ── Voice & language ────────────────────────────────────────────────────
    #[command(description = "🎙 Choose your teacher's voice: /voices")]
    Voices,

    #[command(description = "🌐 Change language: /lang")]
    Lang,

    #[command(description = "🎓 Toggle teacher mode: /teacher on|off")]
    Teacher(String),

    #[command(description = "🔄 Reset conversation: /reset")]
    Reset,
}
