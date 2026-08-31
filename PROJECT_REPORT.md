# ANUBIS Core — Full Project Report & Roadmap

**Author:** automated analysis · **Date:** 2026-08-28
**Scope:** architecture audit, current-state assessment, and a concrete scaling /
UX / payments / security roadmap.

> ⚠️ **Current status flag:** the committed `HEAD` (`3cfab11`) compiles cleanly, but
> the **uncommitted working tree does not compile** — `keyboards.rs` was rewritten to
> `main_menu`/`upgrade_menu` while `handlers.rs` still calls the removed
> `consent_keyboard()`, and the new `menu:*` / `pay:*` callbacks plus the `Upgrade`
> command are not wired into command parsing or the callback handler. **Phase 0 must
> fix this before anything else.**

---

## 1. What this project is

**ANUBIS Core v2.0.0** is a **local-first, open-source, Rust-based voice-AI
Telegram bot.** Everything runs on a single machine ("bare metal"); no audio,
transcript, or model traffic leaves the host unless you point a sidecar at an
external service.

### Feature surface
- **TTS** — `/speak <text>` via Piper (MIT, CPU) and/or Kokoro (Apache-2.0, CPU
  HTTP sidecar). 20 catalogue voices across 10 languages.
- **Voice cloning** — `/clone` (consent + watermark + rate-limited), speak back
  with `/myvoice` via a local Chatterbox-Multilingual (MIT) sidecar.
- **Local LLM "brain"** — `/ask <text>` through **Noxis Core**, an OpenAI-compatible
  client to a local `llama.cpp`/`ollama` endpoint.
- **Accounts & credits** — per-user SQLite records, free daily generation quota,
  and purchasable "credits" (payment UI drafted but not wired up).
- **Security controls** — rate limiting, consent gating, LSB watermarking,
  admin bans, input sanitization, append-only audit log.
- **Admin panel** — `/stats /users /dailyactive /ban /unban /grant` (+ `/mystats`).

### Languages currently shipped (i18n)
English, Arabic, Italian, French, Spanish, German, Russian, Hindi, Turkish,
Portuguese. Static string tables per language; menu/payment strings already
added to all 10 locales.

---

## 2. Architecture

```
                ┌──────────────────────────────────────────────┐
   Telegram ───▶│  anubis-core (Rust binary; teloxide long-poll)│
                │  • command / message / callback handlers      │
                │  • Noxis Core (LLM client)                    │
                │  • TTS router (Piper / Kokoro)                │
                │  • Clone client (Chatterbox)                  │
                │  • security: rate-limit · consent · watermark │
                │    · sanitize · audit                         │
                │  • SQLite (r2d2 pool) · LRU audio cache       │
                │  • ffmpeg glue (WAV ⇄ OGG)                    │
                └───┬───────────────────────┬───────────────────┘
        HTTP/local  │                       │  HTTP/local
                    ▼                       ▼
            llama.cpp server          Chatterbox server
            (GGUF, local LLM)        (MIT voice cloning)
           + Kokoro TTS sidecar
```

### Module map (`src/`)
| Module | Lines | Role |
|---|---|---|
| `bot/mod.rs` | 88 | Dispatcher/dependency wiring, `AppState`, `run()` |
| `bot/handlers.rs` | 679 | command / message / callback logic (largest bot file) |
| `bot/commands.rs` | 64 | `BotCommands` enum (all commands) |
| `bot/keyboards.rs` | 73 | inline keyboard builders (being rewritten in WIP) |
| `bot/stats.rs` | 302 | user + admin statistics queries & formatting |
| `db.rs` | 343 | SQLite schema + CRUD, audit, credit ledger |
| `config.rs` | 167 | TOML + env-driven config |
| `i18n.rs` | 669 | localized string tables (10 languages) |
| `noxis/mod.rs` | 88 | LLM client (`/v1/chat/completions`) |
| `tts/*` | ~360 | engine trait, Piper, Kokoro, router, voices, presets |
| `clone/mod.rs` | 109 | Chatterbox clone client + sample storage |
| `audio.rs` | 100 | ffmpeg WAV⇄OGG, duration probe |
| `cache.rs` | 38 | LRU audio cache (xxh64 keys) |
| `security/*` | ~150 | rate limit, sanitize, watermark |
| `error.rs` | 60 | error enum |
| `main.rs` | 95 | startup / composition |

