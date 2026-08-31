#!/usr/bin/env bash
# ANUBIS-NOXIS: One-shot local runner (open-source, no external APIs required)
# Usage: ./run_local.sh [--download] [--serve] [--bot] [--all]
#   --download : download all permissive models (Piper voices, Kokoro, Chatterbox)
#   --serve    : launch all sidecars in background (Kokoro:8880, Clone:8008, LLM:8080)
#   --bot      : run the Rust bot (needs ANUBIS_TELEGRAM_TOKEN in env)
#   --all      : do all of the above

set -euo pipefail
cd "$(dirname "$0")"

ROOT="."
PIPER_DIR="$ROOT/voices"
KOKORO_DIR="$ROOT/kokoro"
CLONE_DIR="$ROOT/clone_server"
LLM_DIR="$ROOT/llm"
CLONES_DIR="$ROOT/clones"
AUDIO_DIR="$ROOT/audio_output"

mkdir -p "$PIPER_DIR" "$KOKORO_DIR" "$CLONE_DIR" "$LLM_DIR" "$CLONES_DIR" "$AUDIO_DIR"

download_models() {
  echo "==> [1/3] Downloading Piper voices (MIT)…"
  PIPER_BASE="https://huggingface.co/rhasspy/piper-voices/resolve/main"
  # Format: "dest_path_without_ext:repo_subdir"
  declare -A VOICES=(
    ["en/en_US/amy/medium/en_US-amy-medium"]="en/en_US/amy/medium"
    ["en/en_US/ryan/high/en_US-ryan-high"]="en/en_US/ryan/high"
    ["en/en_GB/alan/low/en_GB-alan-low"]="en/en_GB/alan/low"
    ["en/en_US/lessac/medium/en_US-lessac-medium"]="en/en_US/lessac/medium"
    ["ar/ar_JO/kareem/medium/ar_JO-kareem-medium"]="ar/ar_JO/kareem/medium"
    ["it/it_IT/riccardo/x_low/it_IT-riccardo-x_low"]="it/it_IT/riccardo/x_low"
    ["it/it_IT/paola/medium/it_IT-paola-medium"]="it/it_IT/paola/medium"
    ["fr/fr_FR/siwis/medium/fr_FR-siwis-medium"]="fr/fr_FR/siwis/medium"
    ["es/es_ES/carlfm/x_low/es_ES-carlfm-x_low"]="es/es_ES/carlfm/x_low"
    ["de/de_DE/eva_k/x_low/de_DE-eva_k-x_low"]="de/de_DE/eva_k/x_low"
    ["ru/ru_RU/irina/medium/ru_RU-irina-medium"]="ru/ru_RU/irina/medium"
    ["hi/hi_IN/deepika/medium/hi_IN-deepika-medium"]="hi/hi_IN/deepika/medium"
    ["tr/tr_TR/dfki/medium/tr_TR-dfki-medium"]="tr/tr_TR/dfki/medium"
    ["pt/pt_BR/faber/medium/pt_BR-faber-medium"]="pt/pt_BR/faber/medium"
  )
  for dest in "${!VOICES[@]}"; do
    src="${VOICES[$dest]}"
    mkdir -p "$PIPER_DIR/$(dirname "$dest")"
    for ext in onnx onnx.json; do
      # The repo uses the same filename in each subdir
      filename="$(basename "$dest").$ext"
      echo "    -> $(basename "$dest").$ext"
      curl -fsSL "$PIPER_BASE/$src/$filename" -o "$PIPER_DIR/$dest.$ext" || echo "    (skip $filename)"
    done
  done

  echo "==> [2/3] Installing Kokoro (Apache-2.0) via pip…"
  python3 -m pip install --quiet kokoro torch soundfile flask 2>/dev/null || true

  echo "==> [3/3] Installing Chatterbox-Multilingual (MIT) via pip…"
  python3 -m pip install --quiet chatterbox-tts torch soundfile flask 2>/dev/null || true

  echo "    (Optional) Install llama.cpp for local LLM: https://github.com/ggerganov/llama.cpp"
  echo "    Models: pick any MIT/Apache GGUF (e.g. Mistral-7B-Instruct, Phi-3-mini, Gemma-2B)"
}

