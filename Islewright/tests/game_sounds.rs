// SPDX-License-Identifier: GPL-3.0-or-later
//! Ported from GameSoundsTest: which sounds each action asks for, and when.

mod common;

use common::*;
use dayapp::rules::{DiceValue, Kind};

use DiceValue::{Brick, Gold, Grain, Lumber, Ore, Wool};

#[test]
fn a_game_starts_in_silence() {
    let (_game, r) = recorded(tarnholm());
    assert_eq!("[]", r.take());
}

#[test]
fn rolling_sounds_the_rattle_and_holding_a_die_the_tick() {
    let (mut game, r) = recorded(tarnholm());
    game.dice_touched(0);
    assert_eq!(
        "[]",
        r.take(),
        "before the first roll a tap does nothing, silently"
    );
    game.roll();
    assert_eq!("[ROLL@0]", r.take());
    game.dice_touched(2);
    assert_eq!("[DIE@0]", r.take());
    game.dice_touched(2);
    assert_eq!("[DIE@0]", r.take(), "releasing sounds too");
}

#[test]
fn a_tap_out_of_range_sounds_nothing() {
    let (mut game, r) = recorded(tarnholm());
    game.roll();
    r.take();
    game.dice_touched(9);
    assert_eq!("[]", r.take());
}

#[test]
fn ending_the_turn_and_rolling_in_one_tap_sounds_both_apart() {
    let (mut game, r) = recorded(tarnholm());
    game.built_resource_this_turn = true;
    game.rolls = 3;
    game.end_turn_and_roll();
    assert_eq!("[TURN_END@0, ROLL@250]", r.take());
}

#[test]
fn each_kind_of_build_has_its_own_sound() {
    let (mut game, r) = recorded(tarnholm());
    game.playsheet.roads.fill(true);
    game.playsheet.roads[1] = false;

    give_dice(&mut game, &[Brick, Lumber]);
    game.build_road(1);
    assert_eq!("[ROAD@0]", r.take());

    give_dice(&mut game, &[Lumber, Brick, Wool, Grain]);
    game.build_village(0);
    assert_eq!("[VILLAGE@0]", r.take());

    give_dice(&mut game, &[Grain, Grain, Ore, Ore, Ore]);
    game.build_city(0);
    assert_eq!("[CASTLE@0]", r.take());

    give_dice(&mut game, &[Wool, Grain, Ore]);
    game.build_knight(1);
    assert_eq!("[TRADER@0]", r.take());
}

#[test]
fn a_build_that_cannot_be_done_is_silent() {
    let (mut game, r) = recorded(tarnholm());
    give_dice(&mut game, &[Brick]);
    game.build_road(1);
    game.build_city(0);
    assert_eq!("[]", r.take());
}

/// Dunholm's bank: slot 2 is grain, slot 3 wool. A village from lumber and brick on the dice
/// takes both.
#[test]
fn two_banked_resources_in_one_build_are_two_retriggered_ticks_not_one_stack() {
    let (mut game, r) = recorded(dunholm());
    game.playsheet.set_trader_count(3);
    game.playsheet.roads[17] = true;
    give_dice(&mut game, &[Lumber, Brick]);
    game.build_village(0);
    assert_eq!("[VILLAGE@0, BANKED@0, BANKED@110]", r.take());
    assert!(
        !(game.playsheet.can_use_knight_resource(2) || game.playsheet.can_use_knight_resource(3)),
        "both really were spent"
    );
}

#[test]
fn a_pair_of_gold_paying_for_something_is_one_coin_sound() {
    let (mut game, r) = recorded(tarnholm());
    give_dice(&mut game, &[Brick, Gold, Gold]);
    game.build_road(1);
    assert_eq!("[ROAD@0, GOLD@0]", r.take());
}

#[test]
fn gold_that_pays_nothing_is_silent() {
    let (mut game, r) = recorded(tarnholm());
    give_dice(&mut game, &[Brick, Lumber, Gold, Gold]);
    game.build_road(1);
    assert_eq!("[ROAD@0]", r.take());
}

