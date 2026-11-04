// SPDX-License-Identifier: GPL-3.0-or-later
//! The played games: a game is recorded once, at its end, and the record draws the same end
//! again (the same pieces, turns, score and running scores), whatever the settings are now.

mod common;

use common::*;
use dayapp::rules::game_log::{GameLog, KEPT, PlayedGame};
use dayapp::rules::{DiceValue, Game};

use DiceValue::{Grain, Ore};

/// Dunholm won on turn 7 with a castle, as game_race plays it.
fn won_race() -> Game {
    let mut game = Game::new(dunholm());
    score_eight(&mut game);
    empty_turns(&mut game, 6);
    give_dice(&mut game, &[Grain, Grain, Ore, Ore, Ore]);
    game.build_city(1);
    assert!(game.is_game_done());
    game
}

/// Tarnholm played out to its turn cap with a few pieces on it.
fn capped_out() -> Game {
    let mut game = Game::new(tarnholm());
    game.playsheet.roads[0] = true;
    game.playsheet.roads[1] = true;
    game.playsheet.villages[0] = true;
    let cap = game.map.turn_cap as usize;
    empty_turns(&mut game, cap);
    assert!(game.is_game_done());
    game
}

fn same_end(a: &Game, b: &Game) {
    assert_eq!(a.map.id, b.map.id);
    assert_eq!(a.turns_taken, b.turns_taken);
    assert_eq!(a.playsheet.score(), b.playsheet.score());
    assert_eq!(a.playsheet.roads, b.playsheet.roads);
    assert_eq!(a.playsheet.villages, b.playsheet.villages);
    assert_eq!(a.playsheet.cities, b.playsheet.cities);
    assert_eq!(a.playsheet.traders, b.playsheet.traders);
    assert_eq!(a.playsheet.resources_avail, b.playsheet.resources_avail);
    for t in 1..=a.turns_taken {
        assert_eq!(
            a.playsheet.turn_score_of(t),
            b.playsheet.turn_score_of(t),
            "turn {t}"
        );
    }
    assert!(b.is_game_done());
}

#[test]
fn a_won_race_is_recorded_once_and_draws_the_same_end() {
    let mut game = won_race();
    let played = game.take_finished().expect("recorded at the end");
    assert!(game.take_finished().is_none(), "and only once");
    assert_eq!((7, 10), (played.turns, played.score));
    let back = played.restore().unwrap();
    same_end(&game, &back);
    assert!(!back.can_roll(), "nothing is left to play");
}

#[test]
fn a_restored_race_ends_at_the_score_it_reached_whatever_the_target_is_now() {
    let mut game = won_race();
    game.target_score = 12;
    let played = PlayedGame::of(&game, 0);
    let back = played.restore().unwrap();
    assert_eq!(10, back.target_score);
    assert!(back.is_game_done());
}

#[test]
fn a_game_that_runs_out_of_turns_is_recorded() {
    let mut game = capped_out();
    let played = game.take_finished().expect("recorded at the end");
    assert_eq!(game.map.turn_cap, played.turns);
    same_end(&game, &played.restore().unwrap());
}

#[test]
fn a_game_in_progress_or_restarted_records_nothing() {
    let mut game = Game::new(tarnholm());
    empty_turns(&mut game, 3);
    assert!(game.take_finished().is_none());
    let mut game = won_race();
    game.start_new_game(dunholm());
    assert!(game.take_finished().is_none(), "a restart drops it");
}

#[test]
fn the_log_reads_back_what_it_wrote() {
    let mut log = GameLog::default();
    log.add(won_race().take_finished().unwrap());
    log.add(capped_out().take_finished().unwrap());
    let back = GameLog::from_json(&log.to_json()).unwrap();
    assert_eq!(log, back);
}

#[test]
fn a_record_that_does_not_fit_its_island_is_left_out() {
    let mut log = GameLog::default();
    log.add(won_race().take_finished().unwrap());
    let good = log.to_json();
    let bad = good.replacen("\"cities\":\"", "\"cities\":\"1", 1);
    let doc = format!("[{},{}]", &bad[1..bad.len() - 1], &good[1..good.len() - 1]);
    assert_eq!(1, GameLog::from_json(&doc).unwrap().games.len());
    let unknown = good.replace("\"dunholm\"", "\"atlantis\"");
    assert!(GameLog::from_json(&unknown).unwrap().games.is_empty());
    assert!(GameLog::from_json("{}").is_err(), "not an array at all");
}

#[test]
fn the_log_keeps_the_newest() {
    let played: PlayedGame = won_race().take_finished().unwrap();
    let mut log = GameLog::default();
    for n in 0..KEPT + 5 {
        let mut p = played.clone();
        p.date = n as i64;
        log.add(p);
    }
    assert_eq!(KEPT, log.games.len());
    assert_eq!(5, log.games[0].date);
    assert_eq!((KEPT + 4) as i64, log.games.last().unwrap().date);
}
