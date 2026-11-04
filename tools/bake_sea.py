#!/usr/bin/env python3
"""Bake the animated sea: each island's board at every step of the water's loop, packed into
one PNG the game plays back.

Run from the repository root:

    python3 -I tools/bake_sea.py [tarnholm|dunholm] [--frames DIR]

For each step 1..LOOP_STEPS this runs tools/bake_board.py with BAKE_LOOP_FRAME set (the deep
ocean, the shallow water and the beach waves each at their own frame for that step; see
LOOP_FRAME there), then packs the boards into resource/assets/art/sea_<ID>.png:
504 wide and 504 * LOOP_STEPS tall, row y of step s at row y * LOOP_STEPS + (s - 1). Laid out
that way, PNG's Up filter predicts each row from the same row one step earlier, and the water
changes little from step to step, so the file is about two thirds the size of the boards apart.
The game takes the rows back apart (`assets::split_steps`).

Step 1 is the still board, board_<ID>.png, pixel for pixel; the bake checks it. With --frames
DIR the boards already baked there (DIR/<ID>_01.png ...) are packed without baking again.
"""

import os
import subprocess
import sys

from PIL import Image

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
ART = os.path.join(ROOT, "resource", "assets", "art")
WORK = os.path.join(ROOT, "build", "sea")
LOOP_STEPS = 21
MAPS = ["tarnholm", "dunholm"]


def frame_path(frames, island, step):
    return os.path.join(frames, "%s_%02d.png" % (island, step))


def bake(island, frames):
    for step in range(1, LOOP_STEPS + 1):
        env = dict(os.environ, BAKE_LOOP_FRAME=str(step), BAKE_OUT=frame_path(frames, island, step))
        subprocess.run([sys.executable, "-I", os.path.join(HERE, "bake_board.py"), island],
                       check=True, env=env, stdout=subprocess.DEVNULL)


def pack(island, frames):
    boards = [Image.open(frame_path(frames, island, s)).convert("RGB")
              for s in range(1, LOOP_STEPS + 1)]
    still = Image.open(os.path.join(ART, "board_%s.png" % island)).convert("RGB")
    if boards[0].tobytes() != still.tobytes():
        raise SystemExit("FAIL: step 1 of %s is not the still board" % island)
    w, h = boards[0].size
    out = Image.new("RGB", (w, h * LOOP_STEPS))
    for y in range(h):
        for s, board in enumerate(boards):
            out.paste(board.crop((0, y, w, y + 1)), (0, y * LOOP_STEPS + s))
    path = os.path.join(ART, "sea_%s.png" % island)
    out.save(path, format="PNG", optimize=True)
    print("wrote", path, os.path.getsize(path), "bytes")


def record_sources():
    """Adds every Wesnoth file the loop's steps read to tools/wesnoth-sources.txt (a still
    bake records only step 1's), with the licence and author bake_board.py checks them by."""
    sys.path.insert(0, HERE)
    sys.dont_write_bytecode = True
    import bake_board
    used = set()
    for step in range(1, LOOP_STEPS + 1):
        used.add("water/ocean%02d.png" % step)
        used.add("water/water%02d.png" % (1 + (step - 1) * 17 // LOOP_STEPS))
        for shape in ("concave", "convex"):
            used.add("water/waves-%s-A%02d.png" % (shape, 1 + (step - 1) * 13 // LOOP_STEPS))
    path = os.path.join(HERE, "wesnoth-sources.txt")
    with open(path, encoding="utf-8") as fh:
        lines = fh.read().splitlines()
    head = [line for line in lines if line.startswith("#")]
    rows = {line.split(" | ")[0]: line for line in lines if line and not line.startswith("#")}
    for rel in used:
        rows[rel] = "%s | %s | %s | %s" % ((rel,) + bake_board.WESNOTH_LICENCES[rel])
    head[0] = ("# Wesnoth files read by tools/bake_board.py, at every step of the water's loop for"
               " tools/bake_sea.py (paths relative to data/core/images/terrain/)")
    with open(path, "w", encoding="utf-8") as fh:
        fh.write("\n".join(head + [rows[k] for k in sorted(rows)]) + "\n")
    print("recorded", len(used), "files of the loop in", path)


def main():
    args = sys.argv[1:]
    frames = None
    if "--frames" in args:
        i = args.index("--frames")
        frames = args[i + 1]
        del args[i:i + 2]
    islands = args or MAPS
    for island in islands:
        if frames is None:
            os.makedirs(WORK, exist_ok=True)
            bake(island, WORK)
            pack(island, WORK)
        else:
            pack(island, frames)
    record_sources()


if __name__ == "__main__":
    main()
