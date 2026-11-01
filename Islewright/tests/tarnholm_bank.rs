// SPDX-License-Identifier: GPL-3.0-or-later
//! Ported from TarnholmBankTest: paying from Tarnholm's bank. The old test also checked each
//! case against a frozen copy of the pre-map rules (LegacyRules); those comparisons are left
//! out with that copy, and the assertions about current behaviour are kept.

mod common;

use common::*;
use dayapp::rules::game::{self, BuildStatus, RESOURCES_CITY, RESOURCES_ROAD, RESOURCES_VILLAGE};
use dayapp::rules::{Board, BuildPlan, DiceValue, Game, Kind, Playsheet};

use DiceValue::{Brick, Grain, Lumber, Wool};

fn sheet(traders: usize, spent: &[usize]) -> Playsheet {
    let mut s = Playsheet::new(tarnholm());
    s.set_trader_count(traders);
    for &slot in spent {
        s.resources_avail[slot] = false;
    }
    s
}

/// Every trader built, a full bank, no dice: a road comes wholly from the bank.
#[test]
fn road_from_a_full_bank() {
    assert_eq!(
        BuildStatus::WithKnight,
        game::can_build_status(RESOURCES_ROAD, &dice(&[]), &sheet(6, &[]), &mut Vec::new())
    );
}

/// The same bank cannot pay for a castle: it holds one grain, one ore and one "?", which is
/// three of the five resources needed.
#[test]
fn castle_is_beyond_a_full_bank() {
    assert_eq!(
        BuildStatus::None,
        game::can_build_status(RESOURCES_CITY, &dice(&[]), &sheet(6, &[]), &mut Vec::new())
    );
}

/// Brick and lumber on the dice: a road from dice alone, bank untouched.
#[test]
fn road_from_dice() {
    let d = dice(&[Brick, Lumber]);
    assert_eq!(
        BuildStatus::WithDice,
        game::can_build_status(RESOURCES_ROAD, &d, &sheet(0, &[]), &mut Vec::new())
    );
    assert_eq!(
        BuildStatus::WithDice,
        game::can_build_status(RESOURCES_ROAD, &d, &sheet(6, &[]), &mut Vec::new())
    );
}

/// A village with brick, lumber and wool on the dice and traders 1 and 2 built: the missing
/// grain comes from slot 2, and nothing else in the bank moves.
#[test]
fn village_takes_its_grain_from_slot_two() {
    let mut d = dice(&[Brick, Lumber, Wool]);
    let mut s = sheet(2, &[]);
    game::use_resources(RESOURCES_VILLAGE, &mut d, &mut s, &mut Vec::new());
    assert!(!s.resources_avail[2], "slot 2 spent");
    assert!(s.is_knight_resource_used(2), "slot 2 reads as used");
    for slot in [1, 3, 4, 5, 6] {
        assert!(s.resources_avail[slot], "slot {slot} untouched");
    }
    assert!(d[0].used && d[1].used && d[2].used);
    assert!(!(d[3].used || d[4].used || d[5].used));
}

/// The same village when the only thing left in the bank is the "?": it stands in for the
/// grain, slot 6 is spent, and the build plan says the chain spends the wildcard.
#[test]
fn village_takes_the_wildcard_when_nothing_else_fits() {
    let mut d = dice(&[Brick, Lumber, Wool]);
    let mut s = sheet(6, &[1, 2, 3, 4, 5]);

    let chain = BuildPlan::compute(&d, &s, 1)
        .chain_for(Kind::Village, 0)
        .cloned()
        .expect("village 0 is on offer");
    assert_eq!(1, chain.steps.len());
    assert!(chain.spends_bank);
    assert!(chain.spends_wildcard);
    assert_eq!(3, chain.dice_used);
    assert!(s.resources_avail[6], "planning left the live bank alone");

    assert!(s.has_wildcard());
    game::use_resources(RESOURCES_VILLAGE, &mut d, &mut s, &mut Vec::new());
    assert!(!s.resources_avail[6], "slot 6 spent");
    assert!(!s.has_wildcard());
    assert_eq!(0, s.first_available_slot(DiceValue::Any));
}

/// A grain in the bank is preferred to the "?": the exact match goes first.
#[test]
fn exact_match_is_spent_before_the_wildcard() {
    let mut d = dice(&[Brick, Lumber, Wool]);
    let mut s = sheet(6, &[]);
    let chain = BuildPlan::compute(&d, &s, 1)
        .chain_for(Kind::Village, 0)
        .cloned()
        .unwrap();
    assert!(chain.spends_bank);
    assert!(!chain.spends_wildcard);
    game::use_resources(RESOURCES_VILLAGE, &mut d, &mut s, &mut Vec::new());
    assert!(!s.resources_avail[2]);
    assert!(s.resources_avail[6]);
}

/// A game with no recorder can run to its end: the top score call is skipped, not fatal.
#[test]
fn a_game_with_no_recorder_ends_after_fifteen_turns() {
    let mut g = Game::new(tarnholm());
    assert!(same(tarnholm(), g.map));
    assert!(std::ptr::eq(Board::for_map(tarnholm()), g.board));
    g.playsheet.build_road(1);
    for turn in 1..=15 {
        assert!(!g.is_game_done());
        g.new_turn(false);
        assert_eq!(turn, g.turns_taken);
    }
    assert!(g.is_game_done());
    // One road, less two for each of fifteen empty turns
    assert_eq!(1 - 30, g.playsheet.score());
    g.new_turn(false);
    assert_eq!(15, g.turns_taken);

    g.start_new_game(tarnholm());
    assert!(!g.is_game_done());
    assert_eq!(0, g.turns_taken);
    assert_eq!(0, g.playsheet.score());
}

/// From TarnholmEquivalenceTest: ending a turn hands back whatever the turn had set aside from
/// the bank. In play the list only ever names slots that are still available, so nothing
/// moves; a restored game can say otherwise, and then the slot reopens.
#[test]
fn ending_a_turn_returns_what_was_set_aside() {
    let mut g = Game::new(tarnholm());
    g.playsheet.set_trader_count(2);
    for slot in [2, 4, 6] {
        g.playsheet.resources_avail[slot] = false;
    }
    g.knight_resources_mut().extend([
        Grain,
        DiceValue::Any,
        DiceValue::None,
        DiceValue::Gold,
        DiceValue::Ore,
    ]);
    g.new_turn(false);
    assert!(g.playsheet.resources_avail[2]);
    assert!(g.playsheet.resources_avail[6]);
    assert!(!g.playsheet.resources_avail[4]);
    assert!(g.knight_resources().is_empty());
}
