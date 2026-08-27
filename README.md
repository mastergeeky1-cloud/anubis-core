# ANUBIS Core

A local-first, open-source, bare-metal **voice AI system** written in Rust.

ANUBIS Core combines:
- **Noxis Core** — a local LLM "brain" (tool routing, memory, policy) running
  entirely on your machine via a `llama.cpp` sidecar.
- **Speech output** — local TTS engines (Piper, Kokoro) with an 18-voice
  multilingual catalogue (10 languages).
- **Voice cloning** — local, MIT-licensed Chatterbox-Multilingual sidecar
  (replaces the old non-commercial XTTS/F5 path).
- **Telegram surface** — a teloxide bot (`/speak`, `/myvoice`, `/clone`, …).

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
`/start /help /speak /myvoice /clone /clones /voices /setvoice /presets
/lang /credits /ask <text> /ban /unban /grant /stats`

## Security
See [SECURITY.md](./SECURITY.md). Token via env only; cloning requires consent
+ watermark; permissive-licensed models only by default.

## License
MIT — see [LICENSE](./LICENSE).
