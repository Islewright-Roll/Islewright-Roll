#!/usr/bin/env python3
"""Bake the 504x504 island board out of Battle for Wesnoth terrain tiles.

Run from the repository root:

    python3 -I tools/bake_board.py [tarnholm|dunholm]

With no argument every map in MAPS is baked.

Environment:
    WESNOTH_DIR   a Wesnoth data directory that holds data/core/images/terrain: the installed
                  package (default /usr/share/games/wesnoth/1.16) or a source checkout such
                  as ../wesnoth beside this repository
    BAKE_<NAME>   override any option constant below, for example BAKE_FOREST_IN_HEX=0
    BAKE_OUT      write only the board PNG to this path (used for comparisons; the
                  shipped board, preview, geometry and sources files stay untouched)

Outputs:
  per map ID:
    app/src/main/res/drawable-nodpi/board_<ID>.png   the board, 504x504, native pixels
    tools/preview_board_<ID>.png                     3x nearest neighbour preview
    tools/board_geometry_<ID>.txt                    hex centres and sizes, one item per line
    tools/reference/<ID>.map                         the same island as a real Wesnoth map
  once, after a run with no argument:
    tools/wesnoth-sources.txt                        every Wesnoth file read (a run with no argument)

Nothing is ever resampled. Every tile is composited at 1:1 pixels; the preview is
the only scaled image and it uses Image.NEAREST. The script is deterministic: the
terrain variants come from Wesnoth's own position noise (random.Random(1) when that is
switched off) and PNG output carries no timestamps.

How the blending works (read from data/core/terrain-graphics.cfg and
terrain-graphics/new-macros.cfg, ported here as a small rule engine):

* Every hex gets a base image at layer -1000 (-999 for water), a random variant
  of the terrain's stem (grass/green, grass/green2 ... ), exactly like NEW:BASE.
* Edge transitions follow NEW:TRANSITION. For each rule, the receiving hex looks at
  its six neighbours; each neighbour matching the rule's source terrain list marks
  a direction. The marked directions are cut into clockwise runs and the longest
  existing file stem-<run>.png is drawn on the receiving hex, then the remainder is
  tried with shorter runs. A rule whose file does not exist is simply not
  registered by Wesnoth (builder.cpp ignores missing images), which is the same
  fallback used here. The default "transition" flag (one transition per edge,
  first rule in terrain-graphics.cfg wins) is modelled too.
* Beach waves follow NEW:WAVES (rings around the desert hex, frame 1).
* Terrain lists use Wesnoth's own semantics, including the "!" inversion operator
  (src/terrain/translation.cpp terrain_matches).
* water/bottom is a corner (vertex) transition: where a vertex of a water hex
  touches two land hexes, bottom.png is masked with masks/long-concave-2-<vertex>.
* Sprites that are "global" images in WML (mountains, forest, waves) are cut per hex
  with the hex mask, exactly as Wesnoth's image loader does, and every image is drawn in
  Wesnoth's own order (display.cpp generate_hex_key: background group, then foreground
  group, each by hex row, column parity and column, with the hex's images by layer and
  base y).

Deliberate departures from Wesnoth are named constants in the OPTIONS block, so a
comparison against a real Wesnoth render can switch each one off.
"""

import colorsys
import hashlib
import io
import fnmatch
import math
import os
import random
import subprocess
import sys

from PIL import Image, ImageChops

# --------------------------------------------------------------------------
# Options. Environment variables BAKE_<NAME> override them.
# --------------------------------------------------------------------------


def _opt(name, default):
    value = os.environ.get("BAKE_" + name)
    if value is None:
        return default
    if isinstance(default, bool):
        return value.strip().lower() not in ("0", "false", "no", "")
    if isinstance(default, int):
        return int(value)
    return value


# DEPARTURE (asked for by the owner): keep every tree pixel inside the forest hex. Wesnoth
# draws the small forest sprite on a forest beside water, centred on the hex, and lets it hang
# over the neighbours. On: the first small variant, at Wesnoth's own place, and only the piece
# of it that lies in the forest's own hex is drawn (the sprite is cut by the hex mask, as every
# Wesnoth image is; the pieces Wesnoth would draw on the six neighbours are left out). Nothing
# is moved, shrunk or resampled, so inside the hex the trees are pixel for pixel Wesnoth's.
# For the summer deciduous forest (Fds) that is forest/deciduous-summer-small.png, the only
# small variant: 3386 of the hex's 3888 pixels carry tree or tree shadow (87.1%) and 313
# pixels, mostly the shadow on the west side and the foot of the southmost tree, are cut off.
# No whole pixel shift does better than 290 cut off, so the sprite stays where Wesnoth puts it.
# The pine forest (Fp) used before had pine-small.png: 2267 pixels in the hex (58.3%), none
# outside. Off: Wesnoth's own variant for that hex, overhang and all.
FOREST_IN_HEX = _opt("FOREST_IN_HEX", True)
# On: use the large forest sprite always (the old look). Off: what Wesnoth does.
FOREST_FULL_SPRITE = _opt("FOREST_FULL_SPRITE", False)
# Off = Wesnoth. On adds a dry grass verge on the sea side of the farm field (not a Wesnoth
# rule; it was needed before the pebble shore rim (flat/bank-to-ice) was ported).
FIELD_GRASS_VERGE = _opt("FIELD_GRASS_VERGE", False)
# water/bottom opacity in percent. Wesnoth: 100 (the macro's MASKIPF is empty).
BOTTOM_OPACITY = _opt("BOTTOM_OPACITY", 100)
# NEW:WAVES draws the two concave sand/water/other corner rules. Wesnoth: on.
WAVES_THREE_WAY_CONCAVE = _opt("WAVES_THREE_WAY_CONCAVE", True)
# KEPT: the board is a still image, so the animated water and waves are frame 1. The 1.16
# screenshot also shows frame 1 (the water matches pixel for pixel).
WATER_FRAME = _opt("WATER_FRAME", 1)   # 1..17 (water01..17), 1..21 for deep ocean
WAVES_FRAME = _opt("WAVES_FRAME", 1)   # 1..13
# Draw order. "wesnoth": display.cpp order (verified against renders). "layer": plain layer sort.
DRAW_ORDER = _opt("DRAW_ORDER", "wesnoth")
# Pick variants (grass2, pine-small2, ...) with Wesnoth's own position noise (builder.cpp
# get_noise and rule hash), so the same map gets the same tiles. Off: random.Random(1).
WESNOTH_VARIANT_NOISE = _opt("WESNOTH_VARIANT_NOISE", True)
# DEPARTURE (asked for by the owner): the water. Deep ocean Wo is the open sea, ordinary
# shallow water Ww fills every sea hex next to the island (a ring of 12 round the seven hex
# flower, SHALLOW_CODE), and the lake of the island that has one is the coastal reef Wwr
# (LAKE_CODE). Not tropical water. Wesnoth's own water to water transitions blend the three
# (add_water_transitions). SHALLOW_RING off: no ring, the sea touches the land.
# BAKE_SEA, BAKE_SHALLOW and BAKE_LAKE try other codes.
SEA_CODE = _opt("SEA", "Wo")
SHALLOW_CODE = _opt("SHALLOW", "Ww")
SHALLOW_RING = _opt("SHALLOW_RING", True)
LAKE_CODE = _opt("LAKE", "Wwr")
# ~O(50%) in the water transition rules: the alpha of the blended water, in 256ths.
WATER_BLEND_ALPHA = _opt("WATER_BLEND_ALPHA", 128)
# Wesnoth draws a mountain beside desert, dirt or dry grass with mountains/blend-from-dry on
# the mountain and hills/dry on the dry hex (terrain-graphics.cfg:901-904). Both maps use it
# (MAPS "dry_blend").
# BAKE_MOUNTAIN_DRY_BLEND=1 or 0 overrides the per map setting for every map.
MOUNTAIN_DRY_BLEND_OVERRIDE = os.environ.get("BAKE_MOUNTAIN_DRY_BLEND")
MOUNTAIN_DRY_BLEND = False  # set by select_map
# DEPARTURE (asked for by the owner): the mountain sprite of every Mm hex. Wesnoth picks one
# of basic.png, basic2.png, basic3.png by position (and a castle to the north east would make
# it draw basic-castle-ne.png). The default is mountains/basic-castle-ne.png, the variant
# Wesnoth draws for a mountain with a castle to the north east: it has no pixel across the
# hex's NE edge, where the game draws its start road and where the desert hex begins.
# mountains/basic.png (the owner's earlier choice) is the alternative: it reaches about 11 px
# past that edge. Empty (BAKE_MOUNTAIN_SPRITE=) or "wesnoth": Wesnoth's own variant choice,
# which the reference comparison needs. Any file name under data/core/images/terrain/ is accepted.
MOUNTAIN_SPRITE = _opt("MOUNTAIN_SPRITE", "mountains/basic-castle-ne.png")
# DEPARTURE (asked for by the owner): repaint the green of the mountain hex on the sides that
# face desert (the NE edge, and on Dunholm the SE edge too) so the mountain rises out of the sand
# instead of wearing a green skirt. Per pixel colour swap at native resolution, no resampling,
# no blur, no alpha change: a pixel that is green (hue MOUNTAIN_DRY_HUE_MIN..MAX degrees and
# saturation of at least MOUNTAIN_DRY_SAT_MIN percent) takes the colour Wesnoth's own dry art has
# at the same place (mountains/dry*.png for the sprite, hills/dry*.png for the hills base under
# it), with a strength that is full at the hex edge facing the desert and zero at the hex
# centre line, applied through an 8x8 ordered (Bayer) dither so it stays pixel art. Rock and
# snow are not green, so they stay. Off: Wesnoth's own colours.
MOUNTAIN_NE_DRY = _opt("MOUNTAIN_NE_DRY", True)
MOUNTAIN_DRY_HUE_MIN = _opt("MOUNTAIN_DRY_HUE_MIN", 55)
MOUNTAIN_DRY_HUE_MAX = _opt("MOUNTAIN_DRY_HUE_MAX", 170)
MOUNTAIN_DRY_SAT_MIN = _opt("MOUNTAIN_DRY_SAT_MIN", 18)
# The strength is distance towards the desert edge (0 at the hex centre line, 100 at the edge)
# times this percent, capped at 1: 140 makes it full from 71% of the way to the edge.
MOUNTAIN_DRY_REACH = _opt("MOUNTAIN_DRY_REACH", 140)
# DEPARTURE (clearly named, a choice the owner has not made yet): the thin beach. Every coast of
# the island meets the sea the way the desert hex does (a narrow sandy shore with Wesnoth's soft
# foam, no earth bank), without adding a sand hex, so the island keeps its seven hexes and its
# ragged grass, hills and farmland edges. See thin_beach_pass. Off by default; the MAPS entry
# "thin_beach" sets it per map and BAKE_THIN_BEACH=1 or 0 overrides it for every map.
THIN_BEACH_OVERRIDE = os.environ.get("BAKE_THIN_BEACH")
THIN_BEACH = False  # set by select_map
# The sand the shore is drawn from, on the water hexes beside non desert land: sand/desert (the
# colour of the real desert hex, which draws its own shore from the same images) or sand/beach
# (paler).
BEACH_STEM = _opt("BEACH_STEM", "sand/desert")
# How many times that sand is drawn (1: once, as Wesnoth would draw a transition; twice makes
# the dithered ramp read as sand and not as smoke over the water, without widening it).
BEACH_SAND_PASSES = _opt("BEACH_SAND_PASSES", 2)
# Alpha (0..255) at or below which the sand's faint tail is cut off, the rest stretched back.
BEACH_SAND_FLOOR = _opt("BEACH_SAND_FLOOR", 40)
# The farm field hex.
WHEAT_CODE = _opt("WHEAT", "Re^Gvs")
# DEPARTURE (asked for by the owner): the dock, as part of the terrain. Where the game's start
# road meets the sea (the vertex the mountain and the north desert share with the sea hex beside
# them, DOCK_FOOT) a short jetty runs straight up the screen, over the strip of sand there and
# out across the shallows, clear of the mountain's art on its left and of the desert's shore on
# its right. It is authored here, pixel by pixel, in hard alpha (0 or 255) and a palette of six
# muted browns (DOCK_COLOURS), because the owner found Wesnoth's own bridge plank too soft and
# too colourful: a deck 8 px wide of cross planks with a dark edge down each side, and four piles
# at its sides, two pairs, that stand only in water (check_dock_posts refuses the bake if any
# pixel of a pile, above or below the water, is not in water) and go down into it: each pile
# stops at the waterline and its foot goes on under the surface in a few steps of the pile's
# colour over the board's own water (dock_fade), the last of them a chequer. The deck's landward
# end lies over the start road's round end, at the deck's own width: the road arrives at the
# foot of the jetty and the jetty begins there.
# The dock is drawn twice, to the same pixels. It is baked into the board, so that everything
# that samples the board sees it; and its above water art alone is written as a sprite
# (dock.png), which the game draws over the roads and under the King's ship, so that the deck
# lies over the road and not under it. The submerged steps exist only in the board: they are
# derived from the water under them, and the sprite leaves them out, which draws the same pixels.
# Not a Wesnoth rule, so it is off for the reference comparison (BAKE_DOCK=0). The same on both
# maps: their terrain round that vertex is identical.
DOCK = _opt("DOCK", True)
# Every colour of the dock's sprite, as hex, and what it is for. No other colour is ever in it.
DOCK_COLOURS = {
    "a": "84664A",   # plank, light tone
    "b": "755A40",   # plank, dark tone (alternate planks)
    "s": "5A422E",   # seam between planks
    "o": "3E2E20",   # outline down both sides, the seaward end, the landward end
    "d": "4A3626",   # pile, shaded side
    "l": "7C5E42",   # pile, lit edge and cap
}
DOCK_DECK_X = (212, 219)    # board columns of the deck, first and last
DOCK_DECK_Y = (136, 179)    # board rows of the deck, first and last (the vertex's row is 180, the mountain hex's first)
DOCK_PLANK = 4              # rows per plank, its seam being the last of them
# The deck keeps one width to its landward end, and stops at row 179 because row 180 is the
# first of the mountain's hex, which no pixel of the dock may lie in.
# The piles as (first column, top row), each 2 px wide: a seaward pair (west, east) that rise two
# rows above the deck as bollards, and a pair midway. DOCK_ABOVE rows stand out of the water; the
# next row is the waterline (the pile's own pixels a little wet); then
# one row for each step of its fade (dock_fade, DOCK_DEPTH).
DOCK_POSTS = ((210, 134), (220, 134), (210, 154), (220, 154))
DOCK_ABOVE = 7
# The steps under the waterline, one row each: the percent of the pile's colour over the water,
# and whether only the pixels where (x + y) is even are drawn (a chequer, so a step breaks up).
# How many rows of each pile show under the waterline: the water is deeper to seaward, and the
# west side is deeper than the east, where the beach comes in (asked for by the owner).
DOCK_DEPTH = {(210, 134): 8, (220, 134): 7, (210, 154): 5, (220, 154): 4}
DOCK_FADE_FROM, DOCK_FADE_TO = 75, 30


