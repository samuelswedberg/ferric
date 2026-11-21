"""Placeholder album covers used in the Ferric mockups (Ember, Tide, Moss).

Original procedural art made for this project, plus the accent-colour pick
the mockups use (most vivid of 6 median-cut colours, lightened to >= 4.5:1
contrast). Writes <name>.png and background variants into the current
folder and prints the derived colours. Requires numpy and Pillow.
"""
import numpy as np, colorsys, json
from PIL import Image, ImageDraw, ImageFilter
rng = np.random.default_rng(7)
S = 640
def grain(img, amt=14):
    a = np.asarray(img).astype(np.int16)
    a += rng.integers(-amt, amt, a.shape[:2])[..., None]
    return Image.fromarray(np.clip(a, 0, 255).astype(np.uint8))
def vgrad(c1, c2):
    t = np.linspace(0, 1, S)[:, None, None]
    a = (np.array(c1) * (1 - t) + np.array(c2) * t) * np.ones((S, S, 3))
    return Image.fromarray(a.astype(np.uint8))
# 1 Ember: dusk gradient, low sun, dark hills
im = vgrad((40, 12, 30), (232, 96, 44)); d = ImageDraw.Draw(im)
d.ellipse((190, 250, 450, 510), fill=(255, 196, 92))
for i, (h, c) in enumerate([(430, (70, 20, 34)), (500, (38, 10, 24)), (570, (18, 6, 14))]):
    pts = [(x, h + 40 * np.sin(x / 90 + i * 2)) for x in range(0, S + 10, 10)] + [(S, S), (0, S)]
    d.polygon(pts, fill=c)
ember = grain(im)
# 2 Tide: deep blue with layered teal waves
im = vgrad((8, 20, 48), (18, 70, 110)); d = ImageDraw.Draw(im)
for i in range(7):
    y = 200 + i * 60; c = (30 + i * 12, 140 + i * 12, 170 + i * 8)
    pts = [(x, y + 28 * np.sin(x / 60 + i)) for x in range(0, S + 10, 8)] + [(S, S), (0, S)]
    d.polygon(pts, fill=c if i % 2 else tuple(int(v * .7) for v in c))
d.ellipse((440, 70, 540, 170), fill=(236, 240, 220))
tide = grain(im)
# 3 Moss: off-white geometric
im = Image.new("RGB", (S, S), (226, 222, 206)); d = ImageDraw.Draw(im)
d.rectangle((60, 60, 380, 380), fill=(58, 92, 60)); d.ellipse((260, 240, 580, 560), fill=(196, 160, 60))
d.rectangle((60, 420, 220, 580), fill=(30, 36, 32)); d.polygon([(420, 60), (580, 60), (580, 220)], fill=(160, 64, 48))
moss = grain(im, 8)
def bg(img, w=360, h=520):
    # cover-fit to 90x130, blur sigma 6, x0.35, upscale
    small = img.resize((130, 130), Image.LANCZOS).crop((20, 0, 110, 130))
    b = small.filter(ImageFilter.GaussianBlur(6))
    a = (np.asarray(b).astype(float) * 0.35).astype(np.uint8)
    return Image.fromarray(a).resize((w, h), Image.BICUBIC)
def lum(c):
    f = [(v / 255) for v in c]; f = [v / 12.92 if v <= .03928 else ((v + .055) / 1.055) ** 2.4 for v in f]
    return .2126 * f[0] + .7152 * f[1] + .0722 * f[2]
def contrast(a, b):
    l1, l2 = sorted([lum(a), lum(b)], reverse=True); return (l1 + .05) / (l2 + .05)
def accent(img, bgimg):
    q = img.resize((64, 64)).quantize(6, method=Image.Quantize.MEDIANCUT)
    pal = q.getpalette()[:18]; counts = sorted(q.getcolors(), reverse=True)
    best, score = (179, 179, 179), -1
    for n, i in counts:
        c = tuple(pal[i * 3:i * 3 + 3]); h, l, s = colorsys.rgb_to_hls(*[v / 255 for v in c])
        if .25 <= l <= .85 and s * n ** .5 > score: best, score = c, s * n ** .5
    mean = tuple(int(v) for v in np.asarray(bgimg).reshape(-1, 3).mean(0))
    h, l, s = colorsys.rgb_to_hls(*[v / 255 for v in best])
    while contrast(best, mean) < 4.5 and l < .95:
        l += .02; best = tuple(int(v * 255) for v in colorsys.hls_to_rgb(h, l, s))
    return best, mean
out = {}
for name, img in [("ember", ember), ("tide", tide), ("moss", moss)]:
    img.save(f"{name}.png")
    for mode, (w, h) in {"full": (360, 520), "card": (360, 120), "strip": (360, 36)}.items():
        bg(img, w, h).save(f"{name}-bg-{mode}.png")
    acc, mean = accent(img, bg(img))
    on = (0, 0, 0) if contrast(acc, (0, 0, 0)) > contrast(acc, (255, 255, 255)) else (255, 255, 255)
    out[name] = {"accent": "#%02X%02X%02X" % acc, "onAccent": "#%02X%02X%02X" % on, "meanBg": "#%02X%02X%02X" % mean, "contrast": round(contrast(acc, mean), 2)}
print(json.dumps(out, indent=1))
