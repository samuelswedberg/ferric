"""Reference implementation of Ferric's "worn" textures (grit level 2).

This is the source of the textures in design/ferric-ui.figcraft and the
spec for the theme engine (task T03), which ports this logic to Rust.

Run from an empty working folder, after covers.py has written the sample
covers (ember.png, tide.png, moss.png) there:

    python /path/to/design/tools/covers.py
    python /path/to/design/tools/textures.py

Output goes to ./grit/. Requires Python 3.10+, numpy and Pillow.
The textures are procedural; nothing here uses third-party artwork.
"""
import os
import numpy as np
from PIL import Image, ImageDraw, ImageFilter

os.makedirs("grit", exist_ok=True)
rng = np.random.default_rng(11)
O = "grit/"


def arr(im):
    return np.asarray(im).astype(np.float32)


def img(a, mode="RGB"):
    return Image.fromarray(np.clip(a, 0, 255).astype(np.uint8), mode)


def noise(h, w, s=1.0):
    return rng.normal(0, s, (h, w)).astype(np.float32)


def smooth_noise(h, w, blur=2, amp=40):
    n = (noise(h, w) * amp + 128).clip(0, 255).astype(np.uint8)
    return arr(Image.fromarray(n).filter(ImageFilter.GaussianBlur(blur))) / 255


def blobs(h, w, n, rmin, rmax, blur):
    m = Image.new("L", (w, h), 0)
    d = ImageDraw.Draw(m)
    for _ in range(n):
        x, y, r = rng.integers(0, w), rng.integers(0, h), rng.integers(rmin, rmax)
        d.ellipse((x - r, y - r, x + r, y + r), fill=int(rng.integers(60, 255)))
    return arr(m.filter(ImageFilter.GaussianBlur(blur))) / 255


