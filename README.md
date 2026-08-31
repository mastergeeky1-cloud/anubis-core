<p align="center">
  <img src="assets/banner.png" alt="ANUBIS Core" width="100%" />
</p>

<p align="center">
  <strong>Local-first, open-source, bare-metal voice AI — written in Rust.</strong><br/>
  A Telegram voice bot with a streaming real-time WebSocket transport, a local
  LLM "brain", TTS, voice cloning, and secure payments — all on your own machine.
</p>

<p align="center">
  <a href="#features">Features</a> •
  <a href="#architecture">Architecture</a> •
  <a href="#quickstart">Quickstart</a> •
  <a href="#commands">Commands</a> •
  <a href="#real-time-websocket">WebSocket</a> •
  <a href="#configuration">Configuration</a> •
  <a href="#security">Security</a> •
  <a href="#scaling-roadmap">Roadmap</a>
</p>

---

Everything runs **locally**. No audio, transcript, or model traffic leaves
your machine unless you explicitly point a sidecar at an external provider.

## Features

| | |
|---|---|
| 🧠 **Noxis Core** | Local LLM "brain" (llama.cpp / Ollama / OpenAI-compatible). Streaming replies, per-user conversation memory. |
| 🗣 **Streaming WebSocket** | A real-time opcode transport so web apps / clients get live text deltas **and** audio frames as they're generated. |
| 🎙 **TTS** | Piper (CPU) + Kokoro (neural sidecar). 20+ voices across **10 languages**. |
| 🧬 **Voice cloning** | Local, MIT-licensed Chatterbox-Multilingual sidecar (replaces the old XTTS/F5 path). |
| 🎤 **Voice conversation** | Send a voice note → whisper.cpp transcribes locally → Noxis answers → reply in an audible voice. |
| 🔒 **Security** | Consent gate, LSB audio watermark, rate limiting, input sanitization, full audit log, env-only tokens. |
| ⚡ **Monetization** | Telegram Stars payments, idempotent credit granting, daily free quota, optional unlimited mode. |
| 🌐 **i18n** | Full command-center UI localized in 9 languages (EN/AR/IT/FR/ES/DE/RU/HI/TR/PT). |

## Architecture

```
                       ┌──────────────────────────────────────────────┐
   Telegram  long-poll │  anubis-core (Rust binary)   WebSocket  WS   │
      ▶────────────────┤  • teloxide bot            ◀────────────────┤──▶ Web / mobile clients
                       │  • Noxis Core (LLM client)  • axum WS server │
                       │  • security (rate-limit, consent,            │
                       │    watermark, audit, sanitize)               │
                       │  • TTS router (Piper / Kokoro)               │
                       │  • Voice clone (Chatterbox)                  │
                       │  • SQLite + LRU audio cache + ffmpeg glue    │
                       └──────────┬──────────────────┬────────────────┘
                                  │ HTTP/local       │ HTTP/local
                                  ▼                  ▼
                     llama.cpp / Ollama      Chatterbox server
                          (:8080)                 (:8008)
                       Kokoro TTS (:8880)
                       whisper.cpp (:8890)
```

Both transports share the **same core** — Noxis, TTS router, clone engine,
whisper, memory, DB, and LRU cache. The WebSocket transport is a second door
into identical backend logic.

## Quickstart

### 1. Prerequisites

