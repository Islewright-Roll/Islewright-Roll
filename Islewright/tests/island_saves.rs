// SPDX-License-Identifier: GPL-3.0-or-later
//! Ported from IslandSavesTest: one saved game per island, each under its own key. A map of
//! keys to strings stands in for the UI's preferences, as a temporary directory stood in for
//! the app's files. The tests of the old one-file-for-both-islands migration are left out
//! with it.

mod common;

use std::collections::HashMap;

use common::*;
use dayapp::rules::game_state_store::{self, Persist};
use dayapp::rules::json::Json;
use dayapp::rules::{DiceValue, Game, MapDef};

#[derive(Default)]
struct Prefs {
    values: HashMap<String, String>,
}

impl Prefs {
    fn persist(&mut self, game: &Game) {
        let key = game_state_store::key_for(game.map);
        match game_state_store::persist(game) {
            Persist::Save(json) => {
                self.values.insert(key, json);
            }
            Persist::Clear => {
                self.values.remove(&key);
            }
        }
    }

    fn load(&mut self, map: &'static MapDef) -> Option<Game> {
        let key = game_state_store::key_for(map);
        let loaded = game_state_store::load(self.values.get(&key).map(String::as_str), map);
        if loaded.clear_stored {
            self.values.remove(&key);
        }
        loaded.game
    }

    /// What switching islands does with the store: keep the game on screen, bring up the other.
    fn switch_to(&mut self, current: &Game, target: &'static MapDef) -> Game {
        self.persist(current);
        self.load(target).unwrap_or_else(|| Game::new(target))
    }
}

fn tarnholm_game() -> Game {
    let mut game = Game::new(tarnholm());
    game.turns_taken = 3;
    game.rolls = 2;
    game.built_resource_this_turn = true;
    game.dice[1].value = DiceValue::Ore;
    game.dice[1].held = true;
    game.dice[4].value = DiceValue::Wool;
    game.dice[4].used = true;
    game.playsheet.roads[1] = true;
    game.playsheet.traders[1] = true;
    game.playsheet.resources_avail[2] = false;
    game.playsheet.turn_score[1] = 2;
    game.playsheet.turn_score[2] = 5;
    game.playsheet.turn_score[3] = 6;
    game
}

fn dunholm_game() -> Game {
    let mut game = Game::new(dunholm());
    game.turns_taken = 2;
    game.rolls = 1;
    game.dice[0].value = DiceValue::Gold;
    game.dice[0].held = true;
    game.dice[5].value = DiceValue::Brick;
    game.playsheet.roads[2] = true;
    game.playsheet.villages[0] = true;
    game.playsheet.resources_avail[1] = false;
    game.playsheet.turn_score = vec![0, 1, 3];
    game
}

fn same_doc(game: &Game) -> String {
    Json::parse(&game_state_store::to_json(game))
        .unwrap()
        .canonical()
}

#[test]
fn each_island_has_its_own_key() {
    assert_eq!("game-state.json", game_state_store::key_for(tarnholm()));
    assert_eq!(
        game_state_store::DEFAULT_KEY,
        game_state_store::key_for(MapDef::default_map())
    );
    assert_eq!(
        "game-state-dunholm.json",
        game_state_store::key_for(dunholm())
    );
}

#[test]
fn saving_writes_the_games_own_key_only() {
    let mut prefs = Prefs::default();
    prefs.persist(&dunholm_game());
    assert!(prefs.values.contains_key("game-state-dunholm.json"));
    assert!(!prefs.values.contains_key("game-state.json"));
    prefs.persist(&tarnholm_game());
    assert!(prefs.values.contains_key("game-state.json"));
    assert_eq!(2, prefs.values.len());
}

#[test]
fn switching_and_switching_back_restores_each_game_exactly() {
    let mut prefs = Prefs::default();
    let tarn = tarnholm_game();
    let tarn_before = same_doc(&tarn);
    let dun_before = same_doc(&dunholm_game());

    // Tarnholm mid game, over to Dunholm which has no save: a fresh game there
    let on_dunholm = prefs.switch_to(&tarn, dunholm());
    assert!(same(dunholm(), on_dunholm.map));
    assert!(!on_dunholm.has_progress());

    // Two turns of play on Dunholm, replaced here by a known mid game
    let played = dunholm_game();
    let back_on_tarnholm = prefs.switch_to(&played, tarnholm());
    assert!(same(tarnholm(), back_on_tarnholm.map));
    assert_eq!(tarn_before, same_doc(&back_on_tarnholm));
    assert!(back_on_tarnholm.dice[1].held);
    assert_eq!(DiceValue::Ore, back_on_tarnholm.dice[1].value);
    assert_eq!(2, back_on_tarnholm.rolls);
    assert_eq!(3, back_on_tarnholm.turns_taken);
    assert!(!back_on_tarnholm.playsheet.resources_avail[2]);
    assert_eq!(6, back_on_tarnholm.playsheet.turn_score[3]);

    let back_on_dunholm = prefs.switch_to(&back_on_tarnholm, dunholm());
    assert!(same(dunholm(), back_on_dunholm.map));
    assert_eq!(dun_before, same_doc(&back_on_dunholm));
    assert_eq!(2, back_on_dunholm.turns_taken);
    assert!(back_on_dunholm.dice[0].held);
    assert_eq!(3, back_on_dunholm.playsheet.turn_score[2]);

    // And Tarnholm is still exactly as it was left
    assert_eq!(
        tarn_before,
        same_doc(&prefs.switch_to(&back_on_dunholm, tarnholm()))
    );
}

#[test]
fn a_finished_game_clears_only_its_own_islands_save() {
    let mut prefs = Prefs::default();
    prefs.persist(&tarnholm_game());
    prefs.persist(&dunholm_game());

    let mut finished = tarnholm_game();
    finished.turns_taken = tarnholm().turn_cap;
    assert!(finished.is_game_done());
    prefs.persist(&finished);

    assert!(!prefs.values.contains_key("game-state.json"));
    assert!(prefs.values.contains_key("game-state-dunholm.json"));
    assert!(prefs.load(tarnholm()).is_none());
    assert!(prefs.load(dunholm()).is_some());
}

#[test]
fn an_islands_key_holding_another_islands_game_is_cleared_not_loaded() {
    let mut prefs = Prefs::default();
    prefs.values.insert(
        "game-state-dunholm.json".into(),
        game_state_store::to_json(&tarnholm_game()),
    );
    assert!(prefs.load(dunholm()).is_none());
    assert!(!prefs.values.contains_key("game-state-dunholm.json"));
}

#[test]
fn a_document_that_does_not_parse_is_cleared() {
    let loaded = game_state_store::load(Some("{\"schemaVersion\":2,"), dunholm());
    assert!(loaded.game.is_none());
    assert!(loaded.clear_stored);
    assert!(loaded.error.is_some());
    let nothing = game_state_store::load(None, dunholm());
    assert!(nothing.game.is_none() && !nothing.clear_stored);
}

#[test]
fn a_finished_save_is_not_resurrected() {
    let mut prefs = Prefs::default();
    // Dunholm has no turn cap, so a finished game is one that reached its target
    let mut finished = dunholm_game();
    for i in 0..4 {
        finished.playsheet.cities[i] = true;
    }
    finished.playsheet.villages[1] = true;
    assert!(finished.is_game_done());
    prefs.values.insert(
        "game-state-dunholm.json".into(),
        game_state_store::to_json(&finished),
    );
    assert!(prefs.load(dunholm()).is_none());
    assert!(!prefs.values.contains_key("game-state-dunholm.json"));
}
