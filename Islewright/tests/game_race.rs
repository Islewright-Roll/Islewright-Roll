// SPDX-License-Identifier: GPL-3.0-or-later
//! Ported from GameRaceTest: a race ends the moment its target is reached, mid turn, and the
//! turns taken are its result; a points island still ends on its turn cap.

mod common;

use common::*;
use dayapp::rules::{BuildStatus, DiceValue, Game, Kind};

use DiceValue::{Brick, Grain, Lumber, Ore, Wool};

fn usable_dice(game: &Game) -> usize {
    game.dice
        .iter()
        .filter(|d| d.is_usable() && d.value != DiceValue::None)
        .count()
}

#[test]
fn reaching_ten_on_turn_seven_ends_the_game_with_seven() {
    let mut game = Game::new(dunholm());
    score_eight(&mut game);
    empty_turns(&mut game, 6);
    assert_eq!(6, game.turns_taken);
    assert!(!game.is_game_done());

    give_dice(&mut game, &[Grain, Grain, Ore, Ore, Ore]);
    game.build_city(1);

    assert_eq!(10, game.playsheet.score());
    assert!(game.is_game_done());
    assert_eq!(7, game.turns_taken);
    assert!(game.scored);
    assert_eq!(7, game.result_to_record());
    assert_eq!(
        2,
        game.playsheet.turn_score_of(7),
        "turn 7 is scored as a turn"
    );
    assert!(!game.can_roll());
}

#[test]
fn nine_is_not_enough() {
    let mut game = Game::new(dunholm());
    score_eight(&mut game);
    game.playsheet.villages[6] = true;
    assert_eq!(9, game.playsheet.score());
    assert!(!game.is_game_done());
    assert!(!game.scored);
    assert_eq!(
        0,
        game.result_to_record(),
        "no result until the target is met"
    );
}

#[test]
fn a_finished_game_does_not_move_on_and_is_not_recorded_twice() {
    let mut game = Game::new(dunholm());
    let clock = std::rc::Rc::new(std::cell::Cell::new(5000u64));
    let c = clock.clone();
    game.clock = Some(Box::new(move || c.get()));
    score_eight(&mut game);
    empty_turns(&mut game, 6);
    give_dice(&mut game, &[Grain, Grain, Ore, Ore, Ore]);
    game.build_city(1);
    let stamp = game.game_over_at;
    assert_eq!(5000, stamp);
    clock.set(9000);
    game.new_turn(false);
    game.new_turn(false);
    assert_eq!(7, game.turns_taken);
    assert_eq!(7, game.result_to_record());
    assert_eq!(stamp, game.game_over_at);
    // Nothing more can be built on the finished board, though dice remain
    give_dice(&mut game, &[Brick, Lumber]);
    assert!(!game.can_build_any_road());
    assert!(!game.can_build_road(2));
    assert_eq!(BuildStatus::None, game.build_status(Kind::Road));
    game.build_road(1);
    assert!(!game.playsheet.roads[1]);
    assert!(
        game.build_plan().chain_for(Kind::Road, 1).is_none(),
        "the plan is empty on a finished game"
    );
}

/// The target is reached on the first step of a two step chain; the second is not built.
#[test]
fn a_chain_stops_cleanly_after_the_step_that_wins() {
    let mut game = Game::new(dunholm());
    for i in 0..6 {
        game.playsheet.villages[i] = true;
    }
    game.playsheet.cities[0] = true;
    game.playsheet.roads[1] = true;
    game.playsheet.roads[3] = true;
    game.playsheet.roads[4] = true;
    assert_eq!(8, game.playsheet.score());
    // Road 6 (brick, lumber) is the long road and wins; village 6 (lumber, brick, wool, grain)
    // would follow it and would score an eleventh point
    give_dice(&mut game, &[Brick, Lumber, Lumber, Brick, Wool, Grain]);
    assert!(!game.can_build_village(6));
    let chain = game
        .build_plan()
        .chain_for(Kind::Village, 6)
        .cloned()
        .unwrap();
    assert_eq!(2, chain.steps.len(), "road 6 then village 6");
    assert_eq!(Kind::Road, chain.steps[0].kind);
    assert_eq!(6, chain.steps[0].index);

    game.build(Kind::Village, 6);

    assert!(game.playsheet.roads[6]);
    assert_eq!(10, game.playsheet.score());
    assert!(
        !game.playsheet.villages[6],
        "nothing is built after the game is won"
    );
    assert_eq!(
        4,
        usable_dice(&game),
        "the second step's dice are untouched"
    );
    assert!(game.is_game_done());
    assert_eq!(1, game.turns_taken);
    assert_eq!(1, game.result_to_record());
    assert!(game.scored);
}

/// A three step chain that wins on its last step runs all three.
#[test]
fn a_chain_that_wins_on_its_last_step_runs_whole() {
    let mut game = Game::new(dunholm());
    for i in 0..6 {
        game.playsheet.villages[i] = true;
    }
    game.playsheet.cities[0] = true;
    game.playsheet.roads[1] = true;
    give_dice(&mut game, &[Brick, Lumber, Brick, Lumber, Brick, Lumber]);
    game.build(Kind::Road, 6);
    assert!(game.playsheet.roads[3] && game.playsheet.roads[4] && game.playsheet.roads[6]);
    assert!(game.is_game_done());
    assert_eq!(1, game.turns_taken);
}