#[test]
fn a_chain_build_sounds_each_step_ninety_milliseconds_after_the_last() {
    let (mut game, r) = recorded(dunholm());
    give_dice(&mut game, &[Brick, Lumber, Brick, Lumber, Brick, Lumber]);
    let chain = game.build_plan().chain_for(Kind::Road, 4).cloned().unwrap();
    assert_eq!(3, chain.steps.len(), "roads 1, 3 and 4");
    game.build(Kind::Road, 4);
    assert_eq!("[ROAD@0, ROAD@90, ROAD@180]", r.take());
    // The stagger does not leak into the next build
    give_dice(&mut game, &[Brick, Lumber]);
    game.build_road(5);
    assert_eq!("[ROAD@0]", r.take());
}

#[test]
fn a_single_step_build_through_the_chain_entry_is_not_staggered() {
    let (mut game, r) = recorded(dunholm());
    give_dice(&mut game, &[Brick, Lumber]);
    game.build(Kind::Road, 1);
    assert_eq!("[ROAD@0]", r.take());
}

/// The undo is one click; the build's own sounds are not repeated or reversed.
#[test]
fn an_undo_sounds_one_click() {
    let (mut game, r) = recorded(dunholm());
    give_dice(&mut game, &[Brick, Lumber, Brick, Lumber, Brick, Lumber]);
    game.undo();
    assert_eq!("[]", r.take(), "nothing to undo, silently");
    game.build(Kind::Road, 4);
    assert_eq!("[ROAD@0, ROAD@90, ROAD@180]", r.take());
    game.undo();
    assert_eq!("[CLICK@0]", r.take());
    game.undo();
    assert_eq!("[]", r.take(), "one level only");
}

#[test]
fn the_third_trader_on_the_second_island_earns_the_chime() {
    let (mut game, r) = recorded(dunholm());
    game.playsheet.set_trader_count(2);
    give_dice(&mut game, &[Wool, Grain, Ore]);
    game.build_knight(3);
    assert_eq!("[TRADER@0, BONUS@250]", r.take());
    // A fourth trader earns nothing more
    give_dice(&mut game, &[Wool, Grain, Ore]);
    game.build_knight(4);
    assert_eq!("[TRADER@0]", r.take());
}

#[test]
fn the_long_road_on_the_second_island_earns_the_chime() {
    let (mut game, r) = recorded(dunholm());
    game.playsheet.roads[4] = true;
    give_dice(&mut game, &[Brick, Lumber]);
    game.build_road(6);
    assert_eq!("[ROAD@0, BONUS@250]", r.take());
}

#[test]
fn the_first_island_has_no_bonus_to_chime() {
    let (mut game, r) = recorded(tarnholm());
    let long = dunholm().long_road.unwrap();
    game.playsheet.roads.fill(true);
    game.playsheet.roads[long] = false;
    give_dice(&mut game, &[Brick, Lumber]);
    game.build_road(long);
    assert_eq!("[ROAD@0]", r.take());
}

#[test]
fn an_empty_turn_on_the_first_island_is_rueful() {
    let (mut game, r) = recorded(tarnholm());
    game.new_turn(false);
    assert_eq!("[EMPTY_TURN@0]", r.take());
    game.built_resource_this_turn = true;
    game.new_turn(false);
    assert_eq!(
        "[TURN_END@0]",
        r.take(),
        "a turn with a build is a plain turn end"
    );
}

#[test]
fn an_empty_turn_on_the_second_island_costs_nothing_so_it_is_a_plain_turn_end() {
    let (mut game, r) = recorded(dunholm());
    game.new_turn(false);
    assert_eq!("[TURN_END@0]", r.take());
}

#[test]
fn the_ship_appears_on_the_turn_that_leaves_three_and_not_before() {
    let (mut game, r) = recorded(tarnholm());
    let cap = tarnholm().turn_cap;
    for turn in 1..=cap - 4 {
        game.built_resource_this_turn = true;
        game.new_turn(false);
        assert_eq!("[TURN_END@0]", r.take(), "turn {turn}");
    }
    game.built_resource_this_turn = true;
    game.new_turn(false);
    assert_eq!(
        "[TURN_END@0, SHIP_APPEARS@300]",
        r.take(),
        "three turns remain"
    );
    game.built_resource_this_turn = true;
    game.new_turn(false);
    assert_eq!("[TURN_END@0]", r.take(), "two remain: no second horn");
}