def dock_fade(rows):
    """The steps under the waterline for a pile that shows `rows` rows there: the percent of the
    pile's colour over the water, falling evenly from DOCK_FADE_FROM to DOCK_FADE_TO, and whether
    only the pixels where (x + y) is even are drawn. The last two fifths of the rows are a chequer."""
    plain = rows - (2 * rows + 4) // 5
    return [(DOCK_FADE_FROM if rows == 1 else
             round(DOCK_FADE_FROM + (DOCK_FADE_TO - DOCK_FADE_FROM) * i / (rows - 1)), i >= plain)
            for i in range(rows)]


DOCK_WET_PERCENT = 90       # the pile's colour on the waterline row, over the water
DOCK_PAIR_GAP = 4           # the least clear rows between one pair's lowest pixel and the next pair's top
# Where a ship's bowsprit comes to, from the vertex: three columns onto the deck's west edge, a
# little below the jetty's seaward end.
DOCK_MOORING_FROM_FOOT = (-2, -28)

# --------------------------------------------------------------------------
# Paths
# --------------------------------------------------------------------------

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(HERE)
SISTER = os.path.join(os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))), "wesnoth")
PACKAGE = "/usr/share/games/wesnoth/1.16"
# Default art: the installed Wesnoth package, because that is what the reference render
# (tools/reference/render.sh) draws with. The sister checkout's newer art is available
# with WESNOTH_DIR pointing at a wesnoth checkout.
WESNOTH = os.environ.get("WESNOTH_DIR") or (PACKAGE if os.path.isdir(PACKAGE) else SISTER)
TERRAIN = os.path.join(WESNOTH, "data", "core", "images", "terrain")

OUT_ALT = os.environ.get("BAKE_OUT")
DRAWABLE = os.path.join(REPO, "app", "src", "main", "res", "drawable-nodpi")
OUT_SOURCES = os.path.join(HERE, "wesnoth-sources.txt")

