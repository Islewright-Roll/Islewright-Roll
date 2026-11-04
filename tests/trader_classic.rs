// SPDX-License-Identifier: GPL-3.0-or-later
//! Ported from TraderClassicTest: trader resource classic mode, where a banked resource is
//! spent only by dragging it onto a die.

mod common;

use common::*;
use dayapp::rules::game::{self, BuildStatus, RESOURCES_CITY, RESOURCES_ROAD};
use dayapp::rules::game_state_store;
use dayapp::rules::{DiceValue, Game, Kind, Playsheet};

use DiceValue::{Brick, Grain, Lumber, Ore, Wool};

/// Tarnholm with every trader hired and the whole bank open, after a first roll of these dice.
fn game_with(classic: bool, values: [DiceValue; 6]) -> Game {
    let mut g = Game::new(tarnholm());
    g.set_trader_classic(classic);
    g.playsheet.set_trader_count(6);
    g.rolls = 1;
    for (i, v) in values.into_iter().enumerate() {
        g.dice[i].value = v;
    }
    g
}

fn wools(classic: bool) -> Game {
    game_with(classic, [Wool; 6])
}

#[test]
fn with_the_mode_off_the_bank_still_pays_for_a_road() {
    assert_eq!(
        BuildStatus::WithKnight,
        wools(false).can_build_status(RESOURCES_ROAD)
    );
}

#[test]
fn in_classic_mode_the_bank_pays_for_nothing_by_itself() {
    let mut g = wools(true);
    assert_eq!(BuildStatus::None, g.can_build_status(RESOURCES_ROAD));
    assert!(!g.can_build_road(1));
    // The plan's copies of the sheet carry the rule, so it offers nothing from the bank either
    assert!(!g.playsheet.clone().bank_pays_builds);
    assert!(g.build_plan().chain_for(Kind::Road, 1).is_none());
    // And building anyway spends nothing from the bank
    g.build_road(1);
    assert!(!g.playsheet.roads[1]);
    for slot in 1..=6 {
        assert!(g.playsheet.can_use_knight_resource(slot));
    }
}

#[test]
fn turning_the_mode_off_lets_the_bank_pay_again() {
    let mut g = wools(true);
    g.set_trader_classic(false);
    assert_eq!(BuildStatus::WithKnight, g.can_build_status(RESOURCES_ROAD));
}

#[test]
fn overwriting_a_die_spends_the_slot_and_locks_the_die() {
    let mut g = wools(true);
    g.overwrite_die(0, 5, Brick);
    assert_eq!(Brick, g.dice[0].value);
    assert!(g.dice[0].held);
    assert!(g.dice[0].locked);
    assert!(!g.playsheet.can_use_knight_resource(5));
    // A tap does not let it go, and a roll leaves it as it is
    g.dice_touched(0);
    assert!(g.dice[0].is_held());
    g.roll();
    assert_eq!(Brick, g.dice[0].value);
    // A locked die is not a target a second time
    assert!(!g.can_overwrite_die(0, 4));
}

#[test]
fn the_die_then_pays_for_a_build_like_any_other() {
    let mut g = wools(true);
    g.overwrite_die(0, 5, Brick);
    g.overwrite_die(1, 4, Lumber);
    assert!(g.can_build_road(1));
    g.build_road(1);
    assert!(g.playsheet.roads[1]);
    assert!(!g.dice[0].is_usable());
    assert!(!g.dice[1].is_usable());
}

#[test]
fn undo_puts_the_die_and_the_slot_back() {
    let mut g = wools(true);
    g.overwrite_die(2, 1, Ore);
    assert!(g.can_undo());
    g.undo();
    assert_eq!(Wool, g.dice[2].value);
    assert!(!g.dice[2].held);
    assert!(!g.dice[2].locked);
    assert!(g.playsheet.can_use_knight_resource(1));
    // Still in classic mode after the undo
    assert_eq!(BuildStatus::None, g.can_build_status(RESOURCES_ROAD));
}

#[test]
fn a_question_mark_becomes_the_resource_chosen_and_only_a_real_one() {
    let mut g = wools(true);
    g.overwrite_die(0, 6, DiceValue::Any);
    g.overwrite_die(0, 6, DiceValue::Gold);
    g.overwrite_die(0, 6, DiceValue::None);
    assert!(g.playsheet.can_use_knight_resource(6));
    assert_eq!(Wool, g.dice[0].value);
    assert!(g.overwrite_die(0, 6, Grain));
    assert_eq!(Grain, g.dice[0].value);
    assert!(!g.playsheet.can_use_knight_resource(6));
}

#[test]
fn a_slot_only_ever_becomes_its_own_resource() {
    let mut g = wools(true);
    assert!(!g.overwrite_die(0, 1, Grain));
    assert!(g.playsheet.can_use_knight_resource(1));
    assert_eq!(Wool, g.dice[0].value);
}

#[test]
fn nothing_is_overwritten_outside_classic_mode_before_a_roll_or_onto_a_spent_die() {
    let mut usual = wools(false);
    usual.overwrite_die(0, 1, Ore);
    assert_eq!(Wool, usual.dice[0].value);
    assert!(usual.playsheet.can_use_knight_resource(1));

    let mut unrolled = wools(true);
    unrolled.rolls = 0;
    assert!(!unrolled.can_overwrite_die(0, 1));

    let mut spent = wools(true);
    spent.dice[3].use_die();
    assert!(!spent.can_overwrite_die(3, 1));
    // Nor from a slot whose trader is not hired
    spent.playsheet.set_trader_count(2);
    assert!(!spent.can_overwrite_die(0, 3));
}

/// A slot spent by a drag is not handed back when the turn ends.
#[test]
fn the_turn_end_does_not_return_a_slot_a_drag_spent() {
    let mut g = wools(false);
    // In the usual mode the list fills with what the bank offers
    g.can_build_status(RESOURCES_ROAD);
    assert!(!g.knight_resources().is_empty());
    g.set_trader_classic(true);
    assert!(g.knight_resources().is_empty());
    g.overwrite_die(0, 1, Ore);
    g.can_build_status(RESOURCES_CITY);
    assert!(g.knight_resources().is_empty());
    g.new_turn(false);
    assert!(!g.playsheet.can_use_knight_resource(1));
    assert!(!g.dice[0].locked);
    assert!(!g.dice[0].held);
    // And the next turn the other slots are all still there, each spent once
    for slot in 2..=6 {
        assert!(g.playsheet.can_use_knight_resource(slot));
    }
}

#[test]
fn a_locked_die_is_saved() {
    let mut g = wools(true);
    g.overwrite_die(4, 2, Grain);
    let back = game_state_store::from_json(&game_state_store::to_json(&g)).unwrap();
    assert!(back.dice[4].locked);
    assert!(back.dice[4].held);
    assert!(!back.dice[3].locked);
}

#[test]
fn the_static_engine_honours_the_sheets_rule() {
    let mut s = Playsheet::new(tarnholm());
    s.set_trader_count(6);
    s.bank_pays_builds = false;
    let mut d = dice(&[]);
    assert_eq!(
        BuildStatus::None,
        game::can_build_status(RESOURCES_ROAD, &d, &s, &mut Vec::new())
    );
    let spent = game::use_resources(RESOURCES_ROAD, &mut d, &mut s, &mut Vec::new());
    assert_eq!(0, spent.banked);
}
