#!/usr/bin/env python3
"""Bake the pixel fonts in tools/fonts/*.ttf and *.otf into glyph atlases.

Run from the repo root:  python3 -I tools/bake_font.py
Offline and deterministic: reads only tools/fonts/*.ttf and *.otf, never downloads.

Outputs
  app/src/main/res/drawable-nodpi/font_display.png  (Press Start 2P, 8 px)
  app/src/main/res/raw/font_display.txt
  app/src/main/res/drawable-nodpi/font_body.png     (Ark Pixel, 10 px)
  app/src/main/res/raw/font_body.txt
  app/src/main/res/drawable-nodpi/font_numerals.png (Micro 5, 11 px, only 0 to 9 and "+":
  app/src/main/res/raw/font_numerals.txt             the 5 px tall numerals of the turn strip's flags)
  app/src/main/assets/fonts/*.ttf, *.otf         (copies of the display and body font files)

Atlas PNG: RGBA, every pixel is (255,255,255,255) or (255,255,255,0): all white,
alpha only ever 0 or 255 (asserted). Tint it at draw time (e.g. a ColorFilter or Paint colour
with SRC_IN). Load with inScaled=false, no filtering (nearest).

Metrics text file, UTF-8, '\n' line endings, integers separated by single spaces:
  line 1:  height baseline
  line n:  codepoint x y w h advance        (one line per glyph, ascending codepoint)
Meaning, all in atlas pixels (1 unit = 1 unscaled font pixel):
  height    line box height. Every glyph cell is exactly this tall (h == height).
  baseline  distance from the TOP of the line box down to the baseline row. A
            glyph's pixels already sit at the right vertical place inside its
            cell, so you never need the baseline to position glyphs; it is there
            for aligning mixed fonts or underlines.
  codepoint Unicode code point in decimal (32..126, plus the accented letters and
            punctuation of the shipped languages: see CODEPOINTS).
  x y w h   source rectangle in the atlas: left, top, width, height. The cell
            starts at the pen position, so the left bearing is already baked in
            and w may be larger than advance for glyphs that overhang.
  advance   whole-pixel pen advance, the font's own advance rounded to the nearest
            pixel, widened if needed to leave one clear pixel after the ink; it does NOT
            include any extra letter spacing; add spacing yourself if wanted.
Drawing a string: the draw position (X, Y) is the TOP-LEFT of the line box. For
each character, blit atlas rect (x, y, w, h) to (penX, Y) at the same size (or
scaled by an integer), then penX += advance. No kerning. Missing codepoints
should be skipped or drawn as '?'. Cells are packed in rows of 16 glyphs with a
1 px gap right and below each cell.
"""
import os
import shutil
import sys

from PIL import Image, ImageDraw, ImageFont

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
FONTS = os.path.join(ROOT, "tools", "fonts")
PNG_DIR = os.path.join(ROOT, "app", "src", "main", "res", "drawable-nodpi")
RAW_DIR = os.path.join(ROOT, "app", "src", "main", "res", "raw")
ASSET_DIR = os.path.join(ROOT, "app", "src", "main", "assets", "fonts")

# ASCII, then what each shipped language needs. A missing glyph draws as '?', so the lists
# hold each language's whole accented alphabet, not only the letters today's strings use;
# LocalizationTest fails if a string uses a character that is not baked.
NBSP = "\u00a0"  # French puts a no-break space before ? ! : ;
GERMAN = "äöüßÄÖÜ"
SPANISH = "áéíóúüñÁÉÍÓÚÜÑ¡¿"
FRENCH = "àâçèéêëîïôùûüÿœæÀÂÇÈÉÊËÎÏÔÙÛÜŸŒÆ«»" + NBSP
PORTUGUESE = "áàâãçéêíóôõúÁÀÂÃÇÉÊÍÓÔÕÚ"
ITALIAN = "àèéìòùÀÈÉÌÒÙ"
CODEPOINTS = sorted(set(range(32, 127)) | {ord(c) for c in GERMAN + SPANISH + FRENCH + PORTUGUESE + ITALIAN})
PER_ROW = 16
GAP = 1

# The numerals font: Ark Pixel's 7 px tall digits do not fit the 13 px flag of the turn
# strip beside its "Turns" label, so those numbers keep Micro 5's 5 px tall ones.
NUMERALS = sorted(ord(c) for c in "0123456789+ ")

# name, font file, native size, supersample factor (1 = render at native size),
# proportional (fractional advances: round, and keep one clear pixel after the ink),
# codepoints baked, and whether the font file is copied into the app's assets (the
# display and body fonts are also loaded as typefaces by the dialogs)
FONT_SPECS = [
    ("display", "PressStart2P-Regular.ttf", 8, 1, False, CODEPOINTS, True),
    ("body", "ArkPixel10-Regular.otf", 10, 1, True, CODEPOINTS, True),
    ("numerals", "Micro5-Regular.ttf", 11, 1, True, NUMERALS, False),
]


