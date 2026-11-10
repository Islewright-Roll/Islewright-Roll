# tools

## bake_board.py

Bakes the island boards (504x504) out of terrain tiles from Battle for Wesnoth, blending
neighbouring terrains with Wesnoth's own transition images and layering rules. There are two
maps, Tarnholm and Dunholm, defined in the `MAPS` table at the top of the script (the terrain
code and name of each of the seven hexes, the vertices the mountain sprite must leave clear, and
whether the dry mountain blend is on). Run it from the repository root:

```
python3 -I tools/bake_board.py [tarnholm|dunholm]
```

With no argument both maps are baked.

It reads `data/core/images/terrain/` from `$WESNOTH_DIR`, which defaults to the installed
package, `/usr/share/games/wesnoth/1.16` (the art the reference render uses; the sister
checkout `../wesnoth` has pixel identical art for every file read, but not always the same bytes, so the MD5 check can refuse it). It never
modifies it.

Outputs, for each map ID:

* `resource/assets/art/board_<ID>.png`: the board.
* `tools/preview_board_<ID>.png`: the board at 3x with nearest neighbour scaling, for review. Not
  tracked (see `.gitignore`).
* `tools/board_geometry_<ID>.txt`: the board size, hex size and column step, the centre and origin of
  each of the seven hexes in board pixels, and `sea_fill R G B` (average colour of the open
  water, for painting beyond the board), one item per line, for the renderer.
* `tools/reference/<ID>.map`: the same island as a real Wesnoth map (see below).

Written once, by a run with no argument: `tools/wesnoth-sources.txt`, every Wesnoth file read for
either map, with its licence, author and MD5.

