// SPDX-License-Identifier: GPL-3.0-or-later
//! Ported from GameUndoTest: one undo takes back the latest build tap, whole, and nothing
//! that moved the game on past it.

mod common;

use common::*;
use dayapp::rules::game_state_store;
use dayapp::rules::{DiceValue, Game, Kind};

use DiceValue::{Brick as B, Grain, Lumber as L, Ore, Wool};

fn json(game: &Game) -> String {
    game_state_store::to_json(game)
}

#[test]
fn a_fresh_game_has_nothing_to_undo() {
    let mut game = Game::new(tarnholm());
    assert!(!game.can_undo());
    let before = json(&game);
    game.undo();
    assert_eq!(before, json(&game));
}

#[test]
fn undo_takes_back_a_road_and_the_dice_it_spent() {
    let mut game = Game::new(tarnholm());
    give_dice(&mut game, &[B, L, Wool, Grain]);
    game.dice[2].held = true;
    let before = json(&game);

    game.build(Kind::Road, 1);
    assert!(game.playsheet.roads[1]);
    assert!(game.built_resource_this_turn);
    assert!(game.can_undo());

    assert!(game.undo());

    assert_eq!(before, json(&game));
    assert!(!game.playsheet.roads[1]);
    assert!(!game.can_undo());
    assert!(game.dice[2].held);
    assert!(game.dice[0].is_usable());
}

/// A build on roll one forfeits the other rolls; undoing it gives them back.
#[test]
fn undo_after_an_early_build_gives_the_forfeited_rolls_back() {
    let mut game = Game::new(tarnholm());
    give_dice(&mut game, &[B, L]);
    assert_eq!(1, game.rolls);
    assert!(game.would_forfeit_rolls());
    assert!(game.can_roll());

    game.build(Kind::Road, 1);
    assert!(!game.can_roll(), "building ends the roll phase");

    game.undo();
    assert_eq!(1, game.rolls);
    assert!(game.can_roll(), "the player can roll again");
    assert!(game.would_forfeit_rolls());
}

#[test]
fn a_chain_build_is_taken_back_whole_by_one_undo() {
    let mut game = Game::new(dunholm());
    give_dice(&mut game, &[B, L, B, L, B, L]);
    let before = json(&game);

    game.build(Kind::Road, 4);
    assert!(
        game.playsheet.roads[1] && game.playsheet.roads[3] && game.playsheet.roads[4],
        "roads 1, 3 and 4"
    );

    game.undo();

    assert_eq!(before, json(&game));
    assert!(!(game.playsheet.roads[1] || game.playsheet.roads[3] || game.playsheet.roads[4]));
    assert!(!game.can_undo());
}

#[test]
fn a_build_that_changes_nothing_keeps_the_earlier_snapshot() {
    let mut game = Game::new(dunholm());
    give_dice(&mut game, &[B, L]);
    let before = json(&game);
    game.build(Kind::Road, 1);
    // Nothing is left to pay for road 5, and no chain reaches it
    game.build(Kind::Road, 5);
    assert!(!game.playsheet.roads[5]);
    assert!(game.can_undo());
    game.undo();
    assert_eq!(before, json(&game));
}

#[test]
fn two_builds_then_one_undo_reverts_only_the_second() {
    let mut game = Game::new(dunholm());
    give_dice(&mut game, &[B, L, B, L]);
    game.build(Kind::Road, 1);
    let after_first = json(&game);
    game.build(Kind::Road, 3);
    assert!(game.playsheet.roads[1] && game.playsheet.roads[3]);

    game.undo();

    assert_eq!(after_first, json(&game));
    assert!(game.playsheet.roads[1]);
    assert!(!game.playsheet.roads[3]);
    assert!(!game.can_undo(), "one level only");
}

#[test]
fn ending_the_turn_clears_the_undo() {
    let mut game = Game::new(tarnholm());
    give_dice(&mut game, &[B, L]);
    game.build(Kind::Road, 1);
    assert!(game.can_undo());
    game.new_turn(false);
    assert!(!game.can_undo());
    let after = json(&game);
    game.undo();
    assert_eq!(after, json(&game));
    assert!(game.playsheet.roads[1]);
}

/// A roll replaces the dice the snapshot holds; undoing across one would be a free re-roll.
#[test]
fn a_roll_clears_the_undo() {
    let mut game = Game::new(tarnholm());
    give_dice(&mut game, &[B, L]);
    game.build(Kind::Road, 1);
    assert!(game.can_undo());
    game.roll();
    assert!(!game.can_undo());

    let mut other = Game::new(tarnholm());
    give_dice(&mut other, &[B, L]);
    other.build(Kind::Road, 1);
    other.end_turn_and_roll();
    assert!(!other.can_undo());
}

#[test]
fn a_new_game_clears_the_undo() {
    let mut game = Game::new(tarnholm());
    give_dice(&mut game, &[B, L]);
    game.build(Kind::Road, 1);
    game.reset();
    assert!(!game.can_undo());

    give_dice(&mut game, &[B, L]);
    game.build(Kind::Road, 1);
    assert!(game.can_undo());
    game.start_new_game(dunholm());
    assert!(!game.can_undo());
}

#[test]
fn a_build_that_wins_a_race_cannot_be_undone() {
    let mut game = Game::new(dunholm());
    score_eight(&mut game);
    empty_turns(&mut game, 6);
    give_dice(&mut game, &[Grain, Grain, Ore, Ore, Ore]);

    game.build(Kind::City, 1);

    assert!(game.is_game_done());
    assert!(game.scored);
    assert!(!game.can_undo());
    let done = json(&game);
    game.undo();
    assert_eq!(done, json(&game), "the recorded result stands");
    assert!(game.playsheet.cities[1]);
    assert_eq!(7, game.result_to_record());
}

#[test]
fn a_chain_that_wins_a_race_cannot_be_undone_either() {
    let mut game = Game::new(dunholm());
    for i in 0..6 {
        game.playsheet.villages[i] = true;
    }
    game.playsheet.cities[0] = true;
    game.playsheet.roads[1] = true;
    give_dice(&mut game, &[B, L, B, L, B, L]);
    game.build(Kind::Road, 6);
    assert!(game.is_game_done());
    assert!(!game.can_undo());
}

#[test]
fn a_build_that_does_not_win_a_race_can_be_undone() {
    let mut game = Game::new(dunholm());
    score_eight(&mut game);
    give_dice(&mut game, &[B, L]);
    game.build(Kind::Road, 1);
    assert!(!game.is_game_done());
    assert!(game.can_undo());
}
