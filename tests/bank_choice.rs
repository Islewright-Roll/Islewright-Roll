// SPDX-License-Identifier: GPL-3.0-or-later
//! When gold could pay for some of the banked resources a build would spend, which ones are
//! spent is the player's choice: bank_options lists the choices and use_resources_spending
//! honours the one taken. A "?" never enters into it, since a resource that matches is always
//! spent before one.

mod common;

use common::*;
use dayapp::rules::game::{self, RESOURCES_CITY, RESOURCES_ROAD};
use dayapp::rules::{DiceValue, Playsheet};

use DiceValue::{Gold, Grain, Lumber, Ore};

/// Dunholm with these traders hired: trader 1 banks ore, 2 grain, 6 and 7 together a "?".
fn sheet(traders: &[usize]) -> Playsheet {
    sheet_with(dunholm(), traders)
}

#[test]
fn a_castle_with_grain_and_ore_banked_and_a_gold_pair_asks_which_to_spend() {
    // Castle: Grain, Grain, Ore, Ore, Ore. The dice pay a grain and two ore; one grain and
    // one ore are short. The gold pair pays one, the bank the other.
    let s = sheet(&[1, 2]);
    let d = dice(&[Grain, Ore, Ore, Gold, Gold]);
    assert_eq!(
        vec![vec![Grain], vec![Ore]],
        game::bank_options(RESOURCES_CITY, &d, &s)
    );
}

#[test]
fn the_chosen_resource_is_spent_and_the_other_kept() {
    for (spend, kept) in [(Grain, 1), (Ore, 2)] {
        let mut s = sheet(&[1, 2]);
        let mut d = dice(&[Grain, Ore, Ore, Gold, Gold]);
        let spent = game::use_resources_spending(
            RESOURCES_CITY,
            &mut d,
            &mut s,
            &mut Vec::new(),
            Some(&[spend]),
        );
        assert_eq!(1, spent.banked, "spending {spend:?}");
        assert_eq!(2, spent.gold_dice);
        assert!(
            s.resources_avail[kept],
            "spending {spend:?} keeps slot {kept}"
        );
        assert_eq!(1, usable(&d), "only the blank sixth die is left");
    }
}

#[test]
fn nothing_to_ask_without_spare_gold() {
    // Both short resources come from the bank: there is no choice to make
    let s = sheet(&[1, 2]);
    let d = dice(&[Grain, Ore, Ore]);
    assert!(game::bank_options(RESOURCES_CITY, &d, &s).is_empty());
}

#[test]
fn an_odd_gold_still_makes_one_pair() {
    let s = sheet(&[1, 2]);
    let d = dice(&[Grain, Ore, Ore, Gold, Gold, Gold]);
    // Three gold make one pair; two short resources: the bank still pays one, so ask
    assert_eq!(2, game::bank_options(RESOURCES_CITY, &d, &s).len());
    let d = dice(&[Grain, Ore, Gold, Gold, Gold, Gold]);
    // Two pairs, three short (grain, ore, ore): one banked must pay, either grain or ore
    assert_eq!(2, game::bank_options(RESOURCES_CITY, &d, &s).len());
}

#[test]
fn a_wildcard_against_a_matching_resource_is_never_a_question() {
    // Road: Brick, Lumber. Lumber is on a die, brick is short: banked brick and a "?" could
    // both pay, and brick always does
    let mut s = sheet(&[5, 6, 7]);
    let mut d = dice(&[Lumber]);
    assert!(game::bank_options(RESOURCES_ROAD, &d, &s).is_empty());
    game::use_resources(RESOURCES_ROAD, &mut d, &mut s, &mut Vec::new());
    assert!(!s.resources_avail[5], "the brick slot");
    assert!(s.resources_avail[6], "the ? kept");
}

#[test]
fn the_same_resource_twice_is_one_choice() {
    // Grain banked twice would be two identical choices: nothing to ask. Dunholm banks one
    // of each, so a lone grain with a spare pair asks nothing either.
    let s = sheet(&[2]);
    let d = dice(&[Grain, Ore, Ore, Ore, Gold, Gold]);
    assert!(game::bank_options(RESOURCES_CITY, &d, &s).is_empty());
}