- [Rust](https://rustup.rs) (stable)
- [`ffmpeg`](https://ffmpeg.org) on `PATH` (WAV ↔ OGG conversion)
- A Telegram bot token from [@BotFather](https://t.me/BotFather)

### 2. Clone & build

```bash
git clone https://github.com/mastergeeky1-cloud/anubis-core.git
cd anubis-core
cp .env.example .env        # then add your ANUBIS_TELEGRAM_TOKEN
cargo build --release
```

### 3. Configuration

```bash
# Token (REQUIRED) — env only, never in a file next to config
export ANUBIS_TELEGRAM_TOKEN="123:your-token"

# Noxis Core local LLM (llama.cpp-compatible OpenAI-style endpoint)
export ANUBIS_LLM_URL="http://127.0.0.1:8080"      # leave empty to disable /ask
# Optional key for a hosted endpoint (e.g. omniroute):
# export ANUBIS_LLM_URL="https://api.omniroute.ai"  ANUBIS_LLM_KEY="..."

# Sidecars
export ANUBIS_CLONE_URL="http://127.0.0.1:8008"     # Chatterbox voice clone
export ANUBIS_KOKORO_URL="http://127.0.0.1:8880"    # optional Kokoro TTS
export ANUBIS_WHISPER_URL="http://127.0.0.1:8890"   # optional voice input

# WebSocket transport
export ANUBIS_WS_BIND="127.0.0.1:7600"              # bind address
export ANUBIS_WS_TOKEN=""                           # optional shared bearer token
```

### 4. Download permissive voices

```bash
./run_local.sh --download      # fetch Piper voices + sidecars (MIT/Apache)
# or individually via scripts/setup.sh
```

### 5. Run

```bash
./target/release/anubis
```

or the one-shot runner (downloads + sidecars + bot):

```bash
./run_local.sh --all
```

When running manually, the bot also starts the WS server on `ANUBIS_WS_BIND`.

## Commands

| Command | Description |
|---|---|
| `/start` `/menu` `/help` | Start bot, open menu, help |
| `/ask <text>` | Chat with the Noxis Core brain |
| `/speak <text>` | Generate speech from text |
| `/myvoice <text>` | Speak in your cloned voice |
| `/clone` | Clone your voice (send a 30–60s voice note) |
| `/clones` | Manage your clones |
| `/voices` | Browse & pick a voice (20+) |
| `/setvoice <id>` | Set active voice by id |
| `/presets` | Curated voice presets |
| `/lang` | Change interface language |
| `/credits` `/upgrade` | Balance & buy credits (Telegram Stars) |
| `/reset` | Clear conversation memory |
| `/mystats` `/stats` | Your stats / admin stats |

**Voice behavior:** a plain text message with no `/command` goes straight to
the AI. If you've **explicitly selected a voice**, replies come back spoken in
that voice (a persistent session voice); otherwise replies are text with a
"🔊 Speak this" button.

## Real-time WebSocket

Beyond the Telegram bot, ANUBIS exposes a raw WebSocket endpoint for web apps,
desktop clients, and game engines — with **live streaming**.

```
ws://localhost:7600/ws        (ANUBIS_WS_BIND / ANUBIS_WS_TOKEN)

Every frame: [opcode:u8][len:u32 BE][payload]
Client → 0x01 Hello · 0x02 Text · 0x03 Voice · 0x04 Config · 0x05 Ping · 0x06 History
Server → 0x81 Hello · 0x82 TextDelta · 0x83 VoiceChunk · 0x84 Status
         0x85 Error · 0x86 Meta · 0x87 Pong · 0x88 History · 0x89 TextEnd
```

- **Text** streams live as the LLM generates (`TextDelta`), then the reply is
  synthesized and sent as opus `VoiceChunk` frames.
- **Voice** sends audio for on-the-fly whisper transcription.
- **Hello** handshake (auth token + protocol version + session id).

**Try it in the browser:** open `assets/ws-demo.html` (uses the ES-module
client `assets/anubis-ws-client.mjs`).

## Optional whisper sidecar (voice input)

```bash
MODEL=base ./scripts/whisper.sh   # builds whisper.cpp + serves on :8890
```

## Payments

Upgrading uses **Telegram Stars** (XTR) — no card data handled by the bot.
Credits are granted **idempotently**: each Telegram charge ID can credit once,
and the amount is validated against the tier before crediting.

## Security

See [SECURITY.md](./SECURITY.md). Highlights:

- Token via env only (never in `config.toml` or `.env` committed).
- Cloning requires explicit **consent**; every clone is **watermarked** with an
  embedded user id + timestamp for provenance.
- **Rate limiting** per user + **audit log** of every action.
- Input **sanitization** before it reaches the LLM/TTS.

## Scaling Roadmap

1. **Persist memory** — move the in-memory conversation store to SQLite (WAL),
   with TTL, so context survives restarts and replicas can share it.
2. **Webhook** — swap long-poll for a webhook behind Caddy/nginx to scale
   replicas and drop poll overhead.
3. **Worker pool** for synthesis — offload heavy TTS/clone jobs to a queue so
   slow clone synthesis never blocks chat.
4. **Observability** — export tracing to Prometheus/Jaeger; track per-step
   latency, queue depth, error rates.
5. **Neural watermark** — pair the LSB baseline with a robust model watermark
   for anti-removal.
6. **Voice pack marketplace** — a browsable install/uninstall catalog.

## License

[MIT](./LICENSE)

---

<p align="center"><strong>🔱 ANUBIS Core</strong> · local-first · open-source · built with Rust</p>
