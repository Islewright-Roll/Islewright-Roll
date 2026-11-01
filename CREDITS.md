# Credits

Islewright: Roll is free software under the GNU GPL version 3 or later (see `COPYING`).

## Code

* Ridgeline Apps (FatMonkeySmile): the original Resource Dice Game, 2012 to 2013.
  <https://github.com/FatMonkeySmile/android-dice-game>
* Marcus Hoffmann (Bubu): versions 1.13 to 1.15 in 2017, which moved the project to Gradle, kept it
  running on newer Android and returned it to F-Droid. Version 1.15 is where this repository's code
  begins. <https://git.bubu1.eu/Bubu/android-dice-game>
* tmaex: the German translation, whose menu and button strings are still in use.
* Hellmark: preserved the Google Code archive and the F-Droid 1.15 release.
  <https://github.com/Hellmark/android-dice-game>
* The Android Open Source Project: parts of `About.java` and `Rules.java` come from its Photostream
  sample, under the Apache License 2.0.
* Islewright Team: everything since version 1.15.

# Art and sound

## Terrain and the King's ship: The Battle for Wesnoth

The island (mountain, dry hills, summer deciduous forest on leaf litter, pasture, farmland,
desert and sea, and the transitions that blend them) is made from 111 terrain images from
[The Battle for Wesnoth](https://www.wesnoth.org) 1.16.11, and the King's ship is one
Wesnoth unit sprite. Source: <https://github.com/wesnoth/wesnoth>, tag `1.16.11`, commit
`83d4b397f1de466c721e4e393b361c71fc4f5b6c`. The files were read from the Debian package
`wesnoth-1.16-data` 1:1.16.11-1build3, which is that tag. The exact files, with the licence,
author and MD5 of each, are listed in `tools/wesnoth-sources.txt` (terrain) and
`tools/wesnoth-sources-ui.txt` (ship), and the scripts refuse to run on a file that is not
in those tables or whose MD5 differs.

Licence. Wesnoth licenses each art file under either the GNU GPL version 2 or later or
Creative Commons BY-SA 4.0, and since 30 July 2017 takes new art under CC BY-SA 4.0
(<https://wiki.wesnoth.org/Wesnoth:Copyrights>). Wesnoth records the licence of each file:
in its `copyrights.csv` until August 2024, and since then in the `Copyright` tag of the
image itself. All 112 files used here (the 111 terrain images and the ship) are recorded as
GNU GPL v2+. None is CC BY-SA 4.0, and the newest was added in December 2016. They are used
under the GNU GPL version 3, which is the licence of this app (see `COPYING`).

Authors, as recorded by Wesnoth for these files: J.W. Bjerk (Eleazar), Lari Nieminen
(zookeeper), Richard Kettering (Jetrel), Hogne Håskjold (freim) and Philippe Plantier
(Ayin). For 17 of the terrain files Wesnoth records the author as
unknown. The wider list of Wesnoth artists is under "Artwork and Graphics" in
`data/core/about.cfg`.

The dock is authored pixel art, drawn in `tools/bake_board.py` (the deck and four piles that
go down into the water, their submerged part tinted from the board's own water) and baked into
the board as terrain, in six fixed colours. Its above water art is also written as
`dock.png`, which the game draws over the roads. It uses no Wesnoth file.

The King's ship on the board (`piece_kingship.png`) is
`data/core/images/units/transport/transport-galleon.png` from the same 1.16.11 data. Wesnoth
records its author as the FreeCol project and its licence as GNU GPL v2+. Wesnoth has since
replaced this sprite (current Wesnoth has a different sprite under the same name, licensed
CC BY-SA 4.0); the one used here is the 1.16.11 file. Its team colour flag is recoloured to
royal purple and gold.

Changes. The terrain tiles are composited at native resolution with the blending rules of
Wesnoth's terrain graphics, ported to `tools/bake_board.py` (differences from a live Wesnoth
map are in `tools/README.md`). The baked board images (`board_*.png`) and `piece_kingship.png`
are therefore adaptations of the Wesnoth files, and are distributed under the GNU GPL
version 3 like the rest of the app. The scripts that produce them are in `tools/`.

## Fonts

* Press Start 2P, by CodeMan38. SIL Open Font License 1.1
  (`tools/fonts/PressStart2P-OFL.txt`).
* Ark Pixel Font (10 px proportional), by TakWolf. SIL Open Font License 1.1
  (`tools/fonts/ArkPixel-OFL.txt`).
* Micro 5, by The Soft Type Project Authors. SIL Open Font License 1.1
  (`tools/fonts/Micro5-OFL.txt`). Numerals only: the small numbers on the turn strip's flags.

Download locations and checksums are in `tools/fonts/SOURCES.txt`.

## Sound effects

Processed by `tools/process_sfx.sh` from the sources in `tools/sfx-sources.txt` (trimmed,
faded, level matched, mono 44.1 kHz).

From The Battle for Wesnoth 1.16.11 (https://github.com/wesnoth/wesnoth), GNU GPL v2 or
later, used here under GPL v3, authors as recorded in its `copyrights.csv`:

- `slider.wav` (die), `gold.ogg`, `horn-signals/horn-1.ogg`, `join.wav`: Lari Nieminen
- `fanfare-short.wav` (game over): J.W. Bjerk
- `club.ogg` (castle): Leonardo Magno Sampaio

CC0 (public domain):

- Kenney (https://kenney.nl): Casino Audio `dice-shake-3` and `dice-throw-3`; Impact Sounds
  `footstep_concrete_001`, `impactWood_medium_002` (village, played twice),
  `impactMetal_light_000` and `impactBell_heavy_001`; RPG Audio `handleCoins` and `bookFlip1`; Interface Sounds `back_002` (empty turn)
- "Classic fanfare lick" (new best score) by MedicineStorm
  (https://opengameart.org/content/classic-fanfare-lick)

CC BY-SA 4.0 (<https://creativecommons.org/licenses/by-sa/4.0/>), also from The Battle for
Wesnoth 1.16.11:

- `sounds/button.wav` (click): Argentum Age, as recorded in Wesnoth's `copyrights.csv`.
  Changed here by trimming, mixing to mono and a change of level; the result stays under
  CC BY-SA 4.0.

## Everything else

The buttons, panels, dice, roads, villages, castles, traders, resource icons and the launcher icon
are drawn by `tools/make_ui.py` and are part of this project, under the GNU GPL version 3.
