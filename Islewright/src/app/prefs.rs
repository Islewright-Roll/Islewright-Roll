// SPDX-License-Identifier: GPL-3.0-or-later
//! The player's settings, each its own persisted `Pref`, so Day's Settings surface can bind a
//! row straight to it. The Android app kept these inside the top scores document; here that
//! document holds only the scores (see `store`), and a setting lives under its own key.

use day_piece_settings::prelude::Pref;

use crate::rules::high_scores::{EARLY_PROMPT_SMART, MUSIC_VOLUME_DEFAULT, RACE_TARGET_DEFAULT};
use crate::rules::player_color;

/// Sound effects on or off.
pub static SOUND_EFFECTS: Pref<bool> = Pref::new("islewright.sound_effects", || true);
/// Haptic feedback on or off.
pub static HAPTICS: Pref<bool> = Pref::new("islewright.haptics", || true);
/// The sea moves, as Wesnoth's does.
pub static ANIMATE_WATER: Pref<bool> = Pref::new("islewright.animate_water", || true);
/// When to warn before a build that forfeits unused rolls: off, smart or conservative
/// (`high_scores::EARLY_PROMPT_*`).
pub static EARLY_BUILD_PROMPT: Pref<i64> = Pref::new("islewright.early_build_prompt", || {
    EARLY_PROMPT_SMART as i64
});
/// Warn before ending a turn while a build is still affordable.
pub static END_TURN_PROMPT: Pref<bool> = Pref::new("islewright.end_turn_prompt", || true);
/// Trader resource classic mode: banked resources are dragged onto dice, not spent by
/// themselves.
pub static TRADER_CLASSIC: Pref<bool> = Pref::new("islewright.trader_classic", || false);
/// The points that win a race.
pub static RACE_TARGET: Pref<i64> =
    Pref::new("islewright.race_target", || RACE_TARGET_DEFAULT as i64);
/// Show the in-game tutorial.
pub static TUTORIAL: Pref<bool> = Pref::new("islewright.tutorial", || true);
/// The tutorial steps already seen, as a bit set (`tutorial::*`).
pub static TUTORIAL_SEEN: Pref<i64> = Pref::new("islewright.tutorial_seen", || 0);
/// Show the welcome when the app starts.
pub static SHOW_WELCOME: Pref<bool> = Pref::new("islewright.show_welcome", || true);
/// The player's color, as 0xAARRGGBB.
pub static PLAYER_COLOR: Pref<i64> =
    Pref::new("islewright.player_color", || player_color::DEFAULT as i64);
/// The Theme setting's stored name (`Theme::name`).
pub static THEME: Pref<String> = Pref::new("islewright.theme", || "system".to_string());
/// How loud the music plays, 0 to 100.
pub static MUSIC_VOLUME: Pref<i64> =
    Pref::new("islewright.music_volume", || MUSIC_VOLUME_DEFAULT as i64);
/// The island on screen: "tarnholm" or "dunholm".
pub static ISLAND: Pref<String> = Pref::new("islewright.island", || "tarnholm".to_string());
