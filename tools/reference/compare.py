#!/usr/bin/env python3
"""Cut a Wesnoth map screenshot down to the board and compare it with the bake.

Called by render.sh as: python3 -I compare.py SHOT BAKE OUTDIR MAP [TOLERANCE]

Writes OUTDIR/MAP_wesnoth_render.png (504x504, aligned pixel for pixel with the board),
OUTDIR/MAP_side_by_side.png (bake, Wesnoth, difference, each at 2x nearest) and
OUTDIR/MAP_diff_stats.txt, and prints the same statistics. MAP is the map id (tarnholm,
dunholm); every map is written with the same shifts, so one set of offsets serves them all.

Geometry (read from src/display.cpp and src/editor/controller/editor_controller.cpp):
the editor's map screenshot puts hex (x, y) of the playable map, 0 based, at
    pixel x = 54 * x + 27
    pixel y = 72 * y + 36 + (36 if x is odd else 0)
at zoom 72 with the editor theme's border of half a hex. The board puts hex (col, row) at
    pixel x = 54 * col,   pixel y = 72 * row + (36 if col is odd else 0)
and every reference map was written with x = col + 6 and y = row + 2 (the shifts are even, so
column parity is kept). So board pixel (X, Y) is screenshot pixel (X + 351, Y + 180).
"""

import os
import sys

from PIL import Image, ImageChops

BOARD = 504
SHIFT_X = 6   # must match MAP_SHIFT_X in tools/bake_board.py
SHIFT_Y = 2   # must match MAP_SHIFT_Y in tools/bake_board.py
MAP_W = 16    # must match MAP_W and MAP_H in tools/bake_board.py
MAP_H = 9
LEFT = 54 * SHIFT_X + 27
TOP = 72 * SHIFT_Y + 36


def main():
    if len(sys.argv) < 5:
        sys.exit("usage: compare.py SHOT BAKE OUTDIR MAP [TOLERANCE]")
    shot_path, bake_path, outdir, map_id = sys.argv[1:5]
    tolerance = int(sys.argv[5]) if len(sys.argv) > 5 else 16

    shot = Image.open(shot_path).convert("RGBA")
    # Zoom check: at zoom 72 the screenshot is int((W + 1 + 1/3) * 54) by int((H + 1.5) * 72)
    want = (int((MAP_W + 1 + 1 / 3) * 54), int((MAP_H + 1.5) * 72))
    if abs(shot.width - want[0]) > 2 or abs(shot.height - want[1]) > 2:
        sys.exit("screenshot is %dx%d but a 72 px zoom map of %dx%d hexes should be about %dx%d; "
                 "Wesnoth did not render at zoom 72, so the board offsets do not apply"
                 % (shot.width, shot.height, MAP_W, MAP_H, want[0], want[1]))
    if shot.width < LEFT + BOARD or shot.height < TOP + BOARD:
        sys.exit("screenshot %dx%d is too small to hold the board at (%d,%d)"
                 % (shot.width, shot.height, LEFT, TOP))
    crop = shot.crop((LEFT, TOP, LEFT + BOARD, TOP + BOARD))
    if crop.split()[3].getextrema()[0] < 128:
        sys.exit("the cropped Wesnoth render is mostly transparent: the offsets are wrong")
    wes = crop.convert("RGB")
    bake = Image.open(bake_path).convert("RGB")
    if bake.size != (BOARD, BOARD):
        sys.exit("bake image is %s, expected %dx%d" % (bake.size, BOARD, BOARD))

    wes.save(os.path.join(outdir, map_id + "_wesnoth_render.png"), optimize=False)

    diff = ImageChops.difference(bake, wes)
    peak = [max(px) for px in diff.getdata()]
    over = sum(1 for v in peak if v > tolerance)
    total = len(peak)
    mean = sum(peak) / total
    # difference panel: the largest channel difference, amplified 4x, as grey
    panel = Image.new("L", (BOARD, BOARD))
    panel.putdata([min(255, v * 4) for v in peak])
    panel = panel.convert("RGB")

    gap = 8
    w = BOARD * 2
    sheet = Image.new("RGB", (w * 3 + gap * 2, w), (255, 0, 255))
    for i, img in enumerate((bake, wes, panel)):
        sheet.paste(img.resize((w, w), Image.NEAREST), (i * (w + gap), 0))
    sheet.save(os.path.join(outdir, map_id + "_side_by_side.png"), optimize=True)

    text = ("pixels differing by more than %d (largest channel): %d of %d = %.3f%%\n"
            "mean largest-channel difference: %.3f\n" % (tolerance, over, total, 100.0 * over / total, mean))
    with open(os.path.join(outdir, map_id + "_diff_stats.txt"), "w", encoding="utf-8") as fh:
        fh.write(text)
    sys.stdout.write(text)


if __name__ == "__main__":
    main()