def scratches(h, w, n, alpha, lmin, lmax, width=1, curvy=False):
    m = Image.new("L", (w, h), 0)
    d = ImageDraw.Draw(m)
    for _ in range(n):
        x, y = rng.uniform(0, w), rng.uniform(0, h)
        a = rng.uniform(0, np.pi)
        L = rng.uniform(lmin, lmax)
        pts = []
        for t in np.linspace(0, 1, 12):
            bend = (np.sin(t * 3) * L * 0.08) if curvy else 0
            pts.append((x + np.cos(a) * L * t - np.sin(a) * bend, y + np.sin(a) * L * t + np.cos(a) * bend))
        d.line(pts, fill=int(rng.integers(alpha // 2, alpha)), width=width)
    return arr(m) / 255


def overlay(w, h, n_dust=260):
    a = np.zeros((h, w, 4), np.float32)
    g = noise(h, w, 1)
    a[..., :3] = np.where(g[..., None] > 0, 255, 0)
    a[..., 3] = np.abs(g) * 16
    dust = Image.new("L", (w, h), 0)
    d = ImageDraw.Draw(dust)
    for _ in range(n_dust):
        x, y = rng.integers(0, w), rng.integers(0, h)
        r = int(rng.choice([1, 1, 1, 2, 2, 3]))
        d.ellipse((x, y, x + r, y + r), fill=int(rng.integers(70, 160)))
    hair = scratches(h, w, 14, 90, 20, 90, 1, curvy=True) * 255
    scr = scratches(h, w, 26, 60, 30, 200, 1) * 255
    lite = np.maximum(arr(dust), np.maximum(hair, scr))
    a[..., :3] = np.where(lite[..., None] > a[..., 3:4], 245, a[..., :3])
    a[..., 3] = np.maximum(a[..., 3], lite)
    return img(a, "RGBA")


def worn(src, size, fade=.14, ring=True, crease=True, paper=(232, 222, 200), edge=1.0):
    im = Image.open(src).convert("RGB").resize((size, size), Image.LANCZOS)
    a = arr(im)
    p = np.array(paper, np.float32)
    a = a * (1 - fade) + p * fade
    yy, xx = np.mgrid[0:size, 0:size].astype(np.float32)
    de = np.minimum(np.minimum(xx, yy), np.minimum(size - 1 - xx, size - 1 - yy))
    n = smooth_noise(size, size)
    em = np.clip((size * .02 * edge - de) / (size * .02) + (n - .5) * .9, 0, 1) ** 1.5
    cd = np.minimum.reduce([np.hypot(xx, yy), np.hypot(size - xx, yy), np.hypot(xx, size - yy), np.hypot(size - xx, size - yy)])
    cm = np.clip((size * .07 - cd) / (size * .07) + (n - .5) * .6, 0, 1)
    wm = np.clip(np.maximum(em, cm) * .9, 0, 1)[..., None]
    a = a * (1 - wm) + p * wm
    if ring:
        r = np.hypot(xx - size * .5, yy - size * .52)
        R = size * .44
        rm = np.exp(-((r - R) / (size * .012)) ** 2) * .22 + np.exp(-((r - R) / (size * .05)) ** 2) * .06
        rm *= (0.6 + 0.8 * n)
        a = a * (1 - rm[..., None]) + 255 * rm[..., None]
    if crease:
        c = Image.new("L", (size, size), 0)
        ImageDraw.Draw(c).line((size * .62, size, size, size * .64), fill=255, width=max(1, size // 260))
        cm2 = arr(c.filter(ImageFilter.GaussianBlur(1.2))) / 255 * .35
        a = a * (1 - cm2[..., None]) + 255 * cm2[..., None]
    sc = scratches(size, size, 18, 120, size * .05, size * .3) * .25
    a = a * (1 - sc[..., None]) + 245 * sc[..., None]
    a += noise(size, size, 6)[..., None]
    return img(a)


def record(size, label_src):
    yy, xx = np.mgrid[0:size, 0:size].astype(np.float32)
    c = size / 2
    r = np.hypot(xx - c, yy - c)
    th = np.arctan2(yy - c, xx - c)
    base = 16 + 3 * np.sin(r * 1.9) + noise(size, size, 2.5)
    sheen = (np.cos(2 * (th - .6)) ** 8) * 34 * np.exp(-((r - size * .34) / (size * .18)) ** 2)
    a = np.stack([base + sheen] * 3, -1)
    rim = np.clip((r - size * .47) / (size * .03), 0, 1)
    a = a * (1 - rim[..., None] * .4)
    lab = worn(label_src, int(size * .34), fade=.18, ring=False, crease=False, edge=.3)
    L = arr(lab)
    lr = size * .17
    o = int(c - lr)
    region = a[o:o + L.shape[0], o:o + L.shape[1]]
    sub = (r[o:o + L.shape[0], o:o + L.shape[1]] < lr)[..., None]
    a[o:o + L.shape[0], o:o + L.shape[1]] = np.where(sub, L, region)
    a[r < size * .012] = 30
    d = scratches(size, size, 40, 200, size * .03, size * .25, 1, curvy=True) * .35
    a = a * (1 - d[..., None]) + 200 * d[..., None]
    dust = blobs(size, size, 140, 1, 3, 0.6) * .55
    a = a * (1 - dust[..., None]) + 215 * dust[..., None]
    alpha = np.clip((size * .5 - r) * 2, 0, 255)
    return img(np.dstack([a, alpha]), "RGBA")


def plinth(w, h, col=(30, 27, 24)):
    a = np.ones((h, w, 3), np.float32) * np.array(col, np.float32)
    a += noise(h, w, 3)[..., None]
    a += (blobs(h, w, 40, 20, 80, 30) * 10)[..., None]
    a += (scratches(h, w, 90, 120, 8, 60) * 40)[..., None]
    yy, xx = np.mgrid[0:h, 0:w].astype(np.float32)
    v = 1 - .35 * (((xx - w / 2) / (w / 2)) ** 2 + ((yy - h * .3) / (h * .8)) ** 2)
    return img(a * np.clip(v, .5, 1)[..., None])


def card(w, h, col=(214, 200, 172)):
    a = np.ones((h, w, 3), np.float32) * np.array(col, np.float32)
    fib = arr(Image.fromarray((noise(h, w) * 30 + 128).clip(0, 255).astype(np.uint8)).resize((w // 4, h)).resize((w, h)))
    a += ((fib - 128) * .25)[..., None] + noise(h, w, 4)[..., None]
    a -= (blobs(h, w, 10, 30, 90, 25) * 18)[..., None]
    yy, xx = np.mgrid[0:h, 0:w].astype(np.float32)
    de = np.minimum(np.minimum(xx, yy), np.minimum(w - 1 - xx, h - 1 - yy))
    em = np.clip((8 - de) / 8 + noise(h, w, .3), 0, 1) * .6
    a = a * (1 - em[..., None]) + 240 * em[..., None]
    return img(a)


def case(w, h, art_src):
    a = np.zeros((h, w, 4), np.float32)
    a[..., :3] = 40
    a[..., 3] = 255
    inset, spine = 16, 26
    aw, ah = w - inset * 2 - spine, h - inset * 2
    art = worn(art_src, max(aw, ah), fade=.1, ring=False, crease=False, edge=.5).resize((aw, ah))
    a[inset:inset + ah, inset + spine:inset + spine + aw, :3] = arr(art)
    a[inset:inset + ah, inset:inset + spine, :3] = arr(art.crop((0, 0, 1, ah)).resize((spine, ah))) * .7
    yel = np.array([236, 214, 150], np.float32)
    a[..., :3] = a[..., :3] * .86 + yel * .14
    yy, xx = np.mgrid[0:h, 0:w].astype(np.float32)
    a[..., :3] += (np.exp(-(((xx - yy * .7) - w * .35) / (w * .12)) ** 2) * 40)[..., None]
    sc = scratches(h, w, 260, 170, 6, w * .5) * .45 + scratches(h, w, 40, 220, w * .2, w * .9) * .3
    a[..., :3] = a[..., :3] * (1 - sc[..., None]) + 250 * sc[..., None]
    scuff = blobs(h, w, 14, 10, 40, 10) * .18
    a[..., :3] = a[..., :3] * (1 - scuff[..., None]) + 230 * scuff[..., None]
    for x0 in (inset + 4, inset + 12, inset + 20):
        a[inset:inset + ah, x0:x0 + 2, :3] *= .75
    de = np.minimum(np.minimum(xx, yy), np.minimum(w - 1 - xx, h - 1 - yy))
    edge = np.clip(1 - de / 6, 0, 1)
    a[..., :3] = a[..., :3] * (1 - edge[..., None] * .5) + 255 * edge[..., None] * .3
    a[..., :3] += noise(h, w, 5)[..., None]
    return img(a, "RGBA")


def cassette(w, h):
    a = np.zeros((h, w, 4), np.float32)
    a[..., 3] = 255
    a[..., :3] = 34 + noise(h, w, 3)[..., None]
    a[..., :3] += (blobs(h, w, 20, 10, 40, 12) * 12)[..., None]
    sc = scratches(h, w, 120, 120, 6, 80) * .35
    a[..., :3] = a[..., :3] * (1 - sc[..., None]) + 180 * sc[..., None]
    im = img(a, "RGBA")
    d = ImageDraw.Draw(im)
    wx0, wy0, wx1, wy1 = w * .22, h * .5, w * .78, h * .78
    d.rounded_rectangle((wx0, wy0, wx1, wy1), radius=int(h * .14), fill=(12, 12, 13, 255))
    cy = (wy0 + wy1) / 2
    for cx, rr in ((w * .33, h * .17), (w * .67, h * .1)):
        d.ellipse((cx - rr, cy - rr, cx + rr, cy + rr), fill=(78, 54, 38, 255))
        hr = h * .055
        d.ellipse((cx - hr, cy - hr, cx + hr, cy + hr), fill=(222, 218, 206, 255))
        for k in range(6):
            t = k * np.pi / 3
            d.line((cx, cy, cx + np.cos(t) * hr * .75, cy + np.sin(t) * hr * .75), fill=(60, 60, 60, 255), width=2)
    for sx, sy in ((14, 14), (w - 22, 14), (14, h - 22), (w - 22, h - 22), (w / 2 - 4, h - 22)):
        d.ellipse((sx, sy, sx + 8, sy + 8), fill=(80, 80, 82, 255))
    d.polygon([(w * .2, h), (w * .26, h * .86), (w * .74, h * .86), (w * .8, h)], fill=(26, 26, 27, 255))
    m = Image.new("L", (w, h), 0)
    ImageDraw.Draw(m).rounded_rectangle((0, 0, w - 1, h - 1), radius=14, fill=255)
    im.putalpha(m)
    return im


def tape(w, h, col=(226, 212, 172)):
    a = np.ones((h, w, 3), np.float32) * np.array(col, np.float32)
    a += noise(h, w, 5)[..., None]
    cr = arr(Image.fromarray((noise(h, w) * 60 + 128).clip(0, 255).astype(np.uint8)).resize((w // 6, h // 2)).resize((w, h), Image.BICUBIC))
    a += ((cr - 128) * .12)[..., None]
    al = np.ones((h, w), np.float32) * 235
    for side in (0, 1):
        edge = (np.cumsum(rng.normal(0, 1.2, h)) % 10 + 2).astype(int)
        for y in range(h):
            if side == 0:
                al[y, :edge[y]] = 0
            else:
                al[y, w - edge[y]:] = 0
    return img(np.dstack([a, al]), "RGBA")


def metal(w, h, base=128):
    rows = arr(Image.fromarray((noise(h, 1) * 50 + 128).clip(0, 255).astype(np.uint8)).resize((w, h)))
    a = base + (rows - 128) * .25 + noise(h, w, 4)
    grime = blobs(h, w, 30, 30, 120, 40) * 35 + blobs(h, w, 60, 4, 16, 6) * 25
    yy, xx = np.mgrid[0:h, 0:w].astype(np.float32)
    de = np.minimum(np.minimum(xx, yy), np.minimum(w - 1 - xx, h - 1 - yy))
    a -= grime + np.clip(40 - de, 0, 40) * .9
    a += scratches(h, w, 140, 200, 10, 120) * 60
    return img(np.stack([a, a * .99, a * .96], -1))


def photo(src, size, border=16):
    art = worn(src, size, fade=.2, ring=False, crease=True, paper=(236, 226, 204), edge=.6)
    A = arr(art) * np.array([1.04, 1.0, .9])
    s = size + border * 2
    out = np.ones((s, s, 3), np.float32) * np.array([236, 230, 214], np.float32)
    out += noise(s, s, 4)[..., None]
    out[border:border + size, border:border + size] = A
    return img(out)


def vu(w, h):
    a = np.ones((h, w, 3), np.float32) * np.array([226, 210, 160], np.float32)
    a += noise(h, w, 4)[..., None]
    a -= (blobs(h, w, 12, 20, 60, 20) * 22)[..., None]
    yy, xx = np.mgrid[0:h, 0:w].astype(np.float32)
    v = ((xx - w / 2) / (w * .7)) ** 2 + ((yy - h * .6) / (h * .9)) ** 2
    a *= np.clip(1 - v * .35, .6, 1)[..., None]
    im = img(a)
    d = ImageDraw.Draw(im)
    cx, cy, R = w / 2, h * 1.05, h * .82
    for i in range(21):
        t = np.radians(-48 + i * 4.8)
        r1 = R - (14 if i % 5 == 0 else 8)
        col = (170, 40, 30) if i > 14 else (40, 34, 28)
        d.line((cx + np.sin(t) * r1, cy - np.cos(t) * r1, cx + np.sin(t) * R, cy - np.cos(t) * R), fill=col, width=3 if i % 5 == 0 else 2)
    t = np.radians(10)
    d.line((cx, cy, cx + np.sin(t) * (R + 4), cy - np.cos(t) * (R + 4)), fill=(20, 18, 16), width=3)
    g = arr(im)
    glare = np.exp(-((yy - h * .18) / (h * .12)) ** 2) * 26
    return img(g + glare[..., None])


overlay(720, 1040).save(O + "overlay.png")
worn("ember.png", 480).save(O + "sleeve-ember.png")
record(480, "ember.png").save(O + "record-ember.png")
plinth(720, 1040).save(O + "plinth.png")
card(656, 240).save(O + "card.png")
case(440, 300, "tide.png").save(O + "case-tide.png")
cassette(560, 350).save(O + "cassette.png")
tape(520, 96).save(O + "tape-long.png")
tape(150, 52).save(O + "tape-short.png")
metal(720, 1040).save(O + "metal.png")
photo("moss.png", 320).save(O + "photo-moss.png")
vu(280, 170).save(O + "vu.png")
print("done")
