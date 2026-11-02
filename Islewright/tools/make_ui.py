#!/usr/bin/env python3
"""Generate every non-terrain UI PNG for Islewright, and extract a few Wesnoth sprites.

Run from the repository root as:  python3 -I tools/make_ui.py

Environment: WESNOTH_DIR (default: a wesnoth checkout beside this repository), read only.

Everything here is drawn at 1:1 art pixels, no antialiasing, in tiny fixed
palettes. The app scales the whole art bitmap by an integer factor with
nearest neighbour, so these sprites sit beside Wesnoth's 72 px terrain.

Outputs go to app/src/main/res/drawable-nodpi/. Side files:
  tools/pieces.txt              anchors for the cropped Wesnoth pieces and the road sprites
  tools/wesnoth-sources-ui.txt  Wesnoth commit plus every Wesnoth file read
  tools/preview_ui.png          contact sheet (untracked, never git add)
"""
import hashlib
import math
import io
import os
import subprocess
import sys

from PIL import Image, ImageDraw, ImageFont

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "app/src/main/res/drawable-nodpi")
WESNOTH = os.environ.get("WESNOTH_DIR", os.path.join(os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))), "wesnoth"))
IMG = os.path.join(WESNOTH, "data/core/images")

TEAM_MAGENTA = ["F49AC1", "3F0016", "55002A", "690039", "7B0045", "8C0051", "9E005D",
                "B10069", "C30074", "D6007F", "EC008C", "EE3D96", "EF5BA1", "F172AC",
                "F287B6", "F6ADCD", "F8C1D9", "FAD5E5", "FDE9F1"]

FLAG_GREEN = ["00C800"] + ["00%02X00" % v for v in range(0xFF, -1, -1) if v != 0xC8]
ANCHORS = {}
outputs = {}        # file name -> (Image, limited?)
wesnoth_read = []   # paths relative to WESNOTH_DIR


def rgb(h):
    return (int(h[0:2], 16), int(h[2:4], 16), int(h[4:6], 16), 255)


def art(rows, legend, size=None):
    """Stamp an ASCII picture. '.' is transparent. Every row must be the same width."""
    w = len(rows[0])
    h = len(rows)
    if size is not None:
        assert (w, h) == size, "art is %dx%d, wanted %dx%d" % (w, h, size[0], size[1])
    im = Image.new("RGBA", (w, h), (0, 0, 0, 0))
    px = im.load()
    for y, row in enumerate(rows):
        assert len(row) == w, "row %d is %d wide, expected %d: %r" % (y, len(row), w, row)
        for x, ch in enumerate(row):
            if ch != ".":
                px[x, y] = rgb(legend[ch])
    return im


def put(name, im, limited=True):
    outputs[name] = (im, limited)


# ---------------------------------------------------------------- buttons and panel

OUTLINE = "1E140A"
FACE = "A0783C"
FACE_PRESSED = "7A5A2C"
FACE_DISABLED = "6E6A60"
BEVEL_LIGHT = "E0B870"
BEVEL_LIGHT_DISABLED = "8C8880"
BEVEL_DARK = "5A3C14"
TEXT = "FFF4D6"

# 6x6 corners; edge cross-sections are read off the corner's last row / column.
# O outline, L bevel light, F face, D bevel dark, W rivet gleam, J field top-left edge
# (lighter), K field bottom-right edge (darker). Light comes from the top left.
BTN_TL = ["..OOOO", ".OLLLL", "OLWLFF", "OLLDDD", "OLFDJJ", "OLFDJF"]
BTN_TR = ["OOOO..", "LLLOO.", "FFWLDO", "DDLDDO", "JKLFDO", "FKLFDO"]
BTN_BL = ["OLFDJF", "OLFDJK", "OLWLLL", "OLLDFF", ".ODDDD", "..OOOO"]
BTN_BR = ["FKLFDO", "KKLFDO", "LLWLDO", "FFLDDO", "DDDOO.", "OOOO.."]


def nine_from_corners(tl, tr, bl, br, legend, size=24, b=6):
    """Build a size x size 9-slice. Edges repeat the corner's last row / column,
    so they are uniform along their length and stretch cleanly."""
    im = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    ctl, ctr, cbl, cbr = (art(c, legend) for c in (tl, tr, bl, br))
    im.paste(ctl, (0, 0))
    im.paste(ctr, (size - b, 0))
    im.paste(cbl, (0, size - b))
    im.paste(cbr, (size - b, size - b))
    px = im.load()
    for x in range(b, size - b):
        for y in range(b):
            px[x, y] = ctl.getpixel((b - 1, y))
            px[x, size - b + y] = cbl.getpixel((b - 1, y))
    for y in range(b, size - b):
        for x in range(b):
            px[x, y] = ctl.getpixel((x, b - 1))
            px[size - b + x, y] = ctr.getpixel((x, b - 1))
    centre = ctl.getpixel((b - 1, b - 1))
    for y in range(b, size - b):
        for x in range(b, size - b):
            px[x, y] = centre
    return im


def make_buttons():
    states = {
        "ui_button_normal.png": dict(O=OUTLINE, L=BEVEL_LIGHT, F=FACE, D=BEVEL_DARK, W=TEXT,
                                     J="B48C4C", K="86622E"),
        # pressed: no light bevel, darker face, rivets dulled, inner edges swapped so the
        # field reads as sunk
        "ui_button_pressed.png": dict(O=OUTLINE, L=FACE_PRESSED, F=FACE_PRESSED, D=BEVEL_DARK,
                                      W=BEVEL_DARK, J=BEVEL_DARK, K=FACE_PRESSED),
        "ui_button_disabled.png": dict(O=OUTLINE, L=BEVEL_LIGHT_DISABLED, F=FACE_DISABLED,
                                       D=BEVEL_DARK, W=BEVEL_LIGHT_DISABLED, J="7C7870",
                                       K="5E5A52"),
    }
    for name, legend in states.items():
        put(name, nine_from_corners(BTN_TL, BTN_TR, BTN_BL, BTN_BR, legend))


# O dark outline, K dark brown, B brass line, S parchment shade, P parchment,
# W stud gleam, A stud brass.
PAN_TL = ["..OOOO", ".ODDDD", "ODBBBB", "ODBWAS", "ODBAKP", "ODBSPP"]
PAN_TR = ["OOOO..", "DDDDO.", "BBBBDO", "SWABDO", "PAKBDO", "PPPBDO"]
PAN_BL = ["ODBSPP", "ODBWAP", "ODBAKP", "ODBBBB", ".ODDDD", "..OOOO"]
PAN_BR = ["PPPBDO", "PWABDO", "PAKBDO", "BBBBDO", "DDDDO.", "OOOO.."]


def make_panel():
    legend = dict(O=OUTLINE, D=BEVEL_DARK, B=BEVEL_LIGHT, S="D8C494", P="F0E0B8",
                  W=TEXT, A=FACE, K=BEVEL_DARK)
    im = nine_from_corners(PAN_TL, PAN_TR, PAN_BL, PAN_BR, legend)
    # the generic builder takes the centre from the TL corner's last pixel (P): good.
    put("ui_panel.png", im)


# Dark kit. Same 24x24 geometry and corner drawing as the light panel, in the HUD's dark wood.
# The face is one flat colour on purpose: the centre cell is stretched in both directions, and
# a grain of any kind would turn into thick bars. The grain lives in the 6 px frame instead,
# where the edges repeat the corner's last row and column.
DARK = dict(
    outline="120B05",       # frame outline, darker than the HUD so the panel keeps its edge
    rim="4E3722",           # outer wood rim, lighter than the face
    brass="B8914F",         # inner line, studs, rule lines
    shade="241A10",         # thin shadow just inside the brass line
    face="33261A",          # panel face
    gleam="F0D898",         # stud gleam
    stud_dark="5A3C14",
    ink="D2BE94",           # primary text
    ink2="B8A47C",          # secondary text
    disabled="8A7A5C",      # disabled text
    rule="B8914F",          # divider rules and the title rule
    field_face="1E150C",    # recessed field, slider track, radio box
    field_edge_dark="120B05",    # top and left edge of a recessed box
    field_edge_light="5A4630",   # bottom and right edge of a recessed box
    link="7DB8D8",          # links
    mark="E0B870",          # selection mark in a radio box, caret, slider thumb highlight
    thumb="B8914F",         # slider thumb face
    thumb_edge="E0B870",    # slider thumb top edge
)


def make_dark_panel():
    legend = dict(O=DARK["outline"], D=DARK["rim"], B=DARK["brass"], S=DARK["shade"],
                  P=DARK["face"], W=DARK["gleam"], A=DARK["brass"], K=DARK["stud_dark"])
    put("ui_panel_dark.png", nine_from_corners(PAN_TL, PAN_TR, PAN_BL, PAN_BR, legend))


# ---------------------------------------------------------------- dice

def rounded(draw, box, fill):
    x0, y0, x1, y1 = box
    draw.rectangle([x0, y0 + 1, x1, y1 - 1], fill=fill)
    draw.rectangle([x0 + 1, y0, x1 - 1, y1], fill=fill)


def make_dice():
    O, SH = rgb(OUTLINE), rgb("5A3C14")
    # The dark kit's accent (DARK["mark"], E0B870) with a highlight and a shade in proportion;
    # every icon was checked on it (the wheat sheaf's pale tips are the weakest, and darkening
    # the face only makes them worse).
    face, hi, sh = rgb("E0B870"), rgb("F0D090"), rgb("B8914F")
    # The held frame is in the team magenta, so it takes the player colour at draw time
    # F49AC1 is the reference shade, which becomes the player colour itself; EC008C is its shade
    amb, ambd = rgb("F49AC1"), rgb("EC008C")

    def body(held):
        im = Image.new("RGBA", (28, 28), (0, 0, 0, 0))
        d = ImageDraw.Draw(im)
        rounded(d, (1, 1, 27, 27), SH)                      # hard 1 px drop shadow
        rounded(d, (0, 0, 26, 26), O)
        rounded(d, (1, 1, 25, 25), face)
        d.line([(2, 1), (24, 1)], fill=hi)                  # top highlight
        d.line([(1, 2), (1, 24)], fill=hi)                  # left highlight
        d.line([(2, 25), (24, 25)], fill=sh)                # bottom shade
        d.line([(25, 2), (25, 24)], fill=sh)                # right shade
        if held:
            d.rectangle([1, 1, 25, 25], outline=amb)
            d.rectangle([2, 2, 24, 24], outline=ambd)
            for p in ((1, 1), (25, 1), (1, 25), (25, 25)):  # keep the rounded corners
                d.point(p, fill=O)
        return im

    put("die_face.png", body(False))
    put("die_face_held.png", body(True))

    im = Image.new("RGBA", (28, 28), (0, 0, 0, 0))
    d = ImageDraw.Draw(im)
    rounded(d, (0, 0, 26, 26), O)
    rounded(d, (1, 1, 25, 25), rgb("33261A"))
    d.line([(2, 25), (24, 25)], fill=rgb("5A4630"))
    d.line([(25, 2), (25, 24)], fill=rgb("5A4630"))
    d.line([(2, 2), (24, 2)], fill=rgb("1E140A"))
    d.line([(2, 2), (2, 24)], fill=rgb("1E140A"))
    put("die_slot_empty.png", im)


# ---------------------------------------------------------------- roads

# F49AC1 is the reference shade, which becomes the player colour itself: the kerb's body.
# Lighter and darker key shades light and shade it.
# On the board the kerb is a little darker than the colour itself (F172AC maps to about 89%
# of it), so it holds its own against the bright terrain; EC008C still marks the joints.
KERB_LIT, KERB_MID, KERB_DARK = "F172AC", "F172AC", "EC008C"
COBBLE = ["8A7A62", "B09E7C", "D0C29E"]
MORTAR = "4E3E2C"


def cobble_pick(i, j):
    return COBBLE[((i * 7 + j * 13 + (i * j) % 5) % 3)]


def make_road(w, h, p0, p1):
    """Rasterise a road of half width 3.6 along p0 -> p1 (pixel-centre distance, hard edges).
    Outer 2 px each side are kerb stones in the three team-colour shades; the 4 px between
    are cobbles. Round end caps overlap neatly at hex corners."""
    im = Image.new("RGBA", (w, h), (0, 0, 0, 0))
    px = im.load()
    (x0, y0), (x1, y1) = p0, p1
    dx, dy = x1 - x0, y1 - y0
    length = math.hypot(dx, dy)
    ux, uy = dx / length, dy / length
    nx, ny = -uy, ux
    R = 3.6
    for y in range(h):
        for x in range(w):
            cx, cy = x + 0.5 - x0, y + 0.5 - y0
            s = cx * ux + cy * uy
            d = cx * nx + cy * ny
            ss = min(max(s, 0), length)
            dist = math.hypot(cx - ss * ux, cy - ss * uy)
            if dist > R:
                continue
            ad = abs(d)
            sa = int(math.floor(s + 0.5 + 100))
            if s < 0 or s > length:
                # cap region: kerb colour ring by distance, cobbles inside
                if dist > 2.6:
                    c = KERB_MID if dist <= 3.3 else KERB_DARK
                elif dist > 1.9:
                    c = KERB_LIT
                else:
                    c = COBBLE[1]
            elif ad > 2.9:
                c = KERB_DARK if sa % 6 == 0 else KERB_MID
            elif ad > 1.9:
                c = KERB_DARK if sa % 6 == 0 else KERB_LIT
            else:
                cell = int(math.floor((s + (1.5 if d > 0 else 0)) / 3.0))
                row = 0 if d < 0 else 1
                if int(math.floor(s + 100)) % 3 == 0 and False:
                    c = MORTAR
                else:
                    c = cobble_pick(cell, row)
                    if int(math.floor(s * 1.0 + (1.5 if row else 0.0) + 300)) % 3 == 0:
                        c = MORTAR
            px[x, y] = rgb(c)
    return im


def make_roads():
    put("road_h.png", make_road(44, 8, (4, 4), (40, 4)))
    put("road_slash.png", make_road(26, 44, (4, 40), (22, 4)))   # "/"
    put("road_backslash.png", make_road(26, 44, (4, 4), (22, 40)))   # "\"


# ---------------------------------------------------------------- resource icons
# Each icon carries its own small palette (outline plus three tones).

LUMBER = dict(o="2A1A0E", d="5C3A1E", m="8B5A2B", l="C8955A")
LOG16 = [".oooooooooooooo.",
         "ollldllllllllllo",
         "olmldmmmmmmmmmmo",
         "olmldmmmmmmmmmmo",
         "ollldddddddddddo",
         ".oooooooooooooo."]
LOG12 = [".oooooooooo.",
         "olldlllllllo",
         "olmdmmmmmmmo",
         "olldddddddo".ljust(12, "o")[:11] + "o",
         ".oooooooooo."]
LOG12[3] = "olld" + "ddddddd" + "o"

GRAIN = dict(o="4A3000", d="B8860B", m="E6B422", l="FFE08A", r="C8502D")
# palette for the sack candidate only (never written out)
GRAIN_SACK = dict(GRAIN, k="F0E0B8", s="C9A66B", t="8B6B3A")


class PixelCanvas:
    """Fill pixels get a 1 px outline (4-neighbourhood); thin pixels (awns, stalks) do not."""

    def __init__(self, n, pal=GRAIN):
        self.n, self.pal, self.fill, self.thin = n, pal, {}, {}

    def f(self, x, y, c):
        self.fill[(x, y)] = c

    def t(self, x, y, c):
        self.thin[(x, y)] = c

    def line(self, p0, p1, c):
        n = max(abs(p1[0] - p0[0]), abs(p1[1] - p0[1]), 1)
        for i in range(n + 1):
            self.t(int(round(p0[0] + (p1[0] - p0[0]) * i / float(n))),
                   int(round(p0[1] + (p1[1] - p0[1]) * i / float(n))), c)

    def render(self):
        im = Image.new("RGBA", (self.n, self.n), (0, 0, 0, 0))

        def put(x, y, c):
            if 0 <= x < self.n and 0 <= y < self.n:
                im.putpixel((x, y), rgb(self.pal[c]))
        for (x, y) in self.fill:
            for dx, dy in ((-1, 0), (1, 0), (0, -1), (0, 1)):
                if (x + dx, y + dy) not in self.fill:
                    put(x + dx, y + dy, "o")
        for (x, y), c in self.thin.items():
            if (x, y) not in self.fill:
                put(x, y, c)
        for (x, y), c in self.fill.items():
            put(x, y, c)
        return im