PACKAGE_TAG = "1.16.11"
PACKAGE_COMMIT = "83d4b397f1de466c721e4e393b361c71fc4f5b6c"
# Licence and author of every Wesnoth file this script reads (paths relative to TERRAIN). The
# installed 1.16 files carry no record of their own, so licence and author are copied from the last
# revision of Wesnoth's copyrights.csv that still listed images (d4b2d151ce3, 2024-08-23; since then
# Wesnoth keeps the record in each PNG's EXIF Copyright and Artist tags). The MD5 is that of the
# bytes of the 1.16.11 file, which is what is read. For 15 files (the long-concave and long-convex
# masks, three mountain sprites) the csv row holds the MD5 of a later, recompressed file with
# identical pixels; the MD5 below is the 1.16.11 one. Every file read must be in this table with a
# matching MD5, or the bake fails before it draws anything.
WESNOTH_LICENCES = {
    "alphamask.png":
        ("GNU GPL v2+", "Philippe Plantier(Ayin)", "350228f29384f9f9c7b239f092d4b5e8"),
    "embellishments/farm-veg-spring-n.png":
        ("GNU GPL v2+", "J.W. Bjerk(Eleazar)", "45a47dbb019d0aba2f826332df90f402"),
    "embellishments/farm-veg-spring-nw.png":
        ("GNU GPL v2+", "J.W. Bjerk(Eleazar)", "e8c94de8156fda51f1e372583d8841b6"),
    "embellishments/farm-veg-spring-se.png":
        ("GNU GPL v2+", "J.W. Bjerk(Eleazar)", "32d681aaf021d7db0650aed61f4c15f3"),
    "embellishments/farm-veg-spring3.png":
        ("GNU GPL v2+", "J.W. Bjerk(Eleazar)", "b4d0d187e3dc8996bf5fc673010a40ca"),
    "flat/bank-ne.png":
        ("GNU GPL v2+", "unknown", "28299037a4b662c6abe892ba3ff939bd"),
    "flat/bank-nw.png":
        ("GNU GPL v2+", "J.W. Bjerk(Eleazar)", "ec9dfb6d60298da4e93cbd4cb4ad06fb"),
    "flat/bank-s.png":
        ("GNU GPL v2+", "unknown", "3a345cfb4174359114bbc0db00adafaf"),
    "flat/bank-se.png":
        ("GNU GPL v2+", "unknown", "e59a15f25f81e28f06743c4aeec4f821"),
    "flat/bank-sw.png":
        ("GNU GPL v2+", "unknown", "4950d3db926ca94515281d4c08b1f4ee"),
    "flat/bank-to-ice-n-ne.png":
        ("GNU GPL v2+", "J.W. Bjerk(Eleazar)", "830a33a8b5e2cf33b8e2745812e476ef"),
    "flat/bank-to-ice-n.png":
        ("GNU GPL v2+", "J.W. Bjerk(Eleazar)", "646eb5b1c2403f9fd0990d8b825cf44c"),
    "flat/bank-to-ice-ne.png":
        ("GNU GPL v2+", "J.W. Bjerk(Eleazar)", "6cd542d8c2d453cc78ebb5500e0b4c61"),
    "flat/bank-to-ice-nw-n.png":
        ("GNU GPL v2+", "unknown", "d5a392b24d703b5e5ea26e352e90c1ca"),
    "flat/bank-to-ice-nw.png":
        ("GNU GPL v2+", "J.W. Bjerk(Eleazar)", "83aed34491273dab2d5a4f7fd4ebecde"),
    "flat/bank-to-ice-se-s.png":
        ("GNU GPL v2+", "unknown", "9b37714941486844d4c998ef9078cd6a"),
    "flat/bank-to-ice-se.png":
        ("GNU GPL v2+", "J.W. Bjerk(Eleazar)", "d1d77b190658698a8544ceca8c21463b"),
    "flat/bank-to-ice-sw.png":
        ("GNU GPL v2+", "unknown", "a6fee90fa4bd07e26d090edcba51e96b"),
    "flat/dirt-n.png":
        ("GNU GPL v2+", "J.W. Bjerk(Eleazar)", "a9765d6e917e8bacb11489c75bb1f607"),
    "flat/dirt-ne.png":
        ("GNU GPL v2+", "J.W. Bjerk(Eleazar)", "777cd06acd9b49e8a3815cc145bda4a5"),
    "flat/dirt-s-sw-nw.png":
        ("GNU GPL v2+", "Richard Kettering(Jetrel);J.W. Bjerk(Eleazar)", "114d5e7212a39a0e575920b84a8f184c"),
    "flat/dirt-se.png":
        ("GNU GPL v2+", "J.W. Bjerk(Eleazar)", "ea4053721683d75c21a51030681a244b"),
    "flat/dirt-sw.png":
        ("GNU GPL v2+", "J.W. Bjerk(Eleazar)", "63dd7660c45b034e12aac31fae84d8cf"),
    "flat/dirt3.png":
        ("GNU GPL v2+", "Richard Kettering(Jetrel);J.W. Bjerk(Eleazar)", "13c79d8edbbdb42af70699677e70992a"),
    "forest/deciduous-summer-small.png":
        ("GNU GPL v2+", "Richard Kettering(Jetrel)", "e550d92e3ecf1688167b6bb5ed8242eb"),
    "grass/green-abrupt-n.png":
        ("GNU GPL v2+", "J.W. Bjerk(Eleazar)", "c169feac153a81ba0cf5e81e327ea29c"),
    "grass/green-abrupt-ne.png":
        ("GNU GPL v2+", "J.W. Bjerk(Eleazar)", "f106b8612543813730d6a70e7ec0cde7"),
    "grass/green-abrupt-nw.png":
        ("GNU GPL v2+", "J.W. Bjerk(Eleazar)", "09bb51acaad520c4f751835b47e7d7e6"),
    "grass/green-abrupt-s.png":
        ("GNU GPL v2+", "J.W. Bjerk(Eleazar)", "e45f27954233f3fe153dd24db683b8f0"),
    "grass/green-long-se.png":
        ("GNU GPL v2+", "J.W. Bjerk(Eleazar)", "8a21d52fb8625ab5eb79a96e9209cba3"),
    "grass/green-long-sw.png":
        ("GNU GPL v2+", "J.W. Bjerk(Eleazar)", "adc9261ba6df3112d71ead6bc99bab52"),
    "grass/green-medium-n.png":
        ("GNU GPL v2+", "J.W. Bjerk(Eleazar)", "ceac6d97b00c9e31828c9c9dce467571"),
    "grass/green-medium-ne.png":
        ("GNU GPL v2+", "J.W. Bjerk(Eleazar)", "db5cd115c4107e262cdf768a118da9a2"),
    "grass/green-medium-nw.png":
        ("GNU GPL v2+", "J.W. Bjerk(Eleazar)", "2561f95609b9d51d71fc58ad262603e6"),
    "grass/green-medium-s.png":
        ("GNU GPL v2+", "J.W. Bjerk(Eleazar)", "ac8148a48499b0f2c421acf77121692b"),
    "grass/green-medium-se.png":
        ("GNU GPL v2+", "J.W. Bjerk(Eleazar)", "fdd5f1937f941ba4d8273c8db9de1c84"),
    "grass/green5.png":
        ("GNU GPL v2+", "J.W. Bjerk(Eleazar)", "39a6962f707cca0bfd7ffadba55827ad"),
    "grass/leaf-litter-long-ne.png":
        ("GNU GPL v2+", "J.W. Bjerk(Eleazar)", "4d337e0e4122f0dc16305d3038ec2150"),
    "grass/leaf-litter-n.png":
        ("GNU GPL v2+", "J.W. Bjerk(Eleazar)", "ebc68a3bb377fa4daad0674f8a08b197"),
    "grass/leaf-litter-nw.png":
        ("GNU GPL v2+", "J.W. Bjerk(Eleazar)", "834ce611ab7751f40765ac412e448360"),
    "grass/leaf-litter-se.png":
        ("GNU GPL v2+", "J.W. Bjerk(Eleazar)", "867b5b08d2812adfc3112f35462dec71"),
    "grass/leaf-litter-sw.png":
        ("GNU GPL v2+", "J.W. Bjerk(Eleazar)", "9e5af0ea4a0034b28a85f554709bf0cd"),
    "grass/leaf-litter6.png":
        ("GNU GPL v2+", "J.W. Bjerk(Eleazar)", "235b7b8829e20ea4d09cd5da4da34d49"),
    "hills/dry-n.png":
        ("GNU GPL v2+", "J.W. Bjerk(Eleazar)", "40b64c0f51b17ff655a956edbb592562"),
    "hills/dry-ne.png":
        ("GNU GPL v2+", "J.W. Bjerk(Eleazar)", "eff52e160d1c8f8b4f6a7ed701df7021"),
    "hills/dry-nw.png":
        ("GNU GPL v2+", "J.W. Bjerk(Eleazar)", "0f4b898550ee93c76535a6fbd0399e5e"),
    "hills/dry-s.png":
        ("GNU GPL v2+", "J.W. Bjerk(Eleazar)", "e7b3e6ddcdbecb8cfe7c0ad912bd20d8"),
    "hills/dry-se.png":
        ("GNU GPL v2+", "J.W. Bjerk(Eleazar)", "97f88429cfd1affca532d0781008c779"),
    "hills/dry-sw.png":
        ("GNU GPL v2+", "J.W. Bjerk(Eleazar)", "bdda266bed0e3ee158644010b37ca607"),
    "hills/dry-to-water-ne.png":
        ("GNU GPL v2+", "unknown", "9311d9909766033de79077326dceae6e"),
    "hills/dry-to-water-nw.png":
        ("GNU GPL v2+", "unknown", "0dece52a169e154675221944df9d2eea"),
    "hills/dry-to-water-s.png":
        ("GNU GPL v2+", "unknown", "08d7890f0d292e635c9a5f50a10ee8b6"),
    "hills/dry-to-water-sw.png":
        ("GNU GPL v2+", "unknown", "34f6b1b01897d98d659f9b7715a9c992"),
    "hills/dry2.png":
        ("GNU GPL v2+", "unknown", "1e2a85f1c296b3d6c7be35852400ae39"),
    "hills/dry3.png":
        ("GNU GPL v2+", "J.W. Bjerk(Eleazar)", "c9573c1293adb06cece8344145dd82bf"),
    "hills/regular-to-water-ne.png":
        ("GNU GPL v2+", "unknown", "5858670a29c7a49b39adab31a805e2d0"),
    "hills/regular-to-water-nw.png":
        ("GNU GPL v2+", "unknown", "f089f4779c5a9d50dbfb508da885a321"),
    "hills/regular-to-water-s.png":
        ("GNU GPL v2+", "unknown", "d08221539c6319222a1b312f82e89321"),
    "hills/regular-to-water-se.png":
        ("GNU GPL v2+", "unknown", "810ce33fb18219283c88967d01f74e50"),
    "hills/regular2.png":
        ("GNU GPL v2+", "unknown", "18feabda4f6dcaf7ae41e49058b52c3e"),
    "masks/7hex-bl-l.png":
        ("GNU GPL v2+", "Lari Nieminen(zookeeper)", "7d475dc4f10d7c82f449421a8ba5e3c8"),
    "masks/7hex-bl.png":
        ("GNU GPL v2+", "Lari Nieminen(zookeeper)", "22cbae6c024136545b6f2286521eae3a"),
    "masks/7hex-br-r.png":
        ("GNU GPL v2+", "Lari Nieminen(zookeeper)", "92b606ccc71153e8176ef5b102813a0b"),
    "masks/7hex-br.png":
        ("GNU GPL v2+", "Lari Nieminen(zookeeper)", "b01825b11c663b86722560a0aa999772"),
    "masks/7hex-l.png":
        ("GNU GPL v2+", "Lari Nieminen(zookeeper)", "9e3e23e951f44e47fa9c8ecf395cf8c4"),
    "masks/7hex-r.png":
        ("GNU GPL v2+", "Lari Nieminen(zookeeper)", "c3e6d51155c1d762f9cc70b099920d6f"),
    "masks/7hex-tl-tr.png":
        ("GNU GPL v2+", "Lari Nieminen(zookeeper)", "473a2352731fd08489c3b85495f253fe"),
    "masks/7hex-tl.png":
        ("GNU GPL v2+", "Lari Nieminen(zookeeper)", "7fcf0928556a7c84109dbefe43645152"),
    "masks/7hex-tr-tl.png":
        ("GNU GPL v2+", "Lari Nieminen(zookeeper)", "95c9d04a47a6a97d50d299bd10a1f1b9"),
    "masks/7hex-tr.png":
        ("GNU GPL v2+", "Lari Nieminen(zookeeper)", "341151b6063a949a126f2281ed0bddb2"),
    "masks/long-concave-2-bl.png":
        ("GNU GPL v2+", "Lari Nieminen(zookeeper)", "8fbaf6d938fe97dcbdd3cb13e3eb2167"),
    "masks/long-concave-2-br.png":
        ("GNU GPL v2+", "Lari Nieminen(zookeeper)", "daf7e745d78fc68c31a08ed09cff3a10"),
    "masks/long-concave-2-l.png":
        ("GNU GPL v2+", "Lari Nieminen(zookeeper)", "8e5af4cdbf4a30b4225d7fdeb36b69ad"),
    "masks/long-concave-2-r.png":
        ("GNU GPL v2+", "Lari Nieminen(zookeeper)", "d19bb635ebf6b38a443d6e8713774487"),
    "masks/long-concave-2-tl.png":
        ("GNU GPL v2+", "Lari Nieminen(zookeeper)", "e67ded35acef826557cee7252a6aef16"),
    "masks/long-concave-2-tr.png":
        ("GNU GPL v2+", "Lari Nieminen(zookeeper)", "86094790d479378cc6bfee0a9df3bd59"),
    "masks/long-convex-bl-br.png":
        ("GNU GPL v2+", "Lari Nieminen(zookeeper)", "db43b6339266ab031ffd4e2923efabeb"),
    "masks/long-convex-bl-l.png":
        ("GNU GPL v2+", "Lari Nieminen(zookeeper)", "536e5f84206adc267984483ad75b915b"),
    "masks/long-convex-br-bl.png":
        ("GNU GPL v2+", "Lari Nieminen(zookeeper)", "222dea69073249b9f597922a4512e294"),
    "masks/long-convex-br-r.png":
        ("GNU GPL v2+", "Lari Nieminen(zookeeper)", "18752b3c53c22db061256fb76df8458a"),
    "masks/long-convex-l-bl.png":
        ("GNU GPL v2+", "Lari Nieminen(zookeeper)", "a242da53680b70c535a8dad4dbde03ef"),
    "masks/long-convex-l-tl.png":
        ("GNU GPL v2+", "Lari Nieminen(zookeeper)", "5b6d32fea2184aa78d194ce53c445b85"),
    "masks/long-convex-r-br.png":
        ("GNU GPL v2+", "Lari Nieminen(zookeeper)", "89c399922324f013e68d47926568bb67"),
    "masks/long-convex-r-tr.png":
        ("GNU GPL v2+", "Lari Nieminen(zookeeper)", "5daadb7806188e60d3b16c90953115e9"),
    "masks/long-convex-tl-l.png":
        ("GNU GPL v2+", "Lari Nieminen(zookeeper)", "03c2840cadfe35280ea071d91221271e"),
    "masks/long-convex-tl-tr.png":
        ("GNU GPL v2+", "Lari Nieminen(zookeeper)", "fe765214ac3c592e6bc08e021e0ea015"),
    "masks/long-convex-tr-r.png":
        ("GNU GPL v2+", "Lari Nieminen(zookeeper)", "3a09492e9c3bd0cd3913034c12c296e8"),
    "masks/long-convex-tr-tl.png":
        ("GNU GPL v2+", "Lari Nieminen(zookeeper)", "7f67ecbc84909727f50908256808be8f"),
    "mountains/basic-castle-ne.png":
        ("GNU GPL v2+", "Hogne Håskjold(freim)", "0a1503c7579218da246ab4f81a6a3242"),
    "mountains/blend-from-dry-s.png":
        ("GNU GPL v2+", "J.W. Bjerk(Eleazar)", "d95d02b959cf0935381b726cb147fd3b"),
    "mountains/blend-from-dry-se.png":
        ("GNU GPL v2+", "J.W. Bjerk(Eleazar)", "1fc88be020695e5075f3dfe15594047a"),
    "mountains/dry-castle-ne.png":
        ("GNU GPL v2+", "J.W. Bjerk(Eleazar)", "ce879ae4f8f2436efdff996a6c9b510d"),
    "sand/desert-n-ne.png":
        ("GNU GPL v2+", "J.W. Bjerk(Eleazar)", "8ff49442e16c95d5cbc8c7529fbf03d4"),
    "sand/desert-n.png":
        ("GNU GPL v2+", "J.W. Bjerk(Eleazar)", "f5f713c2a4a71ff0f89f6dd009ebfa8b"),
    "sand/desert-ne-se-s-sw.png":
        ("GNU GPL v2+", "J.W. Bjerk(Eleazar)", "163e1b738a80053bf33960c5730434fb"),
    "sand/desert-ne-se.png":
        ("GNU GPL v2+", "J.W. Bjerk(Eleazar)", "5242e5aaeef2fc00c92b0c9e689a2027"),
    "sand/desert-ne.png":
        ("GNU GPL v2+", "J.W. Bjerk(Eleazar)", "9acc4413eb45d0597c9fbbd9c9ef30e1"),
    "sand/desert-nw-n.png":
        ("GNU GPL v2+", "J.W. Bjerk(Eleazar)", "ae2e05358b36015a2ebac03cf5a18c91"),
    "sand/desert-nw.png":
        ("GNU GPL v2+", "J.W. Bjerk(Eleazar)", "d1c564a05358455709834fcf9c92e7d7"),
    "sand/desert-s.png":
        ("GNU GPL v2+", "J.W. Bjerk(Eleazar)", "3ef00b6a38e7033481610c0d6002707d"),
    "sand/desert-se.png":
        ("GNU GPL v2+", "J.W. Bjerk(Eleazar)", "0866911d82e8c1162ae39024f10407ed"),
    "sand/desert-sw-nw.png":
        ("GNU GPL v2+", "J.W. Bjerk(Eleazar)", "078da853ba31b82c4239043edeeaca3a"),
    "sand/desert-sw.png":
        ("GNU GPL v2+", "J.W. Bjerk(Eleazar)", "907a2b19c78b3f3a08545c91bbd6498a"),
    "sand/desert2.png":
        ("GNU GPL v2+", "J.W. Bjerk(Eleazar)", "503e9920f012bfaeb21d28f7ba6868fb"),
    "sand/desert4.png":
        ("GNU GPL v2+", "Richard Kettering(Jetrel)", "3cb8cbecd6855503363faf2abb034964"),
    "water/bottom.png":
        ("GNU GPL v2+", "Lari Nieminen(zookeeper)", "2a00254927aa64ce50188ad141400e7c"),
    "water/ocean01.png":
        ("GNU GPL v2+", "Lari Nieminen(zookeeper)", "aca7e46d882696fbcda8d74e7ac96d7c"),
    "water/reef2.png":
        ("GNU GPL v2+", "J.W. Bjerk(Eleazar)", "a360baaf050d192da20a3c8db0826cf4"),
    "water/water01.png":
        ("GNU GPL v2+", "Lari Nieminen(zookeeper)", "366f4290d7f0f558bd0dd88501bdb9f7"),
    "water/waves-concave-A01.png":
        ("GNU GPL v2+", "Lari Nieminen(zookeeper)", "d29168a464a7f3ad2c9ac24a6d90c1b7"),
    "water/waves-convex-A01.png":
        ("GNU GPL v2+", "Lari Nieminen(zookeeper)", "3dac898262edbcd86b6131f96e880838"),
}
# The map a BAKE_OUT run bakes when no map is named
DEFAULT_MAP = "tarnholm"


def out_board(map_id):
    return os.path.join(DRAWABLE, "board_%s.png" % map_id)


def out_preview(map_id):
    return os.path.join(HERE, "preview_board_%s.png" % map_id)


def out_geometry(map_id):
    return os.path.join(HERE, "board_geometry_%s.txt" % map_id)


def out_map(map_id):
    return os.path.join(HERE, "reference", "%s.map" % map_id)


BOARD = 504
MARGIN = 144  # working canvas margin so overhanging sprites never clip early
TILE = 72
COL_STEP = 54  # horizontal distance between hex columns (3/4 of 72)

# Where the board sits inside the Wesnoth reference map: map x = col + MAP_SHIFT_X,
# map y = row + MAP_SHIFT_Y. MAP_SHIFT_X is a multiple of 6 and MAP_SHIFT_Y of 2 because
# Wesnoth picks the piece of the water sheet from (x mod 6, y mod 2).
MAP_SHIFT_X = 6
MAP_SHIFT_Y = 2
MAP_W = 16  # playable map width: board columns -1..9
MAP_H = 9   # playable map height: board rows -1..6

# --------------------------------------------------------------------------
# Hex grid (Wesnoth coordinates, flat topped, odd columns shifted down)
# --------------------------------------------------------------------------

GX = range(-1, 10)  # grid columns that can touch the board (plus a ring of margin)
GY = range(-1, 8)
DIRS = ["n", "ne", "se", "s", "sw", "nw"]
OPPOSITE = {i: (i + 3) % 6 for i in range(6)}


