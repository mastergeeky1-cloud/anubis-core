#!/usr/bin/env python3
"""Generate the ANUBIS repo banner (GitHub social preview).

Creates a dark, modern banner with a gradient, an audio-wave motif, the ANUBIS
wordmark, and a feature tagline. Pure PIL, no external deps.
"""
from PIL import Image, ImageDraw, ImageFont, ImageFilter
import math

W, H = 1280, 640

# ── helpers ───────────────────────────────────────────────────────────────

def lerp(a, b, t):
    return int(a + (b - a) * t)

def hsv2rgb(h, s, v):
    i = int(h * 6)
    f = h * 6 - i
    p = v * (1 - s)
    q = v * (1 - f * s)
    t = v * (1 - (1 - f) * s)
    if i % 6 == 0: r, g, b = v, t, p
    elif i % 6 == 1: r, g, b = q, v, p
    elif i % 6 == 2: r, g, b = p, v, t
    elif i % 6 == 3: r, g, b = p, q, v
    elif i % 6 == 4: r, g, b = t, p, v
    else: r, g, b = v, p, q
    return int(r * 255), int(g * 255), int(b * 255)

def font(sz, path="/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf"):
    return ImageFont.truetype(path, sz)

def font_reg(sz):
    return ImageFont.truetype("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf", sz)

def glow(draw, pos, radius, color, alpha):
    x, y = pos
    for r in range(radius, 0, -1):
        a = int(alpha * (1 - r / radius))
        draw.ellipse([x - r, y - r, x + r, y + r], fill=(*color, a))

# ── base gradient canvas ──────────────────────────────────────────────────

img = Image.new("RGB", (W, H), (11, 14, 22))
dr = ImageDraw.Draw(img, "RGBA")

# Vertical-ish diagonal gradient from deep indigo (top-left) to near-black (bottom).
for y in range(H):
    t = y / H
    r = lerp(24, 10, t)
    g = lerp(20, 12, t)
    b = lerp(46, 18, t)
    dr.rectangle([0, y, W, y + 1], fill=(r, g, b))

# Soft radial glows for depth.
glow_layer = Image.new("RGBA", (W, H), (0, 0, 0, 0))
gd = ImageDraw.Draw(glow_layer)
glow(gd, (int(W * 0.82), int(H * 0.25)), 420, (124, 92, 255), 44)   # violet
glow(gd, (int(W * 0.78), int(H * 0.85)), 380, (34, 211, 238), 30)    # cyan
glow(gd, (int(W * 0.15), int(H * 0.7)), 320, (236, 72, 153), 22)     # pink
img = Image.alpha_composite(img.convert("RGBA"), glow_layer)
dr = ImageDraw.Draw(img, "RGBA")

# ── audio waveform (left-hand motif) ─────────────────────────────────────

WAVE_X, WAVE_Y, WAVE_W = 90, H // 2, 330
bar_w = 7
gap = 9
base = 46
n = int(WAVE_W / (bar_w + gap))
bars = []
for i in range(n):
    # organic pseudo-random heights, peaking near the middle
    x = WAVE_X + i * (bar_w + gap)
    q = (i - n / 2) / (n / 2)
    hgt = base * (0.5 + 0.5 * abs(1 - abs(q))) * (0.7 + 0.3 * math.sin(i * 1.7))
    bars.append((x, hgt))