**Total:** ~3,500 lines of Rust. Single binary, SQLite, no external messaging bus.
Sidecars (llama.cpp, Chatterbox, Kokoro) are Python/CPP processes started via
`scripts/setup.sh` / `run_local.sh`.

### Data model (`db.rs`)
- `users` — balance, language, active voice, consent timestamp, ban flag.
- `voice_clones` — per-user cloned voice metadata (`clones/<uid>/` files on disk).
- `credit_log` — ledger of credit deltas (free/paid/admin).
- `audit_log` — append-only security/audit trail.

---

## 3. Security assessment (current)

Present controls:
- ✅ Token only via env (`ANUBIS_TELEGRAM_TOKEN`), never committed; refuses to start empty.
- ✅ Env-first config with all secrets overridable, `.env` gitignored.
- ✅ Rate limiting (per-user speak/clone windows) — in-memory `DashMap`.
- ✅ Consent gate + LSB watermark on cloned WAVs + per-user storage isolation.
- ✅ Input sanitization (control chars stripped) before TTS/LLM.
- ✅ Admin-only commands (`is_admin` check on admin_ids).
- ✅ Hardened systemd unit (NoNewPrivileges, ProtectSystem, PrivateTmp).
- ✅ MIT/Apache-2.0 default models only (removed CPML/CC-BY-NC XTTS/F5).

Gaps / weaknesses:
- ❌ **Rate limiter is in-memory and per-process** — resets on restart, not shared
  across replicas, no persistence/IP-based limits.
- ❌ **No cap on stored clones / clone sample size beyond duration** — disk-fill risk.
- ❌ **Audit log unbounded** — grows forever, no retention/rotation/pruning.
- ❌ **No encryption at rest** (SQLite is plaintext; clone WAVs plaintext on disk).
- ❌ **Token/endpoint secrets visible to any process with same user** — no
  filesystem permission hardening documented beyond systemd.
- ❌ **No per-user IP / request fingerprinting** — Telegram user IDs are the only identity.
- ❌ **No formal dependency-audit / CVE pipeline** and no `cargo-audit` step in CI.
- ❌ **Watermark is only LSB** (removable), acknowledged as baseline.
- ❌ **Payment flow not implemented** (see §6) — the strings/UI exist, the security
  of a real payment path does not.

---

## 4. Current-state blockers (fix in Phase 0)

1. **Working tree does not compile.** `keyboards::consent_keyboard()` was replaced
   by `main_menu` / `upgrade_menu` but `handlers.rs:115` still calls it. Also
   `menu:*` / `pay:*` callbacks and the `Upgrade` command are unimplemented, so the
   new keyboards panic at runtime if ever reached.
2. **`daily_active(30)` loops 30 sequential SQLite queries** per call — fine at
   small scale, wasteful at scale.
3. **The `top_users` JOIN** in `admin_stats()` doubles rows via the `voice_clones`
   join — counts are inflated/incorrect.
4. **`stats.rs` uses unparameterized PG-style `[user_id]` in some queries** while
   `db.rs` uses `params![]` — inconsistent, and rusqlite positional `?` with a
   named array can silently mismatch (worked because values are positional).
5. **Piper model search is O(n) on disk** (recursive read_dir) per voice listing.

---

## 5. Scaling plan

### 5.1 Immediate (single node, many users)
- **Task queue & concurrency control.** Today every `/speak` spawns a Piper
  subprocess inline; under load this thrashes CPU and Telegram API rate limits.
  Introduce a bounded work queue (e.g. `tokio::sync::mpsc` + N worker tasks) with
  per-engine parallelism cap and a small job queue. Return an estimate / use the
  "thinking" message to signal progress.
