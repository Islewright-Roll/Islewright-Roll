// SPDX-License-Identifier: GPL-3.0-or-later
//! Ported from GameStateStoreTest: the saved game names its map, keeps traders as a vector,
//! and refuses anything it cannot restore whole. The tests of the old count-only format are
//! left out with that format.

mod common;

use common::*;
use dayapp::rules::game_state_store::{self, from_json, from_json_with_maps};
use dayapp::rules::json::Json;
use dayapp::rules::{Game, MapDef};

fn maps() -> Vec<&'static MapDef> {
    vec![tarnholm(), race()]
}

/// A race game part way through, with traders built in no tier order at all.
fn race_game() -> Game {
    let mut game = Game::new(race());
    game.rolls = 1;
    game.turns_taken = 4;
    game.playsheet.traders = bits("0010100100");
    game.playsheet.roads[0] = true;
    game.playsheet.roads[5] = true;
    game.playsheet.villages[3] = true;
    game.playsheet.villages[0] = true;
    game.playsheet.cities[2] = true;
    game.playsheet.turn_score = vec![0, 0, 7];
    game
}

fn tree(game: &Game) -> Json {
    Json::parse(&game_state_store::to_json(game)).unwrap()
}

#[test]
fn the_schema_is_two_and_names_the_map() {
    let root = tree(&Game::new(tarnholm()));
    assert_eq!(
        Some(2),
        root.get("schemaVersion").and_then(Json::as_i64_lenient)
    );
    assert_eq!(
        Some("tarnholm"),
        root.get("game").unwrap().get("mapId").unwrap().as_str()
    );
    assert!(
        !root
            .get("game")
            .unwrap()
            .get("playsheet")
            .unwrap()
            .has("knights")
    );
}

#[test]
fn a_save_round_trips_on_a_map_with_unordered_traders() {
    let game = race_game();
    let json = game_state_store::to_json(&game);
    assert_eq!(
        Some("race-fixture"),
        tree(&game)
            .get("game")
            .unwrap()
            .get("mapId")
            .unwrap()
            .as_str()
    );

    let back = from_json_with_maps(&json, &maps()).unwrap();
    assert!(same(race(), back.map));
    // Tiers 3, 5 and 8 and nothing between them: not a count, a vector
    assert_eq!(bits("0010100100"), back.playsheet.traders);
    assert_eq!(3, back.playsheet.trader_count());
    assert_eq!(game.playsheet.roads, back.playsheet.roads);
    assert_eq!(game.playsheet.villages, back.playsheet.villages);
    assert_eq!(game.playsheet.cities, back.playsheet.cities);
    assert_eq!(game.playsheet.turn_score, back.playsheet.turn_score);
    assert_eq!(4, back.turns_taken);
    assert_eq!(game.playsheet.score(), back.playsheet.score());
    assert_eq!(tree(&game).canonical(), tree(&back).canonical());
}

#[test]
fn a_save_round_trips_on_the_default_map() {
    let mut game = Game::new(tarnholm());
    game.playsheet.traders = bits("0110000");
    let back = from_json_with_maps(&game_state_store::to_json(&game), &maps()).unwrap();
    assert!(same(tarnholm(), back.map));
    assert_eq!(bits("0110000"), back.playsheet.traders);
}

#[test]
fn an_unknown_map_id_is_refused_so_the_save_is_cleared() {
    let json = game_state_store::to_json(&race_game());
    // The shipped list does not hold the fixture, so to this build it is a stranger
    let e = from_json(&json).expect_err("an unknown map id must not load");
    assert!(e.0.contains("race-fixture"), "{e}");

    let e = from_json_with_maps(&json.replace("race-fixture", "no-such-map"), &maps())
        .expect_err("an unknown map id must not fall back to the default map");
    assert!(e.0.contains("no-such-map"), "{e}");
}