def origin(x, y):
    """Pixel origin (top left of the 72x72 tile) of hex (x, y) in board coordinates."""
    return (COL_STEP * x, TILE * y + (TILE // 2 if x % 2 else 0))


def centre(x, y):
    ox, oy = origin(x, y)
    return (ox + TILE // 2, oy + TILE // 2)


def neighbour(x, y, d):
    if x % 2 == 0:
        deltas = [(0, -1), (1, -1), (1, 0), (0, 1), (-1, 0), (-1, -1)]
    else:
        deltas = [(0, -1), (1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0)]
    dx, dy = deltas[d]
    return (x + dx, y + dy)


def parse_code(text):
    if "^" in text:
        base, ov = text.split("^", 1)
        return (base, ov)
    return (text, None)


def code_str(code):
    return code[0] + ("^" + code[1] if code[1] else "")


# The centre hex sits at col 4, row 3, so its centre is (252, 252), the middle of the
# board. The six outer hexes keep their positions relative to it. Both maps use the same
# seven hexes; only the terrain differs.
CX, CY = 4, 3
SEA = parse_code(SEA_CODE)
SHALLOW = parse_code(SHALLOW_CODE)
LAKE = parse_code(LAKE_CODE)
WHEAT = parse_code(WHEAT_CODE)
HEXES = {
    "NW": (CX - 1, CY - 1),
    "SW": (CX - 1, CY),
    "S": (CX, CY + 1),
    "SE": (CX + 1, CY),
    "NE": (CX + 1, CY - 1),
    "N": (CX, CY - 1),
    "CENTRE": (CX, CY),
}

# One entry per map: the terrain code and the name of each of the seven hexes, and the
# board points that the mountain sprite must leave clear (a village or castle vertex; if
# the art covers one, the 72x72 basic-tile.png is used instead). Anything not listed is
# open sea, SEA_CODE, except the ring of SHALLOW_CODE hexes beside the island.
MAPS = {
    "tarnholm": {
        "terrain": {
            "NW": (("Mm", None), "mountain"),
            "SW": (WHEAT, "wheat"),
            "S": (("Gg", None), "pasture"),
            "SE": (("Gll", "Fds"), "forest"),
            "NE": (("Hhd", None), "hills"),
            "N": (("Dd", None), "desert"),
            "CENTRE": (LAKE, "water"),
        },
        "mountain_clear": [("village 0", (234, 216))],
        "dry_blend": True,
        "thin_beach": True,
    },
    "dunholm": {
        "terrain": {
            "NW": (("Mm", None), "mountain"),
            "SW": (WHEAT, "wheat"),
            "S": (("Gg", None), "pasture"),
            "SE": (("Gll", "Fds"), "forest"),
            "NE": (("Hhd", None), "hills"),
            "N": (("Dd", None), "desert"),
            "CENTRE": (("Dd", None), "desert"),
        },
        "mountain_clear": [("village 2", (234, 216)), ("castle 0", (180, 252))],
        "dry_blend": True,
        "thin_beach": True,
    },
}

# The map being baked; select_map fills these in.
MAP_ID = None
ISLAND = {}
RING = set()
NAMES = {}
MOUNTAIN_CLEAR = []


def select_map(map_id):
    """Make map_id the current map and clear all per bake state (the image cache and the
    record of source files are kept: they only depend on Wesnoth's data)."""
    global MAP_ID, ISLAND, RING, NAMES, MOUNTAIN_CLEAR, MOUNTAIN_DRY_BLEND, THIN_BEACH
    spec = MAPS[map_id]
    MAP_ID = map_id
    ISLAND = {HEXES[n]: code for n, (code, _) in spec["terrain"].items()}
    NAMES = {HEXES[n]: (n, label) for n, (_, label) in spec["terrain"].items()}
    RING = set()
    if SHALLOW_RING:
        for (hx, hy) in ISLAND:
            for d in range(6):
                n = neighbour(hx, hy, d)
                if n not in ISLAND:
                    RING.add(n)
    MOUNTAIN_CLEAR = spec["mountain_clear"]
    MOUNTAIN_DRY_BLEND = spec["dry_blend"]
    if MOUNTAIN_DRY_BLEND_OVERRIDE is not None:
        MOUNTAIN_DRY_BLEND = MOUNTAIN_DRY_BLEND_OVERRIDE.strip().lower() not in ("0", "false", "no", "")
    THIN_BEACH = spec["thin_beach"]
    if THIN_BEACH_OVERRIDE is not None:
        THIN_BEACH = THIN_BEACH_OVERRIDE.strip().lower() not in ("0", "false", "no", "")
    ITEMS.clear()
    FLAGS.clear()
    LOG.clear()


_MAPGRID = None
if os.environ.get("BAKE_MAP"):
    # Experiment mode: take the terrain from a Wesnoth map file laid out like tools/reference/tarnholm.map.
    _MAPGRID = {}
    with open(os.environ["BAKE_MAP"], encoding="utf-8") as _fh:
        for _fy, _line in enumerate(_fh.read().strip().splitlines()):
            for _fx, _tok in enumerate(_line.split(",")):
                _MAPGRID[(_fx - 1 - MAP_SHIFT_X, _fy - 1 - MAP_SHIFT_Y)] = parse_code(_tok.strip())


VIEW = {}  # (x, y) -> code that replaces the real terrain while a rule runs (terrain_view)
ONLY = None  # when a set, apply_transition only draws on these receiving hexes


class terrain_view:
    """Run rules as if some hexes had other terrain (the thin beach: the water hexes beside the
    land count as sand, or the coastal land counts as sand). Only the rule engine sees the view;
    the bases, the water and the overlays always use the real terrain."""

    def __init__(self, view, only=None):
        self.view, self.only = view, only

    def __enter__(self):
        global VIEW, ONLY
        self.saved = (VIEW, ONLY)
        VIEW, ONLY = self.view, self.only

    def __exit__(self, *exc):
        global VIEW, ONLY
        VIEW, ONLY = self.saved


def terrain_at(x, y):
    if (x, y) in VIEW:
        return VIEW[(x, y)]
    if _MAPGRID is not None:
        return _MAPGRID.get((x, y), SEA)
    if (x, y) in ISLAND:
        return ISLAND[(x, y)]
    return SHALLOW if (x, y) in RING else SEA


# --------------------------------------------------------------------------
# Wesnoth terrain list matching (src/terrain/translation.cpp terrain_matches)
# --------------------------------------------------------------------------


def _match_one(code, pat):
    if "^" in pat:
        pb, po = pat.split("^", 1)
    else:
        pb, po = pat, "*"  # no overlay given means any overlay
    base, ov = code
    if not fnmatch.fnmatchcase(base, pb):
        return False
    if po == "*":
        return True
    if ov is None:
        return False
    return fnmatch.fnmatchcase(ov, po)


def tmatch(code, pattern_list):
    result = True
    for item in pattern_list.split(","):
        item = item.strip()
        if item == "*":
            return result
        if item == "!":
            result = not result
            continue
        if _match_one(code, item):
            return result
    return not result


# --------------------------------------------------------------------------
# Image access with a record of every file read
# --------------------------------------------------------------------------

_SOURCES = set()
_CACHE = {}


def exists(rel):
    return os.path.isfile(os.path.join(TERRAIN, rel))


def check_licence(rel, data):
    """Fail the bake unless this Wesnoth file has a recorded licence and author in
    WESNOTH_LICENCES and the bytes read match the recorded MD5 (an altered or swapped file, such
    as one from a newer Wesnoth that may carry a different licence, stops the bake)."""
    if rel not in WESNOTH_LICENCES:
        raise SystemExit("FAIL: %s has no licence and author recorded in WESNOTH_LICENCES" % rel)
    got = hashlib.md5(data).hexdigest()
    want = WESNOTH_LICENCES[rel][2]
    if got != want:
        raise SystemExit("FAIL: %s has MD5 %s, the licence record is for %s" % (rel, got, want))


def load(rel):
    if rel not in _CACHE:
        path = os.path.join(TERRAIN, rel)
        with open(path, "rb") as fh:
            data = fh.read()
        check_licence(rel, data)
        im = Image.open(io.BytesIO(data))
        im.load()
        _CACHE[rel] = im.convert("RGBA")
        _SOURCES.add(rel)
    return _CACHE[rel]


def variants(stem):
    """Existing images stem.png, stem2.png ... stem11.png (NEW:BASE variations)."""
    found = []
    for suffix in [""] + [str(i) for i in range(2, 12)]:
        rel = "%s%s.png" % (stem, suffix)
        if exists(rel):
            found.append(rel)
    return found


def contiguous_variants(stem):
    """Variations ";2;3;...;11" as Wesnoth builds them: it stops at the first missing file."""
    found = []
    for suffix in [""] + [str(i) for i in range(2, 12)]:
        rel = "%s%s.png" % (stem, suffix)
        if not exists(rel):
            break
        found.append(rel)
    return found


_M32 = 0xFFFFFFFF


def _hash_str(text):
    """builder.cpp hash_str."""
    h = 0
    for ch in text.encode("ascii"):
        h = ((((h << 9) & _M32) | (h >> 23)) ^ ch) & _M32
    return h


def rule_hash(*image_strings):
    """building_rule::get_hash: the sum of hash_str over every image string of the rule."""
    h = sum(_hash_str(t) for t in image_strings) & _M32
    return h or 105533


def _noise(x, y, index):
    """builder.cpp get_noise."""
    a = ((x + 92872973) & _M32) ^ 918273
    b = ((y + 1672517) & _M32) ^ 128123
    c = ((index + 127390) & _M32) ^ 13923787
    abc = (a * b * c + a * b + b * c + a * c + a + b + c) & _M32
    return (abc * abc) & _M32


def map_loc(col, row):
    """Position of a board hex in the Wesnoth reference map (playable, 0 based)."""
    return (col + MAP_SHIFT_X, row + MAP_SHIFT_Y)


def pattern_origin(hexpos):
    """The rule origin for a rule whose reference tile 1 sits at pattern position (1, 1)
    (every five line map: mountains, forests, overlays): builder.cpp legacy_difference."""
    x, y = map_loc(*hexpos)
    return (x - 1, y - 2 if x % 2 == 0 else y - 1)


def variant_index(loc, image_string, count, constraints=1):
    """Wesnoth picks variant (noise / 7919) % count. A rule's hash adds the image string once
    per constraint tile that carries the image: a single tile base rule has one, the seven tile
    flower rules (mountains, forests, overlays) add it seven times."""
    return (_noise(loc[0], loc[1], rule_hash(*([image_string] * constraints))) // 7919) % count


_HEXMASK = None


def hexmask():
    global _HEXMASK
    if _HEXMASK is None:
        _HEXMASK = load("alphamask.png").split()[3]
    return _HEXMASK


def min_alpha(img, mask_l):
    """Wesnoth mask_surface: new alpha is the smaller of image and mask alpha."""
    out = img.copy()
    out.putalpha(ImageChops.darker(out.split()[3], mask_l))
    return out


def hexed(img):
    """Every terrain image is cut with the hex mask when Wesnoth loads it."""
    return min_alpha(img, hexmask())


def is_empty(img):
    return img.split()[3].getbbox() is None


# --------------------------------------------------------------------------
# Draw list
# --------------------------------------------------------------------------


class Item:
    def __init__(self, layer, basey, img, px, py, hx, hy, note, seq):
        self.layer, self.basey, self.img = layer, basey, img
        self.px, self.py, self.hx, self.hy = px, py, hx, hy
        self.note, self.seq = note, seq


ITEMS = []
LOG = []  # fallback and decision messages for the report


def add_tile_item(layer, img, hexpos, note, basey=TILE // 2):
    """A 72x72 image belonging to one hex (hex mask applied, as Wesnoth does)."""
    ox, oy = origin(*hexpos)
    ITEMS.append(Item(layer, basey, hexed(img), ox, oy, hexpos[0], hexpos[1], note, len(ITEMS)))


def add_global_image(layer, img, anchor, base_y, note, extra_mask=None, only_anchor=False, skip=()):
    """A WML "global" image: cut into one hex mask clipped piece per hex of the seven hex
    flower around the anchor, the image centre on the anchor hex centre (builder.cpp and
    picture.cpp). Each piece is sorted with the base y of its own tile.
    Returns the number of opaque pixels dropped by only_anchor."""
    w, h = img.size
    ax, ay = centre(*anchor)
    left, top = ax - w // 2, ay - h // 2
    anchor_oy = origin(*anchor)[1]
    dropped = 0
    cells = [(0, 0, anchor[0], anchor[1])]
    for d in range(6):
        nx, ny = neighbour(anchor[0], anchor[1], d)
        cells.append((0, 0, nx, ny))
    for _, _, hx, hy in cells:
        ox, oy = origin(hx, hy)
        box = (ox - left, oy - top, ox - left + TILE, oy - top + TILE)
        if (hx, hy) in skip:
            continue  # the rule marks this tile no_draw=yes
        piece = img.crop(box)
        if extra_mask is not None:
            piece = min_alpha(piece, extra_mask.crop(box))
        piece = hexed(piece)
        if is_empty(piece):
            continue
        if only_anchor and (hx, hy) != tuple(anchor):
            dropped += sum(1 for v in piece.split()[3].getdata() if v > 0)
            continue
        # base y inside the rule: the rule pattern puts its reference tile at pixel y 108
        basey = base_y - (108 + (oy - anchor_oy))
        ITEMS.append(Item(layer, basey, piece, ox, oy, hx, hy, note, len(ITEMS)))
    return dropped


def draw_key(it):
    if DRAW_ORDER == "layer":
        return (it.layer, 0, 0, 0, 0, it.basey, it.seq)
    background = it.layer < 0 or (it.layer == 0 and it.basey < 36 + 18)  # builder.hpp UNITPOS
    # display.cpp generate_hex_key: group, then y, then x parity, then layer, then x / 2.
    return (0 if background else 1, it.hy, it.hx & 1, (it.hx + 3) // 2, it.layer, it.basey, it.seq)


# --------------------------------------------------------------------------
# NEW:TRANSITION port
# --------------------------------------------------------------------------

FLAGS = set()  # (flag, (x, y), direction) already claimed


def run_name(start, length):
    return "-".join(DIRS[(start + i) % 6] for i in range(length))


def maximal_runs(marked):
    """Circular maximal runs of marked directions, as (start, length)."""
    if len(marked) == 6:
        return [(0, 6)]
    runs = []
    for d in sorted(marked):
        if (d - 1) % 6 not in marked:
            length = 1
            while (d + length) % 6 in marked:
                length += 1
            runs.append((d, length))
    return runs


def apply_transition(rule_name, source, receiver, layer, stem, flag="transition", tweak=None):
    """Draw stem-<run>.png on every receiving hex, like NEW:TRANSITION."""
    for (x, y) in [(x, y) for x in GX for y in GY]:
        if ONLY is not None and (x, y) not in ONLY:
            continue
        if not tmatch(terrain_at(x, y), receiver):
            continue
        marked = set()
        for d in range(6):
            nx, ny = neighbour(x, y, d)
            if tmatch(terrain_at(nx, ny), source):
                marked.add(d)
        if not marked:
            continue
        wanted = maximal_runs(marked)
        drawn = []
        remaining = set(marked)
        for length in range(6, 0, -1):
            for start in range(6):
                dirs = [(start + i) % 6 for i in range(length)]
                if not all(d in remaining for d in dirs):
                    continue
                if any(
                    (flag, (x, y), d) in FLAGS
                    or (flag, neighbour(x, y, d), OPPOSITE[d]) in FLAGS
                    for d in dirs
                ):
                    continue
                rel = "%s-%s.png" % (stem, run_name(start, length))
                if not exists(rel):
                    continue
                for d in dirs:
                    FLAGS.add((flag, (x, y), d))
                    FLAGS.add((flag, neighbour(x, y, d), OPPOSITE[d]))
                    remaining.discard(d)
                drawn.append(rel)
                add_tile_item(layer, tweak(load(rel)) if tweak else load(rel), (x, y), "%s %s" % (rule_name, rel))
        # Q2 bookkeeping: report runs that had no single file and fell back
        for start, length in wanted:
            rel = "%s-%s.png" % (stem, run_name(start, length))
            if not exists(rel):
                used = [r for r in drawn if r.startswith(stem + "-")]
                LOG.append(
                    "fallback %s: hex (%d,%d) wants %s (missing); drew %s%s"
                    % (rule_name, x, y, rel, used if used else "nothing",
                       ", left over %s" % sorted(remaining) if remaining else "")
                )


# --------------------------------------------------------------------------
# water/bottom: vertex (corner) transition, NEW:GENERIC_CORNER_TRANSITION
# --------------------------------------------------------------------------

VERTEX = ["tr", "r", "br", "bl", "l", "tl"]  # vertex j sits between DIRS[j] and DIRS[j+1]


def corner_masks(x, y, source, mask_stem):
    """The vertex masks NEW:GENERIC_CORNER_TRANSITION (and the water transitions, which have
    the same three rule shapes) draw on hex (x, y), as (concave, convex) lists of mask files.
    Concave: both neighbours across the vertex are source terrain. Convex: one neighbour is
    the source hex itself while the neighbour on the other side of the vertex is not source
    terrain. The masks are the vertex shaped long-concave-2 and long-convex masks; the vertex
    order is tr, r, br, bl, l, tl (VERTEX), vertex j between DIRS[j] and DIRS[j + 1]."""
    concave = []
    convex = []
    nbr = [neighbour(x, y, d) for d in range(6)]
    for j in range(6):
        if tmatch(terrain_at(*nbr[j]), source) and tmatch(terrain_at(*nbr[(j + 1) % 6]), source):
            concave.append("masks/%s-concave-2-%s.png" % (mask_stem, VERTEX[j]))
    # convex: this hex is the receiver at direction m of a source hex L. Seen from the
    # receiver, L is in direction (m + 3) % 6. The vertex of L between L's neighbour
    # (m) and L's neighbour (m + 1) gives the mask convex-V[m]-V[m-1], and the vertex
    # between (m - 1) and (m) gives convex-V[m-1]-V[m] (macro rules 7, 8 and 6).
    for d in range(6):
        lx, ly = nbr[d]
        if not tmatch(terrain_at(lx, ly), source):
            continue
        m = (d + 3) % 6  # direction of this receiver as seen from L
        ln = [neighbour(lx, ly, k) for k in range(6)]
        if not tmatch(terrain_at(*ln[(m + 1) % 6]), source):
            convex.append("masks/%s-convex-%s-%s.png" % (mask_stem, VERTEX[m], VERTEX[(m - 1) % 6]))
        if not tmatch(terrain_at(*ln[(m - 1) % 6]), source):
            convex.append("masks/%s-convex-%s-%s.png" % (mask_stem, VERTEX[(m - 1) % 6], VERTEX[m]))
    return concave, convex


def apply_corner(rule_name, source, receiver, layer, stem, mask_stem, opacity=100):
    """NEW:GENERIC_CORNER_TRANSITION. A receiving hex gets the image masked at every vertex
    where the corner rules of corner_masks fire."""
    for x in GX:
        for y in GY:
            if not tmatch(terrain_at(x, y), receiver):
                continue
            concave, convex = corner_masks(x, y, source, mask_stem)
            for kind, files in (("concave", concave), ("convex", convex)):
                if not files:
                    continue
                mask = Image.new("RGBA", (TILE, TILE), (0, 0, 0, 0))
                for rel in sorted(set(files)):  # ~BLIT of the masks; a flag stops repeats
                    mask.alpha_composite(load(rel))
                alpha = mask.split()[3]
                if opacity != 100:
                    alpha = alpha.point(lambda v: v * opacity // 100)
                add_tile_item(layer, min_alpha(load(stem + ".png"), alpha), (x, y),
                              "%s %s (%d,%d)" % (rule_name, kind, x, y))


def apply_water_transition(rule_name, source, receiver, layer, sheet_rel):
    """NEW:WATER_342_180_TRANSITION (terrain-graphics.cfg:1015-1016): the water of the source
    terrain is drawn at half opacity on the neighbouring water hexes, one image per vertex,
    cut from the source's own water sheet at the receiving hex's place in the sheet and masked
    by the same long-concave-2 and long-convex vertex masks as the corner rules."""
    sheet = load(sheet_rel)
    for x in GX:
        for y in GY:
            if not tmatch(terrain_at(x, y), receiver):
                continue
            concave, convex = corner_masks(x, y, source, "long")
            for kind, files in (("concave", concave), ("convex", convex)):
                for rel in sorted(set(files)):
                    img = min_alpha(water_crop(sheet, x, y), load(rel).split()[3])
                    alpha = img.split()[3].point(lambda v: (v * WATER_BLEND_ALPHA) >> 8)
                    img.putalpha(alpha)
                    add_tile_item(layer, img, (x, y), "%s %s %s (%d,%d)" % (rule_name, kind, rel, x, y))


# --------------------------------------------------------------------------
# NEW:WAVES: beach waves (desert coast), layer -499
# --------------------------------------------------------------------------


def flower(x, y):
    """The hex and its six neighbours."""
    return [(x, y)] + [neighbour(x, y, d) for d in range(6)]


def apply_waves(land, water, layer, stem, water_only=False):
    frame = "A%02d" % WAVES_FRAME
    convex = load("%s-convex-%s.png" % (stem, frame))
    concave = load("%s-concave-%s.png" % (stem, frame))

    def kind(code):
        if tmatch(code, water):
            return "W"
        if tmatch(code, land):
            return "L"
        return "O"

    def draw(img, mask_name, x, y, skip=()):
        mask = load("masks/7hex-%s.png" % mask_name).split()[3]
        if water_only:
            # the foam of the thin beach goes on the water hexes (and on real desert, where
            # Wesnoth draws it anyway), never over grass or hills
            skip = set(skip) | {h for h in flower(x, y) if not tmatch(terrain_at(*h), "W*,D*")}
        # base=90,144 on a 180x216 image: the rule's own reference y
        add_global_image(layer, img, (x, y), 144, "waves %s (%d,%d)" % (mask_name, x, y), mask, skip=skip)

    for x in GX:
        for y in GY:
            h = kind(terrain_at(x, y))
            if h == "O":
                continue
            for j in range(6):
                na, nb = neighbour(x, y, j), neighbour(x, y, (j + 1) % 6)
                a, b = kind(terrain_at(*na)), kind(terrain_at(*nb))
                v, vprev, vnext = VERTEX[j], VERTEX[(j - 1) % 6], VERTEX[(j + 1) % 6]
                me = (x, y)
                # Rules 1 and 2 draw on all seven tiles. The four three-way rules mark some
                # tiles no_draw=yes (the centre tile and the "other land" tile), listed here.
                if h == "L" and a == "W" and b == "W":
                    draw(convex, v, x, y)
                elif h == "W" and a == "L" and b == "L":
                    draw(concave, v, x, y)
                elif h == "L" and a == "W" and b == "O":
                    draw(convex, v, x, y, skip={me, nb})
                elif h == "W" and a == "O" and b == "L" and WAVES_THREE_WAY_CONCAVE:
                    draw(concave, v + "-" + vnext, x, y, skip={me, na})
                elif h == "L" and a == "O" and b == "W":
                    draw(convex, v, x, y, skip={me, na})
                elif h == "W" and a == "L" and b == "O" and WAVES_THREE_WAY_CONCAVE:
                    draw(concave, v + "-" + vprev, x, y, skip={me, nb})


# --------------------------------------------------------------------------
# Water (NEW:WATER_342_180) and the water family
# --------------------------------------------------------------------------


def water_crop(sheet, x, y):
    """The 72x72 piece of the sheet Wesnoth uses for hex (x, y): the 342x180 sheet is cut
    by (x mod 6, y mod 2), see WATER_342_180_TILE_VARIANTS in new-macros.cfg."""
    sx = COL_STEP * (x % 6)
    sy = TILE * (y % 2) + (TILE // 2 if x % 2 else 0)
    return sheet.crop((sx, sy, sx + TILE, sy + TILE))


def add_water():
    sheets = {}

    def sheet(rel):
        if rel not in sheets:
            sheets[rel] = load(rel)
        return sheets[rel]

    for x in GX:
        for y in GY:
            code = terrain_at(x, y)
            if not tmatch(code, "W*"):
                continue
            base = code[0]
            if base.startswith("Wo"):
                rel = "water/ocean%02d.png" % WATER_FRAME   # terrain-graphics.cfg:705
            else:
                rel = "water/water%02d.png" % WATER_FRAME   # terrain-graphics.cfg:706
            add_tile_item(-999, water_crop(sheet(rel), x, y), (x, y), "water %s (%d,%d)" % (base, x, y))
            if base in ("Wwg", "Wog", "Wwrg"):
                add_tile_item(-502, load("water/overlay-gray.png"), (x, y), "gray overlay")
            if base in ("Wwt", "Wot", "Wwrt"):
                add_tile_item(-504, load("water/overlay-tropical.png"), (x, y), "tropical overlay")
            if base == "Wwf":
                add_tile_item(-519, load("water/ford.png"), (x, y), "ford overlay")


def add_water_transitions():
    """terrain-graphics.cfg:1015-1016, drawn at -550 and -551 (below the coast): deep ocean
    blends onto the shallow water beside it, and shallow water onto the deep ocean."""
    apply_water_transition("ocean-onto-shallow", "Wo*", "!,Wo*,!,W*,Sm", -550, "water/ocean%02d.png" % WATER_FRAME)
    apply_water_transition("shallow-onto-ocean", "Ww*", "!,Ww*,!,W*,Sm", -551, "water/water%02d.png" % WATER_FRAME)


def add_reefs():
    """terrain-graphics.cfg:493: NEW:OVERLAY Wwr,Wwrt,Wwrg water/reef at layer -320, one of
    reef.png, reef2.png, reef3.png, reef4.png picked like every other overlay variant."""
    for y in GY:
        for x in GX:
            code = terrain_at(x, y)
            if code[0] not in ("Wwr", "Wwrt", "Wwrg"):
                continue
            files = contiguous_variants("water/reef")
            if WESNOTH_VARIANT_NOISE:
                rel = files[variant_index(pattern_origin((x, y)), "water/reef@V.png:", len(files), 7)]
            else:
                rel = files[random.Random(x * 1000 + y).randrange(len(files))]
            add_tile_item(-320, load(rel), (x, y), "reef %s (%d,%d)" % (rel, x, y))


# --------------------------------------------------------------------------
# Compose
# --------------------------------------------------------------------------

SMALL_FOREST_FILTER = "C*,K*,X*,Q*,W*,Ai,M*,*^Qh*,*^V*,*^B*,_off^_usr"  # terrain-graphics.cfg:185


# NEW:FOREST image stems by overlay (terrain-graphics.cfg:114 pine, :136 summer deciduous)
FOREST_STEMS = {"Fp": "forest/pine", "Fds": "forest/deciduous-summer"}


def forest_is_small(x, y):
    """NEW:FOREST draws the -small sprite when any neighbour is in the filter list."""
    for d in range(6):
        if tmatch(terrain_at(*neighbour(x, y, d)), "!,_dum,!," + SMALL_FOREST_FILTER):
            return True
    return False


def grow(mask, radius):
    """Distance transform with whole pixel offsets only: an L image where each pixel holds the
    smallest r (0..radius) such that some pixel of mask lies within a disc of radius r, or
    radius + 1 when none does. mask is an L image, 255 inside."""
    rings = {}
    for dx in range(-radius, radius + 1):
        for dy in range(-radius, radius + 1):
            r = int(math.ceil(math.sqrt(dx * dx + dy * dy)))
            if 0 < r <= radius:
                rings.setdefault(r, []).append((dx, dy))
    reached = [mask]
    cur = mask
    for r in range(1, radius + 1):
        for dx, dy in rings[r]:
            cur = ImageChops.lighter(cur, ImageChops.offset(mask, dx, dy))
        reached.append(cur)
    dist = Image.new("L", mask.size, radius + 1)
    for r in range(radius, -1, -1):
        dist.paste(r, mask=reached[r])
    return dist


def beach_hexes():
    """(sea, land): the water hexes that have land beside them, and the land hexes that have
    water beside them."""
    sea, land = set(), set()
    for x in GX:
        for y in GY:
            code = terrain_at(x, y)
            near = [terrain_at(*neighbour(x, y, d)) for d in range(6)]
            if tmatch(code, "W*"):
                if any(not tmatch(c, "W*") for c in near):
                    sea.add((x, y))
            elif any(tmatch(c, "W*") for c in near):
                land.add((x, y))
    return sea, land


def sand_floor(img):
    """The beach sand with the faintest part of its ramp cut off (alpha below BEACH_SAND_FLOOR
    goes to 0, the rest is stretched back to 0..255): the wisps at the tail of the dither read as
    smoke over water."""
    if not BEACH_SAND_FLOOR:
        return img
    lo = BEACH_SAND_FLOOR
    out = img.copy()
    out.putalpha(img.split()[3].point(lambda v: 0 if v <= lo else min(255, (v - lo) * 255 // (255 - lo))))
    return out


def thin_beach_pass():
    """THIN_BEACH: every coast meets the sea the way the desert hex does. Nothing is drawn on a
    land hex that the beach off board does not draw (the assertion in bake_map checks it).

    1. Sand: on each water hex beside non desert land, the sand image of the run of directions
       the land lies in (sand/<stem>-<run>.png, longest run first, as NEW:TRANSITION), layer
       -510, as if that land were a beach. Beside real desert, Wesnoth's own rule (sand/desert
       onto the neighbour of a desert hex) already draws the same images the same way.
    2. Land onto sand: every land rule that has sand as receiver (hills/regular, hills/dry,
       grass/green-medium and its dry and abrupt kin, the farmland edge, flat/dirt) is run with
       the coastal water hexes standing in for sand, drawing on those hexes only, so the ragged
       fringes of grass, hills, mountain and field reach out over the sand.
    3. The earth bank family (flat/bank, flat/bank-to-ice, hills-to-water, the grass-abrupt
       and dirt rim against water) is not drawn at all.
    4. Water/bottom as the beach off board; Wesnoth's beach foam (the waves images) with every
       coast counted as sand, on the water hexes (and the desert hex as before).
    """
    coast_sea, coast_land = beach_hexes()
    sandy = {h: ("Ds", None) for h in coast_land if not tmatch(terrain_at(*h), "D*")}
    with terrain_view(sandy):
        for i in range(BEACH_SAND_PASSES):
            apply_transition("beach-shore", "Ds", "W*", -510 + i, BEACH_STEM, flag="shore%d" % i,
                             tweak=sand_floor)
    wet = {h: ("Ds", None) for h in coast_sea}
    with terrain_view(wet, only=coast_sea):
        land_rules(False)
    apply_corner("water-bottom", "!,D*,Hd,Chw,Khw,Khs,W*,S*,Xv,Qx*,A*,Rra,Rrd,_*", "W*",
                 -500, "water/bottom", "long", opacity=BOTTOM_OPACITY)
    apply_waves("!,W*", "W*", -499, "water/waves", water_only=True)


def run_transitions():
    """Every edge, corner and wave rule of the map, in terrain-graphics.cfg file order. With the
    thin beach on, see thin_beach_pass: the land rules run for the land to land edges only and
    the coast is the beach."""
    land_rules(THIN_BEACH)
    if THIN_BEACH:
        thin_beach_pass()
    else:
        water_coast_rules()
    # Water to water transitions (below everything else)
    add_water_transitions()


def land_rules(beach):
    # :1228 hills and mountains fringe onto everything that is not mountain,
    #       hills, ice, water or swamp (shares the default "transition" flag)
    if MOUNTAIN_DRY_BLEND:
        # :901 and :903 mountains beside dry terrain (desert, dirt, dry grass): the mountain
        #      hex gets mountains/blend-from-dry at layer 0 (own "inside" flag) and the dry
        #      hex gets hills/dry, which claims the edge before hills/regular below does
        dry = "Hd,Hhd,Rb,Re,Rd,D*,Gd,Ha,A*,U*,Ql*"
        apply_transition("mountain-blend-from-dry", dry, "Mm", 0, "mountains/blend-from-dry", flag="inside")
        apply_transition("hills-dry", "Mm", dry, -166, "hills/dry")
    apply_transition("hills", "Mm,Hh", "!,M*,Hh,Ai,W*,S*", -180, "hills/regular")
    # :912 dry hills fringe onto everything that is not mountain, dry hills, ice, water or
    #      swamp: the desert, the leaf litter under the forest (and the sand of the thin beach)
    apply_transition("hills-dry-fringe", "Md,Hhd", "!,M*,Hhd,Ai,W*,S*", -183, "hills/dry")
    # :1240-1262 grass blending. The "inside" flag rules claim one fringe per edge,
    #       then the plain rules claim the transition flag.
    roads = "Re,Rb,Rd,Rp,Exos"
    apply_transition("green-long-inside", "Gg", "Gs,Gd,Gll," + roads, -251, "grass/green-long",
                     flag="inside")
    apply_transition("dry-long-inside", "Gd", "Gg,Gs,Gll," + roads, -252, "grass/dry-long",
                     flag="inside")
    # :936 and :938 leaf litter against grass. Where it meets green grass the green grass has
    #      already claimed the edge's "inside" flag (green-long on the leaf litter hex), so the
    #      first of these draws nothing there and the second puts leaf-litter-long on the grass
    #      hex: the two fringes cross, one on each side of the edge.
    apply_transition("leaf-litter-long-inside", "Gll", "Gg,Gs,Gd," + roads, -253, "grass/leaf-litter-long",
                     flag="inside")
    apply_transition("leaf-litter-long", "Gll", "Gg,Gs,Gd", -254, "grass/leaf-litter-long")
    apply_transition("dry-long", "Gd", "Gg,Gs,Gll", -255, "grass/dry-long")
    apply_transition("green-long", "Gg", "Gs,Gd,Gll", -256, "grass/green-long")
    # :1261 grass onto roads, desert, snow, caves
    apply_transition("green-medium", "Gg", "R*,D*,Aa,Ur,Urc,Isa", -261, "grass/green-medium")
    apply_transition("dry-medium", "Gd", "R*,D*,Aa,Ur,Urc,Isa", -262, "grass/dry-medium")
    # :1265 grass onto everything that is not grass or hills/mountain/castle,
    #       which includes the sea: this is the green coast fringe. The thin beach drops it
    #       on the water (coast = "!" list gains W*).
    coast = ",W*" if beach else ""
    # :947 leaf litter onto everything that is not leaf litter, chasm, water, ice or castle: the
    #      desert (Dunholm's centre) and the sand of the thin beach. Its edge with the dry hills
    #      is already claimed by hills/dry above, its edge with the grass by leaf-litter-long.
    apply_transition("leaf-litter", "Gll", "!,Gll,Q*,W*,Ai,C*,K*", -270, "grass/leaf-litter")
    apply_transition("grass-abrupt", "Gg*,Qx*", "!,Gg*,Q*,Mm,Ms,Hh,C*,K*" + coast, -271, "grass/green-abrupt")
    apply_transition("dry-abrupt", "Gd", "!,Gd,Q*,C*,K*" + coast, -273, "grass/dry-abrupt")
    if FIELD_GRASS_VERGE and not beach:
        # Not a Wesnoth rule, see FIELD_GRASS_VERGE.
        apply_transition("field-verge", "Re^Gvs", "W*", -272, "grass/dry-abrupt")
    # :956 flat/bank: receivers (!,Rra,!,R*,Gll,Ias) are dirt roads and leaf litter only.
    #      Dropped by the thin beach: the earth bank round the coast.
    if not beach:
        apply_transition("bank", "W*,Ai", "!,Rra,!,R*,Gll,Ias", -300, "flat/bank")
    # :961 desert fringe onto roads and dirt (R* includes Re, the farm field), above most
    #      of the desert transitions
    apply_transition("desert-onto-dirt", "Dd", "R*", -319, "sand/desert")
    # :1292 farmland edge, own flag so it stacks with the grass rules
    apply_transition("farm", "*^Gvs", "!,*^Gvs,C*,K*,*^F*,M*,H*,W*,Q*", -330,
                     "embellishments/farm-veg-spring", flag="transition2")
    # :1296 desert onto land that is not roads, desert, water
    apply_transition("desert", "Dd", "!,R*,Dd,W*,S*,Ai,Q*", -510, "sand/desert")
    # :1379-1380 dirt fringes onto neighbours that are not dirt, roads, water, desert ...
    apply_transition("dirt-inside", "!,Re,Rr*,Hh*,M*,Q*,D*,T*", "Re", -379, "flat/dirt", flag="inside")
    # (with the coastal water standing in for sand, the thin beach lets the dirt fringe reach out
    #  over it: Wesnoth has no dirt onto sand image, so this is the field's own ragged edge)
    dirt_receivers = "!,Re,Rr*,W*,Ai,Q*,T*" if ONLY is not None else "!,Re,Rr*,W*,Ai,Q*,D*,T*"
    apply_transition("dirt", "Re", dirt_receivers, -380, "flat/dirt")
    if not beach:
        # :1315 hills and mountains reaching into water
        apply_transition("hills-to-water", "Mm,Hh", "Ai,W*,S*", -482, "hills/regular-to-water",
                         flag="non_submerged")
        # :996 the same for dry hills
        apply_transition("dry-hills-to-water", "Md,Hhd,Mv", "Ai,W*,S*", -482, "hills/dry-to-water",
                         flag="non_submerged")
        # :997 flat/bank-to-ice drawn on the water hex beside dirt and grass: the brown pebble
        #      rim round every grass, dirt and field coast (the name is misleading, it is the shore)
        apply_transition("bank-to-ice", "!,Rra,!,R*,G*,Uue,Ias", "Ai,W*", -483, "flat/bank-to-ice",
                         flag="non_submerged")


def water_coast_rules():
    # Water bottom on water vertices that touch two non-desert land hexes
    apply_corner("water-bottom", "!,D*,Hd,Chw,Khw,Khs,W*,S*,Xv,Qx*,A*,Rra,Rrd,_*", "W*",
                 -500, "water/bottom", "long", opacity=BOTTOM_OPACITY)
    # Beach waves around desert (NEW:WAVES)
    apply_waves("D*,Hd,Rrd", "W*", -499, "water/waves")


def bayer(n):
    """n x n ordered dither matrix (n a power of two), values 0..n*n-1."""
    m = [[0]]
    size = 1
    while size < n:
        m = [[4 * m[y % size][x % size] + [[0, 2], [3, 1]][y // size][x // size]
              for x in range(size * 2)] for y in range(size * 2)]
        size *= 2
    return m


BAYER8 = bayer(8)


def is_green(r, g, b):
    h, sat, _ = colorsys.rgb_to_hsv(r / 255.0, g / 255.0, b / 255.0)
    return (MOUNTAIN_DRY_HUE_MIN <= h * 360 <= MOUNTAIN_DRY_HUE_MAX
            and sat * 100 >= MOUNTAIN_DRY_SAT_MIN)


def dry_neighbours(hexpos):
    return [d for d in range(6) if tmatch(terrain_at(*neighbour(hexpos[0], hexpos[1], d)), "D*")]


def dry_strength(hexpos):
    """Function (board x, y) -> 0..1: how far a point of the mountain hex is towards its
    desert neighbours. 1 on the shared edge's midline, 0 on the hex's centre line; for several
    desert neighbours the largest."""
    cx, cy = centre(*hexpos)
    dirs = []
    for d in range(6):
        n = neighbour(hexpos[0], hexpos[1], d)
        if tmatch(terrain_at(*n), "D*"):
            nx, ny = centre(*n)
            dist = math.hypot(nx - cx, ny - cy)
            dirs.append(((nx - cx) / dist, (ny - cy) / dist, dist / 2.0))

    def strength(x, y):
        best = 0.0
        for ux, uy, half in dirs:
            best = max(best, ((x + 0.5 - cx) * ux + (y + 0.5 - cy) * uy) / half)
        return min(1.0, max(0.0, best * MOUNTAIN_DRY_REACH / 100.0))
    return strength


def dry_remap(img, dry, left, top, strength):
    """img with every green pixel swapped for the pixel of dry (same size) at the same place
    where the dither of the strength says so. left, top: where img sits on the board."""
    out = img.copy()
    px, dpx, opx = img.load(), dry.load(), out.load()
    for y in range(img.height):
        for x in range(img.width):
            r, g, b, a = px[x, y]
            if not a or not is_green(r, g, b):
                continue
            bx, by = left + x, top + y
            if strength(bx, by) * 64 > BAYER8[by % 8][bx % 8] + 0.5:
                opx[x, y] = dpx[x, y][:3] + (a,)
    return out


def bake():
    rng = random.Random(1)
    size = BOARD + 2 * MARGIN
    canvas = Image.new("RGBA", (size, size), (0, 0, 0, 255))

    add_water()
    add_reefs()

    # --- base tiles (NEW:BASE), layer -1000 --------------------------------
    for y in GY:
        for x in GX:
            code = terrain_at(x, y)
            base = code[0]
            if tmatch(code, "W*"):
                continue
            if base == "Gg":
                stem = "grass/green"
            elif base == "Gd":
                stem = "grass/dry"
            elif base == "Gll":
                stem = "grass/leaf-litter"  # terrain-graphics.cfg:464
            elif base == "Re":
                stem = "flat/dirt"
            elif base in ("Hh", "Mm"):
                stem = "hills/regular"  # terrain-graphics.cfg:338 hills under mountains
            elif base == "Hhd":
                stem = "hills/dry"  # terrain-graphics.cfg:476
            elif base == "Dd":
                stem = "sand/desert"
            else:
                raise SystemExit("no base for " + base)
            if WESNOTH_VARIANT_NOISE:
                files = contiguous_variants(stem)
                loc = map_loc(x, y)
                prob = {"Gg": 20, "Gd": 25}.get(base)
                # NEW:BASE_P: a rule for the plain tile with a probability, tried first
                if prob is not None and _noise(loc[0], loc[1], rule_hash(stem + ".png")) % 100 <= prob:
                    rel = files[0]
                else:
                    rel = files[variant_index(loc, stem + "@V.png", len(files))]
            else:
                files = variants(stem)
                if base in ("Gg", "Gd") and rng.random() < (0.2 if base == "Gg" else 0.25):
                    # NEW:BASE_P Gg 20 / Gd 25: the plain tile is the common, generic one
                    rel = files[0]
                else:
                    rel = files[rng.randrange(len(files))]
            tile = load(rel)
            if base == "Mm" and MOUNTAIN_NE_DRY and dry_neighbours((x, y)):
                dries = contiguous_variants("hills/dry")
                dry = load(dries[files.index(rel) % len(dries)] if rel in files else dries[0])
                tile = dry_remap(tile, dry, *origin(x, y), dry_strength((x, y)))
            add_tile_item(-1000, tile, (x, y), "base %s (%d,%d) %s" % (base, x, y, rel))

    # --- transitions in terrain-graphics.cfg file order ---------------------
    run_transitions()

    # --- overlays ------------------------------------------------------------
    for y in GY:
        for x in GX:
            code = terrain_at(x, y)
            if code[1] == "Gvs":
                # terrain-graphics.cfg:511 NEW:OVERLAY *^Gvs farm-veg-spring, layer -81
                files = contiguous_variants("embellishments/farm-veg-spring")
                if WESNOTH_VARIANT_NOISE:
                    rel = files[variant_index(pattern_origin((x, y)), "embellishments/farm-veg-spring@V.png:", len(files), 7)]
                else:
                    rel = files[rng.randrange(len(files))]
                add_tile_item(-81, load(rel), (x, y), "farm overlay %s" % rel)

    # --- layer 0 sprites: forest and mountain ---------------------------------
    for y in GY:
        for x in GX:
            code = terrain_at(x, y)
            if code[1] in FOREST_STEMS:
                small = forest_is_small(x, y) and not FOREST_FULL_SPRITE
                # NEW:FOREST: on hills and mountains the -sparse stem, elsewhere the plain one
                stem = FOREST_STEMS[code[1]] + ("-sparse" if tmatch(code, "H*,M*") else "")
                if small:
                    stem += "-small"
                files = contiguous_variants(stem)
                if FOREST_IN_HEX:
                    rel = files[0]
                elif WESNOTH_VARIANT_NOISE:
                    rel = files[variant_index(pattern_origin((x, y)), stem + "@V.png", len(files), 7)]
                else:
                    rel = files[rng.randrange(len(files))]
                # NEW:FOREST: base=90,161 centre=90,144 on an image centred on the hex
                dropped = add_global_image(0, load(rel), (x, y), 161, "forest (%d,%d) %s" % (x, y, rel),
                                           only_anchor=FOREST_IN_HEX)
                LOG.append("info: forest sprite %s, %s; pixels outside the hex dropped: %d"
                           % (rel, "small (beside water)" if small else "full size", dropped))
    # mountains/basic.png is 180x216 with its centre on the hex centre (the image is
    # exactly the seven hex flower). If its art covers a point in MOUNTAIN_CLEAR (a village or
    # castle vertex the mountain hex touches) fall back to the 72x72 basic-tile.png.
    mountain_sprite = "mountains/basic.png"
    mountain_files = contiguous_variants("mountains/basic")
    mountains = []  # (art, left, top) of every mountain sprite drawn, for the dock's check
    for (mx, my) in [(x, y) for x in GX for y in GY]:
        if terrain_at(mx, my)[0] != "Mm":
            continue
        if MOUNTAIN_SPRITE and MOUNTAIN_SPRITE != "wesnoth":
            mountain_sprite = MOUNTAIN_SPRITE  # DEPARTURE, see MOUNTAIN_SPRITE
        elif WESNOTH_VARIANT_NOISE:
            mountain_sprite = mountain_files[variant_index(pattern_origin((mx, my)), "mountains/basic@V.png", len(mountain_files), 7)]
        art = load(mountain_sprite)
        cx, cy = centre(mx, my)
        left, top = cx - art.width // 2, cy - art.height // 2
        bbox = art.split()[3].getbbox()
        LOG.append("info: mountain %s art bbox on board: x %d..%d y %d..%d"
                   % (mountain_sprite, left + bbox[0], left + bbox[2], top + bbox[1], top + bbox[3]))
        covered = False
        for label, (px, py) in MOUNTAIN_CLEAR:
            hit = (0 <= px - left < art.width and 0 <= py - top < art.height
                   and art.getpixel((px - left, py - top))[3] > 128)
            covered = covered or hit
            LOG.append("info: mountain %s covers %s vertex (%d,%d): %s" % (mountain_sprite, label, px, py, hit))
        if covered:
            mountain_sprite = "mountains/basic-tile.png"
            LOG.append("info: mountain switched to basic-tile.png")
            art = load(mountain_sprite)
            left, top = cx - art.width // 2, cy - art.height // 2
        if MOUNTAIN_NE_DRY and dry_neighbours((mx, my)):
            dry_rel = mountain_sprite.replace("/basic", "/dry")
            if exists(dry_rel):
                art = dry_remap(art, load(dry_rel), left, top, dry_strength((mx, my)))
                LOG.append("info: mountain %s repainted towards %s on the desert side" % (mountain_sprite, dry_rel))
            else:
                LOG.append("info: no dry twin of %s, mountain not repainted" % mountain_sprite)
        add_global_image(0, art, (mx, my), 90, "mountain (%d,%d)" % (mx, my))
        mountains.append((art, left, top))

    # --- draw -----------------------------------------------------------------
    for it in sorted(ITEMS, key=draw_key):
        canvas.alpha_composite(it.img, (it.px + MARGIN, it.py + MARGIN))

    if DOCK:
        piece, (left, top) = dock_piece()
        check_dock(piece, left, top, mountains)
        check_dock_posts(canvas, piece, left, top)
        sunk = dock_sink(canvas)
        canvas.alpha_composite(piece, (left + MARGIN, top + MARGIN))
        LOG.append("info: dock (authored, %d colours above water, plus %d pixels under the water derived from the board) at board x %d..%d y %d..%d, foot (%d,%d), mooring (%d,%d)"
                   % ((len(DOCK_COLOURS), sunk, left, left + piece.width - 1, top, top + piece.height - 1)
                      + dock_foot() + dock_mooring()))

    return canvas.crop((MARGIN, MARGIN, MARGIN + BOARD, MARGIN + BOARD)).convert("RGB")


# --------------------------------------------------------------------------
# The dock (DEPARTURE, see DOCK)
# --------------------------------------------------------------------------


def dock_foot():
    """The vertex where the start road meets the sea: the NE vertex of the NW hex, which is the
    W vertex of the N hex."""
    nw = centre(*HEXES["NW"])
    n = centre(*HEXES["N"])
    foot = (nw[0] + 18, nw[1] - 36)
    assert foot == (n[0] - 36, n[1]), foot
    return foot


def dock_mooring():
    foot = dock_foot()
    return (foot[0] + DOCK_MOORING_FROM_FOOT[0], foot[1] + DOCK_MOORING_FROM_FOOT[1])


def check_dock(piece, left, top, mountains):
    """The dock stands on sand and water only: no pixel of it may lie over a pixel of a mountain
    sprite (which rises above its own hex), nor inside any land hex but a desert one (the foot of
    the deck is on the corner of the desert's sand)."""
    px = piece.load()
    over_mountain = in_land = 0
    for y in range(piece.height):
        for x in range(piece.width):
            if px[x, y][3] == 0:
                continue
            bx, by = left + x, top + y
            for art, ml, mt in mountains:
                if 0 <= bx - ml < art.width and 0 <= by - mt < art.height and art.getpixel((bx - ml, by - mt))[3] > 0:
                    over_mountain += 1
            for pos, (name, terrain) in NAMES.items():
                ox, oy = origin(*pos)
                if (terrain != "desert" and 0 <= bx - ox < TILE and 0 <= by - oy < TILE
                        and hexmask().getpixel((bx - ox, by - oy)) > 0):
                    in_land += 1
    LOG.append("info: dock pixels over a mountain sprite: %d, inside a land hex that is not desert: %d (both must be 0)"
               % (over_mountain, in_land))
    if over_mountain or in_land:
        raise SystemExit("the dock lies on the mountain or on land: %d, %d pixels" % (over_mountain, in_land))


def dock_spans(piece, left, top):
    """Per row of the dock's art, each run of columns it touches: 'y:x0-x1' (a row with a gap, such
    as the piles beside the deck's seaward end, gives one word per run)."""
    px = piece.load()
    out = []
    for y in range(piece.height):
        x = 0
        while x < piece.width:
            if px[x, y][3] == 0:
                x += 1
                continue
            x1 = x
            while x1 + 1 < piece.width and px[x1 + 1, y][3] > 0:
                x1 += 1
            out.append("%d:%d-%d" % (top + y, left + x, left + x1))
            x = x1 + 1
    return out


def dock_grid():
    """The dock's above water art as {(board x, board y): key of DOCK_COLOURS}: the deck and the
    piles down to their last row out of the water."""
    grid = {}
    # the deck: cross planks of two alternating tones with a seam row between them, a dark edge
    # down each long side and across both ends
    for y in range(DOCK_DECK_Y[0], DOCK_DECK_Y[1] + 1):
        k, r = divmod(y - DOCK_DECK_Y[0], DOCK_PLANK)
        x0, x1 = DOCK_DECK_X
        for x in range(x0, x1 + 1):
            if x in (x0, x1) or y in DOCK_DECK_Y:
                grid[(x, y)] = "o"
            elif r == DOCK_PLANK - 1:
                grid[(x, y)] = "s"
            else:
                grid[(x, y)] = "a" if k % 2 == 0 else "b"
    # the piles: a lit left edge, a shaded right one and a lit cap
    for x0, y0 in DOCK_POSTS:
        for y in range(y0, y0 + DOCK_ABOVE):
            grid[(x0, y)] = "l"
            grid[(x0 + 1, y)] = "l" if y == y0 else "d"
    return grid


def dock_piece():
    """The authored dock, above the water, and where its top left goes on the board: the sprite
    that is also written as dock.png. The box is the art's own, in the colours of DOCK_COLOURS
    and hard alpha only."""
    foot = dock_foot()
    assert foot == (216, 180), foot     # the numbers above are board pixels round this vertex
    grid = dock_grid()
    left, top = min(x for x, _ in grid), min(y for _, y in grid)
    width, height = max(x for x, _ in grid) - left + 1, max(y for _, y in grid) - top + 1
    piece = Image.new("RGBA", (width, height), (0, 0, 0, 0))
    out = piece.load()
    for (x, y), key in grid.items():
        hx = DOCK_COLOURS[key]
        out[x - left, y - top] = (int(hx[0:2], 16), int(hx[2:4], 16), int(hx[4:6], 16), 255)
    return piece, (left, top)


def dock_post_pixels(x0, y0):
    """Every pixel a pile gives to the board, as (x, y, kind) with kind 'above' (the sprite's own),
    'wet' (the waterline row)
    or the percent of its fade step; the pixels of the board that a chequer step leaves alone are
    not listed."""
    out = []
    for y in range(y0, y0 + DOCK_ABOVE):
        out += [(x0, y, "above"), (x0 + 1, y, "above")]
    w = y0 + DOCK_ABOVE
    out += [(x0, w, "wet"), (x0 + 1, w, "wet")]
    for i, (percent, chequer) in enumerate(dock_fade(DOCK_DEPTH[(x0, y0)])):
        for x in (x0, x0 + 1):
            if not chequer or (x + w + 1 + i) % 2 == 0:
                out.append((x, w + 1 + i, percent))
    return out


def dock_post_footprints():
    """Every board pixel each pile covers, above and below the water, and the rows it spans:
    a list of (x0, first row, last row, pixels)."""
    return [(x0, y0, y0 + DOCK_ABOVE + DOCK_DEPTH[(x0, y0)], [(x, y) for x, y, _ in dock_post_pixels(x0, y0)])
            for x0, y0 in DOCK_POSTS]


def blend(base, over, percent):
    """The colour of `over` at percent over `base`, rounded per channel."""
    return tuple(int(round(b + (o - b) * percent / 100.0)) for b, o in zip(base, over))


def dock_sink(canvas):
    """Draw the parts of the piles that are in the water straight on the board, opaque: the
    waterline row and each step of its fade, the pile's colour (the lit
    edge's on the left column, the shaded one's on the right) at that step's percent over the
    pixel of the board that is there. Done on the board as composited so far, before the
    sprite goes on, so the board stays fully opaque and the sprite over it draws the same
    pixels. Returns how many pixels it drew."""
    px = canvas.load()
    drawn = 0
    for x0, y0 in DOCK_POSTS:
        for x, y, kind in dock_post_pixels(x0, y0):
            if kind == "above":
                continue
            base = px[x + MARGIN, y + MARGIN][:3]
            key = "l" if x == x0 else "d"
            hx = DOCK_COLOURS[key]
            pile = (int(hx[0:2], 16), int(hx[2:4], 16), int(hx[4:6], 16))
            colour = blend(base, pile, DOCK_WET_PERCENT if kind == "wet" else kind)
            px[x + MARGIN, y + MARGIN] = colour + (255,)
            drawn += 1
    return drawn


def check_dock_posts(canvas, piece, left, top):
    """Every pile stands in water all the way down: the board as composited so far (before the
    dock goes on) must show water at each pixel a pile covers, above the waterline, on it and
    under it. Water here is what the moored ship's test also calls open sea: not
    dry land (red no more than 32 above blue), and the pixel is in no land hex. The murky
    shallows count, they are the sea in front of the beach; the stricter 'blue clearly above red
    and green' is logged beside it because only the open sea off the jetty's head passes that.
    A pile that fails refuses the bake. The piles must also be two pairs, a pile each side of the
    deck at the same rows, with at least DOCK_PAIR_GAP clear rows between one pair and the next."""
    px = canvas.load()
    for x0, y0, y1, pixels in dock_post_footprints():
        bad = []
        blue = 0
        for (x, y) in pixels:
            r, g, b = px[x + MARGIN, y + MARGIN][:3]
            land_hex = False
            for pos, (name, terrain) in NAMES.items():
                ox, oy = origin(*pos)
                if terrain != "water" and 0 <= x - ox < TILE and 0 <= y - oy < TILE and hexmask().getpixel((x - ox, y - oy)) > 0:
                    land_hex = True
            if r > b + 32 or land_hex:
                bad.append((x, y, (r, g, b)))
            if b > r + 20 and b > g:
                blue += 1
        LOG.append("info: dock pile x %d..%d rows %d..%d (waterline row %d, last row under water %d): %d of %d pixels clear blue water, none dry land or in a land hex (water all the way down: %s)"
                   % (x0, x0 + 1, y0, y1, y0 + DOCK_ABOVE, y1, blue, len(pixels), "yes" if not bad else "NO"))
        if bad:
            raise SystemExit("the dock's pile at x %d, rows %d..%d is not in water: %s" % (x0, y0, y1, bad[:4]))
    rows = sorted({y0 for _, y0 in DOCK_POSTS})
    assert len(DOCK_POSTS) == 4 and len(rows) == 2, "four piles in two pairs"
    for y0 in rows:
        xs = sorted(x0 for x0, y in DOCK_POSTS if y == y0)
        assert len(xs) == 2 and xs[0] + xs[1] + 1 == DOCK_DECK_X[0] + DOCK_DECK_X[1], ("a pair is not mirrored about the deck", y0, xs)
    clear = rows[1] - (rows[0] + DOCK_ABOVE + max(d for (_, y), d in DOCK_DEPTH.items() if y == rows[0])) - 1
    LOG.append("info: dock piles: %d clear rows between the seaward pair and the midway pair (at least %d)" % (clear, DOCK_PAIR_GAP))
    if clear < DOCK_PAIR_GAP:
        raise SystemExit("the dock's two pairs of piles crowd each other: %d clear rows" % clear)


# --------------------------------------------------------------------------
# Outputs
# --------------------------------------------------------------------------


def wesnoth_commit():
    try:
        out = subprocess.run(["git", "-C", WESNOTH, "rev-parse", "HEAD"], capture_output=True,
                             text=True, check=True)
        return "checkout commit " + out.stdout.strip()
    except Exception:
        return "installed data directory " + WESNOTH


def write_sources():
    lines = ["# Wesnoth files read by tools/bake_board.py (paths relative to data/core/images/terrain/)",
             "# Read from the installed 1.16 data directory: wesnoth-1.16-data 1:1.16.11-1build3",
             "# which is Wesnoth tag %s, commit %s" % (PACKAGE_TAG, PACKAGE_COMMIT),
             "# Licence and author are Wesnoth's own record for the file: its row in copyrights.csv at",
             "# revision d4b2d151ce3 (the last to list images), checked by the bake against the MD5.",
             "# path | licence | author | MD5"]
    for rel in sorted(_SOURCES):
        lines.append("%s | %s | %s | %s" % ((rel,) + WESNOTH_LICENCES[rel]))
    with open(OUT_SOURCES, "w", encoding="utf-8") as fh:
        fh.write("\n".join(lines) + "\n")


def assert_geometry():
    cx = {name: centre(*HEXES[name]) for name in HEXES}
    c = centre(CX, CY)
    assert c == (BOARD // 2, BOARD // 2) == (252, 252), c
    assert (36 + 54 * CX, 72 * CY + 36) == c, "centre hex must follow x = 36 + 54*col, y = 72*row + 36"
    ne, se = cx["NE"], cx["SE"]
    assert se[0] == ne[0] and se[1] - ne[1] == 72, "NE/SE E vertices must be 72 px apart"
    vil0 = (cx["NW"][0] + 36, cx["NW"][1])                  # E vertex of NW
    assert vil0 == (cx["N"][0] - 18, cx["N"][1] + 36), "NW's E vertex is also the SW vertex of N"
    assert cx["N"] == (c[0], c[1] - 72) and cx["S"] == (c[0], c[1] + 72)
    assert cx["NW"] == (c[0] - 54, c[1] - 36) and cx["SW"] == (c[0] - 54, c[1] + 36)
    assert cx["NE"] == (c[0] + 54, c[1] - 36) and cx["SE"] == (c[0] + 54, c[1] + 36)
    # no land fringe may touch the board edge: the outermost sprite reach is the mountain
    # flower (90 px from a hex centre horizontally, 108 vertically)
    for name, (hx, hy) in cx.items():
        assert 90 <= hx <= BOARD - 90 and 108 <= hy <= BOARD - 108, (name, hx, hy)
    # every vertex the mountain must leave clear belongs to a hex of this map
    for label, (px, py) in MOUNTAIN_CLEAR:
        assert any((px - hx, py - hy) in ((36, 0), (-36, 0), (18, 36), (-18, 36), (18, -36), (-18, -36))
                   for hx, hy in cx.values()), (label, px, py)
    return cx


def sea_fill(board):
    """Average colour of open water: the top and bottom 96 rows hold no land."""
    rows = [board.crop((0, 0, BOARD, 96)), board.crop((0, BOARD - 96, BOARD, BOARD))]
    total = [0, 0, 0]
    count = 0
    for part in rows:
        for r, g, b in part.getdata():
            total[0] += r
            total[1] += g
            total[2] += b
            count += 1
    return tuple(int(round(t / count)) for t in total)


def write_geometry(board, path, image_name):
    cx = assert_geometry()
    lines = [
        "# Hex geometry of %s (504x504). Pixel coordinates, origin top left." % image_name,
        "# Flat topped hexes. Vertices from centre: E (+36,0) NE (+18,-36) NW (-18,-36)",
        "# W (-36,0) SW (-18,+36) SE (+18,+36).",
        "board_size %d %d" % (BOARD, BOARD),
        "hex_size 72 72",
        "hex_width 72",
        "hex_height 72",
        "column_step 54",
        "row_step 72",
        "odd_column_shift 36",
        "vertex_radius 36",
        "sea_fill %d %d %d" % sea_fill(board),
    ]
    for pos in sorted(NAMES, key=lambda p: ["NW", "SW", "S", "SE", "NE", "N", "CENTRE"].index(NAMES[p][0])):
        name, terrain = NAMES[pos]
        c = cx[name]
        o = origin(*pos)
        lines.append("hex %s %s col=%d row=%d centre=%d,%d origin=%d,%d"
                     % (name, terrain, pos[0], pos[1], c[0], c[1], o[0], o[1]))
    if DOCK:
        piece, (left, top) = dock_piece()
        lines.append("# The dock baked into the board: the vertex where the start road meets the sea, the box")
        lines.append("# of its above water art (left, top, width, height; deck and piles), the point a ship's")
        lines.append("# bowsprit comes to, and where the game draws dock.png (the same art, over the roads).")
        lines.append("dock_foot %d %d" % dock_foot())
        lines.append("dock_box %d %d %d %d" % (left, top, piece.width, piece.height))
        lines.append("dock_sprite %d %d" % (left, top))
        lines.append("dock_mooring %d %d" % dock_mooring())
        lines.append("# Every run of the dock's above water art as row:first column-last column (pixels with any alpha).")
        lines.append("dock_spans " + " ".join(dock_spans(piece, left, top)))
    with open(path, "w", encoding="utf-8") as fh:
        fh.write("\n".join(lines) + "\n")


def write_map(path):
    """The island as a plain Wesnoth map file: comma separated terrain codes, one line per
    row, with the one hex border ring the format includes (gamemap default_border = 1)."""
    rows = []
    for fy in range(MAP_H + 2):
        row = []
        for fx in range(MAP_W + 2):
            col = (fx - 1) - MAP_SHIFT_X
            r = (fy - 1) - MAP_SHIFT_Y
            row.append(code_str(terrain_at(col, r)))
        rows.append(", ".join(row))
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8") as fh:
        fh.write("\n".join(rows) + "\n")


# The beach off board draws the earth bank (flat/bank, from 20 to 25 px deep on the field hex,
# strong in the first 10 px) on the land. The thin beach drops it, so the land is only required to
# be untouched deeper than LAND_MARGIN px inside its edge; nearer than that, only the bank differs.
LAND_MARGIN = 22
BEACH_LAND_CHECK = _opt("BEACH_LAND_CHECK", True)


def check_land_untouched(board_on):
    """THIN_BEACH draws on water hexes only (and drops the bank on the land). Bake the same map
    with the beach off and require every pixel of the land union deeper than LAND_MARGIN px
    inside its edge to be identical; the log also counts the differing pixels deeper than 6
    and 14 px, which are the removed bank."""
    global THIN_BEACH
    saved_log = list(LOG)
    ITEMS.clear()
    FLAGS.clear()
    THIN_BEACH = False
    try:
        board_off = bake()
    finally:
        THIN_BEACH = True
        LOG[:] = saved_log
    size = BOARD + 2 * MARGIN
    land = Image.new("L", (size, size), 0)
    for (x, y) in [(x, y) for x in GX for y in GY]:
        if not tmatch(terrain_at(x, y), "W*"):
            ox, oy = origin(x, y)
            land.paste(255, (ox + MARGIN, oy + MARGIN), hexmask())
    depth = grow(ImageChops.invert(land), LAND_MARGIN + 1).crop((MARGIN, MARGIN, MARGIN + BOARD, MARGIN + BOARD))
    diff = ImageChops.difference(board_on, board_off).convert("L").point(lambda v: 255 if v else 0)
    counts = {}
    for limit in (6, 14, LAND_MARGIN):
        inside = depth.point(lambda v, limit=limit: 255 if v > limit else 0)
        counts[limit] = sum(1 for v in ImageChops.darker(diff, inside).getdata() if v)
    LOG.append("info: thin beach: pixels that differ from the beach off board, deeper than 6 px inside the land: %d, "
               "than 14 px: %d, than %d px: %d (must be 0)" % (counts[6], counts[14], LAND_MARGIN, counts[LAND_MARGIN]))
    if counts[LAND_MARGIN]:
        raise SystemExit("thin beach changed %d land pixels deeper than %d px inside the land"
                         % (counts[LAND_MARGIN], LAND_MARGIN))


def bake_map(map_id):
    """Bake one map and write its outputs (or only BAKE_OUT)."""
    select_map(map_id)
    board = bake()
    assert board.size == (BOARD, BOARD)
    if THIN_BEACH and BEACH_LAND_CHECK:
        check_land_untouched(board)
    print("== %s" % map_id)
    for line in LOG:
        print(line)
    if OUT_ALT:
        if os.environ.get("BAKE_MAP_OUT"):
            write_map(os.environ["BAKE_MAP_OUT"])
        board.save(OUT_ALT, format="PNG", optimize=False)
        print("wrote", OUT_ALT)
        return
    os.makedirs(DRAWABLE, exist_ok=True)
    path = out_board(map_id)
    board.save(path, format="PNG", optimize=False)
    board.resize((BOARD * 3, BOARD * 3), Image.NEAREST).save(out_preview(map_id), format="PNG", optimize=False)
    write_geometry(board, out_geometry(map_id), os.path.basename(path))
    write_map(out_map(map_id))
    if DOCK:
        write_dock_sprite(map_id)
    print("wrote", path)


DOCK_SPRITES = {}   # map id -> the bytes of its dock sprite, for the check that every map draws the same one


def write_dock_sprite(map_id):
    """The dock's above water art alone as dock.png, in the piece's own bounding box, RGBA, alpha 0
    or 255. It is authored and does not depend on the board under it, so every map has the same
    one; the bake refuses to go on if two maps ever differ (the app would then need one each)."""
    piece, (left, top) = dock_piece()
    assert {px[3] for px in piece.getdata()} <= {0, 255}
    buf = io.BytesIO()
    piece.save(buf, format="PNG", optimize=False)
    DOCK_SPRITES[map_id] = buf.getvalue()
    if len(set(DOCK_SPRITES.values())) != 1:
        raise SystemExit("the dock sprite differs between maps: %s" % sorted(DOCK_SPRITES))
    path = os.path.join(DRAWABLE, "dock.png")
    with open(path, "wb") as fh:
        fh.write(DOCK_SPRITES[map_id])
    print("wrote", path)


def main():
    if not os.path.isdir(TERRAIN):
        raise SystemExit("Wesnoth terrain images not found at %s (set WESNOTH_DIR)" % TERRAIN)
    args = sys.argv[1:]
    if len(args) > 1 or any(a not in MAPS for a in args):
        raise SystemExit("usage: bake_board.py [%s]" % "|".join(MAPS))
    if args:
        ids = args
    elif OUT_ALT:
        ids = [DEFAULT_MAP]  # BAKE_OUT writes one PNG
    else:
        ids = list(MAPS)
    for map_id in ids:
        bake_map(map_id)
    if not OUT_ALT and set(ids) == set(MAPS):
        write_sources()  # the union of every map's sources, so only a full run writes it


if __name__ == "__main__":
    sys.exit(main())
