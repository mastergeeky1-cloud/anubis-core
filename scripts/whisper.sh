#!/usr/bin/env bash
# ANUBIS Core — local whisper.cpp transcription sidecar (voice-to-voice).
#
# Installs whisper.cpp and downloads a small multilingual GGML Whisper model,
# then runs `whisper-server -di` (OpenAI-compatible) on 127.0.0.1:8890 so the
# bot can transcribe voice messages locally. No audio leaves the machine.
#
# Model options (quality vs size):
#   tiny   = ~75 MB   (fastest, many languages)
#   base   = ~142 MB  (good default)
#   small  = ~466 MB  (better accuracy, slower on CPU)
#   medium = ~1.5 GB  (best multilingual accuracy, needs beefier CPU)
#
# Usage:  MODEL=base ./scripts/whisper.sh
set -euo pipefail
cd "$(dirname "$0")/.."

MODEL="${MODEL:-base}"
WHISPER_DIR="./whisper"
SERVER_PORT="${ANUBIS_WHISPER_PORT:-8890}"
LANG="${WHISPER_LANG:-auto}"   # "auto" = multilingual model auto-detect, or e.g. "en"

mkdir -p "$WHISPER_DIR"

echo "==> Building whisper.cpp (this takes a minute)…"
if [[ ! -d "$WHISPER_DIR/whisper.cpp" ]]; then
  git clone --depth 1 https://github.com/ggerganov/whisper.cpp "$WHISPER_DIR/whisper.cpp"
  make -C "$WHISPER_DIR/whisper.cpp" -j"$(nproc 2>/dev/null || echo 2)" whisper-server
fi

echo "==> Downloading Whisper ${MODEL}.ggml (multilingual, permissively licensed)…"
MODEL_FILE="$WHISPER_DIR/models/ggml-${MODEL}.bin"
if [[ ! -f "$MODEL_FILE" ]]; then
  bash "$WHISPER_DIR/whisper.cpp/models/download-ggml-model.sh" "$MODEL"
  mv "$WHISPER_DIR/whisper.cpp/models/ggml-${MODEL}.bin" "$MODEL_FILE"
fi

echo "==> Starting whisper-server on 127.0.0.1:$SERVER_PORT (OpenAI-compatible /audio/transcriptions)…"
ARGS=(
  --host 127.0.0.1
  --port "$SERVER_PORT"
  --model "$MODEL_FILE"
  --di          # OpenAI-compatible mode
  --threads 4
)
if [[ "$LANG" != "auto" ]]; then
  ARGS+=(--language "$LANG")
fi
exec "$WHISPER_DIR/whisper.cpp/build/bin/whisper-server" "${ARGS[@]}"