#[test]
fn a_save_needs_its_map_id_and_traders_of_the_right_length() {
    let json = game_state_store::to_json(&Game::new(tarnholm()));
    assert!(
        from_json_with_maps(&json.replace("\"mapId\":\"tarnholm\",", ""), &maps()).is_err(),
        "a save without a mapId must not load"
    );

    // Seven entries are right for Tarnholm; the race map's ten are not
    let race_traders = tree(&race_game())
        .get("game")
        .unwrap()
        .get("playsheet")
        .unwrap()
        .get("traders")
        .unwrap()
        .clone();
    let mut root = Json::parse(&json).unwrap();
    root.get_mut("game")
        .unwrap()
        .get_mut("playsheet")
        .unwrap()
        .put("traders", race_traders);
    let e = from_json_with_maps(&root.to_compact(), &maps())
        .expect_err("traders of the wrong length must not load");
    assert!(e.0.contains("traders"), "{e}");
}

/// Dunholm once had a cap of twenty turns, so its saves held 21 turn scores. Such a document
/// still loads, and plays on past twenty: a map with no cap takes any length.
#[test]
fn a_dunholm_save_with_twenty_one_turn_scores_still_loads() {
    let mut game = race_game();
    game.playsheet.turn_score = vec![0; 21];
    game.playsheet.turn_score[1] = 2;
    game.playsheet.turn_score[2] = 2;
    game.playsheet.turn_score[3] = 2;
    game.playsheet.turn_score[4] = 7;
    let json = game_state_store::to_json(&game);
    assert!(json.contains("\"turnScore\":[0,2,2,2,7,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]"));

    let mut back = from_json_with_maps(&json, &maps()).unwrap();
    assert_eq!(21, back.playsheet.turn_score.len());
    assert_eq!(5, back.playsheet.turn_score_of(4));
    back.turns_taken = 24;
    back.new_turn(false);
    assert_eq!(25, back.turns_taken);
    assert_eq!(26, back.playsheet.turn_score.len());
    assert_eq!(0, back.playsheet.turn_score_of(30));
}

#[test]
fn a_save_with_the_wrong_number_of_turn_scores_is_still_refused_on_a_capped_map() {
    let json = game_state_store::to_json(&Game::new(tarnholm()))
        .replace("\"turnScore\":[0,", "\"turnScore\":[0,0,");
    let e = from_json_with_maps(&json, &maps())
        .expect_err("Tarnholm has fifteen turns, so sixteen entries exactly");
    assert!(e.0.contains("turnScore"), "{e}");
}

/// A race saves the target it is played to, which the setting may have moved.
#[test]
fn a_race_saves_its_own_target_and_a_save_without_one_races_to_the_islands_own() {
    let mut game = race_game();
    game.target_score = 15;
    let json = game_state_store::to_json(&game);
    let root = Json::parse(&json).unwrap();
    assert_eq!(
        Some(15),
        root.get("game")
            .unwrap()
            .get("targetScore")
            .and_then(Json::as_i64_lenient)
    );
    assert_eq!(
        15,
        from_json_with_maps(&json, &maps()).unwrap().target_score
    );

    // Without the field the race is to the island's 10
    let mut old = root.clone();
    old.get_mut("game").unwrap().remove("targetScore");
    assert_eq!(
        10,
        from_json_with_maps(&old.to_compact(), &maps())
            .unwrap()
            .target_score
    );

    // A hand edit past what the island can give is pulled back into reach
    let mut stray = root.clone();
    stray.get_mut("game").unwrap().put("targetScore", 50);
    assert_eq!(
        19,
        from_json_with_maps(&stray.to_compact(), &maps())
            .unwrap()
            .target_score
    );
}

#[test]
fn a_points_map_writes_no_target() {
    let root = tree(&Game::new(tarnholm()));
    assert!(!root.get("game").unwrap().has("targetScore"));
    assert_eq!(
        0,
        from_json_with_maps(&root.to_compact(), &maps())
            .unwrap()
            .target_score
    );
}

#[test]
fn a_future_schema_is_refused() {
    let json = game_state_store::to_json(&Game::new(tarnholm()))
        .replace("\"schemaVersion\":2", "\"schemaVersion\":3");
    let e = from_json_with_maps(&json, &maps()).expect_err("schema 3 is not understood");
    assert!(e.0.contains("schemaVersion"), "{e}");
}

/// The old count-only format (schema 1) is no longer read.
#[test]
fn the_old_schema_is_refused() {
    let json = game_state_store::to_json(&Game::new(tarnholm()))
        .replace("\"schemaVersion\":2", "\"schemaVersion\":1");
    assert!(from_json(&json).is_err());
}