#[test]
fn the_cap_ends_the_race_with_nothing_to_record() {
    let mut game = Game::new(capped());
    score_eight(&mut game);
    empty_turns(&mut game, 19);
    assert!(!game.is_game_done());
    assert_eq!(19, game.turns_taken);
    game.new_turn(false);
    assert!(game.is_game_done());
    assert_eq!(20, game.turns_taken);
    assert!(!game.scored, "nothing recorded");
    assert_eq!(0, game.result_to_record());
    empty_turns(&mut game, 3);
    assert_eq!(20, game.turns_taken, "never past the cap");
    assert!(!game.scored);
}

#[test]
fn reaching_ten_on_the_last_turn_still_counts() {
    let mut game = Game::new(capped());
    score_eight(&mut game);
    empty_turns(&mut game, 19);
    give_dice(&mut game, &[Grain, Grain, Ore, Ore, Ore]);
    game.build_city(1);
    assert!(game.is_game_done());
    assert_eq!(20, game.turns_taken);
    assert_eq!(20, game.result_to_record());
    assert!(game.scored);
}

#[test]
fn a_new_game_starts_fresh() {
    let mut game = Game::new(dunholm());
    score_eight(&mut game);
    empty_turns(&mut game, 6);
    give_dice(&mut game, &[Grain, Grain, Ore, Ore, Ore]);
    game.build_city(1);
    assert!(game.is_game_done());
    game.reset();
    assert!(!game.is_game_done());
    assert!(!game.scored);
    assert_eq!(0, game.turns_taken);
    assert_eq!(0, game.playsheet.score());
}

#[test]
fn tarnholm_still_ends_at_fifteen_with_its_score() {
    let mut game = Game::new(tarnholm());
    game.playsheet.roads[1] = true;
    game.playsheet.roads[2] = true;
    game.playsheet.villages[0] = true;
    assert!(game.playsheet.score() > 0);
    // Turns in which something was built, so no empty turn is charged
    for _ in 0..14 {
        game.built_resource_this_turn = true;
        game.new_turn(false);
    }
    assert!(!game.is_game_done());
    assert!(!game.scored);
    game.built_resource_this_turn = true;
    game.new_turn(false);
    assert!(game.is_game_done());
    assert_eq!(15, game.turns_taken);
    assert!(game.scored, "the record path was taken");
    assert_eq!(game.playsheet.score(), game.result_to_record());
    assert!(game.result_to_record() > 0);
}

#[test]
fn tarnholm_never_ends_mid_turn_on_score() {
    let mut game = Game::new(tarnholm());
    for i in 0..game.playsheet.villages.len() {
        game.playsheet.villages[i] = true;
    }
    for i in 0..game.playsheet.cities.len() {
        game.playsheet.cities[i] = true;
    }
    assert!(game.playsheet.score() > 10);
    assert!(!game.is_game_done());
    give_dice(&mut game, &[Brick, Lumber]);
    game.playsheet.roads[1] = true;
    game.build_road(2);
    assert!(game.playsheet.roads[2]);
    assert!(!game.is_game_done());
    assert!(!game.scored);
    assert_eq!(0, game.turns_taken);
}

#[test]
fn a_tarnholm_game_with_no_points_still_counts_as_scored() {
    let mut game = Game::new(tarnholm());
    empty_turns(&mut game, 15);
    assert!(game.is_game_done());
    assert!(game.scored);
    assert_eq!(0, game.result_to_record());
}

#[test]
fn the_last_turn_comes_from_the_map() {
    // What the roll button tests to show "Finish": turns_taken + 1 == map.turn_cap
    assert_eq!(15, tarnholm().turn_cap);
    assert_eq!(20, capped().turn_cap);
    let mut game = Game::new(capped());
    empty_turns(&mut game, 14);
    assert!(game.turns_taken + 1 != game.map.turn_cap);
    empty_turns(&mut game, 5);
    assert!(game.turns_taken + 1 == game.map.turn_cap);

    // Dunholm has no cap, so no turn is ever its last
    let mut dun = Game::new(dunholm());
    for _ in 0..60 {
        assert!(dun.turns_taken + 1 != dun.map.turn_cap);
        dun.new_turn(false);
    }
}

#[test]
fn dunholm_never_runs_out_of_turns() {
    let mut game = Game::new(dunholm());
    score_eight(&mut game);
    empty_turns(&mut game, 60);
    assert!(!game.is_game_done());
    assert_eq!(60, game.turns_taken);
    assert!(!game.scored);
    assert_eq!(
        0,
        game.playsheet.turn_score_of(60),
        "every turn is scored, past the old cap too"
    );
    give_dice(&mut game, &[Grain, Grain, Ore, Ore, Ore]);
    game.build_city(1);
    assert!(game.is_game_done());
    assert_eq!(61, game.turns_taken);
    assert_eq!(61, game.result_to_record());
    assert_eq!(2, game.playsheet.turn_score_of(61));
}

/// The chain's trader step reads the sheet through is_trader_built.
#[test]
fn a_trader_chain_builds_each_trader_in_order_on_tarnholm() {
    let mut game = Game::new(tarnholm());
    give_dice(&mut game, &[Wool, Grain, Ore, Wool, Grain, Ore]);
    game.build(Kind::Knight, 2);
    assert!(game.playsheet.is_trader_built(1));
    assert!(game.playsheet.is_trader_built(2));
    assert!(!game.playsheet.is_trader_built(3));
}

#[test]
fn a_dunholm_trader_is_one_step_whichever_tier() {
    let mut game = Game::new(dunholm());
    give_dice(&mut game, &[Wool, Grain, Ore]);
    game.build(Kind::Knight, 9);
    assert!(game.playsheet.is_trader_built(9));
    assert!(!game.playsheet.is_trader_built(1));
    assert_eq!(1, game.playsheet.trader_count());
}
