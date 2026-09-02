#!/usr/bin/env bash
# ANUBIS Voice Teacher — local model + sidecar setup (fully offline-capable).
# Downloads only OPEN-SOURCE, PERMISSIVELY-LICENSED components:
#   • Piper (MIT) voice models
#   • Kokoro (Apache-2.0) via a tiny local server
#   • llama.cpp server + a small GGUF LLM for Noxis Core
#
# Edit the LLM_MODEL url below to your preferred GGUF.
set -euo pipefail
cd "$(dirname "$0")"

PIPER_DIR="./voices"
KOKORO_DIR="./kokoro"
LLM_DIR="./llm"

echo "==> Creating dirs"
mkdir -p "$PIPER_DIR" "$KOKORO_DIR" "$LLM_DIR" ./audio_output

echo "==> Piper voices (MIT)"
# Piper voice packs: each is <lang>/<id>.onnx + .onnx.json
PIPER_BASE="https://huggingface.co/rhasspy/piper-voices/resolve/main"
declare -A VOICES=(
  ["en/en_US-amy-medium"]="en/en_US/amy/medium"
  ["en/en_US-amy-high"]="en/en_US/amy/high"
  ["en/en_US-ryan-high"]="en/en_US/ryan/high"
  ["en/en_US-ryan-medium"]="en/en_US/ryan/medium"
  ["en/en_US-lessac-medium"]="en/en_US/lessac/medium"
  ["en/en_US-lessac-high"]="en/en_US/lessac/high"
  ["en/en_US-hubert-high"]="en/en_US/hubert/high"
  ["en/en_GB-alan-low"]="en/en_GB/alan/low"
  ["en/en_GB-cori-high"]="en/en_GB/cori/high"
  ["en/en_GB-northern_english_male-medium"]="en/en_GB/northern_english_male/medium"
  ["ar/ar_JO-kareem-medium"]="ar/ar_JO/kareem/medium"
  ["it/it_IT-riccardo-x_low"]="it/it_IT/riccardo/x_low"
  ["it/it_IT-paola-medium"]="it/it_IT/paola/medium"
  ["fr/fr_FR-siwis-medium"]="fr/fr_FR/siwis/medium"
  ["es/es_ES-carlfm-x_low"]="es/es_ES/carlfm/x_low"
  ["de/de_DE-thorsten-medium"]="de/de_DE/thorsten/medium"
  ["de/de_DE-thorsten-high"]="de/de_DE/thorsten/high"
  ["ru/ru_RU-irinia-medium"]="ru/ru_RU/irinia/medium"
  ["hi/hi_IN-deepika-medium"]="hi/hi_IN/deepika/medium"
  ["tr/tr_TR-dfki-medium"]="tr/tr_TR/dfki/medium"
  ["pt_BR/pt_BR-faber-medium"]="pt/pt_BR/faber/medium"
)
for dest in "${!VOICES[@]}"; do
  src="${VOICES[$dest]}"
  mkdir -p "$PIPER_DIR/$(dirname "$dest")"
  for ext in onnx onnx.json; do
    f="$dest.$ext"
    echo "    -> $f"
    curl -fsSL "$PIPER_BASE/$src/$ext" -o "$PIPER_DIR/$f" || echo "    (skip $f)"
  done
done

echo "==> Community Arabic voice packs (more Arabic voices!)"
# These are community Piper models not in the official rhasspy/piper-voices
# tree. Each downloads a .onnx + .onnx.json into ./voices/ar.
declare -A AR_COMMUNITY=(
  ["ar/ar_JO-kareem-low"]="ar/ar_JO/kareem/low"
  ["ar/ar-zayd0-diacritized"]="https://huggingface.co/neurlang/piper-onnx-zayd0-arabic-diacritized/resolve/main/piper-onnx-zayd0-arabic-diacritized"
  ["ar/ar_AE-emirati-female"]="https://huggingface.co/vadimbelsky/arabic-emirati-female-piper/resolve/main/arabic-emirati-female-model"
)
for dest in "${!AR_COMMUNITY[@]}"; do
  src="${AR_COMMUNITY[$dest]}"
  mkdir -p "$PIPER_DIR/$(dirname "$dest")"
  # If src is a full URL, download directly; else resolve from the official tree.
  if [[ "$src" == http* ]]; then
    echo "    -> $dest (community)"
    curl -fsSL "$src.onnx" -o "$PIPER_DIR/$dest.onnx" || echo "    (skip $dest.onnx)"
    curl -fsSL "$src.onnx.json" -o "$PIPER_DIR/$dest.onnx.json" || echo "    (skip $dest.onnx.json)"
  else
    for ext in onnx onnx.json; do
      echo "    -> $dest.$ext"
      curl -fsSL "$PIPER_BASE/$src/$ext" -o "$PIPER_DIR/$dest.$ext" || echo "    (skip $dest.$ext)"
    done
  fi
done

echo "==> Piper binary"
if ! command -v piper >/dev/null 2>&1; then
  echo "    Install piper: https://github.com/rhasspy/piper (or apt on some distros)"
fi

echo "==> Kokoro sidecar (Apache-2.0) — start script written to ./kokoro/serve.sh"
cat > "$KOKORO_DIR/serve.sh" <<'EOF'
#!/usr/bin/env bash
# Tiny Kokoro HTTP server. Requires: pip install kokoro flask torch
python3 - <<'PY'
from flask import Flask, request, send_file
import io, torch, tempfile
from kokoro import KPipeline
app = Flask(__name__)
pipeline = KPipeline(lang_code="a")  # American English default
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
chmod +x "$KOKORO_DIR/serve.sh"

echo "==> Noxis Core LLM (llama.cpp, local GGUF)"
echo "    Install llama.cpp: https://github.com/ggerganov/llama.cpp"
echo "    Pick a small permissive GGUF (e.g. a Mistral/Llama derivative, Apache/MIT)."
echo "    Then run: ./llm/serve.sh"
cat > "$LLM_DIR/serve.sh" <<'EOF'
#!/usr/bin/env bash
# Local LLM for Noxis Core. Replace MODEL with your GGUF path.
# brew/apt install llama.cpp   (provides llama-server)
MODEL="${LLM_MODEL:-./llm/model.gguf}"
llama-server --model "$MODEL" --host 127.0.0.1 --port 8080 --alias local
EOF
chmod +x "$LLM_DIR/serve.sh"

echo ""
echo "Setup complete. Start each sidecar in its own terminal, then run the bot:"
echo "  ./kokoro/serve.sh      # terminal 1 (neural TTS, optional)"
echo "  ./llm/serve.sh         # terminal 2 (local LLM, for /ask)"
echo "  export ANUBIS_TELEGRAM_TOKEN=...  # NEVER commit this"
echo "  cargo run --release"