The bake is deterministic and pure nearest neighbour: tiles are composited at native pixels and
never resampled. The script holds a table of every Wesnoth file it may read (`WESNOTH_LICENCES`:
licence, author and MD5, from Wesnoth's own record) and fails before drawing if a file it loads is
not in the table or its bytes do not match the MD5. The MD5s are those of the 1.16.11 files, so
`WESNOTH_DIR` pointing at a newer checkout can fail on files Wesnoth has since recompressed or
replaced; that is the guard working.

### Terrain on the islands

| Hex | Tarnholm | Dunholm | Wesnoth terrain |
|---|---|---|---|
| NW | mountain | mountain | `Mm` (hills base plus `mountains/basic-castle-ne.png`, see Options) |
| SW | wheat | wheat | `Re^Gvs` (tilled dirt with the farmland overlay) |
| S | pasture | pasture | `Gg` |
| SE | forest | forest | `Gll^Fds` (summer deciduous forest on leaf litter) |
| NE | hills | hills | `Hhd` (dry hills) |
| N | desert | desert | `Dd` |
| centre | lake | desert | `Wwr` (coastal reef) on Tarnholm, `Dd` on Dunholm |
| the 12 sea hexes beside the island | shallow sea | shallow sea | `Ww` (shallow water) |
| everywhere else | open sea | open sea | `Wo` (deep ocean) |

Both islands are centred: the centre hex is at col 4, row 3, so its centre is (252, 252), the
middle of the board, and all land fringes sit well inside the edges.

Dunholm has no lake. Its two desert hexes merge into one sandy expanse with no seam (the desert
transition excludes desert receivers), the centre desert has no coast so it has no waves, and the
north desert gets waves on its three sea edges. Three rules matter only there: the desert fringe
onto dirt (`Dd` onto `R*`, which includes the farm field, `terrain-graphics.cfg:961`), and
`mountains/blend-from-dry` plus `hills/dry` where the mountain meets desert or dirt
(`terrain-graphics.cfg:901-904`). There the forest's leaf litter also fringes onto the centre
desert (`grass/leaf-litter`, `:947`), where on Tarnholm it meets the lake. The mountain sprite is checked against every vertex a village
or castle sits on (`mountain_clear` in `MAPS`): on both maps `mountains/basic-castle-ne.png` leaves them
clear (and so do `basic.png` and `basic3.png`: none covers a village or castle vertex).

The water is Wesnoth's own, no tropical water: deep ocean `Wo` as the open sea, ordinary shallow
water `Ww` in every sea hex touching the island, and on Tarnholm the reef `Wwr` (water with
`water/reef` rocks on top, `terrain-graphics.cfg:493`) in the centre. Wesnoth's water to water
transitions blend the three (`terrain-graphics.cfg:1015-1016`: the deep ocean's water at half
opacity onto the shallow hexes and the shallow water onto the deep ones, one image per vertex with
the long concave and convex masks). `sea_fill` in each geometry file is the average colour of the
open sea. The older looks are still available: `BAKE_SHALLOW_RING=0` (no ring), `BAKE_SEA`,
`BAKE_SHALLOW` and `BAKE_LAKE` take other codes.

Wheat is `Re^Gvs`, which is how Wesnoth draws farmland: tilled dirt with the crop row overlay.

### How it blends

The script is a port of the rules in `data/core/terrain-graphics.cfg` and
`terrain-graphics/new-macros.cfg` (read from the 1.16 package; the macros are identical in the
sister checkout): base images, `NEW:TRANSITION` runs of neighbouring hexes with the one
transition per edge flags, the corner (vertex) transitions that give the coast its halo
(`water/bottom` with the concave and convex masks), the pebble shore rim (`flat/bank-to-ice`),
beach waves around the desert with their `no_draw` tiles, and hills and mountains reaching into
the sea. Images are cut with the hex mask, masks use Wesnoth's smaller-alpha rule, and
everything is drawn in Wesnoth's own order (`display.cpp` `generate_hex_key`). Tile variants are
chosen with Wesnoth's own position noise (`builder.cpp` `get_noise` and the rule hash), so the
reference map gets the same grass, hills, dirt, desert, mountain and farm variants.

### Options

Every departure from Wesnoth, and every knob, is a named constant at the top of
`bake_board.py`, and any of them can be set from the environment as `BAKE_<NAME>` (for example
`BAKE_FOREST_IN_HEX=0`). With `BAKE_OUT=some.png` only that PNG is written, so a comparison
never touches the shipped board.

| Constant | Default | Meaning |
|---|---|---|
| `MOUNTAIN_DRY_BLEND` | on, per map in `MAPS` | The dry mountain blend above (`BAKE_MOUNTAIN_DRY_BLEND=1` or `0` overrides the per map setting). It is Wesnoth's rule; on both maps it cuts the residual against Wesnoth (Tarnholm from 310 to 86 pixels). |
| `FOREST_IN_HEX` | on | Owner's request: trees stay inside the forest hex. The first small variant of the forest's sprite (`forest/deciduous-summer-small.png`, the only one) is drawn where Wesnoth draws it, and only the piece of it inside the forest's own hex is kept: the sprite is cut by the hex mask and the pieces Wesnoth would draw on the neighbours are left out. Nothing is moved, shrunk or resampled. 3386 of the hex's 3888 pixels carry tree or tree shadow (87.1%; the pines used before covered 2267, 58.3%), and 313 pixels are cut off, mostly shadow on the west side and the foot of the southmost tree (no whole pixel shift cuts fewer than 290). Off: Wesnoth's own variant, overhang and all. |
| `SEA_CODE`, `SHALLOW_CODE`, `LAKE_CODE`, `SHALLOW_RING` | `Wo`, `Ww`, `Wwr`, on | Owner's request: deep sea, a ring of ordinary shallow water, a reef lake. Set `BAKE_SHALLOW_RING=0` and `BAKE_LAKE=Ww`, `BAKE_SEA=Ww` for the old look. |
| `MOUNTAIN_SPRITE` | `mountains/basic-castle-ne.png` | Owner's request. Wesnoth draws this variant for a mountain with a castle to its north east; it has no pixel across the hex's NE edge, where the game draws its start road and the desert begins. `mountains/basic.png` is the alternative (it reaches about 11 px past that edge). `wesnoth` (or empty): Wesnoth's own variant choice, which the reference comparison needs. |
| `MOUNTAIN_NE_DRY` | on | Owner's request: repaint the mountain's green on the sides facing desert (NE; on Dunholm SE as well). Every green pixel (hue `MOUNTAIN_DRY_HUE_MIN`..`MAX` = 55..170 degrees, saturation at least `MOUNTAIN_DRY_SAT_MIN` = 18%) of the sprite and of the hills base under it takes Wesnoth's own dry colour at the same place (`mountains/dry-castle-ne.png`, `hills/dry*.png`), with a strength that is 1 at the shared edge and 0 at the hex centre line (times `MOUNTAIN_DRY_REACH` = 140%, capped), thresholded through an 8x8 Bayer matrix: pixel art, no blending, no resampling, alpha untouched. Rock and snow are not green and stay. Used together with Wesnoth's own dry blend (`mountains/blend-from-dry`, `hills/dry`), which looks better with it than without. |
| `DOCK` | on | Owner's request: the dock is terrain, not a piece. It is authored in the script (no Wesnoth file), in hard alpha and six fixed colours (`DOCK_COLOURS`): a deck 8 px wide (board columns 212 to 219, rows 136 to 179) of cross planks in two alternating tones with a seam between them and a dark edge down both sides and across both ends at one width to its landward end, which lies over the start road's round end; and four 2 px piles (`DOCK_POSTS`), two pairs mirrored about the deck, a seaward pair (top row 134) and a midway pair (top row 154, the farthest landward row at which all four piles stand in water on both boards, see below). Each pile has `DOCK_ABOVE` = 7 rows out of the water, then a waterline row (the pile 90% over the water), then its rows under the water, a different number for each pile (`DOCK_DEPTH`: 8 seaward west, 7 seaward east, 5 midway west, 4 midway east, deeper to seaward and to the west), falling evenly from 75 to 30 percent of the pile's colour over the board's own water pixel (`dock_fade`), the last two fifths of them on a chequer of the pixels where x + y is even, so it goes down into the water and breaks up. Those pixels under the water are computed at bake time against the board as composited (`dock_sink`), and are in the board only, so the board stays fully opaque. The dock is composited after everything else, running straight up the screen from the vertex where the start road meets the sea (216, 180). It is baked into the board (everything that samples the board sees it) and its above water art alone is also written as `resource/assets/art/dock.png` (RGBA, the art's own 12 x 46 box, alpha 0 or 255 only, the same pixels as the board has there, one file for both boards; the bake refuses to go on if two maps ever differ), which the game draws over both road passes and under the Crown's ship, so the deck lies over the road and not under it. The bake fails if any pixel of the dock's above water art lies over a mountain sprite or inside a land hex that is not desert, if the board under any pixel of a pile, above or below the water, is dry land or in a land hex (`check_dock_posts`; the murky shallows count as water, the clear blue test is only logged), or if the pairs are not mirrored or leave fewer than `DOCK_PAIR_GAP` = 4 clear rows between them (4 now). The midway row: the desert's beach and hex edge close in on the east side further south (the desert's hex reaches the pile's columns from about row 165), so with piles 13 rows long and the seaward pair fixed the lowest midway top row at which both the west and the east pile stand in water on both boards is 154. The geometry files gain `dock_foot`, `dock_box`, `dock_sprite` (the top left of `dock.png` on the board, mirrored as `voyage::DOCK_SPRITE`), `dock_mooring` (where the Crown's ship puts its bowsprit) and `dock_spans` (each run of the above water art's columns per row), which the app's tests read. Not a Wesnoth rule: `BAKE_DOCK=0` for the reference comparison. `DOCK_COLOURS`, `DOCK_DECK_X`, `DOCK_DECK_Y`, `DOCK_PLANK`, `DOCK_POSTS`, `DOCK_ABOVE`, `DOCK_DEPTH` and `DOCK_MOORING_FROM_FOOT` are beside it in the script. |
| `THIN_BEACH` | on, per map in `MAPS` | A thin sandy shore all round the coast, see below. `BAKE_THIN_BEACH=1` or `0` overrides it for every map. |
| `FOREST_FULL_SPRITE` | off | On: the large forest sprite always (Wesnoth uses the small one beside water). |
| `FIELD_GRASS_VERGE` | off | On: a dry grass verge on the field's sea side. Not a Wesnoth rule. |
| `BOTTOM_OPACITY` | 100 | `water/bottom` opacity in percent. Wesnoth: 100. |
| `WAVES_THREE_WAY_CONCAVE` | on | The two concave sand/water/other beach wave rules. Wesnoth: on. |
| `LOOP_FRAME` | 1 | The step of the water's loop, 1 to 21: the deep ocean's frame, with the shallow water's and the beach waves' frames for that moment (`bake_sea.py` bakes every step). Step 1 is frame 1 of all three, the still board, as the 1.16 screenshot shows. |
| `WESNOTH_VARIANT_NOISE` | on | Wesnoth's own variant choice. Off: `random.Random(1)`. |
| `DRAW_ORDER` | `wesnoth` | `layer` sorts by layer only. |

### Comparing against Wesnoth

`tools/reference/<ID>.map` is a real Wesnoth map of each island (the plain terrain grid the
editor saves, with its one hex border ring). `tools/reference/render.sh` renders it with
`wesnoth --screenshot`, cuts the screenshot to the 504x504 board, and compares it with
`board_<ID>.png`:

```
tools/reference/render.sh [tarnholm|dunholm] [--data-dir DIR] [--bake PNG] [--tolerance N]
```

`WESNOTH_BIN` names the binary (default `wesnoth`). Wesnoth runs with a throwaway user data
directory. The script writes `tools/reference/<ID>_wesnoth_render.png`, `<ID>_side_by_side.png`
(bake, Wesnoth, difference, each at 2x) and `<ID>_diff_stats.txt`. `--data-dir` is passed through
to Wesnoth, but the 1.16 binary cannot run the sister checkout's newer data (it fails with
"Unknown widget type scroll_text"), so the package's own data is the only usable reference.

Last result, pixels whose largest channel differs by more than 16. The faithful column is
`BAKE_FOREST_IN_HEX=0 BAKE_MOUNTAIN_SPRITE=wesnoth BAKE_MOUNTAIN_NE_DRY=0 BAKE_THIN_BEACH=0 BAKE_DOCK=0`
(everything Wesnoth does, none of the departures; the water, the reef, the shallow ring and their
transitions are all in it). The shipped column is the board as committed:

| Map | faithful | shipped board |
|---|---|---|
| Tarnholm | 86 of 254016, 0.034% | 17095, 6.73% |
| Dunholm | 88 of 254016, 0.035% | 14484, 5.70% |

The faithful residual is the same 86 and 88 pixels as with the regular hills and the pine forest,
all in the field hex; the dry hills, the leaf litter and the deciduous forest add none (no pixel
of that side of the board is off by more than 7). The shipped difference is the departures: the
thin beach round the whole coast (about 10300 pixels outside the seven hexes), the mountain
sprite and its repaint, the dock, and the forest's overhang that is left out. The stats and side by side
files in `tools/reference/` are the shipped boards. The maps carry the shipped codes (`Wo`, `Ww`,
`Wwr`).

### The thin beach (departure, on by default)

`THIN_BEACH` (per map in `MAPS`, or `BAKE_THIN_BEACH=1` for every map) makes every coast meet the
sea the way the desert hex does, a narrow sandy shore with Wesnoth's soft foam and no earth bank,
without adding a sand hex: the island keeps its seven hexes and the ragged edges of its grass,
hills, mountain and field. The committed boards have it on. With it on (`thin_beach_pass`), all of
it drawn on water hexes only:

* Sand: on each water hex beside non desert land, `sand/desert-<run>.png` (`BEACH_STEM`), the
  image of the run of directions the land lies in, longest run first as `NEW:TRANSITION`, layer
  -510, drawn `BEACH_SAND_PASSES` (2) times with the faint tail of its alpha ramp (at or below
  `BEACH_SAND_FLOOR` = 40) cut off. Beside real desert Wesnoth's own `desert` rule already draws the
  same images the same way, so the shore continues into the desert's without a seam.
* Land onto sand: the land rules run with the coastal water hexes standing in for sand and draw
  on those hexes only: `hills/dry` (the dry hills and the mountain), `grass/green-medium`
  (the grass onto sand), `grass/leaf-litter` (the forest floor onto sand), `embellishments/farm-veg-spring` (the field's rows) and `flat/dirt` (the
  field's own ragged edge; Wesnoth has no dirt onto sand image, so the dirt rule is let loose on
  the sand, `terrain-graphics.cfg:1379`).
* Dropped: `flat/bank`, `flat/bank-to-ice`, `hills/regular-to-water`, `hills/dry-to-water`, the abrupt grass onto water.
* Kept as the beach off board: `water/bottom`, the water transitions, the shallow ring. Wesnoth's
  beach foam (`water/waves`, convex and concave) is drawn with every coast counted as sand, on
  water hexes (and the desert hex as before).

Every thin beach bake also bakes the map with the beach off and checks that the land is untouched
deeper than 22 px inside its edge (`LAND_MARGIN`; the earth bank of the field is 20 to 25 px deep,
strong in the first 10, which is why the check is not at 6 px). The log lists how many pixels still
differ deeper than 6 and 14 px: those are the removed bank. `BAKE_BEACH_LAND_CHECK=0` skips it.


## bake_sea.py

Bakes the moving sea: runs `bake_board.py` once for each of the 21 steps of the water's loop
(`BAKE_LOOP_FRAME`: Wesnoth's deep ocean frame for frame, its shallow water and beach waves each
at the frame their own loops would be on) and packs each island's 21 boards into
`resource/assets/art/sea_<ID>.png`, row by row so PNG's Up filter compresses each step against
the one before. Step 1 must equal the still board. It also adds every frame it read to
`wesnoth-sources.txt`. `--frames DIR` packs boards already baked there.

```
python3 -I tools/bake_sea.py [tarnholm|dunholm] [--frames DIR]
```
## make_ui.py

Draws every piece of interface art that is not Wesnoth terrain: the 9-slice button in its
three states, the panel frame and a dark-wood panel, dice, roads, villages, castles, traders, resource icons, legend
glyphs and the launcher icon, all in limited palettes with no antialiasing. It reads no Wesnoth
art.

```
python3 -I tools/make_ui.py
```

It writes `tools/pieces.txt` (sprite sizes and anchors, for the renderer) and an untracked
contact sheet, `tools/preview_ui.png`.

## bake_font.py

Bakes the pixel fonts in `tools/fonts/` into white glyph atlases and metrics files (Press Start 2P
for titles and buttons, Ark Pixel 10 px for body text, and a third atlas of only 0 to 9 and "+" from
Micro 5 for the numbers on the turn strip's flags), and copies the display and body fonts into the
app's assets. It runs offline. The metrics format is documented at the
top of the script.

```
python3 -I tools/bake_font.py
```

## encode_music.sh

Builds the soundtrack in `resource/assets/music/` (`music_<slug>.<ext>`) from the artist's
original downloads. It does not download anything and the originals are never committed:
put them in one directory and pass it in. See `tools/music-sources.txt` for where they come from.

```
tools/encode_music.sh /path/to/originals
```

A lossless original (FLAC or WAV) is encoded to the `FORMAT` set at the top of the script: Opus
at `OPUS_BITRATE` (192k, VBR, in an Ogg container, the chosen format), or Ogg Vorbis at
`VORBIS_QUALITY`, or FLAC level 8. Title and artist tags are kept and cover art is dropped. A lossy
original (MP3 and the like) is copied byte for byte, never re-encoded. To change the format, edit
the variables and run it again; the script removes the older `music_<slug>.*` first. The app has
no music player yet, and the format has to be one every platform's player reads (issue #4). It
needs `ffmpeg` with libopus and libvorbis and prints each file's duration, size and the
original's sha256.

## process_sfx.sh

Builds the sixteen sound effects in `resource/assets/sfx/` (`sfx_<moment>.wav`, 16-bit mono 44.1 kHz, the format Day plays everywhere) from the
original files. It does not download anything. Which source plays which moment, the per-moment
loudness offsets, and where every original came from (URL, author, licence, sha256) are in
`tools/sfx-sources.txt`; a swap is a one-line change there. Wesnoth's sounds are read from the
installed package, the rest from a directory of originals you pass in:

```
tools/process_sfx.sh /path/to/originals [/path/to/audition-dir]
```

With the second argument it also writes `NN_moment_chosen.wav` and `NN_moment_alt.wav`, an
`index.html` with a player for each, and `report.txt` (loudness before and after). Shaping,
loudness matching and the output formats are described at the top of `tools/process_sfx.py`.
Needs ffmpeg with libvorbis, ffprobe and python3 with numpy. The output is byte-identical between runs.