def render_glyph(font, ch, scale, cw, height):
    """Render ch into a cw x height white/alpha cell, 1-bit, at pen x = 0."""
    img = Image.new("L", (cw * scale, height * scale), 0)
    d = ImageDraw.Draw(img)
    d.fontmode = "1"
    d.text((0, 0), ch, font=font, fill=255, anchor="la")
    if scale > 1:
        img = img.resize((cw, height), Image.NEAREST)
    return img


def bake(name, ttf, size, scale, proportional, codepoints, copy_to_assets):
    path = os.path.join(FONTS, ttf)
    font = ImageFont.truetype(path, size * scale)
    ascent, descent = font.getmetrics()
    height = -(-(ascent + descent) // scale)
    baseline = -(-ascent // scale)
    cells = []
    for cp in codepoints:
        ch = chr(cp)
        bb = font.getbbox(ch, anchor="la")
        right = -(-bb[2] // scale)
        adv = int(round(font.getlength(ch) / scale))
        if proportional and ch not in (" ", NBSP):
            # The body font is proportional; keep at least
            # one clear pixel after the ink so neighbours never touch.
            adv = max(adv, right + 1)
        cw = max(adv, right, 1)
        cells.append((cp, adv, render_glyph(font, ch, scale, cw, height)))

    rows = [cells[i:i + PER_ROW] for i in range(0, len(cells), PER_ROW)]
    width = max(sum(c[2].width + GAP for c in r) for r in rows)
    atlas = Image.new("RGBA", (width, len(rows) * (height + GAP)), (255, 255, 255, 0))
    lines = ["%d %d" % (height, baseline)]
    for ri, row in enumerate(rows):
        x, y = 0, ri * (height + GAP)
        for cp, adv, cell in row:
            solid = cell.point(lambda v: 255 if v >= 128 else 0)
            white = Image.new("RGBA", cell.size, (255, 255, 255, 255))
            atlas.paste(white, (x, y), solid)
            lines.append("%d %d %d %d %d %d" % (cp, x, y, cell.width, cell.height, adv))
            x += cell.width + GAP

    alphas = set(atlas.getchannel("A").getdata())
    assert alphas <= {0, 255}, "grey pixels in %s: %s" % (name, sorted(alphas))
    for px in set(atlas.getdata()):
        assert px[:3] == (255, 255, 255), "non-white colour in %s: %s" % (name, px)

    atlas.save(os.path.join(PNG_DIR, "font_%s.png" % name))
    with open(os.path.join(RAW_DIR, "font_%s.txt" % name), "w", encoding="utf-8", newline="\n") as f:
        f.write("\n".join(lines) + "\n")
    if copy_to_assets:
        shutil.copyfile(path, os.path.join(ASSET_DIR, ttf))
    digits = sorted({dict((c[0], c[1]) for c in cells)[ord(d)] for d in "0123456789"})
    print("%s: %dx%d atlas, height %d baseline %d, digit advances %s" % (
        name, atlas.width, atlas.height, height, baseline, digits))


PREVIEW_TEXT = "Turn 15, 3rd Roll  Score 42  Finish  \u00c4\u00d6\u00dc\u00e4\u00f6\u00fc\u00df 0123456789"


def preview(names):
    """tools/preview_font.png: text drawn from the atlas via the metrics only."""
    strips = []
    for name in names:
        atlas = Image.open(os.path.join(PNG_DIR, "font_%s.png" % name))
        with open(os.path.join(RAW_DIR, "font_%s.txt" % name), encoding="utf-8") as f:
            rows = [list(map(int, ln.split())) for ln in f.read().splitlines()]
        height = rows[0][0]
        glyphs = {r[0]: r[1:] for r in rows[1:]}
        width = sum(glyphs[ord(c)][4] for c in PREVIEW_TEXT) + 4
        strip = Image.new("RGBA", (width, height + 4), (40, 30, 50, 255))
        pen = 2
        for c in PREVIEW_TEXT:
            x, y, w, h, adv = glyphs[ord(c)]
            cell = atlas.crop((x, y, x + w, y + h))
            strip.alpha_composite(cell, (pen, 2))
            pen += adv
        strips.append(strip)
    w = max(s.width for s in strips)
    out = Image.new("RGBA", (w, sum(s.height for s in strips)), (40, 30, 50, 255))
    y = 0
    for s in strips:
        out.paste(s, (0, y))
        y += s.height
    out = out.resize((out.width * 4, out.height * 4), Image.NEAREST)
    out.save(os.path.join(ROOT, "tools", "preview_font.png"))


def main():
    for d in (PNG_DIR, RAW_DIR, ASSET_DIR):
        os.makedirs(d, exist_ok=True)
    for spec in FONT_SPECS:
        bake(*spec)
    preview([spec[0] for spec in FONT_SPECS if spec[5] is CODEPOINTS])


if __name__ == "__main__":
    sys.exit(main())
