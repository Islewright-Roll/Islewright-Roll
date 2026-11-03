#!/usr/bin/env python3
"""The costs key's glyphs at 16 px, for the wide layout (a tablet or a desktop), where the key
shows the 16 px resource icons and its 12 px glyphs would look shrunken beside them. Drawn anew
at this size in the 12 px glyphs' palettes, not scaled from them.

Run from the repository root:

    python3 -I tools/make_glyphs16.py

Writes resource/assets/art/glyph_<road|village|castle|trader>_16.png. make_ui.py, whose
helpers and palettes this reuses, still writes to the Android tree the game had before Day.
"""

import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

from PIL import Image  # noqa: E402

import make_ui as ui  # noqa: E402

OUT = os.path.join(os.path.dirname(HERE), "resource", "assets", "art")

# The village: a gabled roof in team magenta (p, its shade P) over a whitewashed wall with a
# door and a window, as the 12 px one, with a taller wall and a door two pixels wide.
VILLAGE = ["................",
           ".......kk.......",
           "......kppk......",
           ".....kppppk.....",
           "....kppppppk....",
           "...kppppppppk...",
           "..kppppppppppk..",
           ".kppppppppppppk.",
           "kPPPPPPPPPPPPPPk",
           ".kwwwwwwwwwwwwk.",
           ".kwwddwwwwwbbwk.",
           ".kwwddwwwwwbbWk.",
           ".kwwddwwwwwwwWk.",
           ".kwwddwwwwwwwWk.",
           ".kkkkkkkkkkkkkk.",
           "................"]

# The castle: two towers under cone roofs either side of a crenellated wall, a banner and a
# gate, in the castle glyph's stone; roofs and banner in team magenta.
TOWER = [".pP.", "ppPP", "ppPP", "LLsS"] + ["LLsS"] * 10
WALL = ["......", "......", "......", ".s..s.",
        "ssssss", "spppPs", "sPPPPs", "ssssss", "ssssss",
        "sskkss", "skddks", "skddks", "skddks", "skddks"]
CASTLE = [t + w + t for t, w in zip(TOWER, WALL)]

# The trader's hand cart: a sack on a flat bed, two spoked wheels, the shaft rising behind.
TRADER = ["..............",
          "....tt.......t",
          "...cccs......t",
          "..ccccccs...t.",
          ".cccccccs...t.",
          ".ccccccsc..t..",
          ".scccccs...t..",
          "bbbbbbbbbbbt..",
          "wttwwwwwttwww.",
          ".tcct...tcct..",
          "tcssct.tcssct.",
          "tcssct.tcssct.",
          ".tcct...tcct..",
          "..tt.....tt..."]


def cell(rows, legend):
    """A 14 x 14 picture in the middle of a 16 px cell, with the glyphs' 1 px outline."""
    im = Image.new("RGBA", (16, 16), (0, 0, 0, 0))
    im.alpha_composite(ui.art(rows, legend, (14, 14)), (1, 1))
    ui.finish_outline(im)
    return im


def road():
    """A stretch of road corner to corner and off the cell, kerbs in team magenta."""
    im = ui.road_ribbon([(-2, 18), (18, -2)], 4.0, 2.0, n=16)
    ui.finish_outline(im)
    return im


def main():
    glyphs = {
        "road": road(),
        "village": ui.art(VILLAGE, ui.GLYPH, (16, 16)),
        "castle": cell(CASTLE, ui.CASTLE_GLYPH),
        "trader": cell(TRADER, ui.CART_GLYPH),
    }
    for name, im in glyphs.items():
        path = os.path.join(OUT, "glyph_%s_16.png" % name)
        im.save(path, format="PNG", optimize=False)
        print("wrote", path)


if __name__ == "__main__":
    main()