- **DB pooling & connection tuning.** Already pooled (r2d2, max 8). Set
  `busy_timeout`, enable `WAL` (already on), and add `PRAGMA synchronous=NORMAL`
  for write throughput.
- **Persistent rate limiting.** Move hit-counters to SQLite (or a small Redis if
  you later run multiple nodes) so limits survive restarts and scale horizontally.
- **Cache tiering.** The LRU is per-process memory only. Add an optional
  filesystem cache (`audio_cache/<hash>.ogg`) for hot phrases, and a configurable
  global TTL/eviction.
- **Batch/async downloads.** Telegram file download + ffmpeg conversion already
  run on tokio; keep them off the main dispatcher via the worker pool.

### 5.2 Multi-node / horizontal
- Move the **dispatcher to webhook** (teloxide supports webhooks; nginx/TLS term)
  instead of long-polling when running behind a load balancer.
- **Shared state service**: SQLite is single-writer. For horizontal scale, move to
  Postgres (via `sqlx`), or keep SQLite but add a shared Redis for rate limits,
  cache, and distributed locking of the clone/Chatterbox work.
- **Stateless sidecar fan-out**: Piper/Kokoro/Chatterbox become a pool of worker
  processes behind a load balancer; the Rust bot becomes a thin router.
- Orbit for team scaling: containerize the binary + sidecars (`Dockerfile`,
  `docker-compose`), run behind a reverse proxy.

### 5.3 Reliability & observability
- Structured event logs already via `tracing`; add: per-command latency, cache
  hit rate, queue depth, sidecar health checks.
- Health-check endpoint + Prometheus metrics (`/metrics`) for the dispatcher and
  sidecars.
- Graceful shutdown of in-flight synthesis (currently `ctrlc` handler only).
- CI already builds/test on 3 OSes; add `cargo audit`, `cargo deny`, and a
  coverage gate.

---

## 6. Interface upgrade: real buttons, icons, effects, style

### 6.1 Inline keyboards (already being added — finish them)
- **Main command center** `/start`/`/menu` → `main_menu()` grid: 🗣 Speak,
  🧠 Ask, 🎤 Clone, 🎙 Voices, 💳 Credits, 📊 Stats, 🌐 Language, ⭐ Upgrade.
- Wire every `menu:*` / `mode:*` callback to actually **edit** the message and show
  a sub-menu, using `edit_message_text` + `edit_message_reply_markup` (keeps one
  clean "app" feel instead of many scattered messages).
- **Back navigation** in every screen (`🔙 Back → menu:home`).
- **Voice picker** — paginated (◀ ▶ pages) with a ✅ checkmark on the active voice,
  quality badge (`low/med/high`), and gender icons already present.
- **Consent flow** — restore an inline Agree/Decline keyboard (currently broken).

### 6.2 Effects & "feel"
- **Typing/recording actions**: send `ChatAction::RecordingVoice` / `Typing` while
  synthesizing, then delete the "thinking" message.
- **In-line status**: use `edit_message_text` with spinner states (⏳ → ✅/❌)
  on a single persistent status message.
- **Toast-style feedback**: `answer_callback_query` with short confirmations
  ("Voice set! ✅", "Clone deleted 🗑") on every button press.
- **Response formatting**: consistent MarkdownV2 with clear headings, dividers,
  and monospace where useful; consistent emoji keying.

### 6.3 Style / presentation
- A **Welcome card** using `send_photo` with a generated banner (HTML/SVG → PNG) so
  `/start` looks premium instead of a text wall. Regenerate per language.
- **Primary button emphasis**: Telegram shows only one row of buttons per line; use
  row ordering to make Speak/Upgrade prominent.
- **WebApp WebView** (optional, high-effort) for a full rich interface served over
  HTTPS — a real branded app with charts, voice preview, credit shop.

### 6.4 Commands to add
- `/menu` (explicit), `/upgrade`, `/cancel` (clear pending clone state), inline
  `@bot <text>` mode for quick synthesis.

---

## 7. Real payments (Telegram Stars)