start_sidecars() {
  echo "==> Starting Kokoro TTS sidecar on 127.0.0.1:8880…"
  cd "$KOKORO_DIR"
  cat > serve.sh <<'EOF'
#!/usr/bin/env bash
python3 - <<'PY'
from flask import Flask, request, send_file
import io, torch, tempfile
from kokoro import KPipeline
app = Flask(__name__)
pipeline = KPipeline(lang_code="a")
VOICE = {"af_heart":"af_heart","am_adam":"am_adam","bm_george":"bm_george","af_alloy":"af_alloy"}
@app.post("/v1/audio/speech")
def tts():
    data = request.get_json(force=True)
    text = data.get("text",""); voice = data.get("voice","af_heart")
    audio = pipeline(text, voice=VOICE.get(voice, voice))
    buf = io.BytesIO()
    import soundfile as sf
    samples=[a[0] for a in audio]
    sf.write(buf, samples[0] if samples else [], 24000, format="WAV")
    buf.seek(0)
    return send_file(buf, mimetype="audio/wav")
if __name__ == "__main__":
    app.run(host="127.0.0.1", port=8880)
PY
EOF
  chmod +x serve.sh
  ./serve.sh > kokoro.log 2>&1 &
  KOKORO_PID=$!
  cd "$ROOT"
  echo "    Kokoro PID: $KOKORO_PID"

  echo "==> Starting Chatterbox clone sidecar on 127.0.0.1:8008…"
  cd "$CLONE_DIR"
  cat > serve.sh <<'EOF'
#!/usr/bin/env bash
python3 - <<'PY'
from flask import Flask, request, send_file
from chatterbox.tts import ChatterboxTTS
import torch, io, soundfile as sf, tempfile, os
app = Flask(__name__)
device = "cuda" if torch.cuda.is_available() else "cpu"
model = ChatterboxTTS.from_pretrained(device=device)
@app.post("/tts")
def tts():
    ref = request.files.get("reference_audio")
    text = request.form.get("text","")
    ref_text = request.form.get("reference_text","")
    lang = request.form.get("language","en")
    rp = tempfile.mktemp(suffix=".wav"); ref.save(rp)
    wav = model.generate(text, audio_prompt_path=rp, audio_prompt_text=ref_text)
    buf = io.BytesIO(); sf.write(buf, wav, model.sr, format="WAV"); buf.seek(0)
    os.remove(rp)
    return send_file(buf, mimetype="audio/wav")
if __name__ == "__main__":
    app.run(host="127.0.0.1", port=8008)
PY
EOF
  chmod +x serve.sh
  ./serve.sh > clone.log 2>&1 &
  CLONE_PID=$!
  cd "$ROOT"
  echo "    Clone PID: $CLONE_PID"

  if command -v llama-server >/dev/null 2>&1; then
    echo "==> Starting llama.cpp LLM sidecar on 127.0.0.1:8080…"
    MODEL="${LLM_MODEL:-$LLM_DIR/model.gguf}"
    if [[ -f "$MODEL" ]]; then
      cd "$LLM_DIR"
      llama-server --model "$MODEL" --host 127.0.0.1 --port 8080 --alias local > llm.log 2>&1 &
      LLM_PID=$!
      cd "$ROOT"
      echo "    LLM PID: $LLM_PID"
    else
      echo "    No GGUF model at $MODEL — skipping LLM sidecar (set LLM_MODEL or place model.gguf in ./llm/)"
    fi
  else
    echo "    llama.cpp not installed — skipping LLM sidecar (install from https://github.com/ggerganov/llama.cpp)"
  fi

  # Optional: local whisper.cpp transcription sidecar (voice-to-voice).
  echo "==> Starting whisper.cpp sidecar on 127.0.0.1:8890 (run ./scripts/whisper.sh first to build)…"
  WHISPER_PID=""
  if [[ -f "./whisper/whisper.cpp/build/bin/whisper-server" ]]; then
    MODEL="${WHISPER_MODEL:-./whisper/models/ggml-base.bin}"
    if [[ -f "$MODEL" ]]; then
      ./whisper/whisper.cpp/build/bin/whisper-server --host 127.0.0.1 --port 8890 \
        --model "$MODEL" --di --threads 4 > whisper.log 2>&1 &
      WHISPER_PID=$!
      echo "    Whisper PID: $WHISPER_PID"
    else
      echo "    No whisper model at $MODEL — run ./scripts/whisper.sh to download. Skipping."
    fi
  else
    echo "    whisper-server not built — run ./scripts/whisper.sh. Skipping."
  fi

  # Save PIDs for cleanup
  echo "$KOKORO_PID $CLONE_PID $LLM_PID $WHISPER_PID" > /tmp/anubis_sidecars.pids
  echo ""
  echo "Sidecars running. Logs: $KOKORO_DIR/kokoro.log $CLONE_DIR/clone.log $LLM_DIR/llm.log whisper.log"
  echo "Stop with: kill \$(cat /tmp/anubis_sidecars.pids 2>/dev/null)"
}

run_bot() {
  echo "==> Building release binary…"
  cargo build --release

  echo "==> Starting ANUBIS bot…"
  if [[ -z "${ANUBIS_TELEGRAM_TOKEN:-}" ]]; then
    echo "ERROR: ANUBIS_TELEGRAM_TOKEN not set."
    echo "  export ANUBIS_TELEGRAM_TOKEN=your_token_from_@BotFather"
    exit 1
  fi
  ./target/release/anubis
}

# Parse args
DO_DOWNLOAD=false
DO_SERVE=false
DO_BOT=false
if [[ $# -eq 0 ]]; then
  DO_DOWNLOAD=true; DO_SERVE=true; DO_BOT=true
else
  for arg in "$@"; do
    case $arg in
      --download) DO_DOWNLOAD=true ;;
      --serve) DO_SERVE=true ;;
      --bot) DO_BOT=true ;;
      --all) DO_DOWNLOAD=true; DO_SERVE=true; DO_BOT=true ;;
      *) echo "Unknown arg: $arg"; exit 1 ;;
    esac
  done
fi

$DO_DOWNLOAD && download_models
$DO_SERVE && start_sidecars
$DO_BOT && run_bot