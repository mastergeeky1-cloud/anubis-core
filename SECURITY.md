# Security Policy — ANUBIS Voice Teacher

This document explains the threat model and the security controls built into
**ANUBIS Voice Teacher**. It is a **local, open-source, bare-metal** voice
system: no audio, transcript, or model traffic leaves the machine unless you
explicitly point a sidecar at an external provider.

## Secrets

- The Telegram bot token is **never** hardcoded. It is read only from the
  `ANUBIS_TELEGRAM_TOKEN` environment variable (or a systemd `EnvironmentFile`).
- Database path, voice dir, and sidecar URLs are all env-configurable.
- Do **not** commit `.env` (it is gitignored). Rotate the token immediately if
  it is ever pasted into chat, logs, or source.

## Account controls

- Every user gets a fixed **free daily quota** of text-to-speech requests
  (default 3/day, configurable via `free_daily`). Exhausting it prompts the
  user to purchase credits with Telegram Stars; purchases are idempotent
  (recorded by charge id) and logged.
- Conversational memory is scoped per user and never shared.
- `/reset` clears a user's conversation memory on demand.

## Input handling

- Outgoing LLM prompts are constructed server-side; users cannot inject system
  instructions through message text.
- User text length is bounded (`max_text_chars`) to keep synthesis and prompt
  sizes safe.

## Supply chain

- All speech models used are **open-source with permissive licenses**:
  - Piper (MIT) — base TTS for all catalog voices
  - Kokoro (Apache-2.0) — optional neural TTS sidecar

## Reporting

Report security issues privately to the maintainer. Do not open public issues
for vulnerabilities.