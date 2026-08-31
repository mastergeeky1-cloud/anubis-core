#!/usr/bin/env python3
"""Generate the ANUBIS Core banner shown in /start and /menu.

Figma isn't available in this environment, so this renders an equivalent
on-brand banner with Pillow: dark gradient, a stylized ANUBIS eye emblem,
wordmark, feature chips and a voice waveform.
"""
import math
import os

from PIL import Image, ImageDraw, ImageFilter, ImageFont

W, H = 800, 400
OUT = os.path.join(os.path.dirname(__file__), "..", "assets", "banner.png")

FONT_BOLD = "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf"
FONT_REG = "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"


def font(path, size):
    return ImageFont.truetype(path, size)


def lerp(a, b, t):
    return tuple(int(a[i] + (b[i] - a[i]) * t) for i in range(3))


# ── Base gradient (deep indigo -> near black) ────────────────────────────────
TOP = (18, 16, 38)
BOT = (8, 7, 18)
GOLD = (247, 200, 92)
CYAN = (86, 214, 214)
WHITE = (240, 242, 255)

img = Image.new("RGB", (W, H))
px = img.load()
for y in range(H):
    t = y / (H - 1)
    base = lerp(TOP, BOT, t)
    for x in range(W):
        # subtle diagonal sheen
        glow = 14 * (1 - abs((x / W) - 0.32)) * (1 - t)
        r = min(255, base[0] + int(glow * 0.6))
        g = min(255, base[1] + int(glow))
        b = min(255, base[2] + int(glow * 1.4))
        px[x, y] = (r, g, b)

# soft radial vignette behind the emblem
vig = Image.new("L", (W, H), 0)
vd = ImageDraw.Draw(vig)
vd.ellipse([150 - 150, 200 - 150, 150 + 150, 200 + 150], fill=70)
vig = vig.filter(ImageFilter.GaussianBlur(60))
img.putalpha(255)
glow_layer = Image.new("RGB", (W, H), CYAN)
img = Image.composite(glow_layer, img, vig)
img = img.convert("RGB")

d = ImageDraw.Draw(img)

# ── ANUBIS eye emblem (left) ─────────────────────────────────────────────────
cx, cy, rx, ry = 150, 200, 78, 42
# almond outline
d.ellipse([cx - rx, cy - ry, cx + rx, cy + ry], outline=GOLD, width=5)
# iris
ir = 30
d.ellipse([cx - ir, cy - ir, cx + ir, cy + ir], outline=CYAN, width=4)
d.ellipse([cx - ir + 6, cy - ir + 6, cx + ir - 6, cy + ir - 6], outline=CYAN, width=2)
# pupil
pr = 12
d.ellipse([cx - pr, cy - pr, cx + pr, cy + pr], fill=GOLD)
# upper lid sweep
d.line([cx - rx, cy - 6, cx - rx + 26, cy - ry + 4], fill=GOLD, width=4)
d.line([cx + rx, cy - 6, cx + rx - 26, cy - ry + 4], fill=GOLD, width=4)
# radiating rays (divine)
for a in range(-60, 61, 20):
    rad = math.radians(a)
    r0 = ir + 14
    r1 = ir + 30
    x0, y0 = cx + r0 * math.cos(rad), cy + r0 * math.sin(rad) * 0.55
    x1, y1 = cx + r1 * math.cos(rad), cy + r1 * math.sin(rad) * 0.55
    d.line([x0, y0, x1, y1], fill=GOLD, width=2)

# ── Wordmark + subtitle (right) ──────────────────────────────────────────────
title = font(FONT_BOLD, 64)
sub = font(FONT_REG, 22)
chip = font(FONT_REG, 18)

d.text((300, 120), "ANUBIS", font=title, fill=WHITE)
d.text((302, 192), "Local-first AI Voice Clone Bot", font=sub, fill=CYAN)

# feature chips
labels = ["TTS", "Voice Clone", "Noxis LLM", "Telegram Stars"]
x = 302
y = 240
for lab in labels:
    tw = d.textlength(lab, font=chip)
    pad = 12
    d.rounded_rectangle([x - 6, y - 4, x + tw + pad, y + 26], radius=8,
                        outline=GOLD, width=2)
    d.text((x + pad / 2, y), lab, font=chip, fill=GOLD)
    x += int(tw) + pad + 16

# ── Voice waveform (bottom) ──────────────────────────────────────────────────
wy = 340
mid = W // 2
for x in range(40, W - 40, 7):
    # mirrored symmetric envelope
    env = math.sin((x - 40) / (W - 80) * math.pi)
    amp = (8 + 26 * abs(math.sin(x / 22.0))) * env
    if amp < 2:
        amp = 2
    col = CYAN if (x % 14 == 0) else GOLD
    d.line([x, wy - amp, x, wy + amp], fill=col, width=3)

os.makedirs(os.path.dirname(OUT), exist_ok=True)
img.save(OUT)
print("wrote", os.path.abspath(OUT), img.size)
