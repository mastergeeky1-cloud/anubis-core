#!/usr/bin/env bash
# Download official Piper ONNX voice models (from rhasspy/piper-voices)
# so every voice in the ANUBIS teacher catalogue works with zero user setup.
# Usage: ./scripts/download_voices.sh [voices_dir]
set -euo pipefail

VOICES_DIR="${1:-./voices}"
BASE="https://huggingface.co/rhasspy/piper-voices/resolve/main"

# id
VOICES=(
  "ar_JO-kareem-low"
  "ar_JO-kareem-medium"
  "de_DE-eva_k-x_low"
  "de_DE-thorsten-high"
  "de_DE-thorsten-medium"
  "en_GB-alan-low"
  "en_GB-cori-high"
  "en_GB-northern_english_male-medium"
  "en_US-amy-medium"
  "en_US-lessac-high"
  "en_US-lessac-medium"
  "en_US-ryan-high"
  "en_US-ryan-medium"
  "es_ES-carlfm-x_low"
  "es_ES-davefx-medium"
  "es_ES-mls_10246-low"
  "fr_FR-siwis-low"
  "fr_FR-siwis-medium"
  "fr_FR-upmc-medium"
  "hi_IN-pratham-medium"
  "hi_IN-priyamvada-medium"
  "it_IT-paola-medium"
  "it_IT-riccardo-x_low"
  "pt_BR-faber-medium"
  "pt_PT-tugao-medium"
  "ru_RU-denis-medium"
  "ru_RU-irina-medium"
  "ru_RU-ruslan-medium"
  "tr_TR-dfki-medium"
)

fetched=0
skipped=0
failed=0
for id in "${VOICES[@]}"; do
  region="${id%%-*}"            # ar_JO, de_DE, ...
  rest="${id#*-}"               # kareem-medium, thorsten-high, ...
  name="${rest%-*}"             # kareem, thorsten, ...
  quality="${rest##*-}"         # low, medium, high, x_low
  lang="${region%%_*}"          # ar, de, en, ...
  dest="$VOICES_DIR/$lang/$region/$name/$quality"
  model="$dest/$id.onnx"
  if [ -f "$model" ]; then
    skipped=$((skipped+1))
    continue
  fi
  mkdir -p "$dest"
  ok=1
  for suffix in ".onnx" ".onnx.json"; do
    url="$BASE/$lang/$region/$name/$quality/$id$suffix"
    out="$dest/$id$suffix"
    if curl -fsSL --retry 3 -o "$out" "$url"; then
      :
    else
      echo "FAILED: $url"
      failed=$((failed+1))
      ok=0
      break
    fi
  done
  if [ "$ok" = 1 ]; then
    echo "OK $id"
    fetched=$((fetched+1))
  fi
done

echo
echo "=== done: $fetched fetched, $skipped already present, $failed failed ==="
