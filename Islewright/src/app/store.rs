// SPDX-License-Identifier: GPL-3.0-or-later
//! What persists between launches, in Day's preferences: each island's game in progress under
//! its own key, and the top scores in one document. The formats are the rules' own
//! (`rules::game_state_store`, `rules::high_scores`); this module only reads and writes them.

use crate::rules::game::Game;
use crate::rules::game_log::{GameLog, PlayedGame};
use crate::rules::game_state_store::{self, Persist};
use crate::rules::high_scores::HighScoresAndSettings;
use crate::rules::map_def::MapDef;

/// The key the top scores live under.
const SCORES_KEY: &str = "islewright.scores.json";
/// The key the played games live under.
const GAMES_KEY: &str = "islewright.games.json";

/// The preferences key for an island's game in progress.
fn game_key(map: &MapDef) -> String {
    format!("islewright.{}", game_state_store::key_for(map))
}

/// The island's game in progress, or None to start a fresh one. A stored game that cannot be
/// used (another island's, a finished one, an unreadable one) is removed so it does not fail
/// again on the next launch.
pub fn load_game(map: &'static MapDef) -> Option<Game> {
    let key = game_key(map);
    let stored = day::prefs::get(&key);
    let loaded = game_state_store::load(stored.as_deref(), map);
    if let Some(error) = &loaded.error {
        eprintln!("islewright: set aside the saved {} game: {error}", map.id);
    }
    if loaded.clear_stored {
        day::prefs::remove(&key);
    }
    loaded.game
}

/// Saves the game if it is still going and removes its save once it is over, so a finished
/// game never comes back.
pub fn save_game(game: &Game) {
    let key = game_key(game.map);
    match game_state_store::persist(game) {
        Persist::Save(json) => {
            day::prefs::set(&key, &json);
        }
        Persist::Clear => {
            day::prefs::remove(&key);
        }
    }
}

/// The top scores. A document that does not parse is kept aside under another key rather
/// than overwritten, so a bug never silently loses a player's scores.
pub fn load_scores() -> HighScoresAndSettings {
    match day::prefs::get(SCORES_KEY) {
        None => HighScoresAndSettings::new(),
        Some(json) => match HighScoresAndSettings::from_json(&json) {
            Ok(scores) => scores,
            Err(e) => {
                eprintln!("islewright: unreadable top scores, kept aside: {e:?}");
                day::prefs::set(&format!("{SCORES_KEY}.unreadable"), &json);
                HighScoresAndSettings::new()
            }
        },
    }
}

pub fn save_scores(scores: &HighScoresAndSettings) {
    day::prefs::set(SCORES_KEY, &scores.to_json());
}

/// Files a result on the island's list and returns its rank (1 is a new best), or 0 when it
/// did not make the list.
pub fn record(map_id: &str, result: i32) -> i32 {
    let mut scores = load_scores();
    let rank = match scores.add_score(map_id, result, crate::clock::now_ms() as i64) {
        Ok(rank) => rank as i32,
        Err(e) => {
            eprintln!("islewright: score not recorded: {e}");
            0
        }
    };
    save_scores(&scores);
    rank
}

/// The played games, oldest first. A document that does not read is kept aside, as the top
/// scores' is.
pub fn load_games() -> GameLog {
    match day::prefs::get(GAMES_KEY) {
        None => GameLog::default(),
        Some(json) => GameLog::from_json(&json).unwrap_or_else(|e| {
            eprintln!("islewright: unreadable played games, kept aside: {e}");
            day::prefs::set(&format!("{GAMES_KEY}.unreadable"), &json);
            GameLog::default()
        }),
    }
}

/// Adds a game that has just ended to the played games, dated now.
pub fn log_game(mut game: PlayedGame) {
    game.date = crate::clock::now_ms() as i64;
    let mut log = load_games();
    log.add(game);
    day::prefs::set(GAMES_KEY, &log.to_json());
}
