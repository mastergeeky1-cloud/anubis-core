#!/usr/bin/env python3
"""
ANUBIS Chatterbox voice-clone sidecar (MIT).
Exposes POST /tts with multipart form:
  reference_audio (WAV), reference_text, text, language
Returns: WAV bytes of `text` in the cloned voice.
Requires GPU (torch CUDA) for reasonable speed; falls back to CPU.
"""
import io
import sys
import torch
import traceback
from flask import Flask, request, Response

app = Flask(__name__)

MODEL = None

def get_model():
    global MODEL
    if MODEL is None:
        try:
            from chatterbox.tts import ChatterboxTTS
        except Exception as e:
            print(f"[clone] import error: {e}", file=sys.stderr)
            raise
        device = "cuda" if torch.cuda.is_available() else "cpu"
        print(f"[clone] loading Chatterbox on {device}...", file=sys.stderr)
        MODEL = ChatterboxTTS.from_pretrained(device=device)
        print("[clone] model loaded", file=sys.stderr)
    return MODEL

@app.route("/health", methods=["GET"])
def health():
    return {"status": "ok", "device": "cuda" if torch.cuda.is_available() else "cpu"}, 200

@app.route("/tts", methods=["POST"])
def tts():
    try:
        ref_audio = request.files.get("reference_audio")
        if ref_audio is None:
            return {"error": "missing reference_audio"}, 400
        text = request.form.get("text", "").strip()
        ref_text = request.form.get("reference_text", "")
        lang = request.form.get("language", "en")
        if not text:
            return {"error": "empty text"}, 400

        ref_bytes = ref_audio.read()
        model = get_model()

        import numpy as np
        import soundfile as sf

        # Save reference wav to a temp file for Chatterbox (it takes a path)
        ref_buf = io.BytesIO(ref_bytes)
        wav_arr, sr = sf.read(ref_buf)
        if wav_arr.ndim > 1:
            wav_arr = wav_arr.mean(axis=1)
        if sr != 22050:
            try:
                import librosa
                wav_arr = librosa.resample(wav_arr, orig_sr=sr, target_sr=22050)
                sr = 22050
            except Exception:
                pass
        import tempfile
        import os
        tmp_ref = tempfile.NamedTemporaryFile(suffix=".wav", delete=False)
        sf.write(tmp_ref.name, wav_arr.astype("float32"), sr, format="WAV")
        tmp_ref.close()

        try:
            audio = model.generate(
                text,
                audio_prompt_path=tmp_ref.name,
                exaggeration=0.5,
                cfg_weight=0.5,
                temperature=0.8,
            )
        finally:
            try:
                os.unlink(tmp_ref.name)
            except Exception:
                pass

        buf = io.BytesIO()
        sf.write(buf, audio.squeeze(0).cpu().numpy(), 22050, format="WAV")
        buf.seek(0)
        return Response(buf.getvalue(), mimetype="audio/wav")
    except Exception as e:
        traceback.print_exc()
        return {"error": str(e)}, 500

if __name__ == "__main__":
    port = int(__import__("os").environ.get("CLONE_PORT", "8008"))
    app.run(host="127.0.0.1", port=port, threaded=True)
