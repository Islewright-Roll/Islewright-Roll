# Working on Islewright

Islewright is a solitaire dice game built with [Day](https://daybrite.dev), which renders native
widgets from one Rust codebase, for iOS, Android and the web. It is being prepared for publication through the
[App Fair](https://appfair.org).

## Day documentation

Start with [For AI Agents](https://daybrite.dev/docs/reference/for-agents), then the
[documentation index](https://daybrite.dev/llms.txt). The Day crates' own sources (under
`~/.cargo/registry/src/*/day-*/`) are the most exact reference for the API this project's Day
version has.

The game needs a few Day changes that are not in a Day release yet. They are on the `islewright`
branch of [Islewright-Roll/day](https://github.com/Islewright-Roll/day/tree/islewright), a fork of
Day: build against a checkout of it with `day patch --local <checkout>`, and send any further Day
change there first.

## Project map

- `src/rules/`: the game itself, with no UI: the islands, dice, building, scoring, the
  tutorial's logic, saved games and top scores. Everything here is unit tested.
- `src/view/`: the game screen. It draws the whole screen into an art-resolution framebuffer
  (`src/art.rs`) and shows it at an integer scale, one image pixel per device pixel, so the
  pixel art stays sharp. Portrait and landscape layouts. A phone is locked to portrait (the
  Android manifest and the iOS Info.plist); a tablet on its side or a desktop window shows the
  upright island in open sea beside one wood panel: the score and menu, a labelled costs key at
  16 px and the turns track winding back and forth in three rows from the top; the bank, the
  dice and the roll button from the bottom. The panel sits on the side that was the bottom of
  the screen.
- `src/app/`: the app around the screen: settings (`prefs.rs`, one Day `Pref` each), saved games
  and scores (`store.rs`), sound (`audio.rs`), the accelerometer (`orientation.rs`), the menu,
  and the How to play, Top Scores, Settings and About pages.
- `src/ui/`: the pixel-art widget kit every dialog and page is built from. Each widget draws the
  game's art and fonts and is its own accessible element (role, label, focus, keyboard), so
  screen readers and large text keep working.
- `src/clock.rs`: wall-clock milliseconds on every target (the web has no `SystemTime`).
- `resource/assets/`: the board art, pieces, bitmap fonts, sound effects (WAV) and the rules
  text per language. Art is compiled in with `include_bytes!`, because the web build has no
  synchronous asset reads.
- `resource/locales/<locale>/`: `game.ftl` (the game's strings) and `app.ftl` (the shell's), in
  English, French, German, Spanish, Indonesian, Italian and Brazilian Portuguese.
- `store/storefront.toml`: the store listing. `website/`: the project site.
- `tools/`: the scripts that generate the art, fonts and sounds from their sources; see
  `tools/README.md` and `CREDITS.md`. `tools/scenarios/` holds game states for the tests and for a
  debug build to open in.
- `Day-appfair.toml`: the App Fair identity (`org.appfair.app.IslewrightRoll`); `Day.toml`'s own id is
  for development builds.

## Conventions

- Every Rust file starts with `// SPDX-License-Identifier: GPL-3.0-or-later` (the App Fair lint
  checks it).
- Add user-facing strings to every locale, and reference them through `res::str::…`.
- New art goes through `tools/` and must have a license and author recorded in
  `CREDITS.md`; the App Fair reviews every asset.
- No network use, analytics, ads or permissions beyond what a feature needs, and keep
  `PRIVACY.md` accurate.
- Comments explain why, in plain sentences.

## Validation

Run `cargo fmt --all -- --check`, `cargo test`, `cargo clippy --all-targets -- -D warnings` and
`day lint`. Build the targets you changed (`day build -t web-dom`, `day build -t
android-material`; iOS builds on macOS, or in the release workflow on a tag), and look at the screens: the web build in a
browser shows the same frames the phones do.
