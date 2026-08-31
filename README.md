# ANUBIS Core

A local-first, open-source, bare-metal **voice AI system** written in Rust.

ANUBIS Core combines:
- **Noxis Core** — a local LLM "brain" (tool routing, memory, policy) running
  entirely on your machine via a `llama.cpp` sidecar (or a hosted
  OpenAI-compatible endpoint such as omniroute), with **streaming** replies.
- **Speech output** — local TTS engines (Piper, Kokoro) with an 20+ voice
  multilingual catalogue (10 languages).
- **Real-time voice conversation** — send a voice message and the bot
  transcribes it locally (whisper.cpp), chats with Noxis Core, and replies in
  an audible voice (voice-to-voice loop).
- **Voice cloning** — local, MIT-licensed Chatterbox-Multilingual sidecar
  (replaces the old non-commercial XTTS/F5 path).
- **Telegram surface** — a teloxide bot with inline-button menus and secure
  **Telegram Stars** payments (`/speak`, `/myvoice`, `/clone`, `/upgrade`, …).

Everything runs locally. No audio, transcript, or model traffic leaves the
machine unless you configure an external provider.

## Architecture

```
                ┌──────────────────────────────────────┐
   Telegram ───▶│  anubis-core (Rust binary)          │
                │   • teloxide bot (long-poll)         │
                │   • Noxis Core (LLM brain client)    │
                │   • security: rate-limit, consent,   │
                │     watermark, audit log, sanitize   │
                │   • TTS router (Piper / Kokoro)      │
                │   • clone client (Chatterbox)        │
                │   • SQLite + LRU cache + ffmpeg glue │
                └───┬───────────────────┬─────────────┘
        HTTP/local  │                   │  HTTP/local
                    ▼                   ▼
            llama.cpp server      Chatterbox server
            (GGUF, local LLM)     (MIT voice cloning)
```

## Quick start

```bash
# 1. Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

# 2. System deps
sudo apt update && sudo apt install -y ffmpeg portaudio19-dev pkg-config

# 3. Download local models (interactive helper)
./scripts/setup.sh

# 4. Configure secrets (NEVER commit)
export ANUBIS_TELEGRAM_TOKEN="your_token_here"
export ANUBIS_LLM_URL="http://127.0.0.1:8080"        # llama.cpp
export ANUBIS_CLONE_URL="http://127.0.0.1:8008"      # Chatterbox
export ANUBIS_CONFIG="./config.toml"

# 5. Build & run
cargo build --release
./target/release/anubis
```

## Commands
`/start /menu /help /speak /ask /myvoice /clone /clones /voices /setvoice
/presets /lang /credits /upgrade /reset /mystats /ban /unban /grant /stats`

**Real-time voice:** just send a voice message and ANUBIS will transcribe it
(locally via whisper.cpp), answer through Noxis Core, and reply as audio.
`/reset` clears the conversation memory used by `/ask` and voice chat.

## Optional whisper sidecar (voice input)
```bash
MODEL=base ./scripts/whisper.sh   # builds whisper.cpp + downloads a model, serves on :8890
# enable in config.toml [whisper] or set ANUBIS_WHISPER_URL until it's the default
```

## Real-time WebSocket transport (streaming chat & voice)

Alongside the Telegram bot, ANUBIS exposes a raw WebSocket endpoint for web
apps, desktop clients, and game engines. It speaks a tiny binary opcode
protocol that supports **live streaming** — you hear the TTS voice and see the
LLM text *before* generation finishes, and can send continuous voice for
on-the-fly transcription.

```
ws://localhost:7600/ws      (bind + optional token via ANUBIS_WS_BIND / ANUBIS_WS_TOKEN)

Every frame:  [opcode:u8][len:u32 BE][payload]
Client → 0x01 Hello · 0x02 Text · 0x03 Voice · 0x04 Config · 0x05 Ping · 0x06 History
Server → 0x81 Hello · 0x82 TextDelta · 0x83 VoiceChunk · 0x84 Status · 0x85 Error
                                0x86 Meta · 0x87 Pong · 0x88 History · 0x89 TextEnd
```

**Try it in the browser:** open `assets/ws-demo.html` (an ES-module client,
`assets/anubis-ws-client.mjs`, is included) to stream a live AI conversation.

The WS server shares the same core (Noxis brain, TTS router, clone engine,
whisper, memory) as the Telegram bot — it is purely a second transport.

## Payments
Upgrading uses **Telegram Stars** (XTR, no card data handled by the bot).
Credits are granted idempotently — each Telegram charge ID can only credit
once, and the amount is validated against the tier before crediting.

## Security
See [SECURITY.md](./SECURITY.md). Token via env only; cloning requires consent
+ watermark; permissive-licensed models only by default.

## License
MIT — see [LICENSE](./LICENSE).
