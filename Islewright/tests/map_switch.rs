// SPDX-License-Identifier: GPL-3.0-or-later
//! Ported from MapSwitchTest: which island the app opens on, and when starting another game
//! would throw something away.

mod common;

use common::*;
use dayapp::rules::{Game, HighScoresAndSettings, MapDef};

#[test]
fn the_setting_names_the_island_the_app_opens_on() {
    // Each island has its own save, so a save cannot say which one opens
    assert!(same(dunholm(), MapDef::for_launch(Some(&dunholm().id))));
    assert!(same(tarnholm(), MapDef::for_launch(Some(&tarnholm().id))));
}

#[test]
fn a_setting_nobody_knows_or_none_at_all_is_the_first_island() {
    assert!(same(tarnholm(), MapDef::for_launch(None)));
    assert!(same(tarnholm(), MapDef::for_launch(Some("atlantis"))));
    assert!(same(
        tarnholm(),
        MapDef::for_launch(Some(&HighScoresAndSettings::new().map))
    ));
}

#[test]
fn an_untouched_game_has_no_progress_on_either_map() {
    for map in MapDef::all() {
        assert!(!Game::new(map).has_progress(), "{}", map.id);
    }
}

#[test]
fn a_roll_a_turn_or_anything_built_is_progress() {
    for map in MapDef::all() {
        let mut rolled = Game::new(map);
        rolled.rolls = 1;
        assert!(rolled.has_progress(), "{} rolled", map.id);

        let mut later = Game::new(map);
        later.turns_taken = 1;
        assert!(later.has_progress(), "{} a turn in", map.id);

        let mut road = Game::new(map);
        road.playsheet.roads[1] = true;
        assert!(road.has_progress(), "{} road", map.id);

        let mut village = Game::new(map);
        village.playsheet.villages[0] = true;
        assert!(village.has_progress(), "{} village", map.id);

        let mut castle = Game::new(map);
        castle.playsheet.cities[0] = true;
        assert!(castle.has_progress(), "{} castle", map.id);

        let mut trader = Game::new(map);
        trader.playsheet.traders[1] = true;
        assert!(trader.has_progress(), "{} trader", map.id);
    }
}

#[test]
fn the_start_road_alone_is_not_progress() {
    for map in MapDef::all() {
        let game = Game::new(map);
        assert!(game.playsheet.roads[0]);
        assert!(!game.has_progress(), "{}", map.id);
    }
}

#[test]
fn a_finished_game_has_nothing_left_to_lose() {
    let mut out_of_turns = Game::new(tarnholm());
    out_of_turns.turns_taken = tarnholm().turn_cap;
    assert!(out_of_turns.is_game_done());
    assert!(!out_of_turns.has_progress());

    let mut won = Game::new(dunholm());
    won.rolls = 1;
    for i in 0..won.playsheet.cities.len() {
        won.playsheet.cities[i] = true;
    }
    won.playsheet.villages[0] = true;
    won.playsheet.villages[1] = true;
    assert!(won.is_game_done());
    assert!(!won.has_progress());
}

#[test]
fn starting_a_new_game_on_another_map_leaves_no_progress_behind() {
    let mut game = Game::new(tarnholm());
    game.rolls = 2;
    game.playsheet.roads[1] = true;
    game.start_new_game(dunholm());
    assert!(same(dunholm(), game.map));
    assert!(!game.has_progress());
}