for i, (x, hgt) in enumerate(bars):
    grad = (0.30, 0.72, 0.82)  # fixed cyan->violet hue range
    hue = lerp_pull = 0.62 if i < n // 2 else 0.75
    c = hsv2rgb(float(hue) - 0.02, 0.75, 0.95)
    # vertical rounded bar
    top = WAVE_Y - hgt / 2
    bot = WAVE_Y + hgt / 2
    dr.rounded_rectangle([x, top, x + bar_w, bot], radius=bar_w // 2, fill=(*c, 220))

# Tie-in dot
dr.ellipse([WAVE_X - 16, WAVE_Y - 4, WAVE_X - 8, WAVE_Y + 4], fill=(236, 72, 153, 230))

# ── wordmark ─────────────────────────────────────────────────────────────

# Icon: a stylized "voice" chevron in a rounded square.
icon_size = 108
left = WAVE_X + WAVE_W + 60
itop = 120
dr.rounded_rectangle([left, itop, left + icon_size, itop + icon_size],
                     radius=26, fill=(124, 92, 255, 255))
# Inner waveform tri-bars for the icon.
ico_cx = left + icon_size // 2
ico_cy = itop + icon_size // 2
for k, (dx, hw, col) in enumerate([(-24, 10, (220, 214, 255)),
                                    (0, 20, (255, 255, 255)),
                                    (24, 10, (220, 214, 255))]):
    gx = ico_cx + dx
    dr.rounded_rectangle([gx - hw, ico_cy - 14, gx - hw + hw * 2, ico_cy + 14],
                         radius=8, fill=col)

# text baseline
tx = left + icon_size + 34
ty = itop + icon_size // 2

# "ANUBIS" — big gradient text (draw solid white then overlay a colored copy)
big_font = font(104)
word = "ANUBIS"
text_w = dr.textlength(word, font=big_font)
# Measure ascent to vertically centre
bbox = big_font.getbbox(word)
ascent = bbox[3] - bbox[1]
# draw white base
dr.text((tx, ty - ascent // 2), word, font=big_font, fill=(255, 255, 255, 255))
# overlay violet gradient copy for vibrancy
grad_copy = Image.new("RGBA", (W, H), (0, 0, 0, 0))
gdr = ImageDraw.Draw(grad_copy)
for ch_i, ch in enumerate(word):
    # per-character gradient: cyan -> violet
    t = ch_i / max(1, len(word) - 1)
    c = hsv2rgb(lerp(0.55, 0.72, t), 0.85, 1.0)
    char_x = tx + dr.textlength(word[:ch_i], font=big_font)
    gdr.text((char_x, ty - ascent // 2), ch, font=big_font, fill=(*c, 255))
img = Image.alpha_composite(img, grad_copy)
dr = ImageDraw.Draw(img, "RGBA")

# tagline under wordmark
tag = "LOCAL-FIRST VOICE AI · RUST"
tag_font = font_reg(30)
tag_y = ty - ascent // 2 + 128
dr.text((tx, tag_y), tag, font=tag_font, fill=(150, 164, 190, 255))

# ── feature chips (right column) ─────────────────────────────────────────

chip_x = int(W * 0.62)
chip_y_start = 120
features = [
    ("🧠", "Noxis Core brain — local LLM"),
    ("🗣", "Streaming real-time voice (WS)"),
    ("🎙", "Clone voices · 20+ TTS voices · 10 langs"),
    ("🔒", "Consent, watermark, rate-limit"),
    ("⚡", "Telegram Stars payments"),
]
chip_font = font_reg(27)
row_h = 62
for i, (icon, label) in enumerate(features):
    y = chip_y_start + i * row_h
    dr.rounded_rectangle([chip_x, y, chip_x + 500, y + 46], radius=23,
                         fill=(255, 255, 255, 12), outline=(124, 92, 255, 90))
    # label may be mixed (emoji renders as box in DejaVu); draw text only
    dr.text((chip_x + 20, y + 9), f"  {label}", font=chip_font,
            fill=(226, 232, 240, 245))

# ── footer ───────────────────────────────────────────────────────────────

foot = "https://github.com/mastergeeky1-cloud/anubis-core"
foot_font = font_reg(24)
dr.text((WAVE_X, H - 60), foot, font=foot_font, fill=(120, 134, 160, 220))

# ── save ────────────────────────────────────────────────────────────────

out = img.convert("RGB")
out.save("/home/ubuntu/anubis-core/assets/banner.png")
print("saved assets/banner.png", out.size)