The strings, `upgrade_menu()` with `pay:100/500/1500` and localized
`payment_*` text already exist — **the actual flow is not implemented.** Plan:

1. **Re-add `Upgrade` command** and wire the `menu:upgrade` callback to
   `upgrade_menu()` (or `send_invoice`).
2. **Telegram Stars API** (native, no 3rd-party, secure):
   - `sendInvoice` with `provider_token=""` and `currency="XTR"` (Telegram Stars),
     product payload e.g. `{"tier":"100","uid":...}`.
   - Handle `Message::pre_checkout_query` → always `answerPreCheckoutQuery(ok=true)`,
     after re-validating product + user.
   - Handle `Message::successful_payment` → **atomically** credit the user's balance
     and log to `credit_log` with reason `stars_payment` + the Telegram-provided
     `telegram_payment_charge_id` / `provider_payment_charge_id`.
3. **Anti-fraud / integrity**:
   - **Idempotency**: store the Telegram `charge_id` in a `payments` table with a
     UNIQUE constraint; the same charge can never grant credits twice (replay-safe).
   - **Verify server-side** product/tier, not trusting the client.
   - **Audit** every successful and failed attempt in `audit_log`.
   - **Admin reconciliation** command `/payments` to list recent payments.
4. **Credit top-up UI** in the same menu; after success, `answer_callback_query`
   with `payment_success` and update the balance button.
5. (Alternative if you later want cards outside Telegram: Stripe Checkout Session →
   webhook → credit, but Telegram Stars is the zero-install, secure default for a
   bot.)

---

## 8. Security hardening roadmap

- **P0 (must do with Phase 0 fixes)**
  - Cap clones per user and total clone directory size; enforce max byte size on
    sample upload (not just duration).
  - Add `cargo audit` + `cargo deny` to CI.
  - Truncate/rotate `audit_log` (configurable retention).
  - Move rate-limit counters into SQLite so limits are durable.
- **P1**
  - Encrypt clone WAVs + DB at rest (e.g. via `libsodium`/AES-GCM with a key from
    env); note performance tradeoff.
  - Per-IP + per-account rate limits using a Redis-less in-process + DB hybrid.
  - Add `RUST_LOG`-based redaction for any token in logs (already no hardcoding).
  - Restrict file permissions on clone dir (`0700`) even outside systemd.
- **P2**
  - Strengthen watermark (non-removable neural watermark where supported).
  - Add abuse-detection heuristics (rapid cloning attempts, duplicate-sample
    fingerprints) and auto-lock on suspicious patterns.
  - Secret rotation tooling + `.env` permission warnings on startup.

---

## 9. Cost & infra notes
- Everything is **local / bare-metal**: the main cost is CPU/GPU time and disk
  (voice models ~1–2 GB, clone server benefits from a GPU, LLM GGUF size).
- For horizontal scaling the added costs are: Postgres/Redis + a webhook
  endpoint with TLS + worker replicas. Recommend containerizing before scaling out.

---

## 10. Suggested phased roadmap (summary)

| Phase | Scope | Outcome |
|---|---|---|
| **0** | Fix broken WIP (consent keyboard, wire menu/pay callbacks, add Upgrade cmd) | Compiles; menu works end-to-end |
| **1** | UX finish: edit-message nav, back buttons, pagination, ChatActions, status edits | Polished button interface + effects |
| **2** | Payments: implement Telegram Stars end-to-end with idempotency + audit | Real, secure payments |
| **3** | Scaling: task queue + workers, persistent rate limits, filesystem cache tier, DB tuning | Handles many concurrent users on one node |
| **4** | Security: clone caps, cargo audit, audit retention, at-rest encryption | Hardened baseline |
| **5** | Observability + multi-node (webhook, Postgres/Redis, containers, metrics) | Horizontal scale-ready |

---

*This document is a living report; the codebase is ~3.5 kLOC Rust, cleanly
modular, and a strong foundation. The highest-value immediate work is fixing the
broken WIP (Phase 0), then completing the already-started button menu and payment
flow.*
