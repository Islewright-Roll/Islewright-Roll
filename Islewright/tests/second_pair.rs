// SPDX-License-Identifier: GPL-3.0-or-later
//! Ported from SecondPairTest: when hiring a trader starts a second team while another is
//! half hired.

mod common;

use common::*;
use dayapp::rules::{MapDef, Playsheet};

fn hired(map: &MapDef, tiers: &[usize]) -> Vec<bool> {
    let mut h = vec![false; map.traders.len() + 1];
    for &t in tiers {
        h[t] = true;
    }
    h
}

fn asks(map: &MapDef, tier: usize, already: &[usize]) -> bool {
    Playsheet::starts_second_pair(map, &hired(map, already), tier)
}

#[test]
fn tarnholm_never_asks() {
    for tier in 1..=6 {
        assert!(!asks(tarnholm(), tier, &[]));
        assert!(!asks(tarnholm(), tier, &[1, 2, 3, 4, 5, 6]));
    }
}

#[test]
fn the_first_trader_of_the_first_pair_does_not_ask() {
    assert!(!asks(dunholm(), 6, &[]));
    assert!(!asks(dunholm(), 7, &[]));
    assert!(!asks(dunholm(), 8, &[1, 2, 3]));
}

#[test]
fn the_second_trader_of_the_same_pair_does_not_ask() {
    assert!(!asks(dunholm(), 7, &[6]));
    assert!(!asks(dunholm(), 6, &[7]));
    assert!(!asks(dunholm(), 9, &[8]));
}

#[test]
fn the_first_trader_of_the_other_pair_asks_while_the_first_pair_is_half_done() {
    assert!(asks(dunholm(), 8, &[6]));
    assert!(asks(dunholm(), 9, &[6]));
    assert!(asks(dunholm(), 8, &[7]));
    assert!(asks(dunholm(), 6, &[8]));
    assert!(asks(dunholm(), 7, &[9]));
    // single traders hired besides change nothing
    assert!(asks(dunholm(), 9, &[1, 2, 6]));
}

#[test]
fn it_does_not_ask_once_the_first_pair_is_complete() {
    assert!(!asks(dunholm(), 8, &[6, 7]));
    assert!(!asks(dunholm(), 9, &[6, 7]));
    assert!(!asks(dunholm(), 6, &[8, 9]));
}

#[test]
fn it_does_not_ask_for_the_second_trader_of_the_other_pair_either() {
    // both pairs half done: finishing either one is no new pair
    assert!(!asks(dunholm(), 7, &[6, 8]));
    assert!(!asks(dunholm(), 9, &[6, 8]));
}

#[test]
fn a_plain_single_trader_never_asks_even_with_a_pair_half_done() {
    for tier in 1..=5 {
        assert!(!asks(dunholm(), tier, &[6]));
        assert!(!asks(dunholm(), tier, &[8]));
    }
}

#[test]
fn a_trader_already_hired_and_a_nonsense_tier_never_ask() {
    assert!(!asks(dunholm(), 6, &[6, 8]));
    assert!(!asks(dunholm(), 0, &[6]));
    assert!(!asks(dunholm(), 10, &[6]));
}

#[test]
fn a_chain_asks_when_any_step_would() {
    let d = dunholm();
    assert!(Playsheet::chain_starts_second_pair(
        d,
        &hired(d, &[6]),
        &[1, 8]
    ));
    assert!(Playsheet::chain_starts_second_pair(
        d,
        &hired(d, &[6]),
        &[8]
    ));
    assert!(!Playsheet::chain_starts_second_pair(
        d,
        &hired(d, &[6]),
        &[7, 8]
    ));
    assert!(!Playsheet::chain_starts_second_pair(
        d,
        &hired(d, &[]),
        &[1, 2]
    ));
    assert!(!Playsheet::chain_starts_second_pair(d, &hired(d, &[]), &[]));
}

#[test]
fn a_chain_that_itself_leaves_a_pair_half_done_before_starting_another_asks() {
    let d = dunholm();
    // 6 then 8 with nothing hired: the first step starts the first pair (fine), the second starts another
    assert!(Playsheet::chain_starts_second_pair(
        d,
        &hired(d, &[]),
        &[6, 8]
    ));
    // 6, 7 then 8: the first pair is whole by then
    assert!(!Playsheet::chain_starts_second_pair(
        d,
        &hired(d, &[]),
        &[6, 7, 8]
    ));
}

#[test]
fn the_rule_does_not_change_what_it_is_given() {
    let d = dunholm();
    let h = hired(d, &[6]);
    Playsheet::chain_starts_second_pair(d, &h, &[8, 9]);
    assert!(!(h[8] || h[9]));
}

#[test]
fn the_playsheet_asks_the_same_about_its_own_traders() {
    let mut s = Playsheet::new(dunholm());
    assert!(!s.traders[6]);
    s.traders[6] = true;
    assert!(Playsheet::starts_second_pair(s.map, &s.traders, 8));
}
