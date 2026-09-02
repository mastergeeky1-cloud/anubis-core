#!/usr/bin/env python3
"""Generate the ANUBIS Voice Teacher repo banner (GitHub social preview).

Dark, clean, modern banner with a teaching + voice motif: a glowing "grad"
cap (scholar), an audio waveform, the ANUBIS wordmark and a teacher tagline.
Pure PIL, no external deps.
"""
from PIL import Image, ImageDraw, ImageFont
import math

W, H = 1280, 640

# ── helpers ───────────────────────────────────────────────────────────────

def lerp(a, b, t):
    return int(a + (b - a) * t)

def font(sz, path="/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf"):
    return ImageFont.truetype(path, sz)

def font_reg(sz):
    return ImageFont.truetype("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf", sz)

def glow(draw, pos, radius, color, alpha):
    x, y = pos
    for r in range(radius, 0, -1):
        a = int(alpha * (1 - r / radius))
        draw.ellipse([x - r, y - r, x + r, y + r], fill=(*color, a))

# ── base gradient canvas: deep slate → soft indigo ────────────────────────

img = Image.new("RGB", (W, H), (10, 13, 20))
dr = ImageDraw.Draw(img, "RGBA")

for y in range(H):
    t = y / H
    r = lerp(20, 8, t)
    g = lerp(24, 10, t)
    b = lerp(48, 16, t)
    dr.rectangle([0, y, W, y + 1], fill=(r, g, b))

# Radial depth glows
glow_layer = Image.new("RGBA", (W, H), (0, 0, 0, 0))
gd = ImageDraw.Draw(glow_layer)
glow(gd, (int(W * 0.78), int(H * 0.22)), 460, (124, 92, 255), 40)   # violet
glow(gd, (int(W * 0.20), int(H * 0.80)), 400, (34, 211, 238), 26)    # cyan
glow(gd, (int(W * 0.60), int(H * 0.90)), 360, (236, 72, 153), 20)    # pink accent
img = Image.alpha_composite(img.convert("RGBA"), glow_layer)
dr = ImageDraw.Draw(img, "RGBA")

# ── audio waveform (left motif) ──────────────────────────────────────────

WAVE_X, WAVE_Y, WAVE_W = 105, H // 2 + 10, 300
bar_w, gap, base = 8, 10, 64
n = int(WAVE_W / (bar_w + gap))
for i in range(n):
    xpos = WAVE_X + i * (bar_w + gap)
    q = (i - n / 2) / (n / 2)
    hgt = base * (0.5 + 0.5 * abs(1 - abs(q))) * (0.7 + 0.3 * math.sin(i * 1.9))
    hue = 0.62 if i < n // 2 else 0.78
    c = (int((0.2 + hue * 0.1) * 255), int((0.75 + (1 - hue) * 0.1) * 255), int(0.92 * 255))
    top = WAVE_Y - hgt / 2
    bot = WAVE_Y + hgt / 2
    dr.rounded_rectangle([xpos, top, xpos + bar_w, bot], radius=bar_w // 2, fill=(*c, 235))

# ── icon: graduate cap (scholar / teacher) ────────────────────────────────

icon_x, icon_y = 430, 130
s = 150  # icon box side
dr.rounded_rectangle([icon_x, icon_y, icon_x + s, icon_y + s], radius=30, fill=(124, 92, 255, 255))
# cap mortarboard
cx, cy = icon_x + s // 2, icon_y + 62
dr.polygon([(cx - 45, cy), (cx + 45, cy), (cx, cy - 34)], fill=(255, 255, 255, 255))
# cap base band
dr.rounded_rectangle([cx - 34, cy, cx + 34, cy + 12], radius=5, fill=(216, 200, 255, 255))
# tassel
dr.line([(cx, cy + 12), (cx + 22, cy + 46)], fill=(255, 255, 255, 255), width=4)
dr.ellipse([cx + 16, cy + 44, cx + 30, cy + 58], fill=(236, 72, 153, 255))

# ── wordmark ─────────────────────────────────────────────────────────────

word_x = icon_x + s + 54
dr.text((word_x, 150), "ANUBIS", font=font(92), fill=(255, 255, 255, 255))
dr.text((word_x, 268), "VOICE  TEACHER", font=font_reg(40), fill=(180, 210, 255, 255))
# thin underline accent
dr.rounded_rectangle([word_x, 322, word_x + 300, 326], radius=2, fill=(236, 72, 153, 255))

# ── tagline ──────────────────────────────────────────────────────────────

tag = "Your multilingual AI language teacher —"
tag2 = "ask, hear it speak, and learn in 10 languages."
dr.text((90, 430), tag, font=font_reg(34), fill=(210, 220, 240, 255))
dr.text((90, 474), tag2, font=font(30), fill=(150, 170, 205, 255))

# ── footer chips ─────────────────────────────────────────────────────────

chips = ["Rust · bare-metal", "Local LLM", "Piper + Kokoro TTS"]
cxw = 90
for label in chips:
    tw = dr.textlength(label, font=font_reg(24))
    pad = 22
    boxw = tw + pad * 2
    dr.rounded_rectangle([cxw, 540, cxw + boxw, 596], radius=28, fill=(28, 34, 52, 230), outline=(70, 80, 110, 255))
    dr.text((cxw + pad, 552), label, font=font_reg(24), fill=(180, 220, 235, 255))
    cxw += boxw + 22

img.convert("RGB").save("assets/banner.png")
print("wrote assets/banner.png")
