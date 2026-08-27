# Security Policy — ANUBIS Core

This document explains the threat model and the security controls built into
ANUBIS Core. It is a **local, open-source, bare-metal** voice system: no data
leaves the machine unless you explicitly point a sidecar at an external service.

## Secrets
- The Telegram bot token is **never** hardcoded. It is read only from the
  `ANUBIS_TELEGRAM_TOKEN` environment variable (or a systemd `EnvironmentFile`).
- Database path, clone dir, voice dir, and sidecar URLs are all env-configurable.
- Do **not** commit `.env` (it is gitignored). Rotate the token immediately if
  it is ever pasted into chat, logs, or source.

## Voice cloning — abuse controls
Voice cloning is a dual-use (impersonation) capability. The following are
enforced:
- **Explicit consent**: a user must accept the consent prompt before any voice
  sample is stored. The timestamp is recorded in the database.
- **Watermarking**: every cloned output carries an attribution watermark
  (LSB steganography for WAV, neural watermark where supported) embedding the
  issuing user id + timestamp, for provenance and takedown.
- **Rate limiting**: clone actions are throttled per user (configurable).
- **Bans**: admins can ban abusive users; banned users are blocked at the
  command layer.
- **Storage isolation**: each user's clones live under `clones/<user_id>/`.

## Input handling
- All user text is sanitized (control-character stripping) before synthesis.
- Outgoing model prompts are constructed server-side; users cannot inject
  system instructions through message text.

## Supply chain
- All speech models used by default are **open-source with permissive licenses**:
  - Piper (MIT) — base TTS
  - Kokoro (Apache-2.0) — fast multilingual TTS
  - Chatterbox-Multilingual (MIT) — voice cloning
- The historically used XTTS-v2 (CPML, non-commercial) and F5-TTS
  (CC-BY-NC weights) are **removed** from the default path to avoid license
  violations. They remain optional only for personal, non-commercial use.

## Reporting
Report security issues privately to the maintainer. Do not open public issues
for vulnerabilities.
