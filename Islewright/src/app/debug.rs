// SPDX-License-Identifier: GPL-3.0-or-later
//! Debug builds only: puts the app into a known state at launch from environment variables, so
//! `tools/parity` can reach the same screens as the Android app's debug scenario harness.
//!
//! `ISLEWRIGHT_SCENARIO` holds a scenario document (`rules::scenario`), `ISLEWRIGHT_TUTORIAL=0`
//! and `ISLEWRIGHT_WELCOME=0` turn those off for this launch and after, `ISLEWRIGHT_EARLY_BUILD`
//! and `ISLEWRIGHT_TRADER_CLASSIC` set those settings. A debug build's menu lists Debug Scenario
//! too, as the Android app's does, which plays the scenario again.

use crate::rules::game::Game;
use crate::rules::scenario::Scenario;

use super::prefs::{EARLY_BUILD_PROMPT, ISLAND, SHOW_WELCOME, TRADER_CLASSIC, TUTORIAL};

/// Applies what the environment asks for to the settings, before the game opens.
pub fn apply_settings() {
    if std::env::var("ISLEWRIGHT_TUTORIAL").as_deref() == Ok("0") {
        TUTORIAL.set(false);
    }
    if std::env::var("ISLEWRIGHT_WELCOME").as_deref() == Ok("0") {
        SHOW_WELCOME.set(false);
    }
    if let Some(when) = std::env::var("ISLEWRIGHT_EARLY_BUILD")
        .ok()
        .and_then(|v| v.parse().ok())
    {
        EARLY_BUILD_PROMPT.set(when);
    }
    if let Ok(classic) = std::env::var("ISLEWRIGHT_TRADER_CLASSIC") {
        TRADER_CLASSIC.set(classic == "1");
    }
    if let Some(s) = scenario() {
        ISLAND.set(s.map.id.clone());
    }
}

/// Applies the scenario, if there is one, over a fresh game on its island.
pub fn apply_scenario(game: &mut Game) {
    if let Some(s) = scenario() {
        game.start_new_game(s.map);
        s.apply_to(game);
    }
}

fn scenario() -> Option<Scenario> {
    let json = std::env::var("ISLEWRIGHT_SCENARIO").ok()?;
    match Scenario::from_json(&json) {
        Ok(s) => Some(s),
        Err(e) => {
            eprintln!("islewright: scenario refused: {e}");
            None
        }
    }
}