def wheat_ear(cv, base, sx, length, kfrom, awns=3, every=1):
    """A wheat ear on a stem running from base up and to the right (sx=1), up and to the left
    (sx=-1) or straight up (sx=0). From stem pixel kfrom the stem carries kernel pairs,
    alternating so the ear shows the herringbone pattern: light kernels on the left, mid on the
    right, a dark shade pixel under each right kernel, and awns fanning out at the tip."""
    x0, y0 = base
    # every > 1: the stem moves one pixel sideways only every `every` rows, so the ear stands
    # nearly upright and leans out by a pixel or two over its whole length
    stem = [(x0 + sx * (i // every), y0 - i) for i in range(length)]
    if every > 1:
        sx_k, sx_a = 0, sx
    else:
        sx_k = sx_a = sx
    for (x, y) in stem:
        cv.f(x, y, "d")
    sx = sx_k
    for i, (x, y) in enumerate(stem):
        if i < kfrom:
            continue
        k = i - kfrom
        if sx == 1 and k % 2 == 0:
            cv.f(x - 1, y, "l"); cv.f(x - 1, y - 1, "l"); cv.f(x + 1, y, "m"); cv.f(x + 1, y + 1, "d")
        elif sx == -1 and k % 2 == 0:
            cv.f(x - 1, y, "l"); cv.f(x - 1, y + 1, "l"); cv.f(x + 1, y, "m"); cv.f(x + 1, y - 1, "d")
        elif sx == 0:
            if k % 2 == 0:
                cv.f(x - 1, y, "l"); cv.f(x - 1, y - 1, "l")
            else:
                cv.f(x + 1, y, "m"); cv.f(x + 1, y - 1, "d")
    tx, ty = stem[-1]
    cv.f(tx, ty - 1, "l")
    sx = sx_a
    for j in range(awns):
        cv.t(tx + sx * (j + 1) + (1 if j % 2 else 0), ty - 2 - j, "d")
        if sx != 0:
            cv.t(tx + sx * j, ty - 2 - j, "d")
        else:
            cv.t(tx - 1 - (j % 2), ty - 2 - j, "d")


def sheaf_v1(n):
    """(previous version, kept for the preview only) A bound sheaf: five herringbone ears splayed at ragged heights, a red band, a skirt
    of cut stalks with uneven ends."""
    cv = PixelCanvas(n)
    if n == 16:
        wheat_ear(cv, (8, 10), 0, 8, 2, 2)
        wheat_ear(cv, (7, 10), -1, 8, 3, 2)
        wheat_ear(cv, (9, 10), 1, 7, 3, 2)
        wheat_ear(cv, (5, 10), -1, 5, 2, 1)
        wheat_ear(cv, (11, 10), 1, 5, 2, 1)
        for x in range(5, 12):
            cv.f(x, 10, "r"); cv.f(x, 11, "r")
        for x, y in ((4, 15), (5, 14), (6, 15), (7, 14), (8, 15), (9, 15), (10, 14), (11, 15), (12, 14)):
            cv.line((x, 12), (x, y), "m" if x % 2 else "d")
        for p in ((3, 14), (4, 13), (13, 13), (12, 13)):
            cv.t(p[0], p[1], "d")
    else:
        wheat_ear(cv, (6, 8), 0, 6, 1, 1)
        wheat_ear(cv, (5, 8), -1, 5, 2, 1)
        wheat_ear(cv, (7, 8), 1, 5, 2, 1)
        for x in range(4, 9):
            cv.f(x, 8, "r")
        for x, y in ((3, 11), (4, 10), (5, 11), (6, 10), (7, 11), (8, 10), (9, 11)):
            cv.line((x, 9), (x, y), "m" if x % 2 else "d")
    return cv.render()


def sheaf(n):
    """(c) A bound sheaf: five herringbone ears rising nearly upright from the band at ragged
    heights, the outer ones leaning out by only a pixel or two, a red band, a skirt of cut stalks
    with uneven ends. Taller than wide."""
    cv = PixelCanvas(n)
    if n == 16:
        wheat_ear(cv, (8, 10), 0, 9, 2, 2)
        wheat_ear(cv, (6, 10), -1, 7, 2, 2, every=4)
        wheat_ear(cv, (10, 10), 1, 8, 2, 2, every=4)
        wheat_ear(cv, (4, 10), -1, 5, 2, 1, every=4)
        wheat_ear(cv, (12, 10), 1, 5, 2, 1, every=4)
        for x in range(4, 13):
            cv.f(x, 10, "r"); cv.f(x, 11, "r")
        for x, y in ((4, 15), (5, 14), (6, 15), (7, 14), (8, 15), (9, 15), (10, 14), (11, 15), (12, 14)):
            cv.line((x, 12), (x, y), "m" if x % 2 else "d")
        for p in ((3, 14), (4, 13), (13, 13), (12, 13)):
            cv.t(p[0], p[1], "d")
    else:
        wheat_ear(cv, (6, 8), 0, 7, 1, 1)
        wheat_ear(cv, (3, 8), -1, 5, 1, 1, every=3)
        wheat_ear(cv, (9, 8), 1, 6, 2, 1, every=3)
        for x in range(3, 10):
            cv.f(x, 8, "r")
        for x, y in ((3, 11), (4, 10), (5, 11), (6, 10), (7, 11), (8, 10), (9, 11)):
            cv.line((x, 9), (x, y), "m" if x % 2 else "d")
    return cv.render()


def single_ear(n):
    """(a) One long ear on a diagonal stalk with a leaf."""
    cv = PixelCanvas(n)
    if n == 16:
        wheat_ear(cv, (2, 15), 1, 12, 4)
        cv.line((5, 12), (1, 9), "m"); cv.line((4, 13), (0, 12), "d")
    else:
        wheat_ear(cv, (1, 11), 1, 9, 3, 2)
        cv.line((4, 8), (1, 6), "m")
    return cv.render()


def crossed_ears(n):
    """(b) Two ears crossing in an X low down, tied."""
    cv = PixelCanvas(n)
    if n == 16:
        wheat_ear(cv, (3, 15), 1, 12, 5)
        wheat_ear(cv, (12, 15), -1, 12, 5)
        for x in range(6, 10):
            cv.f(x, 13, "r"); cv.f(x, 14, "r")
    else:
        wheat_ear(cv, (2, 11), 1, 9, 4, 2)
        wheat_ear(cv, (9, 11), -1, 9, 4, 2)
        for x in range(4, 8):
            cv.f(x, 10, "r")
    return cv.render()


def grain_sack(n):
    """(d) A sack of grain with kernels heaped in its neck."""
    cv = PixelCanvas(n, GRAIN_SACK)
    if n == 16:
        for y, (x0, x1) in {8: (4, 11), 9: (3, 12), 10: (2, 13), 11: (2, 13), 12: (2, 13),
                            13: (3, 12), 14: (4, 11)}.items():
            for x in range(x0, x1 + 1):
                cv.f(x, y, "k" if x < x0 + 3 else ("t" if x > x1 - 2 else "s"))
        for x in range(5, 11):
            cv.f(x, 8, "r")
        for (x, y, c) in ((6, 6, "l"), (8, 6, "m"), (5, 5, "m"), (7, 5, "l"), (9, 5, "d"),
                          (6, 4, "l"), (8, 4, "m"), (7, 3, "l"), (8, 2, "m")):
            cv.f(x, y, c)
    else:
        for y, (x0, x1) in {6: (3, 8), 7: (2, 9), 8: (1, 10), 9: (1, 10), 10: (2, 9)}.items():
            for x in range(x0, x1 + 1):
                cv.f(x, y, "k" if x < x0 + 2 else ("t" if x > x1 - 1 else "s"))
        for x in range(3, 9):
            cv.f(x, 6, "r")
        for (x, y, c) in ((4, 4, "l"), (6, 4, "m"), (5, 3, "l"), (6, 2, "m")):
            cv.f(x, y, c)
    return cv.render()


GRAIN_CANDIDATES = [("a single ear", single_ear), ("b crossed ears", crossed_ears),
                    ("c sheaf, previous", sheaf_v1), ("c sheaf, pulled in (the old icon)", sheaf),
                    ("d grain sack", grain_sack)]


def make_grain16():
    return sheaf(16)


def make_grain12():
    return sheaf(12)


WOOL = dict(o="202020", w="F2F2F2", g="C8C8C8", k="202020")
WOOL16 = ["................",
          "................",
          "...oooo.oooo....",
          "..owwwwowwwwo.oo",
          ".owwwwwwwwwwoooo",
          "owwwgwwwwgwwkkwk",
          "owwwwwwwwwwwkkkk",
          "owgwwwwwgwwwokkk",
          "owwwwwwwwgwwo.kk",
          ".owwgwwwwwwwo..k",
          "..ogwwwwwwwwo...",
          "...oooooooooo...",
          "...kk.kk.kk.kk..",
          "...kk.kk.kk.kk..",
          "................",
          "................"]
WOOL12 = ["............",
          "..ooo.ooo...",
          ".owwwowwwo.o",
          "owwwwwwwwooo",
          "owwgwwwwwkwk",
          "owwwwwwwwkkk",
          "owgwwwgwwok.",
          ".owwwwwwwo..",
          "..oooooooo..",
          "..k.k.k.k...",
          "..k.k.k.k...",
          "............"]

ORE = dict(o="202020", d="5A5A5A", m="8C8C8C", l="D0D0D0", w="F8F8F8")
ORE_A = ["..oooooo..",
         ".ollllmmo.",
         "ollllmmmdo",
         "olllmmmmdo",
         "ommmmmmddo",
         "ommmmmdddo",
         ".ommmdddo.",
         "..oooooo.."]
ORE_B = ["..oooo..",
         ".ollmmo.",
         "ollmmmdo",
         "ommmmddo",
         "ommmdddo",
         ".oddddo.",
         "..oooo.."]
ORE_C = ["..oooo..",
         ".ollmmo.",
         "olmmmddo",
         "ommmdddo",
         ".oddddo.",
         "..oooo.."]
ORE_A12 = ["..oooo..",
           ".ollmmo.",
           "ollmmmdo",
           "ommmmddo",
           ".ommddo.",
           "..oooo.."]
ORE_B12 = [".oooo.",
           "olmmdo",
           "ommddo",
           ".oooo."]
SPARK = [(1, 0), (0, 1), (1, 1), (2, 1), (1, 2)]


def make_ore(size):
    im = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    if size == 16:
        parts = [(ORE_A, 0, 2), (ORE_B, 8, 4), (ORE_C, 4, 9)]
        spark = (10, 0)
    else:
        parts = [(ORE_A12, 0, 1), (ORE_B12, 6, 3), (ORE_B12, 3, 6)]
        spark = (8, 0)
    for rows, x, y in parts:
        im.alpha_composite(art(rows, ORE), (x, y))
    for dx, dy in SPARK:
        im.putpixel((spark[0] + dx, spark[1] + dy), rgb(ORE["w"]))
    return im


BRICK = dict(o="3A1208", d="8C2D19", m="C8502D", l="E88A5A")


def make_brick_old(size):
    """(previous version, flat rectangles, kept for the preview only)"""
    im = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    d = ImageDraw.Draw(im)
    o, dk, m, l = (rgb(BRICK[k]) for k in "odml")
    if size == 16:
        spots = [(4, 2, 9, 5), (0, 7, 9, 5), (8, 7, 8, 5)]
    else:
        spots = [(3, 1, 7, 4), (0, 5, 7, 4), (6, 5, 6, 4)]
    for (x, y, w, h) in spots:
        d.rectangle([x, y, x + w - 1, y + h - 1], fill=o)
        d.rectangle([x + 1, y + 1, x + w - 2, y + h - 2], fill=m)
        d.line([(x + 1, y + 1), (x + w - 2, y + 1)], fill=l)
        d.line([(x + 1, y + h - 2), (x + w - 2, y + h - 2)], fill=dk)
    return im


GOLD = dict(o="3A2800", d="A07000", m="E0B000", l="FFF0A0", w="FFFFFF")


def ingot(d, x, y, fw, fh, top_w, dx, dy):
    """One gold bar in three-quarter view. The front face is a trapezoid (wider at the foot),
    the top face is light, the right face dark. (x, y) is the front's top left corner."""
    o, dk, m, l = (rgb(GOLD[k]) for k in "odml")
    x1 = x + top_w
    front = [(x, y), (x1, y), (x1 + 1, y + fh), (x - 1, y + fh)]
    top = [(x, y), (x1, y), (x1 + dx, y - dy), (x + dx, y - dy)]
    right = [(x1, y), (x1 + dx, y - dy), (x1 + dx + 1, y + fh - dy), (x1 + 1, y + fh)]
    d.polygon(right, fill=dk, outline=o)
    d.polygon(front, fill=m, outline=o)
    d.polygon(top, fill=l, outline=o)
    d.line([(x + 1, y + 1), (x1 - 1, y + 1)], fill=l)             # lit upper edge of the front
    d.line([(x, y + fh - 1), (x1, y + fh - 1)], fill=dk)          # shaded foot


def make_gold(size):
    im = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    d = ImageDraw.Draw(im)
    if size == 16:
        ingot(d, 2, 10, 8, 4, 8, 4, 3)       # lower bar
        ingot(d, 4, 5, 7, 4, 7, 4, 3)        # upper bar, set back and to the right
        im.putpixel((6, 3), rgb(GOLD["w"]))
        im.putpixel((7, 3), rgb(GOLD["w"]))
    else:
        ingot(d, 2, 6, 6, 4, 6, 3, 2)
        im.putpixel((4, 4), rgb(GOLD["w"]))
        im.putpixel((5, 4), rgb(GOLD["w"]))
    return im


ANY = dict(o="1A2250", d="3A54B0", m="6A8CE8", l="C0D0FF")
ANY16 = ["..####..",
         ".######.",
         "###..###",
         "###..###",
         ".....###",
         "....###.",
         "...###..",
         "...###..",
         "........",
         "...###..",
         "...###.."]
ANY12 = [".####.",
         "##..##",
         "....##",
         "...##.",
         "..##..",
         "..##..",
         "......",
         "..##.."]


def bevel_glyph(rows, size, pal, offset):
    """Fill shape ('#') with light top-left edges, dark bottom-right edges and a 1 px outline."""
    im = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    h, w = len(rows), len(rows[0])
    fill = set((x, y) for y, r in enumerate(rows) for x, ch in enumerate(r) if ch == "#")
    for (x, y) in fill:
        c = pal["m"]
        if (x, y - 1) not in fill or (x - 1, y) not in fill:
            c = pal["l"]
        elif (x, y + 1) not in fill or (x + 1, y) not in fill:
            c = pal["d"]
        im.putpixel((x + offset[0], y + offset[1]), rgb(c))
    for (x, y) in fill:
        for dx in (-1, 0, 1):
            for dy in (-1, 0, 1):
                n = (x + dx, y + dy)
                if n not in fill:
                    im.putpixel((n[0] + offset[0], n[1] + offset[1]), rgb(pal["o"]))
    return im


def compose_logs16(log):
    im = Image.new("RGBA", (16, 16), (0, 0, 0, 0))
    a = art(log, LUMBER)
    im.alpha_composite(a, (0, 2))
    im.alpha_composite(a.transpose(Image.FLIP_LEFT_RIGHT), (0, 8))
    return im


def compose_logs12(log):
    im = Image.new("RGBA", (12, 12), (0, 0, 0, 0))
    a = art(log, LUMBER)
    im.alpha_composite(a, (0, 1))
    im.alpha_composite(a.transpose(Image.FLIP_LEFT_RIGHT), (0, 6))
    return im


# ---- three-quarter view icons: lumber, brick, wool (the flat versions above stay for the preview)

LOGS = dict(o="2A1A0E", d="5C3A1E", m="8B5A2B", l="C8955A", e="E8C890", r="A87840", c="C49A5C")


def log_disc(n):
    """Round cut end of a log: rows of distance from the centre decide the tones. n is the
    diameter, 7 or 5 (outline included). Returns {(x, y): tone}."""
    c = (n - 1) / 2.0
    out = {}
    for j in range(n):
        for i in range(n):
            r = math.hypot(i - c, j - c)
            if r > c + 0.2:
                continue
            if r > c - 0.4:
                t = "o"
            elif n == 7 and r < 0.6:
                t = "d"
            elif n == 7 and r < 1.6:
                t = "r"
            elif n == 5 and r < 0.6:
                t = "d"
            else:
                t = "e" if (i + j) <= (n - 1) + 1 else "c"
            out[(i, j)] = t
    return out


def draw_log(img, x, y, n, dx, dy):
    """One log lying away from the viewer to the upper right: the cut end is a disc at (x, y)
    and the bark body is that disc swept by (dx, dy). Bark is lit on top, dark underneath."""
    disc = log_disc(n)
    c = (n - 1) / 2.0
    ax, ay = dx / math.hypot(dx, dy), dy / math.hypot(dx, dy)
    nx, ny = ay, -ax                       # normal, pointing up and left
    if nx > 0:
        nx, ny = -nx, -ny
    body = {}
    steps = max(abs(dx), abs(dy)) * 2
    for k in range(steps + 1):
        ox, oy = dx * k / float(steps), dy * k / float(steps)
        for (i, j), t in disc.items():
            if t == "o":
                continue
            px, py = int(round(i + ox)), int(round(j + oy))
            body[(px, py)] = (i + ox, j + oy)
    shape = set(body)
    for (px, py) in shape:
        for ddx, ddy in ((-1, 0), (1, 0), (0, -1), (0, 1)):
            q = (px + ddx, py + ddy)
            if q not in shape:
                img[(x + q[0], y + q[1])] = "o"
    for (px, py), (fx, fy) in body.items():
        v = (fx - c) * nx + (fy - c) * ny
        along = (fx - c) * ax + (fy - c) * ay
        t = "l" if v > c * 0.45 else ("m" if v > -c * 0.5 else "d")
        if t == "m" and int(math.floor(along * 1.5)) % 4 == 1:
            t = "d"                        # a bark streak
        img[(x + px, y + py)] = t
    for (i, j), t in disc.items():
        img[(x + i, y + j)] = t


def stamp(img, size, pal, ox=0, oy=0):
    im = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    for (x, y), t in img.items():
        if 0 <= x + ox < size and 0 <= y + oy < size:
            im.putpixel((x + ox, y + oy), rgb(pal[t]))
        else:
            raise AssertionError("pixel (%d, %d) outside the %d px cell" % (x + ox, y + oy, size))
    return im


def make_logs(size):
    """Three logs, two below and one on top, cut ends toward the viewer on the left, bark
    receding to the upper right."""
    img = {}
    if size == 16:
        n, dx, dy = 7, 3, -2
        for (x, y) in ((0, 9), (6, 9), (3, 3)):          # the top log is drawn last
            draw_log(img, x, y, n, dx, dy)
    else:
        n, dx, dy = 5, 3, -2
        for (x, y) in ((0, 6), (4, 6), (2, 2)):
            draw_log(img, x, y, n, dx, dy)
    return stamp(img, size, LOGS)


BRICKS = dict(o="3A1208", d="8C2D19", m="C8502D", l="E88A5A", h="F4B088")


def draw_brick(d, x, y, fw, fh, dx, dy):
    """One brick in three-quarter view, light from the upper left: lit top, red front, dark right
    end. (x, y) is the top left of the front face, the face covers fw x fh pixels, outline
    included, and the top and the end recede by (dx, dy) to the upper right."""
    o, dk, m, l, h = (rgb(BRICKS[k]) for k in "odmlh")
    x1, y1 = x + fw - 1, y + fh - 1
    right = [(x1, y), (x1 + dx, y - dy), (x1 + dx, y1 - dy), (x1, y1)]
    top = [(x, y), (x1, y), (x1 + dx, y - dy), (x + dx, y - dy)]
    d.polygon(right, fill=dk, outline=o)
    d.rectangle([x, y, x1, y1], fill=m, outline=o)
    d.polygon(top, fill=l, outline=o)
    d.line([(x + 1, y + 1), (x1 - 1, y + 1)], fill=h)             # lit upper edge of the front
    d.line([(x + 1, y1 - 1), (x1 - 1, y1 - 1)], fill=dk)          # shaded foot


def make_brick(size):
    """Three bricks in running bond: two end to end below, one across the joint on top."""
    im = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    d = ImageDraw.Draw(im)
    if size == 16:
        fw, fh, dx, dy = 7, 5, 3, 3
        draw_brick(d, 0, 10, fw, fh, dx, dy)
        draw_brick(d, 6, 10, fw, fh, dx, dy)
        draw_brick(d, 3, 6, fw, fh, dx, dy)
    else:
        # One pixel narrower than the cell allows end to end, so the right brick's side
        # face keeps its outline inside the 12 px cell
        fw, fh, dx, dy = 5, 4, 2, 2
        draw_brick(d, 0, 8, fw, fh, dx, dy)
        draw_brick(d, 4, 8, fw, fh, dx, dy)
        draw_brick(d, 2, 5, fw, fh, dx, dy)
    return im


# Three-quarter sheep: the body turned away to the left, the dark face looking out at the viewer
# from the lower right with its ears out to the sides (the pale dots are eyes), the near legs full
# length and the far pair short and lighter, half hidden behind the body.
SHEEP = dict(o="202020", w="F2F2F2", g="C0C0CC", h="4A403A", n="B8A898")
SHEEP16 = ["................",
           "................",
           "....ooo.ooo.....",
           "..oowwwowwwoo...",
           ".owwwwwwwwwwwo..",
           "owwwgwwwwwwgwwo.",
           "owwwwwwwwwooooo.",
           "owgwgwwwwohhhho.",
           "owwwwwwgohnhhnho",
           "owwwwwwwwohhhho.",
           ".owwgwwwwohhhho.",
           ".ogwwwwwwwohno..",
           "..oogggggggooo..",
           "...oo.hh.oo.hh..",
           "...oo....oo.hh..",
           "................"]
SHEEP12 = ["............",
           "..ooo.oo....",
           ".owwwowwoo..",
           "owwwwwwoooo.",
           "owgwwwohhhho",
           "owwwwwohnhno",
           "owwwgwohhhho",
           ".owwwwwohno.",
           ".ogwwwwwoo..",
           "..oogggooh..",
           "..o..h.o.h..",
           "..o....o...."]


def make_sheep(size):
    return art(SHEEP16 if size == 16 else SHEEP12, SHEEP, (size, size))


# ---- the second round of three-quarter icons: wool (calm face), wheat (a stook), ore, gold
# Lit from the upper left like the lumber, brick and pieces; at most 8 opaque colours each.

# Sheep: the same body, a dark face wedge turned three-quarters with ONE charcoal eye (a step
# lighter than the face, no pale dots, no mouth or muzzle mark), one ear pixel pair out to the side.
SHEEP2 = dict(o="202020", w="F2F2F2", g="C0C0CC", h="2A2A2A", n="4A4A4A")
SHEEP2_16 = ["................",
             "................",
             "....ooo.ooo.....",
             "..oowwwowwwoo...",
             ".owwwwwwwwwwwo..",
             "owwwgwwwwwwgwwo.",
             "owwwwwwwwwoooo..",
             "owgwgwwwwohhhhoo",
             "owwwwwwgwohnhhho",
             "owwwwwwwwwohhhho",
             ".owwgwwwwwohhho.",
             ".ogwwwwwwwoooo..",
             "..oogggggggooo..",
             "...oo.hh.oo.hh..",
             "...oo....oo.hh..",
             "................"]
SHEEP2_12 = ["............",
             "..ooo.oo....",
             ".owwwowwoo..",
             "owwwwwwooooo",
             "owgwwwwohhhh",
             "owwwwwwohnhh",
             "owwwgwwwohho",
             ".owwwwwwohho",
             ".ogwwwwwwooo",
             "..oogggooh..",
             "..o..h.o.h..",
             "..o....o...."]
SHEEP2_NOEYE_16 = [r.replace("n", "h") for r in SHEEP2_16]
SHEEP2_NOEYE_12 = [r.replace("n", "h") for r in SHEEP2_12]


def make_sheep2(size):
    return art(SHEEP2_16 if size == 16 else SHEEP2_12, SHEEP2, (size, size))


def make_sheep2_noeye(size):
    return art(SHEEP2_NOEYE_16 if size == 16 else SHEEP2_NOEYE_12, SHEEP2, (size, size))


# Wheat: a stook seen three-quarters: two sheaves leaning together into a cone, lit on the left,
# shaded on the right, with the dark gap of the open foot showing depth, a band wrapping near the
# top and a tuft of herringbone ears (the outer two darker, behind) out of the apex.
GRAIN3 = dict(o="4A3000", d="B8860B", m="E6B422", l="FFE08A", b="C8502D", B="8C2D19", k="8A6208")
GRAIN3_BACK = dict(GRAIN3, l=GRAIN3["m"], m=GRAIN3["d"], d=GRAIN3["k"])


def _outline_px(im, col):
    out = im.copy()
    px = im.load()
    for y in range(im.height):
        for x in range(im.width):
            if px[x, y][3] == 0:
                for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                    xx, yy = x + dx, y + dy
                    if 0 <= xx < im.width and 0 <= yy < im.height and px[xx, yy][3] and px[xx, yy][:3] != col[:3]:
                        out.putpixel((x, y), col)
                        break
    return out


def stook(n, ay, by, half, cx, bandy, ears):
    im = Image.new("RGBA", (n, n), (0, 0, 0, 0))
    px = im.load()
    for y in range(ay, by + 1):
        w = half * (y - ay + 1) / (by - ay + 1.0)
        for x in range(n):
            dxp = x + 0.5 - cx
            if abs(dxp) <= w:
                t = dxp / max(w, 0.5)
                col = "l" if t < -0.25 else ("m" if t < 0.3 else ("d" if t < 0.75 else "k"))
                if col in "lm" and int(t * 7 + 8) % 3 == 0 and y > ay + 1:
                    col = {"l": "m", "m": "d"}[col]
                px[x, y] = rgb(GRAIN3[col])
    for y in range(by - 3, by + 1):                      # the open foot between the sheaves
        w = (y - (by - 4)) * 0.9
        for x in range(n):
            if abs(x + 0.5 - cx) <= w:
                px[x, y] = rgb(GRAIN3["k"])
    body = _outline_px(im, rgb(GRAIN3["o"]))
    bp = body.load()
    for y in (bandy, bandy + 1):
        w = half * (y - ay + 1) / (by - ay + 1.0)
        for x in range(n):
            if abs(x + 0.5 - cx) <= w + 0.3 and bp[x, y][3]:
                bp[x, y] = rgb(GRAIN3["b" if y == bandy else "B"])
    out = Image.new("RGBA", (n, n), (0, 0, 0, 0))
    for (base, sx, length, kfrom, awns, every, pal) in ears:
        cv = PixelCanvas(n, pal)
        wheat_ear(cv, base, sx, length, kfrom, awns, every)
        out.alpha_composite(cv.render())
    out.alpha_composite(body)
    return out


def make_stook(size):
    if size == 16:
        return stook(16, 5, 15, 6.5, 8, 8, [((6, 8), -1, 6, 1, 1, 4, GRAIN3_BACK),
                                           ((10, 8), 1, 6, 1, 1, 4, GRAIN3_BACK),
                                           ((8, 8), 0, 8, 2, 2, 1, GRAIN3)])
    return stook(12, 5, 11, 4.8, 6, 7, [((4, 7), -1, 5, 1, 1, 3, GRAIN3_BACK),
                                       ((8, 7), 1, 5, 1, 1, 3, GRAIN3_BACK),
                                       ((6, 7), 0, 7, 1, 1, 1, GRAIN3)])


GRAIN_CANDIDATES.append(("e stook (chosen)", make_stook))


# Ore: chunky faceted rocks, each with a light top plane, a mid lit face and a dark shaded face,
# a small heap with one rock in front, copper flecks and a hard white glint.
ORE2 = dict(o="1C2028", d="4C5464", m="808C9C", l="B8C4D0", w="FFFFFF", c="A8502A")
ORE2_R1 = ["...oooo...",
           "..ollllo..",
           ".ollwllldo",
           "ollllllddo",
           "ommlllmddo",
           "ommmmmmddo",
           "ommcmmmddo",
           ".ommmmmddo",
           "..oooooooo"]
ORE2_R2 = ["..oooo.",
           ".ollldo",
           "olwlldo",
           "ommmddo",
           "ommcddo",
           ".ommddo",
           "..ooooo"]
ORE2_R3 = ["...oooooooo..",
           "..ollllllldo.",
           ".ollllwllddo.",
           "ommmmmmdddddo",
           "ommcmmmdddddo",
           "ommmmmmdddddo",
           ".ommmmmddddo.",
           "..ooooooooo.."]
ORE2_S1 = ["..oooo.",
           ".ollldo",
           "ollwldo",
           "ommmddo",
           "ommmddo",
           ".ooooo."]
ORE2_S2 = [".oooo.",
           "olwldo",
           "ommddo",
           "ommddo",
           ".oooo."]
ORE2_S3 = ["..oooooo..",
           ".ollllldo.",
           "ollllwlddo",
           "ommmmmdddo",
           "ommcmmdddo",
           ".ommmmddo.",
           "..oooooo.."]


def make_ore2(size, copper=True):
    im = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    if size == 16:
        parts = [(ORE2_R1, 0, 0), (ORE2_R2, 9, 3), (ORE2_R3, 2, 8)]
    else:
        parts = [(ORE2_S1, 0, 0), (ORE2_S2, 6, 2), (ORE2_S3, 1, 5)]
    for rows, x, y in parts:
        if not copper:
            rows = [r.replace("c", "m") for r in rows]
        im.alpha_composite(art(rows, ORE2), (x, y))
    return im


# Gold: cast bars, trapezoids (the foot wider than the top), a bright top face, a mid front, a
# dark end face, two bars (a front one lower left, a back one upper right) and a four armed gleam.
# The planes are separated by tone, with one outline round each bar.
GOLD2 = dict(o="3A2800", d="A07000", m="E0B000", l="FFF0A0", w="FFFFFF", h="FFD84A")


def gold_bar(im, x, y, fw, fh, dx, dy, foot=1, inner_outline=False):
    W, H = im.size
    x1, y1 = x + fw - 1, y + fh - 1
    front = [(x, y), (x1, y), (x1 + foot, y1), (x - foot, y1)]
    right = [(x1, y), (x1 + dx, y - dy), (x1 + dx + foot, y1 - dy), (x1 + foot, y1)]
    if inner_outline:
        top = [(x, y), (x1, y), (x1 + dx, y - dy), (x + dx, y - dy)]
        d = ImageDraw.Draw(im)
        d.polygon(right, fill=rgb(GOLD2["d"]), outline=rgb(GOLD2["o"]))
        d.polygon(front, fill=rgb(GOLD2["m"]), outline=rgb(GOLD2["o"]))
        d.polygon(top, fill=rgb(GOLD2["l"]), outline=rgb(GOLD2["o"]))
        d.line([(x + 1, y + 1), (x1 - 1, y + 1)], fill=rgb(GOLD2["h"]))
        return
    front = [(x, y), (x1, y), (x1 + foot, y1), (x - foot, y1)]
    top = [(x, y - 1), (x1, y - 1), (x1 + dx, y - dy), (x + dx, y - dy)]
    right = [(x1 + 1, y), (x1 + dx, y - dy + 1), (x1 + dx + foot, y1 - dy), (x1 + foot + 1, y1)]
    layers = [(front, "m"), (top, "l"), (right, "d")]
    mask = Image.new("L", im.size, 0)
    dm = ImageDraw.Draw(mask)
    for pts, _ in layers:
        dm.polygon(pts, fill=255)
    mp = mask.load()
    ins = set((a, b) for a in range(W) for b in range(H) if mp[a, b])
    for (a, b) in sorted(ins):
        for ddx, ddy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
            q = (a + ddx, b + ddy)
            if q not in ins and 0 <= q[0] < W and 0 <= q[1] < H:
                im.putpixel(q, rgb(GOLD2["o"]))
    d = ImageDraw.Draw(im)
    for pts, c in layers:
        d.polygon(pts, fill=rgb(GOLD2[c]))
    d.line([(x, y1), (x1, y1)], fill=rgb(GOLD2["d"]))     # the shaded foot


def make_gold2(size, inner_outline=False):
    im = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    if size == 16:
        # Two bars: the back one upper right, the front one lower left, drawn last so it may
        # cover the back bar's lower left corner. Both outlines stay inside the cell.
        bars = [(7, 4, 5, 5, 2, 3, 1), (2, 9, 5, 5, 2, 3, 1)]
        gleam = (9, 8)
    else:
        # The art ends at row 10, leaving row 11 clear for the cart bed (pieces.txt).
        bars = [(6, 4, 3, 3, 1, 2, 1), (2, 7, 4, 3, 1, 2, 1)]
        gleam = (7, 6)
    for b in bars:
        gold_bar(im, *b, inner_outline=inner_outline)
    # A four armed gleam on the front bar's top right corner: a white heart, pale gold arms.
    gx, gy = gleam
    for (sx, sy) in SPARK:
        if (sx, sy) != (1, 1):
            im.putpixel((gx + sx - 1, gy + sy - 1), rgb(GOLD2["l"]))
    im.putpixel((gx, gy), rgb(GOLD2["w"]))
    return im


def make_resources():
    put("icon_lumber_16.png", make_logs(16))
    put("icon_lumber_12.png", make_logs(12))
    put("icon_grain_16.png", sheaf(16))
    put("icon_grain_12.png", sheaf(12))
    put("icon_wool_16.png", make_sheep2(16))
    put("icon_wool_12.png", make_sheep2(12))
    put("icon_ore_16.png", make_ore2(16))
    put("icon_ore_12.png", make_ore2(12))
    put("icon_brick_16.png", make_brick(16))
    put("icon_brick_12.png", make_brick(12))
    put("icon_gold_16.png", make_gold2(16))
    put("icon_gold_12.png", make_gold2(12))
    put("icon_any_16.png", bevel_glyph(ANY16, 16, ANY, (4, 2)))
    put("icon_any_12.png", bevel_glyph(ANY12, 12, ANY, (3, 1)))


# ---------------------------------------------------------------- legend glyphs, start, badges

K = "1E140A"
GLYPH_ROAD = ["............",
              "........kkk.",
              ".......kccck",
              "......kcmcck",
              ".....kcmcck.",
              "....kcmcck..",
              "...kcmcck...",
              "..kcmcck....",
              ".kcmcck.....",
              "kcccck......",
              ".kkk........",
              "............"]
# The roof is in team magenta (p the reference pink, P its shade), so it takes the player colour
GLYPH_VILLAGE = ["............",
                 ".....kk.....",
                 "....kppk....",
                 "...kppppk...",
                 "..kppppppk..",
                 ".kppppppppk.",
                 "kPPPPPPPPPPk",
                 ".kwwwwwwwwk.",
                 ".kwwddwwbbk.",
                 ".kwwddwwwWk.",
                 ".kkkkkkkkkk.",
                 "............"]
GLYPH = dict(k=K, c="A09078", m="C8B898", p="F49AC1", P="EC008C", w="F0E0B8", W="D8C494",
             d="8B5A2B", b="4F93AD", s="B8B8B8", S="8C8C8C", l="E0B870")
START_ARROW = ["..kkkk..",
               "..kllk..",
               "..kllk..",
               "kkkllkkk",
               "kllllllk",
               ".kllllk.",
               "..kllk..",
               "...kk..."]
# The undo button's arrow, on nothing (the plate is drawn in code): a head pointing left, a
# 2 px stroke running right and hooking back round below it, in the same outline and fill the
# menu button's bars had.
UNDO = ["................",
        ".....kkk........",
        "....kklk........",
        "...kkllk........",
        "..kklllkkkkkk...",
        "..klllllllllkk..",
        "..kllllllllllkk.",
        "..kklllkkkklllk.",
        "...kkllk..kkllk.",
        "....kklk...kllk.",
        ".....kkk..kkllk.",
        ".....kkkkkklllk.",
        ".....kllllllllk.",
        ".....klllllllkk.",
        ".....kkkkkkkkk..",
        "................"]


# The board castle in little: two towers with cone roofs either side of a wall and gatehouse,
# a banner over the gate. Roofs and banner in team magenta (p the reference pink, P its shade),
# so they take the player colour like the castle on the board.
CASTLE_GLYPH = dict(L="C8C0B0", s="9C9488", S="6E665A", k=K, p="F49AC1", P="EC008C", d="4A2C14")
CASTLE = [".p......p.",
          "ppP....ppP",
          "ppP....ppP",
          "LsS.ss.LsS",
          "LsSssssLsS",
          "LsSsppsLsS",
          "LsSsPPsLsS",
          "LsSskksLsS",
          "LsSkddkLsS",
          "LsSkddkLsS"]


def make_castle_glyph():
    """12 px castle: a 10 x 10 picture (two towers, wall and gatehouse, as on the board) inside
    the cell so the 1 px outline ring lands whole inside it."""
    return glyph_from_art(CASTLE, CASTLE_GLYPH)


def glyph_from_art(rows, legend):
    """Stamp a 10x10 picture into the middle of a 12x12 cell and add the 1 px outline."""
    im = Image.new("RGBA", (12, 12), (0, 0, 0, 0))
    im.alpha_composite(art(rows, legend), (1, 1))
    finish_outline(im)
    return im


# legend glyphs sit on the dark HUD, so they stay bright: cream, tan and a warm wood bed.
CART_GLYPH = dict(c="F0E0B8", s="C0AC80", t="6B4420", w="A8743C", b="C8955A", h="F0E0B8",
                  r="C8502D", n="8C2D19")
TRADER_GLYPH_CRATE = ["..........",
                      ".ssssss..t",
                      ".cccctc..t",
                      ".cctccc.t.",
                      ".ssssss.t.",
                      "bbbbbbbbt.",
                      "wttwwttww.",
                      "tcct.tcct.",
                      "tcct.tcct.",
                      ".tt..tt..."]
TRADER_GLYPH_SACK = ["...tt.....",
                     "..cccs...t",
                     ".ccccccs.t",
                     ".cccccsc.t",
                     ".sccccs.t.",
                     "bbbbbbbbt.",
                     "wttwwttww.",
                     "tcct.tcct.",
                     "tcct.tcct.",
                     ".tt..tt..."]


def make_trader_glyph(variant="sack"):
    """12 px hand cart: a low flat bed with a crate (or a sack) on it, two wheels and a shaft.
    Readable as a cart, not an animal, and it matches piece_trader's low open cart."""
    rows = TRADER_GLYPH_CRATE if variant == "crate" else TRADER_GLYPH_SACK
    return glyph_from_art(rows, CART_GLYPH)


def make_trader_glyph_wagon():
    """(previous version, kept for the preview only) 12 px covered wagon."""
    im = Image.new("RGBA", (12, 12), (0, 0, 0, 0))
    d = ImageDraw.Draw(im)
    cream, red, shade = rgb("F0E0B8"), rgb("C8502D"), rgb("C0AC80")
    wood, wdark, hub, tyre = rgb("A8743C"), rgb("6B4420"), rgb("F0E0B8"), rgb("3A2A1A")
    rows = {1: (3, 7), 2: (2, 8), 3: (1, 9), 4: (1, 9), 5: (1, 9), 6: (1, 9)}
    for y, (x0, x1) in rows.items():
        for x in range(x0, x1 + 1):
            c = red if (x - x0) % 2 == 0 else cream
            if y == 6:
                c = shade if c == cream else red
            d.point((x, y), fill=c)
    d.rectangle([0, 7, 10, 8], fill=wood)
    d.line([(0, 8), (10, 8)], fill=wdark)
    d.line([(10, 6), (11, 4)], fill=wdark)
    for cx in (2, 8):
        d.rectangle([cx - 1, 8, cx + 1, 10], fill=tyre)
        d.point((cx, 9), fill=hub)
    finish_outline(im)
    return im


ROAD_GLYPH = dict(k="F49AC1", K="EC008C", a="E0D0A8", b="B8A47C", m="7C6C50")


def road_ribbon(points, half, kerb_w, n=12):
    """Rasterise a road of half width `half` along a polyline in an n x n cell (pixel-centre
    distance, hard edges). The outer `kerb_w` of each side is kerb stone, alternating cream and
    tan every 2 px along the road; the middle is cobbles in two tones with a mortar fleck.
    Roads that run off the cell edge are left open there, so they read as roads that go on."""
    im = Image.new("RGBA", (n, n), (0, 0, 0, 0))
    px = im.load()
    segs = list(zip(points[:-1], points[1:]))
    for y in range(n):
        for x in range(n):
            cx, cy = x + 0.5, y + 0.5
            best = None
            run = 0.0
            for (x0, y0), (x1, y1) in segs:
                dx, dy = x1 - x0, y1 - y0
                ln = math.hypot(dx, dy)
                t = max(0.0, min(1.0, ((cx - x0) * dx + (cy - y0) * dy) / (ln * ln)))
                dist = math.hypot(cx - (x0 + t * dx), cy - (y0 + t * dy))
                side = (cx - x0) * dy - (cy - y0) * dx
                if best is None or dist < best[0]:
                    best = (dist, run + t * ln, side)
                run += ln
            dist, along, side = best
            if dist > half:
                continue
            if dist > half - kerb_w:
                c = ROAD_GLYPH["k"] if int(along / 2.0) % 2 == 0 else ROAD_GLYPH["K"]
            else:
                stone = int((along + (1.5 if side > 0 else 0.0)) / 3.0)
                c = ROAD_GLYPH["a"] if (stone + (1 if side > 0 else 0)) % 2 == 0 else ROAD_GLYPH["b"]
                if (along + (1.5 if side > 0 else 0.0)) % 3.0 < 0.5:
                    c = ROAD_GLYPH["m"]
            px[x, y] = rgb(c)
    return im


# The 7 px bonus badges on the turn strip's label line (a race map's long road and third
# trader): the road is the diagonal ribbon again with the same team magenta kerb, the trader
# a sack on a cart. No outline: they stand on the bar's dark wood.
BADGE_TRADER_COLOURS = dict(c="F0E0B8", d="C0AC80", b="C8955A", f="A8743C", w="6B4420")
BADGE_TRADER = ["..ccc..",
                ".ccdcc.",
                ".ccccc.",
                "bbbbbbb",
                ".fffff.",
                ".w...w.",
                ".w...w."]


def road_glyph_diagonal():
    """(a) A stretch of road running corner to corner and off the cell, team magenta kerb
    stones either side, two tones of cobble between."""
    im = road_ribbon([(-2, 14), (14, -2)], 3.0, 1.6)
    finish_outline(im)
    return im


def road_glyph_flat():
    """(b) A straight horizontal stretch like road_h, running off both sides of the cell."""
    im = road_ribbon([(-2, 6), (14, 6)], 3.6, 1.4)
    finish_outline(im)
    return im


def road_glyph_bend():
    """(c) A road that comes in from the left and bends up and away to the top right, the shape
    of two road pieces meeting at a hex corner."""
    im = road_ribbon([(-2, 9), (3, 9), (9, 3), (9, -2)], 2.9, 1.3)
    finish_outline(im)
    return im


def road_glyph_receding():
    """(d) The road runs away up the cell: kerbs converge, cobbles in staggered rows. Open at the
    top and bottom edge so it does not close into a tent shape."""
    im = Image.new("RGBA", (12, 12), (0, 0, 0, 0))
    px = im.load()
    for y in range(12):
        hw = 2.4 + y * 0.33
        for x in range(12):
            dxc = x + 0.5 - 6.0
            if abs(dxc) > hw:
                continue
            if abs(dxc) > hw - 1.1:
                c = ROAD_GLYPH["k"] if (y // 2) % 2 == 0 else ROAD_GLYPH["K"]
            else:
                stone = (x + (y // 2) % 2 * 2) // 3
                c = ROAD_GLYPH["a"] if (stone + y // 2) % 2 == 0 else ROAD_GLYPH["b"]
                if y % 2 == 1 and (x + (y // 2) % 2 * 2) % 3 == 2:
                    c = ROAD_GLYPH["m"]
            px[x, y] = rgb(c)
    finish_outline(im)
    return im


ROAD_CANDIDATES = [("a diagonal (chosen)", road_glyph_diagonal), ("b flat", road_glyph_flat),
                   ("c bend", road_glyph_bend), ("d receding", road_glyph_receding)]


def make_misc():
    put("glyph_road_12.png", road_glyph_diagonal())
    put("glyph_village_12.png", art(GLYPH_VILLAGE, GLYPH, (12, 12)))
    put("glyph_castle_12.png", make_castle_glyph())
    put("glyph_trader_12.png", make_trader_glyph())
    put("start_marker.png", art(START_ARROW, dict(k=K, l=TEXT), (8, 8)))
    put("badge_road_7.png", road_ribbon([(-1, 8), (8, -1)], 2.7, 1.3, n=7))
    put("badge_trader_7.png", art(BADGE_TRADER, BADGE_TRADER_COLOURS, (7, 7)))
    put("undo_glyph.png", art(UNDO, dict(k=K, l=TEXT), (16, 16)))


# ---------------------------------------------------------------- launcher icon

def make_launcher(S):
    """One flat-top grass hex on the sea with a cottage and a flag. Drawn with hard
    polygons in proportional coordinates, so the 48 and 64 designs match."""
    f = S / 48.0

    def P(v):
        return int(round(v * f))

    sea, wave = rgb("2E6E8E"), rgb("4F93AD")
    grass, grassl = rgb("5E8C2E"), rgb("7DAA3C")
    wall, roof, dark, flag = rgb("F0E0B8"), rgb("C8502D"), rgb("2A1A0E"), rgb("E6B422")
    im = Image.new("RGBA", (S, S), sea)
    d = ImageDraw.Draw(im)
    one = max(1, P(1))
    for (x, y, n) in [(3, 6, 5), (36, 5, 6), (4, 41, 6), (35, 42, 5)]:
        d.rectangle([P(x), P(y), P(x + n) - 1, P(y) + one - 1], fill=wave)

    def hexagon(cx, cy, rw, rh):
        return [(round(cx - rw), round(cy)), (round(cx - rw / 2.0), round(cy - rh)),
                (round(cx + rw / 2.0), round(cy - rh)), (round(cx + rw), round(cy)),
                (round(cx + rw / 2.0), round(cy + rh)), (round(cx - rw / 2.0), round(cy + rh))]

    cx, cy = S // 2, S // 2
    rw, rh = P(20), P(17.5)
    d.polygon(hexagon(cx, cy, rw, rh), fill=dark)
    d.polygon(hexagon(cx, cy, rw - one * 1.2, rh - one), fill=grass)
    for (x, y) in [(9, 21), (12, 33), (19, 37), (30, 36), (37, 30), (36, 20), (13, 14),
                   (33, 13), (10, 27), (38, 25)]:
        d.rectangle([P(x), P(y), P(x) + 2 * one - 1, P(y) + one - 1], fill=grassl)
        d.rectangle([P(x), P(y) - one, P(x) + one - 1, P(y) - 1], fill=grassl)
    # cottage, centred; flag on the ridge
    wx0, wx1, wy0, wy1 = P(16), P(31), P(26), P(35)
    d.rectangle([wx0 - one, wy0, wx1 + one, wy1 + one], fill=dark)
    d.rectangle([wx0, wy0, wx1, wy1], fill=wall)
    ridge = P(17)
    d.polygon([(P(12), P(26)), (P(24) - 1, ridge - one), (P(24), ridge - one), (P(35), P(26))],
              fill=dark)
    d.polygon([(P(14), P(26) - one), (P(24) - 1, ridge), (P(24), ridge), (P(33), P(26) - one)],
              fill=roof)
    d.rectangle([P(12), P(26), P(35), P(26) + one - 1], fill=roof)
    d.rectangle([P(12) - 1 if False else P(12), P(26) + one, P(35), P(26) + one], fill=dark)
    d.rectangle([P(25), P(29), P(28), wy1], fill=dark)           # door
    d.rectangle([P(18), P(29), P(22), P(32)], fill=dark)         # window
    d.rectangle([P(19), P(30), P(21), P(31)], fill=flag)
    # pennant staff from the ridge
    d.rectangle([P(24) - 1, P(9), P(24) - 1 + one - 1, ridge], fill=dark)
    d.polygon([(P(24) - 1 + one, P(9)), (P(31), P(11)), (P(24) - 1 + one, P(13))], fill=flag)
    return im


# ---------------------------------------------------------------- launcher icon, adaptive
# One design on a 108 x 108 art pixel canvas (the adaptive icon's 108 dp), shown 4x nearest
# neighbour. The foreground is a flat topped hexagon medallion 67 px point to point: a little
# inside the circle a round launcher mask cuts (72 px across), so the sea shows as one unbroken
# ring around it, with the island scene in it. The background is the
# deep sea. icon.png and market_icon.png are cut from the same design.

ICON_SEA_DEEP = "0D3947"
ICON_BG = dict(a=ICON_SEA_DEEP, b="12475A", c="1B5A70")
ICON_PAL = dict(
    o="2A1A0E",                          # outline, shared with every sprite
    L="E0B870", D="9A7430",              # medallion rim, lit and shaded
    s="2E6E8E", t="4F93AD",              # sea in the medallion and its wave ticks
    g="5E8C2E", G="7DAA3C",              # grass: shade and bright pasture
    y="D8C494",                          # sand, and the shaded wall
    w="F0E0B8",                          # cream wall, and snow
    k="C8C0B0", m="6E665A",              # stone, lit and shaded
    r="F06A5A", R="D32F2F", Q="8E1A1A",  # roofs: lit, body, shade (the default player red)
    b="5C3A1E",                          # hull and timber
    n="C9A862",                          # desert
    f="7A4E2A",                          # farmland soil
    F="2E5A2E",                          # forest
    h="8E9A3C",                          # hills, olive
    a="3FA39A",                          # the lake
)
ICON_R = 33.5                            # hexagon circumradius: 67 px point to point
ICON_SCENE = ICON_R / 31.5               # the scene was laid out for a 63 px hexagon


def hexnorm(dx, dy):
    return max(abs(dy) * 2.0 / math.sqrt(3.0), abs(dx) + abs(dy) / math.sqrt(3.0))


def quantize_to(im, pal):
    """Map every opaque pixel to the nearest colour of pal (a dict of hex strings)."""
    cols = [(k, rgb(v)[:3]) for k, v in pal.items()]
    out = Image.new("RGBA", im.size, (0, 0, 0, 0))
    sp, op = im.load(), out.load()
    for y in range(im.height):
        for x in range(im.width):
            p = sp[x, y]
            if p[3] < 128:
                continue
            best = min(cols, key=lambda kc: sum((p[i] - kc[1][i]) ** 2 for i in range(3)))
            op[x, y] = best[1] + (255,)
    return out


# ---- the island scene of the launcher icon: the game's seven hex flower seen in perspective

def vhash(ix, iy, seed):
    h = (ix * 374761393 + iy * 668265263 + seed * 1274126177) & 0xFFFFFFFF
    h = ((h ^ (h >> 13)) * 1274126177) & 0xFFFFFFFF
    h ^= h >> 16
    return (h & 0xFFFF) / 65535.0


def vnoise(x, y, seed):
    """Smooth value noise, 0..1."""
    ix, iy = int(math.floor(x)), int(math.floor(y))
    fx, fy = x - ix, y - iy
    fx, fy = fx * fx * (3 - 2 * fx), fy * fy * (3 - 2 * fy)
    a, b = vhash(ix, iy, seed), vhash(ix + 1, iy, seed)
    c, d = vhash(ix, iy + 1, seed), vhash(ix + 1, iy + 1, seed)
    return (a + (b - a) * fx) * (1 - fy) + (c + (d - c) * fx) * fy


ISLE = dict(s=19.0, q=0.64, yh=-80.0, y0=49.0, cx=54.0)
ISLE_TERRAIN = {(0, 0): "lake", (-1, 0): "mountain", (0, -1): "desert", (1, -1): "hills",
                (-1, 1): "farm", (0, 1): "pasture", (1, 0): "forest"}


def isle_plane(X, Y):
    """Screen pixel to ground plane (u, v), in pixels at the middle row. The ground tilts away
    from the viewer: horizon above the icon, so rows further up are narrower and shorter."""
    z0 = ISLE["y0"] - ISLE["yh"]
    z = Y - ISLE["yh"]
    u = (X - ISLE["cx"]) * z0 / z
    v = (z0 * z0 / ISLE["q"]) * (1.0 / z0 - 1.0 / z)
    return u, v


def isle_screen(u, v):
    """Ground plane to screen: (X, Y, scale)."""
    z0 = ISLE["y0"] - ISLE["yh"]
    z = 1.0 / (1.0 / z0 - ISLE["q"] * v / (z0 * z0))
    return ISLE["cx"] + u * z / z0, ISLE["yh"] + z, z / z0


def hex_of(u, v):
    s = ISLE["s"]
    qf = (2.0 / 3.0 * u) / s
    rf = (-u / 3.0 + math.sqrt(3.0) / 3.0 * v) / s
    x, z = qf, rf
    y = -x - z
    rx, ry, rz = round(x), round(y), round(z)
    dx, dy, dz = abs(rx - x), abs(ry - y), abs(rz - z)
    if dx > dy and dx > dz:
        rx = -ry - rz
    elif dy > dz:
        ry = -rx - rz
    else:
        rz = -rx - ry
    return int(rx), int(rz)


def hex_centre(q, r):
    s = ISLE["s"]
    return 1.5 * s * q, s * math.sqrt(3.0) * (q / 2.0 + r)


def isle_terrain(X, Y):
    """Terrain name at a screen pixel, or None for open sea. The hex lookup is warped by smooth
    noise, and where a second, finer warp disagrees the two terrains mix raggedly."""
    u, v = isle_plane(X + 0.5, Y + 0.5)
    r = []
    for amp, fr, sd in ((4.0, 0.08, 11), (1.6, 0.24, 29)):
        wu = u + (vnoise(u * fr, v * fr, sd) - 0.5) * 2 * amp
        wv = v + (vnoise(u * fr + 40, v * fr + 17, sd + 1) - 0.5) * 2 * amp
        r.append(ISLE_TERRAIN.get(hex_of(wu, wv)))
    if r[0] != r[1] and vhash(X, Y, 5) < 0.5:
        return r[1]
    return r[0]


def icon_land(mask):
    """{(x, y): palette key} for every land pixel inside the medallion, and the sea pixels."""
    land = {}
    for (x, y) in sorted(mask):
        t = isle_terrain(x, y)
        if t is None:
            continue
        u, v = isle_plane(x + 0.5, y + 0.5)
        n1 = vnoise(u * 0.11, v * 0.17, 3)
        n2 = vnoise((u + 2.0) * 0.11, (v + 2.0) * 0.17, 3)     # the same relief nudged: shading
        rel = n1 - n2
        h = vhash(x, y, 7)
        if t == "lake":
            c = "a"
            if vnoise(u * 0.35, v * 1.4, 9) > 0.68 and h < 0.8:
                c = "t"
        elif t == "desert":
            c = "n"
            if vnoise(u * 0.07 + 3, v * 0.26, 13) > 0.58:
                c = "y"
            elif h < 0.04:
                c = "y"
        elif t == "hills":
            c = "h"
            if rel > 0.05:
                c = "G"
            elif rel < -0.05:
                c = "g"
        elif t == "mountain":
            c = "h"
            if vnoise(u * 0.10, v * 0.16, 17) > 0.55:
                c = "m"
            elif h < 0.05:
                c = "b"
        elif t == "farm":
            c = "f"
            stripe = (u / 5.5) + (vnoise(u * 0.02, v * 0.03, 31) - 0.5) * 0.6
            fr = stripe - math.floor(stripe)
            if 0.40 < fr < 0.66 and h < 0.88 and vnoise(u * 0.5, v * 0.1, 33) > 0.22:
                c = "g" if h < 0.7 else "G"
        elif t == "pasture":
            c = "G"
            if vnoise(u * 0.5, v * 0.7, 41) > 0.62 and h < 0.5:
                c = "g"
            elif h < 0.03:
                c = "g"
        else:   # forest
            c = "F"
            if vnoise(u * 0.28, v * 0.4, 51) > 0.7 and h < 0.45:
                c = "g"
        land[(x, y)] = c
    return land


def icon_scene(px, mask, n):
    """Draw the island into px: terrain, sand rim, then the upright things."""
    land = icon_land(mask)
    # a thin pale sand rim where land meets the sea, a pixel or two, ragged
    rim = set()
    for (x, y) in land:
        d = 9
        for ddx, ddy, dd in ((1, 0, 1), (-1, 0, 1), (0, 1, 1), (0, -1, 1),
                             (1, 1, 2), (-1, 1, 2), (1, -1, 2), (-1, -1, 2),
                             (2, 0, 2), (-2, 0, 2), (0, 2, 2), (0, -2, 2)):
            q = (x + ddx, y + ddy)
            if q in mask and q not in land:
                d = min(d, dd)
        if d == 1 or (d == 2 and vhash(x, y, 61) < 0.4):
            rim.add((x, y))
    for (x, y), c in sorted(land.items()):
        px[x, y] = rgb(ICON_PAL["y" if (x, y) in rim else c])
    return land


PEAK = [".......o.......",
        "......owo......",
        ".....owwmo.....",
        "....owwwmmo....",
        "...owkwwkmmo...",
        "..owkkkwkmmmo..",
        ".owkkkkkkkmmmo.",
        "okkkkwkkkkmmmmo",
        "okkkkkkmkkkmmmo",
        ".ooooooooooooo."]
PEAK_S = ["....o....",
          "...owo...",
          "..owkmo..",
          ".owkkkmo.",
          "okkwkkmmo",
          ".ooooooo."]
TREE = ["....o....",
        "...ogo...",
        "..oggFo..",
        "..ogggFo.",
        ".ogggFFo.",
        "..ogggFo.",
        ".ogggFFFo",
        "ogggggFFo",
        ".ooobooo.",
        "....b...."]


def icon_medallion(n=108):
    """The hexagon with its outline, rim and island scene, on a transparent n x n canvas."""
    c = n / 2.0
    im = Image.new("RGBA", (n, n), (0, 0, 0, 0))
    px = im.load()
    inner = ICON_R - 3.6
    mask = set()
    for y in range(n):
        for x in range(n):
            dx, dy = x + 0.5 - c, y + 0.5 - c
            h = hexnorm(dx, dy)
            if h > ICON_R:
                continue
            if h > ICON_R - 1.7:
                col = ICON_PAL["o"]
            elif h > inner:
                col = ICON_PAL["L"] if dx + dy < 0 else ICON_PAL["D"]
            else:
                col = ICON_PAL["s"]
                mask.add((x, y))
            px[x, y] = rgb(col)
    # a few wave ticks on the open sea
    def at(x, y):
        """A scene position, pushed out from the centre as the hexagon grew."""
        return (int(round(c + (x - c) * ICON_SCENE)), int(round(c + (y - c) * ICON_SCENE)))

    for (x, y, w) in ((32, 52, 4), (73, 41, 3), (36, 71, 3), (68, 76, 4), (49, 33, 3), (77, 56, 3)):
        x, y = at(x, y)
        for i in range(w):
            if (x + i, y) in mask:
                px[x + i, y] = rgb(ICON_PAL["t"])
    land = icon_scene(px, mask, n)
    red = (211, 47, 47)

    def place(sprite, anchor, at):
        spr = quantize_to(recolor(sprite, red), ICON_PAL)
        layer = Image.new("RGBA", (n, n), (0, 0, 0, 0))
        layer.alpha_composite(spr, (at[0] - anchor[0], at[1] - anchor[1]))
        lp = layer.load()
        for (x, y) in mask:
            if lp[x, y][3]:
                px[x, y] = lp[x, y]

    def spot(q, r, du=0.0, dv=0.0):
        X, Y, k = isle_screen(*[a + b for a, b in zip(hex_centre(q, r), (du, dv))])
        return (int(round(X)), int(round(Y)), k)

    def upright(draw_fn):
        """Draw a small standing thing (peak, tree, mound) over the terrain, inside the medallion."""
        layer = Image.new("RGBA", (n, n), (0, 0, 0, 0))
        draw_fn(ImageDraw.Draw(layer))
        lp = layer.load()
        for (x, y) in mask:
            if lp[x, y][3]:
                px[x, y] = lp[x, y]

    def stamp_art(rows, x, y, scale=1.0):
        """Stamp ASCII art (palette keys, '.' clear) with its bottom centre at (x, y)."""
        spr = art(rows, ICON_PAL)
        if scale != 1.0:
            spr = spr.resize((int(round(spr.width * scale)), int(round(spr.height * scale))), Image.NEAREST)
        place_plain(spr, (x - spr.width // 2, y - spr.height + 1))

    def place_plain(spr, at_):
        layer = Image.new("RGBA", (n, n), (0, 0, 0, 0))
        layer.alpha_composite(spr, at_)
        lp = layer.load()
        for (x, y) in mask:
            if lp[x, y][3]:
                px[x, y] = lp[x, y]

    def conifer(x, y):
        stamp_art(TREE, x, y)

    def mound(x, y, w):
        def f(d):
            hh, G, g = (rgb(ICON_PAL[c]) for c in "hGg")
            for i in range(w):
                t = (i - (w - 1) / 2.0) / (w / 2.0)
                top = y - int(round(2.6 * math.sqrt(max(0.0, 1 - t * t))))
                for yy in range(top, y + 1):
                    d.point((x - w // 2 + i, yy), fill=G if (yy == top and t < 0.3) else (g if (yy == y or t > 0.55) else hh))
        upright(f)

    mx, my, mk = spot(-1, 0)
    stamp_art(PEAK, mx + 5, my + 8, 1.4)
    stamp_art(PEAK_S, mx + 17, my + 11, 1.3)
    hx, hy, hk = spot(1, -1)
    mound(hx - 6, hy + 1, 12); mound(hx + 3, hy + 6, 9)
    fx, fy, fk = spot(1, 0)
    for dx_, dy_ in ((-6, 3), (1, -4), (5, 8), (-1, 9)):
        conifer(fx + dx_, fy + dy_)
    gx, gy, gk = spot(0, 1, 6, 0)
    road = Image.new("RGBA", (n, n), (0, 0, 0, 0))
    ImageDraw.Draw(road).polygon([(gx - 2, gy - 3), (gx + 2, gy - 3), (gx + 4, gy + 9), (gx - 5, gy + 9)],
                                 fill=rgb(ICON_PAL["y"]))
    ImageDraw.Draw(road).line([(gx - 2, gy - 3), (gx - 5, gy + 9)], fill=rgb(ICON_PAL["k"]))
    place_plain(road, (0, 0))
    castle, canchor = make_castle()
    place(castle, canchor, spot(0, 1, 6, 0)[:2])
    return im


def icon_background(n):
    """Deep sea, edge to edge, with a few quiet lighter ticks. Opaque."""
    im = Image.new("RGBA", (n, n), rgb(ICON_BG["a"]))
    px = im.load()
    for i in range(n * n // 90):
        x = (i * 37 + (i * i) % 23) % (n - 4)
        y = (i * 53 + (i * 7) % 17) % n
        col = ICON_BG["b"] if i % 3 else ICON_BG["c"]
        for k in range(2 + i % 3):
            px[x + k, y] = rgb(col)
    return im


def make_icons():
    fg = icon_medallion(108)
    bg = icon_background(108)
    put("icon_foreground.png", fg.resize((432, 432), Image.NEAREST), limited=20)
    put("icon_background.png", bg.resize((432, 432), Image.NEAREST), limited="opaque")
    hexbox = fg.crop((54 - 36, 54 - 32, 54 + 36, 54 + 32))
    z = 2
    legacy = Image.new("RGBA", (192, 192), (0, 0, 0, 0))
    big = hexbox.resize((hexbox.width * z, hexbox.height * z), Image.NEAREST)
    legacy.alpha_composite(big, ((192 - big.width) // 2, (192 - big.height) // 2))
    put("ic_launcher.png", legacy, limited=20)
    n = 96
    store = icon_background(n)
    store.alpha_composite(hexbox, ((n - hexbox.width) // 2, (n - hexbox.height) // 2))
    store = store.resize((n * 6, n * 6), Image.NEAREST).crop((32, 32, 544, 544))
    put("market_icon.png", store, limited="opaque23")


# ---------------------------------------------------------------- authored village and city

SHADOW_ALPHA = 96
ROOF_LIGHT, ROOF_MID, ROOF_EDGE, ROOF_SHADE, ROOF_DEEP = "F49AC1", "F49AC1", "EC008C", "9E005D", "690039"
B_OUT = "2A1A0E"


def poly(d, pts, fill, outline=B_OUT):
    d.polygon(pts, fill=rgb(fill), outline=rgb(outline) if outline else None)


def line(d, p0, p1, col):
    d.line([p0, p1], fill=rgb(col))


def gable_house(im, x0, yb, fw, wh, rise, dx, dy, c):
    """Oblique 3/4 house with the gable end facing the viewer. Front wall pentagon on the
    left, the roof slope running back up to the right, the shaded side wall under it.
    (x0, yb) is the bottom left pixel of the front wall. Light from the top left. The roof is
    drawn only in team magenta, banded parallel to the ridge. No outline yet: the caller adds
    it with finish_outline once everything is drawn."""
    d = ImageDraw.Draw(im)
    xr = x0 + fw - 1
    yt = yb - wh + 1                                   # first full width row of the front wall
    ax, ay = x0 + (fw - 1) // 2, yt - rise             # gable apex
    B = (xr, yt)
    # side wall under the roof's eave
    d.polygon([B, (xr + dx, yt - dy), (xr + dx, yb - dy), (xr, yb)], fill=rgb(c["side"]))
    # roof slope, banded per pixel by u (distance down the slope from the ridge)
    quad = [(ax, ay), B, (xr + dx, yt - dy), (ax + dx, ay - dy)]
    d.polygon(quad, fill=rgb(ROOF_MID))
    ex, ey = B[0] - ax, B[1] - ay
    det = ex * (-dy) - ey * dx
    bands = [ROOF_LIGHT, ROOF_MID, ROOF_MID, ROOF_EDGE, ROOF_MID, ROOF_MID, ROOF_EDGE, ROOF_SHADE]
    # the band count follows the slope length so rows stay about 2 px thick
    n = max(4, min(len(bands), int(round(math.hypot(ex, ey) / 2.2))))
    bands = bands[:n - 1] + [ROOF_SHADE]
    px = im.load()
    fill = rgb(ROOF_MID)
    for yy in range(im.height):
        for xx in range(im.width):
            if px[xx, yy] != fill:
                continue
            rx, ry = xx + 0.5 - ax, yy + 0.5 - ay
            u = (rx * (-dy) - ry * dx) / float(det)
            px[xx, yy] = rgb(bands[min(max(int(u * n), 0), n - 1)])
    # front wall pentagon
    d.polygon([(x0, yt), (ax, ay), (xr, yt), (xr, yb), (x0, yb)], fill=rgb(c["front"]))
    # rake boards: lit on the left, dark on the right where it meets the roof slope
    half = (fw - 1) / 2.0
    for yy in range(ay + 1, yt + 1):
        t = (yy - ay) / float(rise)
        off = int(round(half * t))
        d.point((ax - off, yy), fill=rgb(ROOF_LIGHT))
        d.point((ax - off + 1, yy), fill=rgb(ROOF_MID))
        d.point((ax + off, yy), fill=rgb(ROOF_SHADE))
        d.point((ax + off - 1, yy), fill=rgb(ROOF_EDGE))
    d.line([B, (xr + dx, yt - dy)], fill=rgb(ROOF_DEEP))               # eave line
    d.line([(x0 + 1, yt + 1), (xr - 1, yt + 1)], fill=rgb(c["shade"]))  # shade under the eave
    d.point((ax, ay), fill=rgb(ROOF_LIGHT))
    return dict(xr=xr, yt=yt, ax=ax, ay=ay)


def finish_outline(im):
    """Add a 1 px dark outline around the opaque silhouette (4-neighbourhood)."""
    w, h = im.size
    src = im.copy()
    sp, dp = src.load(), im.load()
    for y in range(h):
        for x in range(w):
            if sp[x, y][3] == 0:
                for nx, ny in ((x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)):
                    if 0 <= nx < w and 0 <= ny < h and sp[nx, ny][3] == 255:
                        dp[x, y] = rgb(B_OUT)
                        break


def ground_shadow(im, pts):
    sh = Image.new("RGBA", im.size, (0, 0, 0, 0))
    ImageDraw.Draw(sh).polygon(pts, fill=(0, 0, 0, SHADOW_ALPHA))
    sh.alpha_composite(im)
    return sh


def crop_to_content(im, anchor):
    bbox = im.getbbox()
    out = im.crop(bbox)
    return out, (anchor[0] - bbox[0], anchor[1] - bbox[1])


def footprint_shadow(x, yb, xr, dx, dy):
    """Base footprint nudged 2 px right and 1 px down: only the sliver that peeks out
    from under the building shows."""
    return [(x + 2, yb + 1), (xr + 2, yb + 1), (xr + dx + 2, yb - dy + 1), (x + dx + 2, yb - dy + 1)]


SHUTTER = "A8743C"


def make_village():
    c = dict(front="F0E0B8", side="C0AC80", shade="D8C494")
    im = Image.new("RGBA", (30, 30), (0, 0, 0, 0))
    x0, yb, fw, wh, rise, dx, dy = 3, 24, 11, 8, 5, 6, 3
    g = gable_house(im, x0, yb, fw, wh, rise, dx, dy, c)
    d = ImageDraw.Draw(im)
    xr, yt = g["xr"], g["yt"]
    d.line([(x0, yb), (xr, yb)], fill=rgb("9C9488"))                         # stone footing
    d.line([(xr + 1, yb), (xr + dx, yb - dy)], fill=rgb("6E665A"))
    d.line([(x0 + 1, yt + 2), (x0 + 1, yb - 1)], fill=rgb("7A5230"))         # timber posts
    d.line([(xr - 1, yt + 2), (xr - 1, yb - 1)], fill=rgb("7A5230"))
    d.line([(x0 + 1, yt + 1), (xr - 1, yt + 1)], fill=rgb("7A5230"))         # beam
    d.rectangle([x0 + 3, yb - 4, x0 + 4, yb - 1], fill=rgb(B_OUT))           # door, left of centre
    d.point((x0 + 4, yb - 2), fill=rgb("E6B422"))
    # windows with their wooden shutters swung open: dark opening between two leaves
    d.line([(x0 + 7, yt + 3), (x0 + 7, yt + 4)], fill=rgb(B_OUT))
    d.line([(x0 + 6, yt + 3), (x0 + 6, yt + 4)], fill=rgb(SHUTTER))
    d.line([(x0 + 8, yt + 3), (x0 + 8, yt + 4)], fill=rgb(SHUTTER))
    d.line([(xr + 3, yt + 1), (xr + 3, yt + 3)], fill=rgb(B_OUT))
    d.line([(xr + 2, yt + 1), (xr + 2, yt + 3)], fill=rgb(SHUTTER))
    d.line([(xr + 4, yt), (xr + 4, yt + 2)], fill=rgb(SHUTTER))
    # chimney sitting on the ridge
    cx, cy = g["ax"] + 3, g["ay"] - 2
    d.rectangle([cx, cy - 2, cx + 2, cy + 1], fill=rgb("9C9488"))
    d.line([(cx + 2, cy - 2), (cx + 2, cy + 1)], fill=rgb("6E665A"))
    d.line([(cx, cy - 2), (cx + 1, cy - 2)], fill=rgb("F0E0B8"))
    anchor = ((x0 + xr + dx) // 2, yb)
    finish_outline(im)
    return crop_to_content(ground_shadow(im, footprint_shadow(x0, yb, xr, dx, dy)), anchor)


def cone_roof(d, x0, x1, base_y, apex_y):
    """Round tower roof: a cone in team magenta, lit left half, shaded right half, ringed.
    Drawn row by row so the flanks step evenly, one pixel out every two rows, and with no
    outline of its own: finish_outline gives it the same single pixel edge as the rest."""
    mid = (x0 + x1) // 2
    h = base_y - apex_y
    for k in range(h + 1):
        yy = apex_y + k
        half = min((k + 1) // 2, mid - x0 + 1)
        ring = k in (h // 3 + 1, 2 * h // 3 + 1)
        for xx in range(mid - half, mid + half + 1):
            if k == h:
                col = ROOF_DEEP                                    # eave
            elif xx <= mid:
                col = ROOF_EDGE if ring else ROOF_MID
            else:
                col = ROOF_DEEP if ring else ROOF_SHADE
            d.point((xx, yy), fill=rgb(col))
    for k in (2, 3):                                               # glint down the lit flank
        d.point((mid - 1, apex_y + k), fill=rgb(ROOF_LIGHT))


def make_castle():
    """Small castle, front on: two round towers with magenta cone roofs, a crenellated
    curtain wall, a raised gatehouse with an arch and a magenta banner over the gate. The
    towers carry arrow slits, no glass."""
    im = Image.new("RGBA", (40, 40), (0, 0, 0, 0))
    d = ImageDraw.Draw(im)
    st, lt, sh, cs = rgb("9C9488"), rgb("C8C0B0"), rgb("6E665A"), rgb("8A8276")
    door = rgb("4A2C14")
    out = rgb(B_OUT)
    yb, ox = 33, 1                     # ox: everything sits 1 px in so the left eave keeps its outline
    # curtain wall between the towers, with merlons
    d.rectangle([8 + ox, 24, 16 + ox, yb], fill=st)
    for x in range(8, 17, 2):
        d.rectangle([x + ox, 22, x + ox, 23], fill=st)
    d.line([(8 + ox, 24), (16 + ox, 24)], fill=lt)
    for yy in (28, 31):
        d.line([(8 + ox, yy), (16 + ox, yy)], fill=cs)
    # gatehouse, raised, centred on x = 12
    d.rectangle([9 + ox, 18, 15 + ox, yb], fill=st)
    for x in (9, 11, 13, 15):
        d.rectangle([x + ox, 16, x + ox, 17], fill=st)
    d.line([(9 + ox, 18), (15 + ox, 18)], fill=lt)
    d.line([(9 + ox, 19), (9 + ox, yb)], fill=lt)
    d.line([(15 + ox, 19), (15 + ox, yb)], fill=sh)
    for yy in (22, 25):
        d.line([(10 + ox, yy), (14 + ox, yy)], fill=cs)
    d.rectangle([10 + ox, 28, 14 + ox, yb], fill=out)              # gate arch
    d.rectangle([11 + ox, 27, 13 + ox, 27], fill=out)
    d.rectangle([11 + ox, 29, 13 + ox, yb], fill=door)
    for x in (11, 13):
        d.line([(x + ox, 28), (x + ox, yb)], fill=out)
    # banner hung over the gate: team magenta, swallow tailed
    bx0, bx1, by0 = 10 + ox, 14 + ox, 19
    for yy in range(by0, by0 + 6):
        for xx in range(bx0, bx1 + 1):
            if yy == by0 + 5 and xx == (bx0 + bx1) // 2:
                continue
            col = ROOF_LIGHT if yy == by0 else (ROOF_SHADE if xx == bx1 else
                                                (ROOF_EDGE if yy >= by0 + 4 else ROOF_MID))
            d.point((xx, yy), fill=rgb(col))
    d.line([(bx0 - 1, by0 - 1), (bx1 + 1, by0 - 1)], fill=out)
    # towers: the body stops 1 px short of x0 and x1, where finish_outline lays a single edge
    for (x0, x1) in ((1 + ox, 7 + ox), (17 + ox, 23 + ox)):
        tt = 16
        d.rectangle([x0 + 1, tt + 1, x1 - 1, yb], fill=st)
        inner = x1 if x0 < 12 else x0                              # seam against the wall
        d.line([(inner, tt + 1), (inner, yb)], fill=out)
        d.rectangle([x0 + 1, tt + 1, x0 + 2, yb], fill=lt)
        d.rectangle([x1 - 2, tt + 1, x1 - 1, yb], fill=sh)
        for yy in range(tt + 4, yb, 4):
            d.line([(x0 + 3, yy), (x1 - 3, yy)], fill=cs)
        mid = (x0 + x1) // 2
        d.line([(mid, tt + 3), (mid, tt + 6)], fill=out)           # arrow slits
        d.line([(mid, tt + 10), (mid, tt + 13)], fill=out)
        cone_roof(d, x0, x1, tt, tt - 9)
    anchor = (12 + ox, yb)
    finish_outline(im)
    foot = [(x + ox, y) for (x, y) in
            ((1, yb + 1), (23, yb + 1), (26, yb), (26, yb + 1), (24, yb + 2), (2, yb + 2))]
    return crop_to_content(ground_shadow(im, foot), anchor)


MERCHANT = ["..hh..",
            ".hhhh.",
            "hhhhhh",
            ".ffff.",
            ".fkff.",
            ".LMMS.",
            "LMMMES",
            "LMMEES",
            "LMMEES",
            "LMMEES",
            "LMEEES",
            "MMEESD",
            "MEESSD",
            ".gg.gg",
            ".gg.gg",
            ".kk.kk"]


CART_L, CART_M, CART_D, CART_T = "C8955A", "A8743C", "6B4420", "4A2E14"
CART_HUB = "F0E0B8"
CARGO_SIZE = 12
TRADER_CARGO = {}      # filled by make_trader: top-left of the 12 px cargo icon, sprite coordinates


def make_trader():
    """A merchant leading a low open hand cart. The cart is seen from the front and a little
    above: one long side plank facing us (with a team magenta stripe), the flat bed above it as a
    four row parallelogram receding to the right, a low back rail framing the bed and two wheels.
    No canopy. The game draws a 12 px resource icon on the bed, the bottom row of its 12 px cell on
    the front bed row, so the cart is plain wood when empty. Light comes from the top left."""
    im = Image.new("RGBA", (40, 40), (0, 0, 0, 0))
    d = ImageDraw.Draw(im)
    g = 24                                        # last row of wheels and boots
    cx0, cx1 = 10, 26                             # side plank, first and last column
    top = g - 7                                   # first plank row
    L, M, D, T = (rgb(c) for c in (CART_L, CART_M, CART_D, CART_T))
    # bed surface: four rows, front row first, each receding row shifted right
    shifts = (0, 1, 1, 2)
    for k, sh in enumerate(shifts):
        y = top - 1 - k
        d.line([(cx0 + sh, y), (cx1 + sh, y)], fill=L if k < 3 else M)
    for x in range(cx0 + 4, cx1 + 1, 5):          # plank seams and a little grain on the bed
        d.point((x, top - 1), fill=M)
    for x in range(cx0 + 8, cx1 + 1, 6):
        d.point((x, top - 3), fill=M)
    # back rail: lit top edge and its inner face, rising two rows above the bed's back edge
    d.line([(cx0 + 2, top - 6), (cx1 + 2, top - 6)], fill=L)
    d.line([(cx0 + 2, top - 5), (cx1 + 2, top - 5)], fill=D)
    # side plank (front): lit top, magenta trim stripe, shaded foot
    d.line([(cx0, top), (cx1, top)], fill=M)
    d.line([(cx0, top + 1), (cx1, top + 1)], fill=rgb(ROOF_EDGE))
    d.line([(cx0, top + 2), (cx1, top + 2)], fill=D)
    # right end board, a thin shaded face going back
    for r in range(3):
        d.point((cx1 + 1, top + r - 1), fill=D)
        d.point((cx1 + 2, top + r - 2), fill=D)
    d.point((cx1 + 1, top - 2), fill=D)
    # wheels, 6 px: dark tyre, wood spokes, cream hub
    for wx in (cx0 + 1, cx1 - 6):
        for yy in range(6):
            for xx in range(6):
                r = math.hypot(xx - 2.5, yy - 2.5)
                if r > 3.0:
                    continue
                if r > 2.2:
                    c = T
                elif r < 1.0:
                    c = rgb(CART_HUB)
                elif xx in (2, 3) and yy in (2, 3):
                    c = rgb(CART_HUB)
                else:
                    c = M if (xx in (2, 3) or yy in (2, 3)) else D
                d.point((wx + xx, g - 5 + yy), fill=c)
    # merchant, left of the cart; his hand is on the shaft
    col = {"L": ROOF_LIGHT, "M": ROOF_MID, "E": ROOF_EDGE, "S": ROOF_SHADE, "D": ROOF_DEEP,
           "f": "E0B890", "g": CART_D, "h": CART_M, "k": B_OUT}
    mx, my = 3, g - 15
    for j, row in enumerate(MERCHANT):
        for i, ch in enumerate(row):
            if ch != ".":
                d.point((mx + i, my + j), fill=rgb(col[ch]))
    d.line([(mx + 6, top + 1), (cx0, top + 1)], fill=D)            # shaft from his hand
    d.point((mx + 6, top), fill=D)
    finish_outline(im)
    foot = [(mx + 1, g + 1), (cx1 + 2, g + 1), (cx1 + 3, g), (cx1 + 3, g + 2), (mx + 2, g + 2)]
    full = ground_shadow(im, foot)
    bbox = full.getbbox()
    # cargo: the 12 px cell is centred on the bed and its bottom row sits on the front bed row
    cargo = (cx0 + 1 + (cx1 - cx0 + 1 - CARGO_SIZE) // 2, top - 1 - (CARGO_SIZE - 1))
    TRADER_CARGO["pos"] = (cargo[0] - bbox[0], cargo[1] - bbox[1])
    xs = [x for x in range(full.width) for y in range(full.height) if full.getpixel((x, y))[3] > 128]
    anchor = ((min(xs) + max(xs)) // 2 - bbox[0], g - bbox[1])
    return full.crop(bbox), anchor


# ---------------------------------------------------------------- King's ship turn tracker
# The HUD's row of turn boxes becomes a strip of sea: a finished turn is a flag (with its score
# on it) or a lowered red rag, the current turn is the King's ship and the turns to come are
# small buoys in the player's colour, with a pier at the right-hand end where the ship arrives on the last turn.

TRACK_WATER = dict(b="1E4C5C", e="143644", t="3C7A8C", u="2A6272")
TRACK_WATER_ROWS = ["eeeeeeeeeeeeee",
                    "bbbbbbbbbbbbbb",
                    "bbbbbbbbbbbbub",
                    "bbtttbbbbbbbbb",
                    "bbbbbbbbbbbbbb",
                    "bbbbbbbbbbbbbb",
                    "bbbbbbbbuubbbb",
                    "bbbbbbbbbbtttb",
                    "bbbbbbbbbbbbbb",
                    "bbbbbbbbbbbbbb",
                    "bbbbuuubbbbbbb",
                    "bbbbbbbbbbbbbb",
                    "eeeeeeeeeeeeee"]
# a turn still to come: a small buoy in the player's colour (team magenta F49AC1, the reference shade that becomes the player colour itself, for the float
# and its top mark, EC008C for its shaded side, recoloured at draw time like the road glyph), with
# a fixed dark outline and waterline so it stays clear of the sea for a light or a dark colour
TRACK_BUOY = dict(o="2A1A0E", k="F49AC1", K="EC008C")
TRACK_BUOY_ROWS = ["..k..",
                   "..o..",
                   ".ooo.",
                   "okkko",
                   "okkKo",
                   "oKKKo",
                   ".ooo."]
# flag on a post: the post is the left edge, the ivory field is 11 x 7 (plus a shaded foot row)
TRACK_FLAG = dict(o="2A1A0E", p="E0B870", s="B8914F", K="8B5A2B")
TRACK_FLAG_ROWS = ["Koooooooooooo",
                   "Kpppppppppppo",
                   "Kpppppppppppo",
                   "Kpppppppppppo",
                   "Kpppppppppppo",
                   "Kpppppppppppo",
                   "Kpppppppppppo",
                   "Kssssssssssso",
                   "Koooooooooooo",
                   "K............",
                   "K............",
                   "K............",
                   "K............"]
TRACK_EMPTY = dict(o="2A1A0E", r="A04A3A", R="6E2E22", K="8B5A2B")
TRACK_EMPTY_ROWS = ["K............",
                    "K............",
                    "K............",
                    "K............",
                    "Koooooooooo..",
                    "KrrrrrrrrRRo.",
                    "KrrRrrrrRoRo.",
                    "KRrrrrrRo.oo.",
                    "Kooorrro.....",
                    "K..oRRo......",
                    "K...oo.......",
                    "K............",
                    "K............"]
# the King's ship as the board shows it: a galleon under full white sail heading right, its flag
# at the masthead in team magenta (P the reference pink, Q its shade) so it flies the player's colour
TRACK_SHIP = dict(o="2A1A0E", g="E6B422", P="F49AC1", Q="EC008C", w="F0E0B8", s="D8C494",
                  M="C8955A", d="5C3A1E")
TRACK_SHIP_ROWS = ["gPPPPPd.......",
                   ".QQQQQd.......",
                   "..dddddddddo..",
                   "..owwwwwwsoo..",
                   "..owgwgwgsoo..",
                   "..owgggggsowo.",
                   "..owwwwwwsowo.",
                   "..owwwwwssowwo",
                   "..oooooooooooo",
                   "ooMMMMMMMMMMMo",
                   ".odgdgdgdgddo.",
                   "..oddddddddo..",
                   "...oooooooo..."]
# the end of the voyage: a castle tower on a small island, its flag in team magenta like the ship's
TRACK_PIER = dict(o="2A1A0E", P="F49AC1", Q="EC008C", L="C8C0B0", D="6E665A",
                  G="6E9A3E", S="D8C08A", f="C8E0E0")
TRACK_PIER_ROWS = ["..............",          # open water over the flag
                   ".....oPPP.....",
                   ".....oQQ......",
                   "...oLoLoLo....",
                   "...oLLLLLo....",
                   "...oLLLLDo....",
                   "...oLooLDo....",
                   "...oLooLDo....",
                   "...oLooLDo....",
                   ".GGGGGGGGGGGG.",
                   "fSSSSSSSSSSSSf",
                   ".ff.f....f.ff.",
                   ".............."]          # the strip's bottom row stays open water, clear of the frame


def make_track():
    put("track_water.png", art(TRACK_WATER_ROWS, TRACK_WATER, (14, 13)))
    put("track_buoy.png", art(TRACK_BUOY_ROWS, TRACK_BUOY, (5, 7)))
    put("track_flag.png", art(TRACK_FLAG_ROWS, TRACK_FLAG, (13, 13)))
    put("track_flag_empty.png", art(TRACK_EMPTY_ROWS, TRACK_EMPTY, (13, 13)))
    put("track_ship.png", art(TRACK_SHIP_ROWS, TRACK_SHIP, (14, 13)))
    put("track_pier.png", art(TRACK_PIER_ROWS, TRACK_PIER, (14, 13)))


# ---------------------------------------------------------------- the King's ship on the board

KINGSHIP = "data/core/images/units/transport/transport-galleon.png"
# The flag's shades, dark to light, in team magenta so the flag flies the player's colour
# (F49AC1, the lightest, is the colour itself)
FLAG_TEAM = ["690039", "8C0051", "B10069", "EC008C", "F49AC1"]
ROYAL_GOLD = "E6B422"


def is_team_colour(p):
    """Wesnoth team colour, exact or antialiased: magenta, so red and blue clearly above green."""
    return p[0] > p[1] + 28 and p[2] > p[1] + 14 and p[0] > 100


def make_kingship():
    """Wesnoth 1.16's transport galleon exactly as shipped: its own soft alpha channel (the
    antialiased edges and the faint wake that is part of the sprite) is kept untouched, nothing is
    scaled, the crop is the bounding box of every pixel with alpha above 0, and no shadow is added
    (the wake is the water line). Only the team colour flag is redrawn, in team magenta shades
    (recoloured to the player's colour at draw time) with a gold fly; each pixel keeps its own alpha. Returns (image, crop box)."""
    src = read_wesnoth_pkg(KINGSHIP).convert("RGBA")
    out = src.copy()
    sp, op = src.load(), out.load()
    team = []
    for y in range(src.height):
        for x in range(src.width):
            p = sp[x, y]
            if p[3] and is_team_colour(p):
                team.append((x, y))
    # team colour: brightness picks the shade, the fly end of each row of the flag is gold
    lum = {q: sum(sp[q][:3]) for q in team}
    lo, hi = min(lum.values()), max(lum.values())
    rows = {}
    for (x, y) in team:
        rows.setdefault(y, []).append(x)
    for (x, y) in team:
        t = (lum[(x, y)] - lo) / float(max(1, hi - lo))
        c = FLAG_TEAM[min(len(FLAG_TEAM) - 1, int(t * len(FLAG_TEAM)))]
        if x >= max(rows[y]) - 1 and y >= min(rows):
            c = ROYAL_GOLD
        op[x, y] = rgb(c)[:3] + (sp[x, y][3],)
    bbox = out.getchannel("A").point(lambda v: 255 if v else 0).getbbox()
    return out.crop(bbox), bbox


# ---------------------------------------------------------------- Wesnoth pieces

def read_wesnoth(rel):
    p = os.path.join(WESNOTH, rel)
    wesnoth_read.append(rel)
    return Image.open(p)


PACKAGE = "/usr/share/games/wesnoth/1.16"
PACKAGE_TAG = "1.16.11"
PACKAGE_COMMIT = "83d4b397f1de466c721e4e393b361c71fc4f5b6c"
# Licence and author of every Wesnoth file this script reads. The installed 1.16 files carry no
# record of their own, so these rows are copied from the last revision of Wesnoth's copyrights.csv
# that still listed images (d4b2d151ce3, 2024-08-23; since then Wesnoth keeps the record in each
# PNG's EXIF Copyright and Artist tags). The MD5 is that row's, and the file read must match it.
WESNOTH_LICENCES = {
    "data/core/images/units/transport/transport-galleon.png":
        ("GNU GPL v2+", "FreeCol", "5bd94d18992557802578ce1e7bc41947"),
}
wesnoth_md5 = {}    # path -> MD5 of the bytes actually read


def read_wesnoth_pkg(rel):
    """Read one file from the installed Wesnoth package (WESNOTH_PKG overrides the path)."""
    p = os.path.join(os.environ.get("WESNOTH_PKG", PACKAGE), rel)
    wesnoth_read.append(rel)
    with open(p, "rb") as fh:
        wesnoth_md5[rel] = hashlib.md5(fh.read()).hexdigest()
    return Image.open(p)


def package_version():
    try:
        return subprocess.check_output(["dpkg-query", "-W", "-f=${Package} ${Version}",
                                        "wesnoth-1.16-data"], text=True,
                                       stderr=subprocess.DEVNULL).strip()
    except (OSError, subprocess.CalledProcessError):
        return "wesnoth 1.16 installed data (package version unknown)"


def extract_pieces():
    notes = []
    for name, (im, anchor), what in (
            ("piece_village.png", make_village(), "cottage; roof"),
            ("piece_castle.png", make_castle(), "castle; tower cone roofs and the gate banner"),
            ("piece_trader.png", make_trader(), "merchant and open hand cart; cloak and the cart's side stripe")):
        put(name, im, limited="piece")
        notes.append("%s: %dx%d, authored in make_ui.py (%s in team magenta F49AC1, "
                     "EC008C, 9E005D, 690039 so it takes the player colour). Anchor (bottom "
                     "centre of the base) = (%d, %d). A ground shadow of alpha 96 trails right "
                     "and below; use alpha > 128 for a footprint."
                     % (name, im.width, im.height, what, anchor[0], anchor[1]))
        ANCHORS[name] = anchor
    ship, sbbox = make_kingship()
    put("piece_kingship.png", ship, limited="wesnoth")
    solid = ship.getchannel("A").point(lambda v: 255 if v >= 128 else 0).getbbox()
    notes.append(
        "piece_kingship.png: %dx%d, Wesnoth 1.16's transport galleon (units/transport/"
        "transport-galleon.png from the installed 1.16 data, GNU GPL v2+, recorded author FreeCol; "
        "NOT the redrawn sprite of the same name in newer Wesnoth) cropped to the bounding box of "
        "every pixel with alpha above 0, its own soft alpha channel kept exactly as shipped "
        "(antialiased edges and the faint wake stay, no shadow added, nothing scaled), the team "
        "colour flag in team magenta and gold (so it IS player coloured: recolour it at draw time). The pixels with alpha of 128 or more, the ship itself, fill "
        "the box x %d to %d, y %d to %d (%d x %d); the rest is wake. It heads to the lower right, "
        "stern at the upper left, keel at about 3 across for 1 down. To moor, put the ship's "
        "CONTACT pixel (%d, %d), the tip of its bowsprit, on the dock's mooring point (the dock is "
        "baked into the board by tools/bake_board.py, which also writes its above water art as "
        "dock.png, drawn over the roads and under the ship; dock_mooring in "
        "tools/board_geometry_*.txt): the ship then lies in open water to the left of the jetty "
        "with its bow at it: only the tip of its bowsprit, three columns, lies over the deck's west "
        "edge; the rest of the ship is clear of the dock and its piles."
        % (ship.width, ship.height, solid[0], solid[2] - 1, solid[1], solid[3] - 1,
           solid[2] - solid[0], solid[3] - solid[1], SHIP_MOOR_CONTACT[0], SHIP_MOOR_CONTACT[1]))
    notes.append("icon_foreground.png and icon_background.png: 432x432, the adaptive icon layers drawn on a "
                 "108x108 art pixel canvas at exactly 4x nearest neighbour. The background is opaque deep "
                 "sea (0D3947) with a few quiet ticks. The foreground is transparent outside a flat topped "
                 "hexagon medallion 67 art px (268 px) point to point, centred, just inside the 72 dp circle, "
                 "with a 1.7 px dark outline, a 2 px lighter rim and an island scene of seven terrain hexes in perspective (mountain, desert, hills, farmland, lake, pasture, forest) with the castle, 20 opaque colours. "
                 "ic_launcher.png (written to res/mipmap-xxxhdpi): 192x192, transparent, the same medallion at 3x (192 x 168 hexagon box), the "
                 "legacy launcher icon. store/market_icon.png: 512x512, opaque, the medallion at 6x on the "
                 "sea. res/mipmap-anydpi-v26/ic_launcher.xml joins the two layers.")
    notes.append("track_*.png: HUD turn tracker sprites, fixed colours: track_water.png 14x13 "
                 "(tiles horizontally, seamless), track_buoy.png 5x7 (a turn still to come: a buoy in team magenta F49AC1 and EC008C, so it takes the player colour through the recolour, with a fixed dark outline), track_flag.png 13x13 "
                 "(post on the left edge, clear ivory field at sprite x 1..11, y 1..7, 11x7, "
                 "for a 5 px tall number), track_flag_empty.png 13x13, track_ship.png 14x13, "
                 "track_pier.png 14x13.")
    return notes


def check_licences():
    """Fail loudly unless every Wesnoth file read has a recorded licence and author, and is the
    very file that record describes."""
    for rel in wesnoth_read:
        if rel not in WESNOTH_LICENCES:
            sys.exit("FAIL: %s has no licence and author recorded in WESNOTH_LICENCES" % rel)
        want = WESNOTH_LICENCES[rel][2]
        if wesnoth_md5.get(rel) != want:
            sys.exit("FAIL: %s has MD5 %s, the licence record is for %s" % (rel, wesnoth_md5.get(rel), want))


# ---------------------------------------------------------------- checks and writing

def count_colours(im):
    return len(set(p[:3] for p in im.getdata() if p[3] > 0))


OBSOLETE = ["track_tick.png", "piece_dock.png", "glyph_city_12.png", "glyph_knight_12.png", "piece_city.png", "piece_knight.png",
            "qmark.png"]


def write_outputs():
    os.makedirs(OUT, exist_ok=True)
    for name in OBSOLETE:
        if os.path.exists(os.path.join(OUT, name)):
            os.remove(os.path.join(OUT, name))
    for name, (im, limited) in sorted(outputs.items()):
        if limited == "piece":
            # authored building: at most 8 non-magenta opaque colours, at least 4 team magenta
            # shades (the roof), a shadow of alpha 96 and nothing else partial.
            mag = set(rgb(h)[:3] for h in TEAM_MAGENTA)
            opaque = set(p[:3] for p in im.getdata() if p[3] == 255)
            plain = opaque - mag
            used = opaque & mag
            assert len(plain) <= 8, "%s has %d non-magenta colours (max 8)" % (name, len(plain))
            assert len(used) >= 4, "%s uses only %d magenta shades (need 4)" % (name, len(used))
            assert set(p[3] for p in im.getdata()) <= {0, SHADOW_ALPHA, 255}, name
        elif limited in (16, 20):
            n = count_colours(im)
            assert n <= limited, "%s has %d opaque colours (max %d)" % (name, n, limited)
            assert set(p[3] for p in im.getdata()) <= {0, 255}, name
        elif limited in ("opaque", "opaque23"):
            n = count_colours(im)
            top = 23 if limited == "opaque23" else 4          # sea only, or sea plus the 20 colour medallion
            assert n <= top, "%s has %d colours (max %d)" % (name, n, top)
            assert set(p[3] for p in im.getdata()) == {255}, name
        elif limited == "wesnoth":
            pass            # Wesnoth's own antialiased art, shipped as is: no colour or alpha limits
        elif limited in ("shadow", "ship"):
            assert set(p[3] for p in im.getdata()) <= {0, SHADOW_ALPHA, 255}, name
            if limited == "shadow":
                n = len(set(p[:3] for p in im.getdata() if p[3] == 255))
                assert n <= 8, "%s has %d opaque colours (max 8)" % (name, n)
        elif limited:
            n = count_colours(im)
            assert n <= 8, "%s has %d opaque colours (max 8)" % (name, n)
            for p in im.getdata():
                assert p[3] in (0, 255), "%s has partial alpha" % name
        # The store listing icon is not an app resource, so it stays out of the APK.
        if name == "market_icon.png":
            dest = os.path.join(ROOT, "store")
        elif name == "ic_launcher.png":
            # The legacy launcher icon; Android 8 and later use the adaptive layers instead.
            dest = os.path.join(ROOT, "app/src/main/res/mipmap-xxxhdpi")
        else:
            dest = OUT
        os.makedirs(dest, exist_ok=True)
        im.save(os.path.join(dest, name), format="PNG")


def dark_kit_notes():
    D = DARK
    f = D["face"]
    inf = D["field_face"]
    lines = [
        "dark kit: ui_panel_dark.png is 24x24, 6 px corners, 7 opaque colours, the same 9-slice as "
        "ui_panel.png, flat face (no grain in the centre cell, it is stretched in both directions). "
        "The existing ui_button_normal, ui_button_pressed and ui_button_disabled work on it as they "
        "are, so there are no ui_button_dark_* files: the brass bevel and caramel face stand clear "
        "of the dark face, and the 1E140A outline sits inside the panel's brass line.",
        "dark kit palette for code-drawn parts (hex, contrast ratio against the colour it sits on, "
        "WCAG relative luminance):",
        "  panel face       #%s  (frame outline #%s, rim #%s, brass line and studs #%s)"
        % (f, D["outline"], D["rim"], D["brass"]),
        "  ink (primary)    #%s  on panel face %.1f:1" % (D["ink"], contrast(D["ink"], f)),
        "  secondary text   #%s  on panel face %.1f:1" % (D["ink2"], contrast(D["ink2"], f)),
        "  disabled text    #%s  on panel face %.1f:1 (disabled, not required to reach 4.5)"
        % (D["disabled"], contrast(D["disabled"], f)),
        "  link             #%s  on panel face %.1f:1 (underline in the same colour)"
        % (D["link"], contrast(D["link"], f)),
        "  rule line        #%s  on panel face %.1f:1 (title rule and dividers, 1 px)"
        % (D["rule"], contrast(D["rule"], f)),
        "  recessed field   face #%s, top and left edge #%s, bottom and right edge #%s; ink on field "
        "face %.1f:1, secondary on field face %.1f:1"
        % (inf, D["field_edge_dark"], D["field_edge_light"], contrast(D["ink"], inf),
           contrast(D["ink2"], inf)),
        "  selection mark   #%s  on field face %.1f:1 (radio dot, caret)"
        % (D["mark"], contrast(D["mark"], inf)),
        "  slider           track = recessed field colours, thumb face #%s with a 1 px #%s top edge; "
        "thumb against panel face %.1f:1" % (D["thumb"], D["thumb_edge"], contrast(D["thumb"], f)),
        "  button label     #FFF4D6 on button face #A0783C %.1f:1 (unchanged)" % contrast("FFF4D6", FACE),
    ]
    return lines


def write_side_files(notes):
    with open(os.path.join(ROOT, "tools/wesnoth-sources-ui.txt"), "w") as fh:
        fh.write("# Wesnoth files read by tools/make_ui.py: only the ship below, for piece_kingship.png.\n")
        fh.write("# Every other UI sprite, icon, building, trader and road is authored in the script\n")
        fh.write("# (the village, castle and trader are original pixel art).\n")
        fh.write("# Read from the installed 1.16 data directory: %s\n" % package_version())
        fh.write("# which is Wesnoth tag %s, commit %s\n" % (PACKAGE_TAG, PACKAGE_COMMIT))
        fh.write("# Licence and author are Wesnoth's own record for the file: its row in copyrights.csv at\n")
        fh.write("# revision d4b2d151ce3 (the last to list images), matched by MD5. Master has a different sprite\n")
        fh.write("# under the same name (CC BY-SA 4.0), so do not swap this file without changing the licence text.\n")
        fh.write("# path | licence | author | MD5\n")
        for rel in sorted(set(wesnoth_read)):
            fh.write("%s | %s | %s | %s\n" % ((rel,) + WESNOTH_LICENCES[rel]))
    road_notes = [
        "road_h.png: 44x8. Place its top-left at (A.x - 4, A.y - 4) where A is the left vertex of the edge "
        "(N edge: NW vertex; S edge: SW vertex). Edge length 36 plus a 4 px round "
        "cap beyond each vertex, so three roads meeting at a vertex overlap in a round blob.",
        "road_slash.png: 26x44, '/' (bottom-left to top-right). Top-left = (BL.x - 4, TR.y - 4) where "
        "BL is the lower-left vertex of the edge and TR the upper-right one. The edge spans "
        "18 px across and 36 px down; the sprite adds 4 px of cap on every side.",
        "road_backslash.png: 26x44, '\\' (top-left to bottom-right). Top-left = (TL.x - 4, TL.y - 4).",
        "NOTE: on a flat-top hex the upper-right edge (named NE) runs top-left to bottom-right "
        "('\\'), so it takes road_backslash.png; the lower-right edge takes road_slash.png ('/'). The "
        "file names describe the shape of the road drawn in them, not a hex edge name.",
        "Road kerbs use magenta F49AC1 (body and lit stones, the player colour itself), EC008C (joints) so they take the "
        "player colour through the team-colour recolour.",
    ]
    tw, th = outputs["piece_trader.png"][0].size
    tax, tay = ANCHORS["piece_trader.png"]
    cgx, cgy = TRADER_CARGO["pos"]
    extra = [
        "Trader (merchant leading a low open hand cart, no canopy): piece_trader.png is %dx%d, anchor "
        "(bottom centre of the wheels and boots) = (%d, %d). Plant the anchor at the hex centre plus "
        "(0, %d) so cart and cargo look centred in a 72 px hex."
        % (tw, th, tax, tay, TRADER_HEX_OFFSET),
        "Trader cargo: draw the 12x12 resource icon with its top-left at sprite pixel (%d, %d), "
        "that is anchor + (%d, %d). The y is negative on purpose: the icon rises 3 px above the "
        "sprite's top row, so do not clip to the sprite bounds. The bottom row of the icon cell "
        "lands on the front row of the bed, so icons whose art ends a row or two above the cell "
        "bottom (wool, lumber, gold 1; ore, any 2; brick 3) rest on the bed 1 to 3 rows back, "
        "which still reads as standing on it. Draw nothing for an empty cart: the bare bed is "
        "plain planks. The bed is 17 px wide (sprite x 8 to 24), the icon is centred on it."
        % (cgx, cgy, cgx - tax, cgy - tay),
        "Trader spacing: two traders in one hex need at least 30 px between anchors to sit side by "
        "side without touching (the sprite is %d wide). At 16 or 22 px the right merchant stands on "
        "the left cart and hides its bed and cargo. If they must be closer, stagger them: offset "
        "the anchors by (-9, -5) and (+9, +5), 18 px apart across and 10 px down, which overlaps "
        "only a wheel and a boot and keeps both cargoes clear."
        % tw,
        "Trader footprint: alpha > 128 gives a figure on the left and a long low box with two "
        "bumps (wheels) under it on the right, about 28 x 18 px. It reads as a person with a cart "
        "if you know to look, and as a plain wide blob if you do not; keep a dark outline or the "
        "cargo icon in any hit-test or highlight overlay.",
        "Footprints: every piece has a ground shadow of alpha 96. Build a footprint silhouette "
        "from pixels with alpha > 128 (this drops the shadow and keeps one solid blob).",
        "glyph_trader_12.png: a flat hand cart carrying a sack (cream, tan, brown tie), kept in place "
        "of the old striped wagon; glyph_road_12.png: a cobbled road running diagonally across the cell with broad "
        "team magenta kerb stones (F49AC1, EC008C), recoloured to the player colour like the road sprites.",
        "icon_any_12.png and icon_any_16.png replace the Wesnoth qmark (any resource).",
        "glyph_castle_12.png and glyph_trader_12.png replace glyph_city_12.png and "
        "glyph_knight_12.png; piece_castle.png and piece_trader.png replace piece_city.png and "
        "piece_knight.png. The old files are deleted by the script.",
    ]
    with open(os.path.join(ROOT, "tools/pieces.txt"), "w") as fh:
        fh.write("\n".join(notes + road_notes + extra + dark_kit_notes()) + "\n")


# ---------------------------------------------------------------- preview

def recolor(im, player, palette=TEAM_MAGENTA):
    """Port of Wesnoth color_range recolor_palette: mid = player colour, max = white,
    min = black. First palette entry is the reference."""
    ref = rgb(palette[0])
    ref_avg = (ref[0] + ref[1] + ref[2]) // 3
    mp = {}
    for h in palette:
        c = rgb(h)
        avg = (c[0] + c[1] + c[2]) // 3
        if avg <= ref_avg:
            t = avg / float(ref_avg)
            new = tuple(int(round(0 + (player[i] - 0) * t)) for i in range(3))
        else:
            t = (255 - avg) / float(255 - ref_avg)
            new = tuple(int(round(255 + (player[i] - 255) * t)) for i in range(3))
        mp[c[:3]] = new
    out = im.copy()
    px = out.load()
    for y in range(out.height):
        for x in range(out.width):
            p = px[x, y]
            if p[3] and p[:3] in mp:
                px[x, y] = mp[p[:3]] + (p[3],)
    return out


def nine_stretch(src, w, h, b=6):
    out = Image.new("RGBA", (w, h), (0, 0, 0, 0))
    S = src.width
    c = S - 2 * b

    def piece(box, size):
        return src.crop(box).resize(size, Image.NEAREST)

    out.paste(piece((0, 0, b, b), (b, b)), (0, 0))
    out.paste(piece((S - b, 0, S, b), (b, b)), (w - b, 0))
    out.paste(piece((0, S - b, b, S), (b, b)), (0, h - b))
    out.paste(piece((S - b, S - b, S, S), (b, b)), (w - b, h - b))
    out.paste(piece((b, 0, S - b, b), (w - 2 * b, b)), (b, 0))
    out.paste(piece((b, S - b, S - b, S), (w - 2 * b, b)), (b, h - b))
    out.paste(piece((0, b, b, S - b), (b, h - 2 * b)), (0, b))
    out.paste(piece((S - b, b, S, S - b), (b, h - 2 * b)), (w - b, b))
    out.paste(piece((b, b, S - b, S - b), (w - 2 * b, h - 2 * b)), (b, b))
    return out


def label_font(size):
    try:
        return ImageFont.load_default(size=size)
    except TypeError:
        return ImageFont.load_default()


def stretched_button(state, w=150, h=20):
    b = nine_stretch(outputs["ui_button_%s.png" % state][0], w, h)
    t = Image.new("RGBA", b.size, (0, 0, 0, 0))
    td = ImageDraw.Draw(t)
    td.fontmode = "1"
    col = (255, 244, 214, 255) if state != "disabled" else (192, 188, 176, 255)
    td.text((w // 2, h // 2 + (1 if state == "pressed" else 0)), "ROLL", fill=col,
            font=label_font(8), anchor="mm")
    b.alpha_composite(t)
    return b


def build_scene(colours):
    """A to-scale mock of the board: a 3 hex wide patch of plain green 72 px hexes, roads
    on several edges, villages and cities planted on corners (anchor on the vertex) and the
    horseman in two hex centres, all recoloured with the team colour port."""
    ox, oy = 14, 36
    scene = Image.new("RGBA", (210, 236), (0, 0, 0, 0))
    centres = {"A": (36, 36), "B": (36, 108), "C": (90, 72), "D": (90, 144), "E": (144, 36),
               "F": (144, 108)}
    for k, (cx, cy) in centres.items():
        hx = Image.new("RGBA", (72, 72), (0, 0, 0, 0))
        hd = ImageDraw.Draw(hx)
        pts = [(18, 0), (54, 0), (71, 36), (54, 71), (18, 71), (0, 36)]
        hd.polygon(pts, fill=(96, 140, 60, 255) if k in "ADF" else (88, 132, 56, 255))
        hd.line(pts + [pts[0]], fill=(70, 112, 48, 255))
        scene.alpha_composite(hx, (ox + cx - 36, oy + cy - 36))
    rh, rsl, rbs = (outputs[n][0] for n in ("road_h.png", "road_slash.png", "road_backslash.png"))
    c0, c1 = colours

    def vert(k, name):
        cx, cy = centres[k]
        dx, dy = {"E": (36, 0), "NE": (18, -36), "NW": (-18, -36), "W": (-36, 0),
                  "SW": (-18, 36), "SE": (18, 36)}[name]
        return cx + dx, cy + dy

    def road(k, edge, col):
        if edge in ("N", "S"):
            a = vert(k, "NW" if edge == "N" else "SW")
            img, pos = rh, (a[0] - 4, a[1] - 4)
        elif edge == "NE":
            a = vert(k, "NE")
            img, pos = rbs, (a[0] - 4, a[1] - 4)
        elif edge == "SE":
            e, se = vert(k, "E"), vert(k, "SE")
            img, pos = rsl, (se[0] - 4, e[1] - 4)
        elif edge == "SW":
            w = vert(k, "W")
            img, pos = rbs, (w[0] - 4, w[1] - 4)
        else:
            w, nw = vert(k, "W"), vert(k, "NW")
            img, pos = rsl, (w[0] - 4, nw[1] - 4)
        scene.alpha_composite(recolor(img, col), (ox + pos[0], oy + pos[1]))

    for k, e in (("A", "NE"), ("A", "SE"), ("C", "N"), ("A", "S"), ("B", "NE")):
        road(k, e, c0)
    for k, e in (("C", "NE"), ("C", "SE"), ("C", "S"), ("D", "NE"), ("F", "N")):
        road(k, e, c1)
    pieces = []
    for key, col, k, name in (("piece_village.png", c0, "A", "E"), ("piece_castle.png", c0, "A", "SE"),
                              ("piece_castle.png", c1, "C", "E"), ("piece_village.png", c1, "C", "SE")):
        v = vert(k, name)
        pieces.append((v[1], key, col, v))
    for k, col in (("A", c0), ("F", c1)):
        cx, cy = centres[k]
        pieces.append((cy + TRADER_HEX_OFFSET, "piece_trader.png", col, (cx, cy + TRADER_HEX_OFFSET)))
    for _, key, col, v in sorted(pieces, key=lambda t: t[0]):
        spr = recolor(outputs[key][0], col)
        ax, ay = ANCHORS[key]
        scene.alpha_composite(spr, (ox + v[0] - ax, oy + v[1] - ay))
        if key == "piece_trader.png":
            cx_, cy_ = TRADER_CARGO["pos"]
            icon = "icon_wool_12.png" if col == c0 else "icon_ore_12.png"
            scene.alpha_composite(outputs[icon][0], (ox + v[0] - ax + cx_, oy + v[1] - ay + cy_))
    return scene


ICON_PLATES = [(42, 30, 20, 255), (242, 232, 204, 255)]
EXTRA_H = 9000

TRADER_HEX_OFFSET = 8       # plant the trader's anchor at the hex centre plus (0, 8)
LIGHT = dict(face="F0E0B8", ink="1E140A", ink2="5A3C14", rule="E0B870", field_face="D8C494",
             field_edge_dark="8C7448", field_edge_light="F8EED0", mark="1E140A", thumb="A0783C")
FONT_DIR = os.path.join(ROOT, "tools/fonts")


def pixel_font(name, size):
    try:
        return ImageFont.truetype(os.path.join(FONT_DIR, name), size)
    except OSError:
        return label_font(size)


def luminance(h):
    c = []
    for i in (0, 2, 4):
        v = int(h[i:i + 2], 16) / 255.0
        c.append(v / 12.92 if v <= 0.03928 else ((v + 0.055) / 1.055) ** 2.4)
    return 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]


def contrast(a, b):
    la, lb = luminance(a), luminance(b)
    if la < lb:
        la, lb = lb, la
    return (la + 0.05) / (lb + 0.05)


ALT_DARK_BUTTONS = {
    "normal": dict(O="120B05", L="F0CC84", F="B8894A", D="6A4A1E", W="FFF4D6", J="C89C58", K="9A7438"),
    "pressed": dict(O="120B05", L="8A6634", F="8A6634", D="4E3414", W="4E3414", J="4E3414", K="8A6634"),
    "disabled": dict(O="120B05", L="6E6454", F="504838", D="2E2618", W="6E6454", J="5C5444", K="403828"),
}


def dark_button_candidates():
    return {st: nine_from_corners(BTN_TL, BTN_TR, BTN_BL, BTN_BR, lg)
            for st, lg in ALT_DARK_BUTTONS.items()}


def dialog_mock(dark, buttons):
    """A 180x120 dialog at 1:1: stretched panel, title and rule, three lines of text, a radio
    row, a recessed field, a slider and two buttons. `buttons` maps state to a 24x24 9-slice."""
    pal = DARK if dark else LIGHT
    panel = nine_stretch(outputs["ui_panel_dark.png" if dark else "ui_panel.png"][0], 180, 120)
    d = ImageDraw.Draw(panel)
    d.fontmode = "1"
    body = pixel_font("ArkPixel10-Regular.otf", 10)
    title = pixel_font("PressStart2P-Regular.ttf", 8)
    d.text((90, 13), "Settings", fill=rgb(pal["ink"]), font=title, anchor="mm")
    d.line([(10, 21), (169, 21)], fill=rgb(pal["rule"]))
    d.text((10, 31), "Warn before building early", fill=rgb(pal["ink"]), font=body, anchor="lm")
    d.text((10, 40), "This forfeits the rolls you have", fill=rgb(pal["ink2"]), font=body, anchor="lm")
    link = DARK["link"] if dark else "2E6E8E"
    d.text((10, 49), "Read the rules", fill=rgb(link), font=body, anchor="lm")
    d.line([(10, 55), (46, 55)], fill=rgb(link))
    for i, (label, on) in enumerate((("Off", False), ("On", True))):
        x = 10 + i * 40
        d.rectangle([x, 60, x + 8, 68], fill=rgb(pal["field_edge_dark"]))
        d.rectangle([x + 1, 61, x + 8, 68], fill=rgb(pal["field_edge_light"]))
        d.rectangle([x + 1, 61, x + 7, 67], fill=rgb(pal["field_face"]))
        if on:
            d.rectangle([x + 3, 63, x + 5, 65], fill=rgb(pal["mark"]))
        d.text((x + 13, 64), label, fill=rgb(pal["ink"]), font=body, anchor="lm")
    d.rectangle([10, 74, 108, 83], fill=rgb(pal["field_edge_dark"]))
    d.rectangle([11, 75, 108, 83], fill=rgb(pal["field_edge_light"]))
    d.rectangle([11, 75, 107, 82], fill=rgb(pal["field_face"]))
    d.text((14, 79), "Tarnholm", fill=rgb(pal["ink"]), font=body, anchor="lm")
    d.rectangle([14 + 30, 77, 14 + 30, 80], fill=rgb(pal["mark"]))
    d.rectangle([116, 77, 169, 80], fill=rgb(pal["field_edge_dark"]))
    d.rectangle([117, 78, 168, 79], fill=rgb(pal["field_face"]))
    d.rectangle([138, 74, 144, 83], fill=rgb(pal["thumb"]))
    d.line([(138, 74), (144, 74)], fill=rgb(pal.get("thumb_edge", pal["rule"])))
    for j, label in enumerate(("OK", "Cancel")):
        b = nine_stretch(buttons["normal"], 74, 16)
        panel.alpha_composite(b, (10 + j * 86, 95))
        d.text((10 + j * 86 + 37, 103), label, fill=rgb(TEXT), font=body, anchor="mm")
    return panel


def place_trader(scene, centre, col, cargo, dx=0, dy=0):
    spr = recolor(outputs["piece_trader.png"][0], col)
    ax, ay = ANCHORS["piece_trader.png"]
    x = centre[0] + dx - ax
    y = centre[1] + TRADER_HEX_OFFSET + dy - ay
    scene.alpha_composite(spr, (x, y))
    if cargo:
        cx, cy = TRADER_CARGO["pos"]
        scene.alpha_composite(outputs[cargo][0], (x + cx, y + cy))


def hex_patch(scene, centre, shade=0):
    hx = Image.new("RGBA", (72, 72), (0, 0, 0, 0))
    hd = ImageDraw.Draw(hx)
    pts = [(18, 0), (54, 0), (71, 36), (54, 71), (18, 71), (0, 36)]
    hd.polygon(pts, fill=(96 - shade, 140 - shade, 60, 255))
    hd.line(pts + [pts[0]], fill=(70, 112, 48, 255))
    scene.alpha_composite(hx, (centre[0] - 36, centre[1] - 36))


def trader_mock(colours):
    """Hex patches at 1:1, shown at 4x by the caller. Rows 1 and 2: the trader empty, with wool,
    with ore and with the any icon, in two player colours. Row 3: two traders in one hex."""
    cell = 76
    scene = Image.new("RGBA", (4 * cell + 4, 3 * cell + 40), (0, 0, 0, 0))
    cargos = (None, "icon_wool_12.png", "icon_ore_12.png", "icon_any_12.png")
    for r, col in enumerate(colours):
        for i, cg in enumerate(cargos):
            c = (38 + i * cell, 38 + r * cell)
            hex_patch(scene, c)
            place_trader(scene, c, col, cg)
    spacings = (16, 22, 30)
    for i, sp in enumerate(spacings):
        c = (38 + i * cell, 38 + 2 * cell)
        hex_patch(scene, c)
        place_trader(scene, c, colours[0], "icon_wool_12.png", dx=-sp // 2)
        place_trader(scene, c, colours[1], "icon_ore_12.png", dx=sp - sp // 2)
    c = (38 + 3 * cell, 38 + 2 * cell)
    hex_patch(scene, c)
    place_trader(scene, c, colours[0], "icon_wool_12.png", dx=-9, dy=-5)
    place_trader(scene, c, colours[1], "icon_ore_12.png", dx=9, dy=5)
    return scene


def footprint_image(im):
    out = Image.new("RGBA", im.size, (0, 0, 0, 0))
    src, dst = im.load(), out.load()
    for y in range(im.height):
        for x in range(im.width):
            if src[x, y][3] > 128:
                dst[x, y] = (0, 0, 0, 102)
    return out



DIGITS_3X5 = {
    "0": ["111", "101", "101", "101", "111"], "1": ["010", "110", "010", "010", "111"],
    "2": ["111", "001", "111", "100", "111"], "3": ["111", "001", "111", "001", "111"],
    "4": ["101", "101", "111", "001", "001"], "5": ["111", "100", "111", "001", "111"],
    "6": ["111", "100", "111", "101", "111"], "7": ["111", "001", "010", "010", "010"],
    "8": ["111", "101", "111", "101", "111"], "9": ["111", "101", "111", "001", "111"]}
HUD_PLATE = (0x2A, 0x1E, 0x14, 255)
SHIP_MOOR_CONTACT = (58, 57)     # the tip of the ship's bowsprit, which goes on MOORING POINT


def draw_digits(im, text, x, y, col=(0x2A, 0x1A, 0x0E, 255)):
    for ch in text:
        for j, row in enumerate(DIGITS_3X5[ch]):
            for i, b in enumerate(row):
                if b == "1":
                    im.putpixel((x + i, y + j), col)
        x += 4


def hud_strip(turns, scores, ship_at, halved=False):
    """1:1 mock of the HUD strip: 209 px of tiled sea, a 10 px pier at the right and `turns`
    cells along the other 199 px. scores maps a 0 based turn to a number, or None for a turn in
    which nothing was built. Turns before ship_at without a score entry show nothing."""
    im = Image.new("RGBA", (213, 17), HUD_PLATE)
    water = outputs["track_water.png"][0]
    sea = Image.new("RGBA", (209, 13), (0, 0, 0, 0))
    for x in range(0, 209, 14):
        sea.alpha_composite(water, (x, 0))
    im.alpha_composite(sea, (2, 2))
    im.alpha_composite(outputs["track_pier.png"][0], (2 + 209 - 14, 2))
    pitch = 199.0 / turns
    for i in range(turns):
        cx = 2 + (i + 0.5) * pitch
        if i == ship_at:
            im.alpha_composite(outputs["track_ship.png"][0], (int(round(cx - 7)), 2))
        elif i > ship_at:
            if halved and i % 2 == 0:
                continue
            im.alpha_composite(recolor(outputs["track_buoy.png"][0], (211, 47, 47)), (int(round(cx - 2.5)), 6))
        elif i in scores:
            x0 = int(round(cx - 6))
            if scores[i] is None:
                im.alpha_composite(outputs["track_flag_empty.png"][0], (x0, 2))
            else:
                f = outputs["track_flag.png"][0].copy()
                txt = str(scores[i])
                draw_digits(f, txt, 6 - (len(txt) * 4 - 1) // 2, 2)
                im.alpha_composite(f, (x0, 2))
    return im


DOCK_MOORING = (214, 152)        # dock_mooring in tools/board_geometry_*.txt (bake_board.py)


def dock_mock():
    """A piece of the baked board at 1:1 round the dock, with the King's ship moored at it."""
    board = Image.open(os.path.join(OUT, "board_tarnholm.png")).convert("RGBA")
    ship = outputs["piece_kingship.png"][0]
    board.alpha_composite(ship, (DOCK_MOORING[0] - SHIP_MOOR_CONTACT[0], DOCK_MOORING[1] - SHIP_MOOR_CONTACT[1]))
    return board.crop((120, 80, 290, 230))


def icon_masks(fg, bg):
    """The adaptive icon as a launcher shows it: the 108 dp layers cut to the central 72 dp by a
    circle, a squircle or a rounded square, at 108 px (1x), 216 px (2x) and 54 px (about 48 dp),
    each made by nearest neighbour from the 432 px composite."""
    comp = bg.copy()
    comp.alpha_composite(fg)
    out = []
    for kind in ("circle", "squircle", "rounded"):
        row = []
        for size in (108, 216, 54):
            im = comp.resize((size, size), Image.NEAREST)
            half = size * 72.0 / 108.0 / 2.0
            m = Image.new("L", (size, size), 0)
            px = m.load()
            c = size / 2.0
            for y in range(size):
                for x in range(size):
                    u, v = (x + 0.5 - c) / half, (y + 0.5 - c) / half
                    if kind == "circle":
                        ok = u * u + v * v <= 1.0
                    elif kind == "squircle":
                        ok = u ** 4 + v ** 4 <= 1.0
                    else:
                        k = 0.3
                        ok = abs(u) <= 1.0 and abs(v) <= 1.0 and not (
                            abs(u) > 1.0 - k and abs(v) > 1.0 - k and
                            (abs(u) - (1.0 - k)) ** 2 + (abs(v) - (1.0 - k)) ** 2 > k * k)
                    if ok:
                        px[x, y] = 255
            cut = Image.new("RGBA", (size, size), (0, 0, 0, 0))
            cut.paste(im, (0, 0), m)
            row.append(cut)
        out.append(row)
    return out


def preview_icon(sheet, d, y, pad, font):
    """Old (commit a0ce51d) and new launcher icon at 4x, masked to a circle and a rounded square,
    then the same at 54 px by nearest neighbour and by a smooth filter."""
    white = (255, 255, 255, 255)
    d.text((pad, y), "launcher icon, old then new, at 4x masked to a circle and a rounded square (central 72 dp), "
           "then at 54 px nearest and smooth", fill=white, font=font)
    y += 18

    def old_layer(name):
        try:
            raw = subprocess.check_output(["git", "-C", ROOT, "show",
                                           "a0ce51d:app/src/main/res/drawable-nodpi/" + name])
        except (OSError, subprocess.CalledProcessError):
            return None
        return Image.open(io.BytesIO(raw)).convert("RGBA")

    def cut(comp, kind, size):
        im = comp.resize((size, size), Image.NEAREST if kind[1] else Image.LANCZOS)
        m = Image.new("L", (size, size), 0)
        md = ImageDraw.Draw(m)
        half = size * 72.0 / 108.0
        box = [(size - half) / 2.0, (size - half) / 2.0, (size + half) / 2.0, (size + half) / 2.0]
        if kind[0] == "circle":
            md.ellipse(box, fill=255)
        else:
            md.rounded_rectangle(box, radius=half * 0.22, fill=255)
        out = Image.new("RGBA", (size, size), (96, 96, 104, 255))
        out.paste(im, (0, 0), m)
        return out

    sets = []
    ofg = old_layer("icon_foreground.png")
    if ofg is not None:
        ob = outputs["icon_background.png"][0].copy()
        ob.alpha_composite(ofg)
        sets.append(ob)
    nb = outputs["icon_background.png"][0].copy()
    nb.alpha_composite(outputs["icon_foreground.png"][0])
    sets.append(nb)
    x = pad
    for comp in sets:
        for kind in (("circle", True), ("rounded", True)):
            c = cut(comp, kind, 432).crop((72, 72, 360, 360))
            sheet.alpha_composite(c, (x, y))
            x += 298
    y += 298
    x = pad
    for comp in sets:
        for nearest in (True, False):
            c = cut(comp, ("circle", nearest), 54)
            sheet.alpha_composite(c.resize((216, 216), Image.NEAREST), (x, y))
            x += 226
    return y + 226


def preview_candidates(sheet, d, y, pad, font):
    """Second round resource icons: old, candidates, chosen, beside lumber, brick and the question
    mark, at 4x on the dark plate and the ivory die face, 16 px then 12 px."""
    white = (255, 255, 255, 255)
    d.text((pad, y), "resource icons round 2 at 4x: lumber, brick, any | wool: old, no eye, chosen | wheat: old, "
           "stook chosen | ore: old, silver flecks, chosen | gold: old, outlined planes, chosen; "
           "16 px then 12 px, dark plate then ivory", fill=white, font=font)
    y += 18
    for size in (16, 12):
        cells = [outputs["icon_lumber_%d.png" % size][0], outputs["icon_brick_%d.png" % size][0],
                 outputs["icon_any_%d.png" % size][0],
                 make_sheep(size), make_sheep2_noeye(size), outputs["icon_wool_%d.png" % size][0],
                 sheaf(size), outputs["icon_grain_%d.png" % size][0],
                 make_ore(size), make_ore2(size, copper=False), outputs["icon_ore_%d.png" % size][0],
                 make_gold(size), make_gold2(size, inner_outline=True), outputs["icon_gold_%d.png" % size][0]]
        for plate in ICON_PLATES:
            wpx = len(cells) * (size * 4 + 12) + 12
            sheet.paste(Image.new("RGBA", (wpx, size * 4 + 16), plate), (pad, y))
            for i, im in enumerate(cells):
                sheet.alpha_composite(im.resize((size * 4, size * 4), Image.NEAREST),
                                      (pad + 12 + i * (size * 4 + 12), y + 8))
            y += size * 4 + 24
    return y + 8


def preview_ship(sheet, d, y, pad, font):
    """The King's ship at 4x, on the deep sea colour and on a piece of the board's open sea."""
    white = (255, 255, 255, 255)
    d.text((pad, y), "King's ship at 4x (Wesnoth's own alpha, flag recoloured): on deep sea 0D3947, then on the "
           "board's open sea", fill=white, font=font)
    y += 18
    board = Image.open(os.path.join(OUT, "board_tarnholm.png")).convert("RGBA")
    sea = board.crop((20, 20, 20 + 80, 20 + 80))
    new = outputs["piece_kingship.png"][0]
    x = pad
    for bgc in (rgb("0D3947"), None):
        w, h = 80, 80
        plate = Image.new("RGBA", (w, h), bgc) if bgc else sea.copy()
        plate.alpha_composite(new, (10, 10))
        sheet.alpha_composite(plate.resize((w * 4, h * 4), Image.NEAREST), (x, y))
        x += w * 4 + 10
    return y + 80 * 4 + 10


def preview_extra(sheet, d, y, W, pad, font):
    white = (255, 255, 255, 255)
    # old and new icons, both plates, with ore, grain, gold and the question mark for the family
    d.text((pad, y), "resource icons old then new at 4x on the dark plate and the ivory die face, 16 px then 12 px: "
           "lumber, brick, wool (old, new each), then ore, grain, gold, any", fill=white, font=font)
    y += 18
    old = {16: dict(lumber=compose_logs16(LOG16), brick=make_brick_old(16),
                    wool=art(WOOL16, WOOL, (16, 16))),
           12: dict(lumber=compose_logs12(LOG12), brick=make_brick_old(12),
                    wool=art(WOOL12, WOOL, (12, 12)))}
    for size in (16, 12):
        for plate in ICON_PLATES:
            cells = []
            for r in ("lumber", "brick", "wool"):
                cells += [old[size][r], outputs["icon_%s_%d.png" % (r, size)][0]]
            cells += [outputs["icon_%s_%d.png" % (r, size)][0] for r in ("ore", "grain", "gold", "any")]
            wpx = len(cells) * (size * 4 + 12) + 12 + 20
            sheet.paste(Image.new("RGBA", (wpx, size * 4 + 16), plate), (pad, y))
            x = pad + 12
            for i, im in enumerate(cells):
                sheet.alpha_composite(im.resize((im.width * 4, im.height * 4), Image.NEAREST), (x, y + 8))
                x += size * 4 + 12 + (10 if i in (1, 3, 5) else 0)
            y += size * 4 + 24
    # the King's ship tracker
    d.text((pad, y + 6), "King's ship tracker at 4x on the HUD plate 2A1E14: 15 turns (flags 1 to 6 with sample scores, "
           "turn 4 empty, ship on turn 7, buoys in the player colour, pier), then 20 turns at a 10 px pitch with no flags and a buoy every second turn",
           fill=white, font=font)
    y += 24
    for strip in (hud_strip(15, {0: 5, 1: 12, 2: 8, 3: None, 4: 23, 5: 7}, 6), hud_strip(20, {}, 9, halved=True)):
        sheet.alpha_composite(strip.resize((strip.width * 4, strip.height * 4), Image.NEAREST), (pad, y))
        y += strip.height * 4 + 10
    # the tracker pieces alone at 8x
    x = pad
    for n in ("track_water.png", "track_buoy.png", "track_flag.png", "track_flag_empty.png",
              "track_ship.png", "track_pier.png"):
        im = outputs[n][0]
        sheet.paste(Image.new("RGBA", (im.width * 8 + 8, im.height * 8 + 8), HUD_PLATE), (x, y))
        sheet.alpha_composite(im.resize((im.width * 8, im.height * 8), Image.NEAREST), (x + 4, y + 4))
        x += im.width * 8 + 20
    y += 13 * 8 + 24
    # dock and board ship
    d.text((pad, y), "the baked dock and the King's ship moored at it, at 4x on the board; then the ship alone at 6x",
           fill=white, font=font)
    y += 18
    dm = dock_mock()
    sheet.alpha_composite(dm.resize((dm.width * 4, dm.height * 4), Image.NEAREST), (pad, y))
    x = pad + dm.width * 4 + 20
    for n in ("piece_kingship.png",):
        im = outputs[n][0]
        bgp = Image.new("RGBA", im.size, rgb("0C3847"))
        bgp.alpha_composite(im)
        sheet.alpha_composite(bgp.resize((im.width * 6, im.height * 6), Image.NEAREST), (x, y))
        x += im.width * 6 + 20
    y += dm.height * 4 + 20
    # adaptive icon
    d.text((pad, y), "adaptive icon as a launcher shows it (central 72 dp, 1x, 2x and 54 px): circle, squircle, "
           "rounded square on a mid grey home screen; at the right the old icon.png and the new layers",
           fill=white, font=font)
    y += 18
    fg, bg = outputs["icon_foreground.png"][0], outputs["icon_background.png"][0]
    rows = icon_masks(fg, bg)
    sheet.paste(Image.new("RGBA", (W - 2 * pad, 2 * 216 + 60), (96, 96, 104, 255)), (pad, y))
    for ci, row in enumerate(rows):
        x = pad + 10 + ci * 400
        sheet.alpha_composite(row[1], (x, y + 10))
        sheet.alpha_composite(row[0], (x + 226, y + 10))
        sheet.alpha_composite(row[2], (x + 226, y + 130))
    old_icon = make_launcher(48).resize((192, 192), Image.NEAREST)
    sheet.alpha_composite(old_icon, (pad + 1230, y + 10))
    legacy = outputs["ic_launcher.png"][0]
    gb = Image.new("RGBA", legacy.size, (96, 96, 104, 255))
    gb.alpha_composite(legacy)
    sheet.alpha_composite(gb, (pad + 1230, y + 214))
    return y + 2 * 216 + 70


def preview():
    Z = 4
    BG = (84, 140, 72, 255)
    font = label_font(11)
    names = sorted(outputs)
    W, pad = 1500, 10
    x, y, rowh = pad, pad, 0
    placed = []
    for n in names:
        im = outputs[n][0]
        z = Z
        if n in ("ic_launcher.png", "market_icon.png"):
            im = im.resize((96, 96), Image.NEAREST)
            z = 2
        w, h = im.width * z, im.height * z
        cw = max(w, 130)
        if x + cw > W - pad:
            x, y, rowh = pad, y + rowh + pad, 0
        placed.append((n, im, z, x, y))
        x += cw + pad
        rowh = max(rowh, h + 16)
    mock_y = y + rowh + pad
    sheet = Image.new("RGBA", (W, mock_y + 330 + 18 + 944 + 20 + 18 + 4 * 112 + 8 + 18 + 2 * 88 + 20 + EXTRA_H), BG)
    d = ImageDraw.Draw(sheet)
    for n, im, z, px_, py in placed:
        sheet.alpha_composite(im.resize((im.width * z, im.height * z), Image.NEAREST),
                              (px_, py + 14))
        d.text((px_, py), n, fill=(255, 255, 255, 255), font=font)
    d.text((pad, mock_y), "9-slice stretched at 3x: buttons 150x20 (normal, pressed, disabled), "
           "panel 160x100; the same at 1:1 on the right", fill=(255, 255, 255, 255), font=font)
    ys = mock_y + 18
    for i, state in enumerate(("normal", "pressed", "disabled")):
        b = stretched_button(state)
        sheet.alpha_composite(b.resize((450, 60), Image.NEAREST), (pad, ys + i * 70))
        sheet.alpha_composite(b, (1150, ys + i * 24))
    panel = nine_stretch(outputs["ui_panel.png"][0], 160, 100)
    sheet.alpha_composite(panel.resize((480, 300), Image.NEAREST), (pad + 480, ys))
    sheet.alpha_composite(panel, (1150, ys + 80))
    sy = mock_y + 330
    d.text((pad, sy), "to-scale board mock at 4x (3 hexes wide): roads, two villages, two castles, "
           "two traders in two player colours; right: the sprites at 7x",
           fill=(255, 255, 255, 255), font=font)
    colours = [(40, 90, 220), (230, 150, 20)]
    scene = build_scene(colours)
    sheet.alpha_composite(scene.resize((scene.width * 4, scene.height * 4), Image.NEAREST),
                          (pad, sy + 18))
    x0 = 880
    for i, c in enumerate(colours):
        for j, key in enumerate(("piece_village.png", "piece_castle.png", "piece_trader.png")):
            v = recolor(outputs[key][0], c)
            sheet.alpha_composite(v.resize((v.width * 7, v.height * 7), Image.NEAREST),
                                  (x0 + (0, 160, 370)[j], sy + 18 + i * 260))
    # icon strip: every 12 px and 16 px icon on the dark plate and on the ivory die face, 4x
    ty = sy + 18 + 944 + 20
    d.text((pad, ty), "icons at 4x on the dark plate (2A1E14) and on the gold die face (E0B870)",
           fill=(255, 255, 255, 255), font=font)
    names12 = ["icon_%s_12.png" % r for r in ("lumber", "grain", "wool", "ore", "brick", "gold", "any")]
    names12 += ["glyph_road_12.png", "glyph_village_12.png", "glyph_castle_12.png", "glyph_trader_12.png"]
    names16 = ["icon_%s_16.png" % r for r in ("lumber", "grain", "wool", "ore", "brick", "gold", "any")]
    for row, (plate, names, size) in enumerate(((ICON_PLATES[0], names12, 12), (ICON_PLATES[1], names12, 12),
                                                (ICON_PLATES[0], names16, 16), (ICON_PLATES[1], names16, 16))):
        ry = ty + 18 + row * (size * 4 + 24)
        sheet.paste(Image.new("RGBA", (len(names) * (size * 4 + 12) + 12, size * 4 + 16), plate),
                    (pad, ry))
        for i, n in enumerate(names):
            im = outputs[n][0]
            sheet.alpha_composite(im.resize((im.width * 4, im.height * 4), Image.NEAREST),
                                  (pad + 12 + i * (size * 4 + 12), ry + 8))
    # wheat candidates: 16 and 12 px, 4x, on both backgrounds
    wy = ty + 18 + 4 * 112 + 8
    d.text((pad, wy), "wheat candidates at 4x: a single ear, b crossed ears, c sheaf previous, c sheaf pulled in (the old icon), d sack, e stook (chosen); "
           "16 px then 12 px, dark plate then ivory", fill=(255, 255, 255, 255), font=font)
    for r, plate in enumerate(ICON_PLATES):
        ry = wy + 18 + r * 88
        sheet.paste(Image.new("RGBA", (len(GRAIN_CANDIDATES) * 2 * 76 + 12, 80), plate), (pad, ry))
        for i, (label, fn) in enumerate(GRAIN_CANDIDATES):
            for j, size in enumerate((16, 12)):
                im = fn(size)
                sheet.alpha_composite(im.resize((size * 4, size * 4), Image.NEAREST),
                                      (pad + 12 + (i * 2 + j) * 76, ry + 8))
    # legend glyph candidates on both plates, beside the village and castle glyphs
    gy = wy + 18 + 2 * 88 + 16
    d.text((pad, gy), "legend glyphs at 8x, dark plate then ivory: old road, road a diagonal (chosen), b flat, "
           "c bend, d receding, old wagon, new cart with crate, new cart with sack (chosen), "
           "village, castle", fill=(255, 255, 255, 255), font=font)
    cands = [art(GLYPH_ROAD, GLYPH, (12, 12))] + [f() for _, f in ROAD_CANDIDATES]
    cands += [make_trader_glyph_wagon(), make_trader_glyph("crate"), make_trader_glyph("sack"),
              outputs["glyph_village_12.png"][0], outputs["glyph_castle_12.png"][0]]
    for row, plate in enumerate(ICON_PLATES):
        ry = gy + 18 + row * 112
        sheet.paste(Image.new("RGBA", (len(cands) * 108 + 12, 104), plate), (pad, ry))
        for i, im in enumerate(cands):
            sheet.alpha_composite(im.resize((96, 96), Image.NEAREST), (pad + 12 + i * 108, ry + 4))
    # dialogs
    dy_ = gy + 18 + 2 * 112 + 16
    d.text((pad, dy_), "dialog mocks at 4x: dark panel with the existing buttons (left), the light "
           "panel (right); below, the dark panel with candidate dark buttons", fill=(255, 255, 255, 255),
           font=font)
    alt = dark_button_candidates()
    cur = {st: outputs["ui_button_%s.png" % st][0] for st in ("normal", "pressed", "disabled")}
    for i, m_ in enumerate((dialog_mock(True, cur), dialog_mock(False, cur))):
        sheet.alpha_composite(m_.resize((720, 480), Image.NEAREST), (pad + i * 740, dy_ + 18))
    m_ = dialog_mock(True, alt)
    sheet.alpha_composite(m_.resize((720, 480), Image.NEAREST), (pad, dy_ + 18 + 500))
    # button states on the dark panel, current then candidate, at 3x
    for j, (label, set_) in enumerate((("current", cur), ("candidate", alt))):
        for i, st in enumerate(("normal", "pressed", "disabled")):
            bx, by = pad + 740 + j * 360, dy_ + 18 + 500 + i * 100
            sheet.alpha_composite(nine_stretch(outputs["ui_panel_dark.png"][0], 112, 32).resize(
                (336, 96), Image.NEAREST), (bx, by))
            bt = nine_stretch(set_[st], 90, 20)
            sheet.alpha_composite(bt.resize((270, 60), Image.NEAREST), (bx + 33, by + 18))
        d.text((pad + 740 + j * 360, dy_ + 18 + 500 + 300), label + " buttons: normal, pressed, disabled",
               fill=(255, 255, 255, 255), font=font)
    # trader mock at 4x
    ty2 = dy_ + 18 + 500 + 500 + 16
    d.text((pad, ty2), "trader at 4x in a 72 px hex, anchor at hex centre + (0, %d): rows 1 and 2 empty, "
           "wool, ore, any in two colours; row 3 two traders 16, 22, 30 apart, then offset diagonally"
           % TRADER_HEX_OFFSET, fill=(255, 255, 255, 255), font=font)
    tm = trader_mock(colours)
    sheet.alpha_composite(tm.resize((tm.width * 4, tm.height * 4), Image.NEAREST), (pad, ty2 + 18))
    # sprites at 12x: empty and loaded, and the footprint
    sy2 = ty2 + 18 + tm.height * 4 + 16
    d.text((pad, sy2), "trader at 12x: empty, loaded with wool, loaded with ore, any; below, its "
           "footprint (alpha > 128, black at 40%) on green", fill=(255, 255, 255, 255), font=font)
    spr = recolor(outputs["piece_trader.png"][0], colours[0])
    cx_, cy_ = TRADER_CARGO["pos"]
    top = min(0, cy_)
    for i, cg in enumerate((None, "icon_wool_12.png", "icon_ore_12.png", "icon_any_12.png")):
        c = Image.new("RGBA", (spr.width, spr.height - top), (0, 0, 0, 0))
        c.alpha_composite(spr, (0, -top))
        if cg:
            c.alpha_composite(outputs[cg][0], (cx_, cy_ - top))
        sheet.alpha_composite(c.resize((c.width * 12, c.height * 12), Image.NEAREST), (pad + i * 360, sy2 + 18))
    fp = Image.new("RGBA", (spr.width, spr.height), (96, 140, 60, 255))
    fp.alpha_composite(footprint_image(spr))
    fy = sy2 + 18 + (spr.height - top) * 12 + 16
    sheet.alpha_composite(fp.resize((fp.width * 12, fp.height * 12), Image.NEAREST), (pad, fy))
    y = fy + fp.height * 12 + 24
    y = preview_extra(sheet, d, y, W, pad, font)
    y = preview_candidates(sheet, d, y, pad, font)
    y = preview_ship(sheet, d, y, pad, font)
    y = preview_icon(sheet, d, y, pad, font)
    sheet = sheet.crop((0, 0, W, y + 10))
    sheet.convert("RGB").save(os.path.join(ROOT, "tools/preview_ui.png"))


def main():
    make_buttons()
    make_panel()
    make_dark_panel()
    make_dice()
    make_roads()
    make_resources()
    make_misc()
    make_track()
    make_icons()
    notes = extract_pieces()
    check_licences()
    write_outputs()
    write_side_files(notes)
    preview()
    print("wrote %d PNGs to %s" % (len(outputs), OUT))


if __name__ == "__main__":
    main()