#[test]
fn the_bell_rings_on_the_turn_end_that_starts_the_last_turn() {
    let (mut game, r) = recorded(tarnholm());
    let cap = tarnholm().turn_cap;
    // Play until two turns remain; the next turn end leaves one
    for _ in 1..=cap - 2 {
        game.built_resource_this_turn = true;
        game.new_turn(false);
    }
    r.take();
    game.built_resource_this_turn = true;
    game.new_turn(false);
    assert!(!game.is_game_done());
    assert_eq!(
        "[TURN_END@0, SHIP_DOCKS@300]",
        r.take(),
        "one remains: the ship has moored"
    );
}

#[test]
fn the_last_turn_ends_with_the_short_fanfare_and_no_bell() {
    let (mut game, r) = recorded(tarnholm());
    let cap = tarnholm().turn_cap;
    for _ in 1..cap {
        game.built_resource_this_turn = true;
        game.new_turn(false);
    }
    r.take();
    game.built_resource_this_turn = true;
    game.new_turn(false);
    assert!(game.is_game_done());
    assert_eq!("[TURN_END@0, GAME_OVER@300]", r.take());
    // A finished game is silent when poked
    game.new_turn(false);
    assert_eq!("[]", r.take());
}

fn play_to_the_last_turn_end(rank: i32) -> String {
    let (mut game, r) = recorded(tarnholm());
    ranking(&mut game, rank);
    game.playsheet.villages[0] = true;
    for _ in 1..tarnholm().turn_cap {
        game.built_resource_this_turn = true;
        game.new_turn(false);
    }
    r.take();
    game.built_resource_this_turn = true;
    game.new_turn(false);
    r.take()
}

#[test]
fn a_new_best_plays_the_bigger_fanfare_instead_of_the_short_one() {
    assert_eq!("[TURN_END@0, TOP_SCORE@300]", play_to_the_last_turn_end(1));
}

#[test]
fn a_good_but_not_best_result_still_gets_the_short_fanfare() {
    assert_eq!("[TURN_END@0, GAME_OVER@300]", play_to_the_last_turn_end(3));
}

#[test]
fn a_race_won_by_a_build_is_the_build_sound_then_the_fanfare_and_no_dock() {
    let (mut game, r) = recorded(dunholm());
    score_eight(&mut game);
    empty_turns(&mut game, 6);
    r.take();
    give_dice(&mut game, &[Grain, Grain, Ore, Ore, Ore]);
    game.build_city(1);
    assert!(game.is_game_done());
    assert_eq!("[CASTLE@0, GAME_OVER@300]", r.take());
}

#[test]
fn a_race_won_with_a_new_best_is_the_big_fanfare() {
    let (mut game, r) = recorded(dunholm());
    ranking(&mut game, 1);
    score_eight(&mut game);
    empty_turns(&mut game, 6);
    r.take();
    give_dice(&mut game, &[Grain, Grain, Ore, Ore, Ore]);
    game.build_city(1);
    assert_eq!("[CASTLE@0, TOP_SCORE@300]", r.take());
}

#[test]
fn a_race_that_runs_out_of_turns_sounds_the_short_fanfare_with_no_bell() {
    let (mut game, r) = recorded(capped());
    for _ in 1..capped().turn_cap {
        game.new_turn(false);
    }
    r.take();
    game.new_turn(false);
    assert_eq!("[TURN_END@0, GAME_OVER@300]", r.take());
}

/// Dunholm has no cap: however long the race runs, a turn end is only a turn end, with no ship
/// or bell.
#[test]
fn a_race_with_no_cap_never_sounds_the_ship_or_the_bell() {
    let (mut game, r) = recorded(dunholm());
    for turn in 1..=40 {
        r.take();
        game.new_turn(false);
        assert_eq!("[TURN_END@0]", r.take(), "turn {turn}");
    }
}
