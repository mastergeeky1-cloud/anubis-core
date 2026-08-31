#!/usr/bin/env python3
"""
ANUBIS Kokoro TTS sidecar (Apache-2.0).
Exposes POST /v1/audio/speech with JSON {"text":..., "voice":..., "format":"wav"} -> WAV bytes.
Runs on CPU. Voice ids match the Rust client's kokoro_voice() map.
"""
import io
import sys
import tempfile
from flask import Flask, request, send_file, Response

try:
    from kokoro import KPipeline
except Exception as e:
    print(f"[kokoro] failed to import KPipeline: {e}", file=sys.stderr)
    sys.exit(1)

app = Flask(__name__)

PIPE = None  # lazily init per lang_code

def get_pipeline(lang_code: str):
    global PIPE
    if PIPE is None:
        PIPE = KPipeline(lang_code=lang_code)
    return PIPE

@app.route("/health", methods=["GET"])
def health():
    return {"status": "ok"}, 200

@app.route("/v1/audio/speech", methods=["POST"])
def speech():
    data = request.get_json(force=True, silent=True) or {}
    text = data.get("text", "").strip()
    voice = data.get("voice", "af_heart")
    if not text:
        return {"error": "empty text"}, 400

    # Kokoro voice prefix encodes language: af/am = en-US, bf/bm = en-GB, etc.
    lang_code = "a"  # default American English
    if voice.startswith("b"):
        lang_code = "b"

    try:
        pipe = get_pipeline(lang_code)
        chunks = []
        # Kokoro returns generator of (graphemes, phonemes, audio)
        for _, _, audio in pipe(text, voice=voice, speed=1.0, split_pattern=r"\n+"):
            chunks.append(audio)
        if not chunks:
            return {"error": "no audio generated"}, 500
        import numpy as np
        audio_np = np.concatenate(chunks, axis=0)
        import soundfile as sf
        buf = io.BytesIO()
        sf.write(buf, audio_np, 24000, format="WAV")
        buf.seek(0)
        return send_file(buf, mimetype="audio/wav", download_name="speech.wav")
    except Exception as e:
        return {"error": str(e)}, 500

if __name__ == "__main__":
    port = int(__import__("os").environ.get("KOKORO_PORT", "8880"))
    app.run(host="127.0.0.1", port=port, threaded=True)
