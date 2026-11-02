# Islewright: Roll

> **Very early work in progress.** This port has not run on a device and has not been published
> anywhere, so wording here and in this folder that describes it as published is a draft of the
> plan. The game that ships today is the Java app at the repository root. The plan and current
> state are in the [App Fair epic](https://github.com/islewright-roll/islewright/issues/8). To
> help, branch from `feature/day-port` and open your pull request against it.

A solitaire dice game: roll, then build roads, villages, castles and traders across a small
island. It runs on iPhone, Android and the web from one Rust codebase, built with
[Day](https://daybrite.dev), and is being prepared for publication through the [App Fair](https://appfair.org).

It is free, with no ads, no in-app purchases, no tracking and no account, and it works offline.
See [PRIVACY.md](PRIVACY.md).

## Playing

Each turn you get three rolls. Tap dice to keep them, roll the rest, then build whatever your
resources pay for. Roads open up sites for villages and castles; two gold stand in for any one
resource; traders bank a resource for a later turn.

- **Tarnholm**: fifteen turns around a lake. Score as much as you can before the King's ship
  reaches the pier.
- **Dunholm**: a race to ten points in as few turns as you can.

Turn a phone on its side and the same controls turn with it, with the island and the icons staying
upright. A tablet on its side moves the score and costs to one side and the dice to the other. The
dice are always on the side that was the bottom of the screen.

With a keyboard: 1 to 6 keep or release a die, 0 rolls, 9 builds the suggested spot, 7 undoes,
and 8 dismisses a tutorial tip. Where the platform passes letters to the game, R or Space also
rolls, B builds, U undoes, Enter dismisses a tip, and M or Escape opens the menu.

## Building

Install Rust with rustup, then the Day CLI, and check the toolchains:

```sh
cargo install --locked day-cli
day doctor
```

Then build and run a target:

```sh
day launch -t android-material   # an emulator or a device over adb
day launch -t web-dom            # serves the web build and opens a browser
day launch -t ios-uikit          # macOS with Xcode 27 or newer
```

`cargo test` runs the rules tests with Day's `mock` backend, and `day lint` checks the
project, its locales and the store listing. The App Fair builds the published identity with
`day --flavor appfair` (see `Day-appfair.toml`).

## Layout

- `src/rules/`: the game rules, scoring and saved state, independent of any UI.
- `src/`: the screens, the board canvas, sound and settings.
- `resource/assets/`: the board art, pieces, bitmap fonts, sound effects and rules text.
- `resource/locales/`: the UI strings, in English, French, German, Spanish, Indonesian, Italian
  and Brazilian Portuguese.
- `store/storefront.toml`: the App Store and Google Play listing.
- `dayscript/`: the walkthrough that tests the app and captures the store screenshots.
- `tools/`: the scripts that generate the art, fonts and sound effects (see `tools/README.md`), and
  `dependency_licenses.py`, which writes `DEPENDENCIES.md`.

## Credits and license

Islewright is free software under the GNU General Public License, version 3 or later; see
[COPYING](COPYING). It began as Resource Dice Game by Ridgeline Apps (2012). Marcus Hoffmann (Bubu)
revived it in 2017, with a German translation by tmaex, and Hellmark preserved the Google Code
archive. Everything since is by Islewright Team.

The island terrain, the King's ship and some sound effects come from The Battle for Wesnoth
under the GNU GPL; the fonts are under the SIL Open Font License; other sounds are CC0 or
CC BY-SA 4.0. [CREDITS.md](CREDITS.md) lists every source, author and license.

Islewright is not affiliated with or endorsed by Catan GmbH. CATAN is a registered trademark of
Catan GmbH.
